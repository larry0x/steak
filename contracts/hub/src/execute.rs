use std::str::FromStr;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, Event,
    Order, Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use eris_staking::DecimalCheckedOps;
use terra_cosmwasm::{TerraMsgWrapper};

use eris_staking::hub::{
    Batch, CallbackMsg, ExecuteMsg, FeeConfig, InstantiateMsg, PendingBatch, UnbondRequest,
};

use crate::constants::get_reward_fee_cap;
use crate::helpers::{query_cw20_total_supply, query_delegation, query_delegations};
use crate::math::{
    compute_mint_amount, compute_redelegations_for_rebalancing, compute_redelegations_for_removal,
    compute_unbond_amount, compute_undelegations, reconcile_batches,
};
use crate::state::State;
use crate::types::{Coins, Delegation, SendFee};

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(deps: DepsMut, env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    let state = State::default();

    if msg.protocol_reward_fee.gt(&get_reward_fee_cap()) {
        return Err(StdError::generic_err("'protocol_reward_fee' greater than max"));
    }

    state.owner.save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;
    state.validators.save(deps.storage, &msg.validators)?;
    state.unlocked_coins.save(deps.storage, &vec![])?;
    state.fee_config.save(
        deps.storage,
        &FeeConfig {
            protocol_fee_contract: deps.api.addr_validate(&msg.protocol_fee_contract)?,
            protocol_reward_fee: msg.protocol_reward_fee,
        },
    )?;

    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: 1,
            ustake_to_burn: Uint128::zero(),
            est_unbond_start_time: env.block.time.seconds() + msg.epoch_period,
        },
    )?;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(msg.owner), // use the owner as admin for now; can be changed later by a `MsgUpdateAdmin`
            code_id: msg.cw20_code_id,
            msg: to_binary(&Cw20InstantiateMsg {
                name: msg.name,
                symbol: msg.symbol,
                decimals: msg.decimals,
                initial_balances: vec![],
                mint: Some(MinterResponse {
                    minter: env.contract.address.into(),
                    cap: None,
                }),
                marketing: None,
            })?,
            funds: vec![],
            label: "stake_token".to_string(),
        }),
        1,
    )))
}

pub fn register_stake_token(
    deps: DepsMut,
    response: SubMsgExecutionResponse,
) -> StdResult<Response> {
    let state = State::default();

    let event = response
        .events
        .iter()
        .find(|event| event.ty == "instantiate_contract")
        .ok_or_else(|| StdError::generic_err("cannot find `instantiate_contract` event"))?;

    let contract_addr_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == "contract_address")
        .ok_or_else(|| StdError::generic_err("cannot find `contract_address` attribute"))?
        .value;

    let contract_addr = deps.api.addr_validate(contract_addr_str)?;
    state.stake_token.save(deps.storage, &contract_addr)?;

    Ok(Response::new())
}

//--------------------------------------------------------------------------------------------------
// Bonding and harvesting logics
//--------------------------------------------------------------------------------------------------

/// NOTE: In a previous implementation, we split up the deposited Luna over all validators, so that
/// they all have the same amount of delegation. This is however quite gas-expensive: $1.5 cost in
/// the case of 15 validators.
///
/// To save gas for users, now we simply delegate all deposited Luna to the validator with the
/// smallest amount of delegation. If delegations become severely unbalance as a result of this
/// (e.g. when a single user makes a very big deposit), anyone can invoke `ExecuteMsg::Rebalance`
/// to balance the delegations.
pub fn bond(
    deps: DepsMut,
    env: Env,
    receiver: Addr,
    uluna_to_bond: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let stake_token = state.stake_token.load(deps.storage)?;
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
    let new_delegation = Delegation {
        validator: validator.clone(),
        amount: uluna_to_bond.u128(),
    };

    // Query the current supply of Staking Token and compute the amount to mint
    let ustake_supply = query_cw20_total_supply(&deps.querier, &stake_token)?;
    let ustake_to_mint = compute_mint_amount(ustake_supply, uluna_to_bond, &delegations);

    let delegate_submsg = SubMsg::reply_on_success(new_delegation.to_cosmos_msg(), 2);

    let mint_msg: CosmosMsg<TerraMsgWrapper> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stake_token.into(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: receiver.to_string(),
            amount: ustake_to_mint,
        })?,
        funds: vec![],
    });

    let event = Event::new("erishub/bonded")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("receiver", receiver)
        .add_attribute("uluna_bonded", uluna_to_bond)
        .add_attribute("ustake_minted", ustake_to_mint);

    Ok(Response::new()
        .add_submessage(delegate_submsg)
        .add_message(mint_msg)
        .add_event(event)
        .add_attribute("action", "erishub/bond"))
}

