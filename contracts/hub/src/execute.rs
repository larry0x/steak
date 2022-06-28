use std::str::FromStr;

use cosmwasm_std::{
    coins, to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, DistributionMsg, Env,
    Event, MessageInfo, Order, Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use osmo_bindings::OsmosisMsg;

use steak::hub::{Batch, CallbackMsg, ExecuteMsg, InstantiateMsg, PendingBatch, UnbondRequest};

use crate::error::ContractError;
use crate::helpers::{query_delegation, query_delegations};
use crate::math::{
    compute_mint_amount, compute_redelegations_for_rebalancing, compute_redelegations_for_removal,
    compute_unbond_amount, compute_undelegations, reconcile_batches,
};
use crate::state::State;
use crate::types::{Coins, Delegation};

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    msg: InstantiateMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();

    state
        .owner
        .save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;
    state.validators.save(deps.storage, &msg.validators)?;
    state.unlocked_coins.save(deps.storage, &vec![])?;
    state
        .total_usteak_supply
        .save(deps.storage, &Uint128::zero())?;

    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: env.block.time.seconds() + msg.epoch_period,
        },
    )?;

    state.steak_denom.save(
        deps.storage,
        &format!("factory/{}/{}", &env.contract.address, &msg.name),
    )?;

    state.distribution_contract.save(
        deps.storage,
        &deps.api.addr_validate(&msg.distribution_contract)?,
    )?;

    state
        .performance_fee
        .save(deps.storage, &Decimal::percent(msg.performance_fee))?;

    Ok(Response::new().add_message(OsmosisMsg::CreateDenom { subdenom: msg.name }))
}

//--------------------------------------------------------------------------------------------------
// Bonding and harvesting logics
//--------------------------------------------------------------------------------------------------

/// NOTE: In a previous implementation, we split up the deposited Osmo over all validators, so that
/// they all have the same amount of delegation. This is however quite gas-expensive: $1.5 cost in
/// the case of 15 validators.
///
/// To save gas for users, now we simply delegate all deposited Osmo to the validator with the
/// smallest amount of delegation. If delegations become severely unbalance as a result of this
/// (e.g. when a single user makes a very big deposit), anyone can invoke `ExecuteMsg::Rebalance`
/// to balance the delegations.
pub fn bond(
    deps: DepsMut,
    env: Env,
    receiver: Addr,
    uosmo_to_bond: Uint128,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let steak_denom = state.steak_denom.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;

    // Query the current delegations made to validators, and find the validator with the smallest
    // delegated amount through a linear search
    // The code for linear search is a bit uglier than using `sort_by` but cheaper: O(n) vs O(n * log(n))
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let mut validator = &delegations[0].validator;
    let mut amount = delegations[0].amount;
    for d in &delegations[1..] {
        if d.amount < amount {
            validator = &d.validator;
            amount = d.amount;
        }
    }

    // Query the current supply of Steak and compute the amount to mint
    let usteak_supply = state.total_usteak_supply.load(deps.storage)?;
    let usteak_to_mint = compute_mint_amount(usteak_supply, uosmo_to_bond, &delegations);

    state
        .total_usteak_supply
        .update(deps.storage, |x| -> StdResult<_> {
            Ok(x.checked_add(usteak_to_mint)?)
        })?;

    let new_delegation = Delegation {
        validator: validator.clone(),
        amount: uosmo_to_bond.u128(),
    };

    let delegate_submsg = SubMsg::reply_on_success(new_delegation.to_cosmos_msg(), 1);

    let mint_msg =
        OsmosisMsg::mint_contract_tokens(steak_denom, usteak_to_mint, receiver.to_string());

    let event = Event::new("steakhub/bonded")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("receiver", receiver)
        .add_attribute("uosmo_bonded", uosmo_to_bond)
        .add_attribute("usteak_minted", usteak_to_mint);

    Ok(Response::new()
        .add_submessage(delegate_submsg)
        .add_message(mint_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/bond"))
}

pub fn harvest(deps: DepsMut, env: Env) -> Result<Response<OsmosisMsg>, ContractError> {
    let withdraw_submsgs = deps
        .querier
        .query_all_delegations(&env.contract.address)?
        .into_iter()
        .map(|d| {
            return SubMsg::reply_on_success(
                CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                    validator: d.validator,
                }),
                1,
            );
        })
        .collect::<Vec<SubMsg<OsmosisMsg>>>();

    let callback_msg = CallbackMsg::Reinvest {}.into_cosmos_msg(&env.contract.address)?;

    Ok(Response::new()
        .add_submessages(withdraw_submsgs)
        .add_message(callback_msg)
        .add_attribute("action", "steakhub/harvest"))
}