pub fn harvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let withdraw_submsgs = deps
        .querier
        .query_all_delegations(&env.contract.address)?
        .into_iter()
        .map(|d| {
            SubMsg::reply_on_success(
                CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                    validator: d.validator,
                }),
                2,
            )
        })
        .collect::<Vec<_>>();

    let callback_msgs = vec![
        CallbackMsg::Reinvest {},
    ]
    .iter()
    .map(|callback| callback.into_cosmos_msg(&env.contract.address))
    .collect::<StdResult<Vec<_>>>()?;

    Ok(Response::new()
        .add_submessages(withdraw_submsgs)
        .add_messages(callback_msgs)
        .add_attribute("action", "erishub/harvest"))
}

/// NOTE:
/// 1. When delegation Luna here, we don't need to use a `SubMsg` to handle the received coins,
/// because we have already withdrawn all claimable staking rewards previously in the same atomic
/// execution.
/// 2. Same as with `bond`, in the latest implementation we only delegate staking rewards with the
/// validator that has the smallest delegation amount.
pub fn reinvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;
    let mut unlocked_coins = state.unlocked_coins.load(deps.storage)?;
    let fee_config = state.fee_config.load(deps.storage)?;

    let uluna_available = unlocked_coins
        .iter()
        .find(|coin| coin.denom == "uluna")
        .ok_or_else(|| StdError::generic_err("no uluna available to be bonded"))?
        .amount;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let mut validator = &delegations[0].validator;
    let mut amount = delegations[0].amount;
    for d in &delegations[1..] {
        if d.amount < amount {
            validator = &d.validator;
            amount = d.amount;
        }
    }

    let protocol_fee_amount = fee_config.protocol_reward_fee.checked_mul(uluna_available)?;
    let uluna_to_bond = uluna_available.saturating_sub(protocol_fee_amount);

    let new_delegation = Delegation::new(validator, uluna_to_bond.u128());

    unlocked_coins.retain(|coin| coin.denom != "uluna");
    state.unlocked_coins.save(deps.storage, &unlocked_coins)?;

    let event = Event::new("erishub/harvested")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("uluna_bonded", uluna_to_bond)
        .add_attribute("uluna_protocol_fee", protocol_fee_amount);

    let mut msgs = vec![new_delegation.to_cosmos_msg()];

    if !protocol_fee_amount.is_zero() {
        let send_fee = SendFee::new(fee_config.protocol_fee_contract, protocol_fee_amount.u128());
        msgs.push(send_fee.to_cosmos_msg());
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(event)
        .add_attribute("action", "erishub/reinvest"))
}

/// NOTE: a `SubMsgExecutionResponse` may contain multiple coin-receiving events, must handle them indivitually
pub fn register_received_coins(
    deps: DepsMut,
    env: Env,
    mut events: Vec<Event>,
    event_type: &str,
    receiver_key: &str,
    received_coins_key: &str,
) -> StdResult<Response> {
    events.retain(|event| event.ty == event_type);
    if events.is_empty() {
        return Ok(Response::new());
    }

    let mut received_coins = Coins(vec![]);
    for event in &events {
        received_coins.add_many(&parse_coin_receiving_event(
            &env,
            event,
            receiver_key,
            received_coins_key,
        )?)?;
    }

    let state = State::default();
    state.unlocked_coins.update(deps.storage, |coins| -> StdResult<_> {
        let mut coins = Coins(coins);
        coins.add_many(&received_coins)?;
        Ok(coins.0)
    })?;

    Ok(Response::new().add_attribute("action", "erishub/register_received_coins"))
}

fn parse_coin_receiving_event(
    env: &Env,
    event: &Event,
    receiver_key: &str,
    received_coins_key: &str,
) -> StdResult<Coins> {
    let receiver = &event
        .attributes
        .iter()
        .find(|attr| attr.key == receiver_key)
        .ok_or_else(|| StdError::generic_err(format!("cannot find `{}` attribute", receiver_key)))?
        .value;

    let received_coins_str = &event
        .attributes
        .iter()
        .find(|attr| attr.key == received_coins_key)
        .ok_or_else(|| {
            StdError::generic_err(format!("cannot find `{}` attribute", received_coins_key))
        })?
        .value;

    let received_coins = if *receiver == env.contract.address {
        Coins::from_str(received_coins_str)?
    } else {
        Coins(vec![])
    };

    Ok(received_coins)
}

//--------------------------------------------------------------------------------------------------
// Unbonding logics
//--------------------------------------------------------------------------------------------------

pub fn queue_unbond(
    deps: DepsMut,
    env: Env,
    receiver: Addr,
    ustake_to_burn: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    let mut pending_batch = state.pending_batch.load(deps.storage)?;
    pending_batch.ustake_to_burn += ustake_to_burn;
    state.pending_batch.save(deps.storage, &pending_batch)?;

    state.unbond_requests.update(
        deps.storage,
        (pending_batch.id.into(), &receiver),
        |x| -> StdResult<_> {
            let mut request = x.unwrap_or_else(|| UnbondRequest {
                id: pending_batch.id,
                user: receiver.clone(),
                shares: Uint128::zero(),
            });
            request.shares += ustake_to_burn;
            Ok(request)
        },
    )?;

    let mut msgs: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    if env.block.time.seconds() >= pending_batch.est_unbond_start_time {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.into(),
            msg: to_binary(&ExecuteMsg::SubmitBatch {})?,
            funds: vec![],
        }));
    }

    let event = Event::new("erishub/unbond_queued")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("receiver", receiver)
        .add_attribute("ustake_to_burn", ustake_to_burn);

    Ok(Response::new()
        .add_messages(msgs)
        .add_event(event)
        .add_attribute("action", "erishub/queue_unbond"))
}

pub fn submit_batch(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let stake_token = state.stake_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;
    let unbond_period = state.unbond_period.load(deps.storage)?;
    let pending_batch = state.pending_batch.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    if current_time < pending_batch.est_unbond_start_time {
        return Err(StdError::generic_err(format!(
            "batch can only be submitted for unbonding after {}",
            pending_batch.est_unbond_start_time
        )));
    }

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let ustake_supply = query_cw20_total_supply(&deps.querier, &stake_token)?;

    let uluna_to_unbond =
        compute_unbond_amount(ustake_supply, pending_batch.ustake_to_burn, &delegations);
    let new_undelegations = compute_undelegations(uluna_to_unbond, &delegations);

    // NOTE: Regarding the `uluna_unclaimed` value
    //
    // If validators misbehave and get slashed during the unbonding period, the contract can receive
    // LESS Luna than `uluna_to_unbond` when unbonding finishes!
    //
    // In this case, users who invokes `withdraw_unbonded` will have their txs failed as the contract
    // does not have enough Luna balance.
    //
    // I don't have a solution for this... other than to manually fund contract with the slashed amount.
    state.previous_batches.save(
        deps.storage,
        pending_batch.id.into(),
        &Batch {
            id: pending_batch.id,
            reconciled: false,
            total_shares: pending_batch.ustake_to_burn,
            uluna_unclaimed: uluna_to_unbond,
            est_unbond_end_time: current_time + unbond_period,
        },
    )?;

    let epoch_period = state.epoch_period.load(deps.storage)?;
    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: pending_batch.id + 1,
            ustake_to_burn: Uint128::zero(),
            est_unbond_start_time: current_time + epoch_period,
        },
    )?;

    let undelegate_submsgs = new_undelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 2))
        .collect::<Vec<_>>();

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: stake_token.into(),
        msg: to_binary(&Cw20ExecuteMsg::Burn {
            amount: pending_batch.ustake_to_burn,
        })?,
        funds: vec![],
    });

    let event = Event::new("erishub/unbond_submitted")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("uluna_unbonded", uluna_to_unbond)
        .add_attribute("ustake_burned", pending_batch.ustake_to_burn);

    Ok(Response::new()
        .add_submessages(undelegate_submsgs)
        .add_message(burn_msg)
        .add_event(event)
        .add_attribute("action", "erishub/unbond"))
}