/// NOTE:
/// 1. When delegation Osmo here, we don't need to use a `SubMsg` to handle the received coins,
/// because we have already withdrawn all claimable staking rewards previously in the same atomic
/// execution.
/// 2. Same as with `bond`, in the latest implementation we only delegate staking rewards with the
/// validator that has the smallest delegation amount.
pub fn reinvest(deps: DepsMut, env: Env) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;
    let mut unlocked_coins = state.unlocked_coins.load(deps.storage)?;

    let total_uosmo_harvest = unlocked_coins
        .iter()
        .find(|coin| coin.denom == "uosmo")
        .ok_or_else(|| StdError::generic_err("no uosmo available to be bonded"))?
        .amount;

    let performance_fee = state.performance_fee.load(deps.storage)?;
    let uosmo_to_bond = total_uosmo_harvest * (Decimal::one() - performance_fee);
    let uosmo_to_send_to_delegation_contract = total_uosmo_harvest - uosmo_to_bond;
    let distribution_contract = state.distribution_contract.load(deps.storage)?;

    let harvest: CosmosMsg<OsmosisMsg> = CosmosMsg::Bank(BankMsg::Send {
        to_address: distribution_contract.to_string(),
        amount: coins(uosmo_to_send_to_delegation_contract.u128(), "uosmo"),
    });

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let mut validator = &delegations[0].validator;
    let mut amount = delegations[0].amount;
    for d in &delegations[1..] {
        if d.amount < amount {
            validator = &d.validator;
            amount = d.amount;
        }
    }

    let new_delegation = Delegation::new(validator, uosmo_to_bond.u128());

    unlocked_coins.retain(|coin| coin.denom != "uosmo");
    state.unlocked_coins.save(deps.storage, &unlocked_coins)?;

    let event = Event::new("steakhub/harvested")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("uosmo_bonded", uosmo_to_bond);

    Ok(Response::new()
        .add_message(new_delegation.to_cosmos_msg())
        .add_message(harvest)
        .add_event(event)
        .add_attribute("action", "steakhub/reinvest"))
}

/// NOTE: a `SubMsgResponse` may contain multiple coin-receiving events, must handle them individually
pub fn register_received_coins(
    deps: DepsMut,
    env: Env,
    mut events: Vec<Event>,
) -> Result<Response<OsmosisMsg>, ContractError> {
    events.retain(|event| event.ty == "coin_received");
    if events.is_empty() {
        return Ok(Response::new());
    }

    let mut received_coins = Coins(vec![]);
    for event in &events {
        received_coins.add_many(&parse_coin_receiving_event(&env, event)?)?;
    }

    let state = State::default();
    state
        .unlocked_coins
        .update(deps.storage, |coins| -> StdResult<_> {
            let mut coins = Coins(coins);
            coins.add_many(&received_coins)?;
            Ok(coins.0)
        })?;

    Ok(Response::new().add_attribute("action", "steakhub/register_received_coins"))
}

fn parse_coin_receiving_event(env: &Env, event: &Event) -> StdResult<Coins> {
    let receiver = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "receiver")
        .ok_or_else(|| StdError::generic_err("cannot find `receiver` attribute"))?
        .value;

    let amount_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "amount")
        .ok_or_else(|| StdError::generic_err("cannot find `amount` attribute"))?
        .value;

    let amount = if *receiver == env.contract.address {
        Coins::from_str(amount_str)?
    } else {
        Coins(vec![])
    };

    Ok(amount)
}

//--------------------------------------------------------------------------------------------------
// Unbonding logics
//--------------------------------------------------------------------------------------------------

pub fn queue_unbond(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    receiver: Addr,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let steak_denom = state.steak_denom.load(deps.storage)?;

    if info.funds.is_empty() {
        return Err(ContractError::NoCoinsSent {});
    }

    if info.funds[0].denom != steak_denom {
        return Err(ContractError::InvalidCoinSent {});
    }

    let usteak_to_burn = info.funds[0].amount;

    let mut pending_batch = state.pending_batch.load(deps.storage)?;
    pending_batch.usteak_to_burn += usteak_to_burn;
    state.pending_batch.save(deps.storage, &pending_batch)?;

    state.unbond_requests.update(
        deps.storage,
        (pending_batch.id, &receiver),
        |x| -> StdResult<_> {
            let mut request = x.unwrap_or_else(|| UnbondRequest {
                id: pending_batch.id,
                user: receiver.clone(),
                shares: Uint128::zero(),
            });
            request.shares += usteak_to_burn;
            Ok(request)
        },
    )?;

    let mut msgs: Vec<CosmosMsg<OsmosisMsg>> = vec![];
    if env.block.time.seconds() >= pending_batch.est_unbond_start_time {
        msgs.push(
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.into(),
                msg: to_binary(&ExecuteMsg::SubmitBatch {})?,
                funds: vec![],
            })
            .into(),
        );
    }

    let event = Event::new("steakhub/unbond_queued")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("receiver", receiver)
        .add_attribute("usteak_to_burn", usteak_to_burn);

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(event)
        .add_attribute("action", "steakhub/queue_unbond"))
}

pub fn submit_batch(deps: DepsMut, env: Env) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let steak_denom = state.steak_denom.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;
    let unbond_period = state.unbond_period.load(deps.storage)?;
    let pending_batch = state.pending_batch.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    if current_time < pending_batch.est_unbond_start_time {
        return Err(ContractError::InvalidSubmitBatch {
            est_unbond_start_time: pending_batch.est_unbond_start_time,
        });
    }

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = state.total_usteak_supply.load(deps.storage)?;

    let uosmo_to_unbond =
        compute_unbond_amount(usteak_supply, pending_batch.usteak_to_burn, &delegations);
    let new_undelegations = compute_undelegations(uosmo_to_unbond, &delegations);

    // NOTE: Regarding the `uosmo_unclaimed` value
    //
    // If validators misbehave and get slashed during the unbonding period, the contract can receive
    // LESS Osmo than `uosmo_to_unbond` when unbonding finishes!
    //
    // In this case, users who invokes `withdraw_unbonded` will have their txs failed as the contract
    // does not have enough Osmo balance.
    //
    // I don't have a solution for this... other than to manually fund contract with the slashed amount.
    state.previous_batches.save(
        deps.storage,
        pending_batch.id,
        &Batch {
            id: pending_batch.id,
            reconciled: false,
            total_shares: pending_batch.usteak_to_burn,
            uosmo_unclaimed: uosmo_to_unbond,
            est_unbond_end_time: current_time + unbond_period,
        },
    )?;

    let epoch_period = state.epoch_period.load(deps.storage)?;
    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: pending_batch.id + 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: current_time + epoch_period,
        },
    )?;

    let undelegate_submsgs = new_undelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 1))
        .collect::<Vec<_>>();

    let burn_msg = OsmosisMsg::burn_contract_tokens(
        steak_denom,
        pending_batch.usteak_to_burn,
        env.contract.address.to_string(),
    );

    state
        .total_usteak_supply
        .update(deps.storage, |x| -> StdResult<_> {
            Ok(x.checked_sub(pending_batch.usteak_to_burn)?)
        })?;

    let event = Event::new("steakhub/unbond_submitted")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("uosmo_unbonded", uosmo_to_unbond)
        .add_attribute("usteak_burned", pending_batch.usteak_to_burn);

    Ok(Response::new()
        .add_submessages(undelegate_submsgs)
        .add_message(burn_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/unbond"))
}

pub fn reconcile(deps: DepsMut, env: Env) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let current_time = env.block.time.seconds();

    // Load batches that have not been reconciled
    let all_batches = state
        .previous_batches
        .idx
        .reconciled
        .prefix(false.into())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<_>>>()?;

    let mut batches = all_batches
        .into_iter()
        .filter(|b| current_time > b.est_unbond_end_time)
        .collect::<Vec<_>>();

    let uosmo_expected_received: Uint128 = batches.iter().map(|b| b.uosmo_unclaimed).sum();

    let unlocked_coins = state.unlocked_coins.load(deps.storage)?;
    let uosmo_expected_unlocked = Coins(unlocked_coins).find("uosmo").amount;

    let uosmo_expected = uosmo_expected_received + uosmo_expected_unlocked;
    let uosmo_actual = deps
        .querier
        .query_balance(&env.contract.address, "uosmo")?
        .amount;

    let uosmo_to_deduct = uosmo_expected
        .checked_sub(uosmo_actual)
        .unwrap_or_else(|_| Uint128::zero());
    if !uosmo_to_deduct.is_zero() {
        reconcile_batches(&mut batches, uosmo_expected - uosmo_actual);
    }

    for batch in &batches {
        state.previous_batches.save(deps.storage, batch.id, batch)?;
    }

    let ids = batches
        .iter()
        .map(|b| b.id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let event = Event::new("steakhub/reconciled")
        .add_attribute("ids", ids)
        .add_attribute("uosmo_deducted", uosmo_to_deduct.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/reconcile"))
}

pub fn withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    user: Addr,
    receiver: Addr,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let current_time = env.block.time.seconds();

    // NOTE: If the user has too many unclaimed requests, this may not fit in the WASM memory...
    // However, this is practically never going to happen. Who would create hundreds of unbonding
    // requests and never claim them?
    let requests = state
        .unbond_requests
        .idx
        .user
        .prefix(user.to_string())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<_>>>()?;

    // NOTE: Osmo in the following batches are withdrawn it the batch:
    // - is a _previous_ batch, not a _pending_ batch
    // - is reconciled
    // - has finished unbonding
    // If not sure whether the batches have been reconciled, the user should first invoke `ExecuteMsg::Reconcile`
    // before withdrawing.
    let mut total_uosmo_to_refund = Uint128::zero();
    let mut ids: Vec<String> = vec![];
    for request in &requests {
        if let Ok(mut batch) = state.previous_batches.load(deps.storage, request.id) {
            if batch.reconciled && batch.est_unbond_end_time < current_time {
                let uosmo_to_refund = batch
                    .uosmo_unclaimed
                    .multiply_ratio(request.shares, batch.total_shares);

                ids.push(request.id.to_string());

                total_uosmo_to_refund += uosmo_to_refund;
                batch.total_shares -= request.shares;
                batch.uosmo_unclaimed -= uosmo_to_refund;

                if batch.total_shares.is_zero() {
                    state.previous_batches.remove(deps.storage, request.id)?;
                } else {
                    state
                        .previous_batches
                        .save(deps.storage, batch.id, &batch)?;
                }

                state
                    .unbond_requests
                    .remove(deps.storage, (request.id, &user))?;
            }
        }
    }

    if total_uosmo_to_refund.is_zero() {
        return Err(ContractError::ZeroWithdrawableAmount {});
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: receiver.clone().into(),
        amount: vec![Coin::new(total_uosmo_to_refund.u128(), "uosmo")],
    });

    let event = Event::new("steakhub/unbonded_withdrawn")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("ids", ids.join(","))
        .add_attribute("user", user)
        .add_attribute("receiver", receiver)
        .add_attribute("uosmo_refunded", total_uosmo_to_refund);

    Ok(Response::new()
        .add_message(refund_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/withdraw_unbonded"))
}

//--------------------------------------------------------------------------------------------------
// Ownership and management logics
//--------------------------------------------------------------------------------------------------

pub fn rebalance(deps: DepsMut, env: Env) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;

    let new_redelegations = compute_redelegations_for_rebalancing(&delegations);

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|rd| SubMsg::reply_on_success(rd.to_cosmos_msg(), 1))
        .collect::<Vec<SubMsg<OsmosisMsg>>>();

    let amount: u128 = new_redelegations.iter().map(|rd| rd.amount).sum();

    let event = Event::new("steakhub/rebalanced").add_attribute("uosmo_moved", amount.to_string());

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "steakhub/rebalance"))
}