pub fn reconcile(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
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

    let uluna_expected_received: Uint128 = batches.iter().map(|b| b.uluna_unclaimed).sum();

    if uluna_expected_received.is_zero() {
        return Ok(Response::new());
    }

    let unlocked_coins = state.unlocked_coins.load(deps.storage)?;
    let uluna_expected_unlocked = Coins(unlocked_coins).find("uluna").amount;

    let uluna_expected = uluna_expected_received + uluna_expected_unlocked;
    let uluna_actual = deps.querier.query_balance(&env.contract.address, "uluna")?.amount;

    if uluna_actual >= uluna_expected {
        return Ok(Response::new());
    }

    let uluna_to_deduct = uluna_expected - uluna_actual;

    reconcile_batches(&mut batches, uluna_to_deduct);

    for batch in &batches {
        state.previous_batches.save(deps.storage, batch.id.into(), batch)?;
    }

    let ids = batches.iter().map(|b| b.id.to_string()).collect::<Vec<_>>().join(",");

    let event = Event::new("erishub/reconciled")
        .add_attribute("ids", ids)
        .add_attribute("uluna_deducted", uluna_to_deduct.to_string());

    Ok(Response::new().add_event(event).add_attribute("action", "erishub/reconcile"))
}

pub fn withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    user: Addr,
    receiver: Addr,
) -> StdResult<Response<TerraMsgWrapper>> {
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

    // NOTE: Luna in the following batches are withdrawn it the batch:
    // - is a _previous_ batch, not a _pending_ batch
    // - is reconciled
    // - has finished unbonding
    // If not sure whether the batches have been reconciled, the user should first invoke `ExecuteMsg::Reconcile`
    // before withdrawing.
    let mut total_uluna_to_refund = Uint128::zero();
    let mut ids: Vec<String> = vec![];
    for request in &requests {
        if let Ok(mut batch) = state.previous_batches.load(deps.storage, request.id.into()) {
            if batch.reconciled && batch.est_unbond_end_time < current_time {
                let uluna_to_refund =
                    batch.uluna_unclaimed.multiply_ratio(request.shares, batch.total_shares);

                ids.push(request.id.to_string());

                total_uluna_to_refund += uluna_to_refund;
                batch.total_shares -= request.shares;
                batch.uluna_unclaimed -= uluna_to_refund;

                if batch.total_shares.is_zero() {
                    state.previous_batches.remove(deps.storage, request.id.into())?;
                } else {
                    state.previous_batches.save(deps.storage, batch.id.into(), &batch)?;
                }

                state.unbond_requests.remove(deps.storage, (request.id.into(), &user))?;
            }
        }
    }

    if total_uluna_to_refund.is_zero() {
        return Err(StdError::generic_err("withdrawable amount is zero"));
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: receiver.clone().into(),
        amount: vec![Coin::new(total_uluna_to_refund.u128(), "uluna")],
    });

    let event = Event::new("erishub/unbonded_withdrawn")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("ids", ids.join(","))
        .add_attribute("user", user)
        .add_attribute("receiver", receiver)
        .add_attribute("uluna_refunded", total_uluna_to_refund);

    Ok(Response::new()
        .add_message(refund_msg)
        .add_event(event)
        .add_attribute("action", "erishub/withdraw_unbonded"))
}