pub fn add_validator(
    deps: DepsMut,
    sender: Addr,
    validator: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    state.validators.update(deps.storage, |mut validators| {
        if validators.contains(&validator) {
            return Err(StdError::generic_err("validator is already whitelisted"));
        }
        validators.push(validator.clone());
        Ok(validators)
    })?;

    let event = Event::new("steakhub/validator_added").add_attribute("validator", validator);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/add_validator"))
}

pub fn remove_validator(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    validator: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    let validators = state.validators.update(deps.storage, |mut validators| {
        if !validators.contains(&validator) {
            return Err(StdError::generic_err(
                "validator is not already whitelisted",
            ));
        }
        validators.retain(|v| *v != validator);
        Ok(validators)
    })?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let delegation_to_remove = query_delegation(&deps.querier, &validator, &env.contract.address)?;
    let new_redelegations = compute_redelegations_for_removal(&delegation_to_remove, &delegations);

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 1))
        .collect::<Vec<SubMsg<OsmosisMsg>>>();

    let event = Event::new("steak/validator_removed").add_attribute("validator", validator);

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "steakhub/remove_validator"))
}

pub fn transfer_ownership(
    deps: DepsMut,
    sender: Addr,
    new_owner: String,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state
        .new_owner
        .save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "steakhub/transfer_ownership"))
}

pub fn accept_ownership(
    deps: DepsMut,
    sender: Addr,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let state = State::default();

    let previous_owner = state.owner.load(deps.storage)?;
    let new_owner = state.new_owner.load(deps.storage)?;

    if sender != new_owner {
        return Err(ContractError::Unauthorized {});
    }

    state.owner.save(deps.storage, &sender)?;
    state.new_owner.remove(deps.storage);

    let event = Event::new("steakhub/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/transfer_ownership"))
}