//--------------------------------------------------------------------------------------------------
// Ownership and management logics
//--------------------------------------------------------------------------------------------------

pub fn rebalance(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;

    let new_redelegations = compute_redelegations_for_rebalancing(&delegations);

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|rd| SubMsg::reply_on_success(rd.to_cosmos_msg(), 2))
        .collect::<Vec<_>>();

    let amount: u128 = new_redelegations.iter().map(|rd| rd.amount).sum();

    let event = Event::new("erishub/rebalanced").add_attribute("uluna_moved", amount.to_string());

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "erishub/rebalance"))
}

pub fn add_validator(
    deps: DepsMut,
    sender: Addr,
    validator: String,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    state.validators.update(deps.storage, |mut validators| {
        if validators.contains(&validator) {
            return Err(StdError::generic_err("validator is already whitelisted"));
        }
        validators.push(validator.clone());
        Ok(validators)
    })?;

    let event = Event::new("erishub/validator_added").add_attribute("validator", validator);

    Ok(Response::new().add_event(event).add_attribute("action", "erishub/add_validator"))
}

pub fn remove_validator(
    deps: DepsMut,
    env: Env,
    sender: Addr,
    validator: String,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    let validators = state.validators.update(deps.storage, |mut validators| {
        if !validators.contains(&validator) {
            return Err(StdError::generic_err("validator is not already whitelisted"));
        }
        validators.retain(|v| *v != validator);
        Ok(validators)
    })?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let delegation_to_remove = query_delegation(&deps.querier, &validator, &env.contract.address)?;
    let new_redelegations = compute_redelegations_for_removal(&delegation_to_remove, &delegations);

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 2))
        .collect::<Vec<_>>();

    let event = Event::new("erishub/validator_removed").add_attribute("validator", validator);

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "erishub/remove_validator"))
}

pub fn transfer_ownership(
    deps: DepsMut,
    sender: Addr,
    new_owner: String,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.new_owner.save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "erishub/transfer_ownership"))
}

pub fn accept_ownership(deps: DepsMut, sender: Addr) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    let previous_owner = state.owner.load(deps.storage)?;
    let new_owner = state.new_owner.load(deps.storage)?;

    if sender != new_owner {
        return Err(StdError::generic_err("unauthorized: sender is not new owner"));
    }

    state.owner.save(deps.storage, &sender)?;
    state.new_owner.remove(deps.storage);

    let event = Event::new("erishub/ownership_transferred")
        .add_attribute("new_owner", new_owner)
        .add_attribute("previous_owner", previous_owner);

    Ok(Response::new().add_event(event).add_attribute("action", "erishub/transfer_ownership"))
}

pub fn update_config(
    deps: DepsMut,
    sender: Addr,
    protocol_fee_contract: Option<String>,
    protocol_reward_fee: Option<Decimal>,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    let mut fee_config = state.fee_config.load(deps.storage)?;

    if let Some(protocol_fee_contract) = protocol_fee_contract {
        fee_config.protocol_fee_contract = deps.api.addr_validate(&protocol_fee_contract)?;
    }

    if let Some(protocol_reward_fee) = protocol_reward_fee {
        if protocol_reward_fee.gt(&get_reward_fee_cap()) {
            return Err(StdError::generic_err("'protocol_reward_fee' greater than max"));
        }
        fee_config.protocol_reward_fee = protocol_reward_fee;
    }

    state.fee_config.save(deps.storage, &fee_config)?;

    Ok(Response::new().add_attribute("action", "erishub/update_config"))
}
