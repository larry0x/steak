use std::collections::HashSet;
use std::iter::FromIterator;
use std::str::FromStr;

use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, DistributionMsg, Env, Event, Order, Response, StdError, StdResult, SubMsg, to_binary, Uint128, WasmMsg};
use kujira::denom::Denom;
use kujira::msg::DenomMsg;

use pfc_steak::DecimalCheckedOps;
use pfc_steak::hub::{
    Batch, CallbackMsg, FeeType, PendingBatch, UnbondRequest,
};
use pfc_steak::hub_tf::{ExecuteMsg, InstantiateMsg};

use crate::contract::REPLY_REGISTER_RECEIVED_COINS;
use crate::helpers::{get_denom_balance, parse_received_fund, query_all_delegations, query_delegation, query_delegations};
use crate::math::{
    compute_mint_amount, compute_redelegations_for_rebalancing, compute_redelegations_for_removal,
    compute_unbond_amount, compute_undelegations, reconcile_batches,
};
use crate::state::{previous_batches, State, unbond_requests, VALIDATORS, VALIDATORS_ACTIVE};
use crate::token_factory;
use crate::token_factory::denom::{MsgBurn, MsgCreateDenom, MsgMint};
use crate::types::{Coins, Delegation};

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(deps: DepsMut, env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    let state = State::default();

    if msg.max_fee_amount > Decimal::from_str("1.00")? {
        return Err(StdError::generic_err("Max fee can not exceed 1/100%"));
    }

    if msg.fee_amount > msg.max_fee_amount {
        return Err(StdError::generic_err("fee can not exceed max fee"));
    }
    let fee_type = FeeType::from_str(&msg.fee_account_type)
        .map_err(|_| StdError::generic_err("Invalid Fee type: Wallet or FeeSplit only"))?;

    state
        .owner
        .save(deps.storage, &deps.api.addr_validate(&msg.owner)?)?;
    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;
    state.unlocked_coins.save(deps.storage, &vec![])?;
    state.prev_denom.save(deps.storage, &Uint128::zero())?;
    state.denom.save(deps.storage, &msg.denom)?;
    state.max_fee_rate.save(deps.storage, &msg.max_fee_amount)?;
    state.fee_rate.save(deps.storage, &msg.fee_amount)?;
    state.fee_account_type.save(deps.storage, &fee_type)?;

    state
        .fee_account
        .save(deps.storage, &deps.api.addr_validate(&msg.fee_account)?)?;

    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: env.block.time.seconds() + msg.epoch_period,
        },
    )?;


    for v in msg.validators {
        VALIDATORS.insert(deps.storage, &v)?;
        VALIDATORS_ACTIVE.insert(deps.storage, &v)?;
    }
    state.kuji_token_factory.save(deps.storage, &msg.kuji_token_factory)?;
    let steak_denom = format!("factory/{0}/{1}", env.contract.address, msg.steak_denom);
    let steak_denom_msg = msg.steak_denom;
    state.steak_denom.save(deps.storage, &steak_denom)?;
    state.steak_minted.save(deps.storage, &Uint128::zero())?;

    if let Some(dust) = msg.dust_collector {
        state.dust_collector.save(deps.storage, &Some(deps.api.addr_validate(&dust)?))?
    } else {
        state.dust_collector.save(deps.storage, &None)?
    }
    if msg.kuji_token_factory {
        todo!()
        /*
        Ok(Response::new().add_submessage(SubMsg::new(DenomMsg::Create {
            subdenom:  Denom::from(steak_denom_msg)
        })))

         */
    } else {
        let c = <MsgCreateDenom as Into<CosmosMsg>>::into(
            MsgCreateDenom { sender: env.contract.address.to_string(), subdenom: steak_denom_msg });

        Ok(Response::new().add_message(c))
    }
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
pub fn bond(deps: DepsMut, env: Env, receiver: Addr, funds: Vec<Coin>, bond_msg: Option<String>) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let amount_to_bond = parse_received_fund(&funds, &denom)?;
    let steak_minted = state.steak_minted.load(deps.storage)?;
    let steak_denom = state.steak_denom.load(deps.storage)?;
    let kuji_version = state.kuji_token_factory.load(deps.storage)?;
    let mut validators: Vec<String> = Default::default();
    for v in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        validators.push(v?);
    }

    let mut validators_wl: HashSet<String> = Default::default();
    for v in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators_wl.insert(v?);
    }
    for v in validators.iter() {
        validators_wl.remove(v);
    }
    let non_active_validator_list = Vec::from_iter(validators_wl);

    // Query the current delegations made to validators, and find the validator with the smallest
    // delegated amount through a linear search
    // The code for linear search is a bit uglier than using `sort_by` but cheaper: O(n) vs O(n * log(n))
    let delegations_non_active = query_delegations(&deps.querier, &non_active_validator_list, &env.contract.address, &denom)?;
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address, &denom)?;

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
        amount: amount_to_bond.u128(),
        denom: denom.clone(),
    };

    // Query the current supply of Steak and compute the amount to mint
    //   let usteak_supply = steak_minted;
    let usteak_to_mint = compute_mint_amount(steak_minted, amount_to_bond, &delegations, &delegations_non_active);
    state.steak_minted.save(deps.storage, &(steak_minted + usteak_to_mint))?;
    // TODO deal with multiple token returns
    state.prev_denom.save(
        deps.storage,
        &get_denom_balance(&deps.querier, env.contract.address.clone(), denom.clone())?,
    )?;

    let delegate_submsg = SubMsg::reply_on_success(
        new_delegation.to_cosmos_msg(),
        REPLY_REGISTER_RECEIVED_COINS,
    );

    let mint_msg = if kuji_version {
        let _k = DenomMsg::Mint {
            denom: Denom::from(steak_denom),
            amount: usteak_to_mint,
            recipient: env.contract.address,
        };
        // CosmosMsg::from(k);
        todo!()
    } else {
        <MsgMint as Into<CosmosMsg>>::into(MsgMint {
            sender: env.contract.address.to_string(),
            amount: Some(token_factory::denom::Coin {
                denom: steak_denom.clone(),
                amount: usteak_to_mint.to_string(),
            }),
        })
    };


    let contract_info = deps.querier.query_wasm_contract_info(receiver.to_string());

    // send the uSteak, optionally calling a smart contract
    let send_transfer_msg: CosmosMsg = match contract_info {
        Ok(_) => {

            //CosmosMsg::Bank(BankMsg::Send { to_address: "".to_string(), amount: vec![] })
            if let Some(exec_msg) = bond_msg {
                CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: receiver.to_string(),
                    msg: to_binary(&exec_msg)?,
                    funds: vec![Coin {
                        denom: steak_denom,
                        amount: usteak_to_mint,
                    }],
                })
            } else {
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: receiver.to_string(),
                    amount: vec![Coin {
                        denom: steak_denom,
                        amount: usteak_to_mint,
                    }],
                })
            }
        }
        Err(_) => {
            CosmosMsg::Bank(BankMsg::Send {
                to_address: receiver.to_string(),
                amount: vec![Coin {
                    denom: steak_denom,
                    amount: usteak_to_mint,
                }],
            })
        }
    };

    let event = Event::new("steakhub/bonded")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("steak_receiver", receiver)
        .add_attribute("denom_bonded", denom)
        .add_attribute("denom_amount", amount_to_bond)
        .add_attribute("usteak_minted", usteak_to_mint);

    Ok(Response::new()
        .add_submessage(delegate_submsg)
        .add_messages(vec![mint_msg, send_transfer_msg])
        //   .add_message(send_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/bond"))
}

pub fn harvest(deps: DepsMut, env: Env) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    state.prev_denom.save(
        deps.storage,
        &get_denom_balance(&deps.querier, env.contract.address.clone(), denom)?,
    )?;

    let withdraw_submsgs = deps
        .querier
        .query_all_delegations(&env.contract.address)?
        .into_iter()
        .map(|d| {
            SubMsg::reply_on_success(
                CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                    validator: d.validator,
                }),
                REPLY_REGISTER_RECEIVED_COINS,
            )
        })
        .collect::<Vec<_>>();

    let callback_msg = CallbackMsg::Reinvest {}.into_cosmos_msg(&env.contract.address)?;

    Ok(Response::new()
        .add_submessages(withdraw_submsgs)
        .add_message(callback_msg)
        .add_attribute("action", "steakhub/harvest"))
}

/// NOTE:
/// 1. When delegation Native denom here, we don't need to use a `SubMsg` to handle the received coins,
/// because we have already withdrawn all claimable staking rewards previously in the same atomic
/// execution.
/// 2. Same as with `bond`, in the latest implementation we only delegate staking rewards with the
/// validator that has the smallest delegation amount.
pub fn reinvest(deps: DepsMut, env: Env) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let fee = state.fee_rate.load(deps.storage)?;

    let mut validators: Vec<String> = Default::default();
    for v in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        validators.push(v?);
    }
    let prev_coin = state.prev_denom.load(deps.storage)?;
    let current_coin =
        get_denom_balance(&deps.querier, env.contract.address.clone(), denom.clone())?;

    if current_coin <= prev_coin {
        return Err(StdError::generic_err("no rewards"));
    }
    let amount_to_bond = current_coin.saturating_sub(prev_coin);
    let mut unlocked_coins = state.unlocked_coins.load(deps.storage)?;

    /*

        if unlocked_coins.is_empty() {
            return Err(StdError::generic_err("no rewards"));
        }
        let amount_to_bond = unlocked_coins
            .iter()
            .find(|coin| coin.denom == denom)
            .ok_or_else(|| StdError::generic_err("no native amount available to be bonded"))?
            .amount;
    */
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address, &denom)?;
    let mut validator = &delegations[0].validator;
    let mut amount = delegations[0].amount;
    for d in &delegations[1..] {
        if d.amount < amount {
            validator = &d.validator;
            amount = d.amount;
        }
    }
    let fee_amount = if fee.is_zero() {
        Uint128::zero()
    } else {
        fee.checked_mul_uint(amount_to_bond)?
    };
    let amount_to_bond_minus_fees = amount_to_bond.saturating_sub(fee_amount);

    let new_delegation = Delegation::new(validator, amount_to_bond_minus_fees.u128(), &denom);

    unlocked_coins.retain(|coin| coin.denom != denom);
    state.unlocked_coins.save(deps.storage, &unlocked_coins)?;

    let event = Event::new("steakhub/harvested")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("denom", &denom)
        .add_attribute("fees_deducted", fee_amount)
        .add_attribute("denom_bonded", amount_to_bond_minus_fees);

    if fee_amount > Uint128::zero() {
        let fee_account = state.fee_account.load(deps.storage)?;
        let fee_type = state.fee_account_type.load(deps.storage)?;

        let send_msgs = match fee_type {
            FeeType::Wallet =>
                vec![CosmosMsg::Bank(BankMsg::Send {
                    to_address: fee_account.to_string(),
                    amount: vec![Coin::new(fee_amount.into(), &denom)],
                })],
            FeeType::FeeSplit => {
                let msg = pfc_fee_split::fee_split_msg::ExecuteMsg::Deposit { flush: false };

                vec![msg.into_cosmos_msg(fee_account, vec![Coin::new(fee_amount.into(), &denom)])?]
            }
        };
        Ok(Response::new()
            .add_message(new_delegation.to_cosmos_msg())
            .add_messages(send_msgs)
            .add_event(event)
            .add_attribute("action", "steakhub/reinvest"))
    } else {
        Ok(Response::new()
            .add_message(new_delegation.to_cosmos_msg())
            .add_event(event)
            .add_attribute("action", "steakhub/reinvest"))
    }
}

/// NOTE: a `SubMsgResponse` may contain multiple coin-receiving events, must handle them individually
pub fn register_received_coins(
    deps: DepsMut,
    env: Env,
    mut events: Vec<Event>,
) -> StdResult<Response> {
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
    receiver: Addr,
    funds: Vec<Coin>,
) -> StdResult<Response> {
    let state = State::default();
    let steak_denom = state.steak_denom.load(deps.storage)?;

    let usteak_to_burn = funds.iter().filter(|p| p.denom == steak_denom).map(|steak_funds| steak_funds.amount).sum::<Uint128>();
    if funds.len() != 1 || usteak_to_burn.is_zero() {
        return Err(StdError::generic_err(format!(
            "you can only send {} tokens to unbond",
            steak_denom
        )));
    }
    let mut pending_batch = state.pending_batch.load(deps.storage)?;
    pending_batch.usteak_to_burn += usteak_to_burn;
    state.pending_batch.save(deps.storage, &pending_batch)?;

    unbond_requests().update(
        deps.storage,
        (pending_batch.id, receiver.as_ref()),
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

    let mut msgs: Vec<CosmosMsg> = vec![];
    if env.block.time.seconds() >= pending_batch.est_unbond_start_time {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.into(),
            msg: to_binary(&ExecuteMsg::SubmitBatch {})?,
            funds: vec![],
        }));
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

pub fn submit_batch(deps: DepsMut, env: Env) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let steak_denom = state.steak_denom.load(deps.storage)?;
    let kuji_version = state.kuji_token_factory.load(deps.storage)?;
    let usteak_supply = state.steak_minted.load(deps.storage)?;
    let mut validators: Vec<String> = Default::default();
    for v in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators.push(v?);
    }

    let unbond_period = state.unbond_period.load(deps.storage)?;
    let pending_batch = state.pending_batch.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    if current_time < pending_batch.est_unbond_start_time {
        return Err(StdError::generic_err(format!(
            "batch can only be submitted for unbonding after {}",
            pending_batch.est_unbond_start_time
        )));
    }
    let mut validators_active: HashSet<String> = Default::default();
    for v in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        validators_active.insert(v?);
    }
    for v in validators.iter() {
        validators_active.remove(v);
    }
    let active_validator_list = Vec::from_iter(validators_active);

    // for unbonding we still need to look at
    // TODO verify denom
    let delegations = query_all_delegations(&deps.querier, &env.contract.address)?;
    let delegations_active = query_delegations(&deps.querier, &active_validator_list, &env.contract.address, &denom)?;
    // let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    let amount_to_unbond =
        compute_unbond_amount(usteak_supply, pending_batch.usteak_to_burn, &delegations, &delegations_active);

    let new_undelegations = compute_undelegations(amount_to_unbond, &delegations, &denom);

    // NOTE: Regarding the `amount_unclaimed` value
    //
    // If validators misbehave and get slashed during the unbonding period, the contract can receive
    // LESS Luna than `amount_to_unbond` when unbonding finishes!
    //
    // In this case, users who invokes `withdraw_unbonded` will have their txs failed as the contract
    // does not have enough Luna balance.
    //
    // I don't have a solution for this... other than to manually fund contract with the slashed amount.
    previous_batches().save(
        deps.storage,
        pending_batch.id,
        &Batch {
            id: pending_batch.id,
            reconciled: false,
            total_shares: pending_batch.usteak_to_burn,
            amount_unclaimed: amount_to_unbond,
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
    state.prev_denom.save(
        deps.storage,
        &get_denom_balance(&deps.querier, env.contract.address.clone(), denom)?,
    )?;

    let undelegate_submsgs = new_undelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), REPLY_REGISTER_RECEIVED_COINS))
        .collect::<Vec<_>>();


    let burn_msg = if kuji_version {
        let _foo = DenomMsg::Burn {
            denom: Denom::from(steak_denom),
            amount: pending_batch.usteak_to_burn,
        };
        todo!()
    } else {
        <MsgBurn as Into<CosmosMsg>>::into(MsgBurn {
            sender: env.contract.address.to_string(),
            amount: Some(token_factory::denom::Coin {
                denom: steak_denom,
                amount: pending_batch.usteak_to_burn.to_string(),
            }),
        })
    };
    // yes.. this will fail if supply is less than the amount to burn. this is intentional.
    state.steak_minted.save(deps.storage, &(usteak_supply - pending_batch.usteak_to_burn))?;


    let event = Event::new("steakhub/unbond_submitted")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("native_unbonded", amount_to_unbond)
        .add_attribute("usteak_burned", pending_batch.usteak_to_burn);

    Ok(Response::new()
        .add_submessages(undelegate_submsgs)
        .add_message(burn_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/unbond"))
}

pub fn reconcile(deps: DepsMut, env: Env) -> StdResult<Response> {
    let state = State::default();
    let current_time = env.block.time.seconds();

    // Load batches that have not been reconciled
    let all_batches = previous_batches()
        .idx
        .reconciled
        .prefix("false".into())
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

    let native_expected_received: Uint128 = batches.iter().map(|b| b.amount_unclaimed).sum();
    let denom = state.denom.load(deps.storage)?;
    let unlocked_coins = state.unlocked_coins.load(deps.storage)?;

    let native_expected_unlocked = Coins(unlocked_coins).find(&denom).amount;

    let native_expected = native_expected_received + native_expected_unlocked;
    let native_actual = deps
        .querier
        .query_balance(&env.contract.address, &denom)?
        .amount;

    let native_to_deduct = native_expected
        .checked_sub(native_actual)
        .unwrap_or_else(|_| Uint128::zero());
    if !native_to_deduct.is_zero() {
        reconcile_batches(&mut batches, native_expected - native_actual);
    }

    for batch in batches.iter_mut() {
        batch.reconciled = true;
        previous_batches().save(deps.storage, batch.id, batch)?;
    }

    let ids = batches
        .iter()
        .map(|b| b.id.to_string())
        .collect::<Vec<_>>()
        .join(",");

    let event = Event::new("steakhub/reconciled")
        .add_attribute("ids", ids)
        .add_attribute("native_deducted", native_to_deduct.to_string());

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/reconcile"))
}

pub fn withdraw_unbonded_admin(
    deps: DepsMut,
    env: Env,
    user: Addr,
    receiver: Addr,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &user)?;

    withdraw_unbonded(deps, env, receiver.clone(), receiver)
}

pub fn withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    user: Addr,
    receiver: Addr,
) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let current_time = env.block.time.seconds();

    // NOTE: If the user has too many unclaimed requests, this may not fit in the WASM memory...
    // However, this is practically never going to happen. Who would create hundreds of unbonding
    // requests and never claim them?
    let requests = unbond_requests()
        .idx
        .user
        .prefix(user.to_string())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<_>>>()?;

    // NOTE: Native in the following batches are withdrawn it the batch:
    // - is a _previous_ batch, not a _pending_ batch
    // - is reconciled
    // - has finished unbonding
    // If not sure whether the batches have been reconciled, the user should first invoke `ExecuteMsg::Reconcile`
    // before withdrawing.
    let mut total_native_to_refund = Uint128::zero();
    let mut ids: Vec<String> = vec![];
    for request in &requests {
        if let Ok(mut batch) = previous_batches().load(deps.storage, request.id) {
            if batch.reconciled && batch.est_unbond_end_time < current_time {
                let native_to_refund = batch
                    .amount_unclaimed
                    .multiply_ratio(request.shares, batch.total_shares);

                ids.push(request.id.to_string());

                total_native_to_refund += native_to_refund;
                batch.total_shares -= request.shares;
                batch.amount_unclaimed -= native_to_refund;

                if batch.total_shares.is_zero() {
                    previous_batches().remove(deps.storage, request.id)?;
                } else {
                    previous_batches()
                        .save(deps.storage, batch.id, &batch)?;
                }
                unbond_requests()
                    .remove(deps.storage, (request.id, user.as_ref()))?;
            }
        }
    }

    if total_native_to_refund.is_zero() {
        return Err(StdError::generic_err("withdrawable amount is zero"));
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: receiver.clone().into(),
        amount: vec![Coin::new(total_native_to_refund.u128(), denom)],
    });

    let event = Event::new("steakhub/unbonded_withdrawn")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("ids", ids.join(","))
        .add_attribute("user", user)
        .add_attribute("receiver", receiver)
        .add_attribute("amount_refunded", total_native_to_refund);

    Ok(Response::new()
        .add_message(refund_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/withdraw_unbonded"))
}

//--------------------------------------------------------------------------------------------------
// Ownership and management logics
//--------------------------------------------------------------------------------------------------

pub fn rebalance(deps: DepsMut, env: Env, minimum: Uint128) -> StdResult<Response> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let mut validators: Vec<String> = Default::default();
    for v in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators.push(v?)
    }
    let mut validators_active: Vec<String> = Default::default();
    for v in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        validators_active.push(v?);
    }

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address, &denom)?;

    let new_redelegations =
        compute_redelegations_for_rebalancing(validators_active, &delegations, minimum);

    state.prev_denom.save(
        deps.storage,
        &get_denom_balance(&deps.querier, env.contract.address, denom)?,
    )?;

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|rd| SubMsg::reply_on_success(rd.to_cosmos_msg(), REPLY_REGISTER_RECEIVED_COINS))
        .collect::<Vec<_>>();

    let amount: u128 = new_redelegations.iter().map(|rd| rd.amount).sum();

    let event = Event::new("steakhub/rebalanced").add_attribute("amount_moved", amount.to_string());

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "steakhub/rebalance"))
}

pub fn add_validator(deps: DepsMut, sender: Addr, validator: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    if VALIDATORS.contains(deps.storage, &validator) {
        return Err(StdError::generic_err("validator is already whitelisted"));
    }
    VALIDATORS.insert(deps.storage, &validator)?;
    VALIDATORS_ACTIVE.insert(deps.storage, &validator)?;

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
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    let denom = state.denom.load(deps.storage)?;

    if !VALIDATORS.contains(deps.storage, &validator) {
        return Err(StdError::generic_err(
            "validator is not already whitelisted",
        ));
    }
    VALIDATORS.remove(deps.storage, &validator)?;
    VALIDATORS_ACTIVE.insert(deps.storage, &validator)?;
    let mut validators: Vec<String> = Default::default();
    for v in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators.push(v?);
    }

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address, &denom)?;
    let delegation_to_remove =
        query_delegation(&deps.querier, &validator, &env.contract.address, &denom)?;
    let new_redelegations =
        compute_redelegations_for_removal(&delegation_to_remove, &delegations, &denom);

    state.prev_denom.save(
        deps.storage,
        &get_denom_balance(&deps.querier, env.contract.address, denom)?,
    )?;

    let redelegate_submsgs = new_redelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), REPLY_REGISTER_RECEIVED_COINS))
        .collect::<Vec<_>>();

    let event = Event::new("steak/validator_removed").add_attribute("validator", validator);

    Ok(Response::new()
        .add_submessages(redelegate_submsgs)
        .add_event(event)
        .add_attribute("action", "steakhub/remove_validator"))
}

pub fn remove_validator_ex(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    validator: String,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    if !VALIDATORS.contains(deps.storage, &validator) {
        return Err(StdError::generic_err(
            "validator is not already whitelisted",
        ));
    }
    VALIDATORS.remove(deps.storage, &validator)?;


    let event = Event::new("steak/validator_removed_ex").add_attribute("validator", validator);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/remove_validator_ex"))
}

pub fn pause_validator(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    validator: String,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;

    if !VALIDATORS_ACTIVE.contains(deps.storage, &validator) {
        return Err(StdError::generic_err(
            "validator is not already whitelisted",
        ));
    }
    VALIDATORS_ACTIVE.remove(deps.storage, &validator)?;

    let event = Event::new("steak/pause_validator").add_attribute("validator", validator);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/pause_validator"))
}

pub fn unpause_validator(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    validator: String,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    VALIDATORS_ACTIVE.insert(deps.storage, &validator)?;

    let event = Event::new("steak/unpause_validator").add_attribute("validator", validator);

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/unpause_validator"))
}

pub fn set_unbond_period(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    unbond_period: u64,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.unbond_period.save(deps.storage, &unbond_period)?;
    let event = Event::new("steak/set_unbond_period")
        .add_attribute("unbond_period", format!("{}", unbond_period));

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/set_unbond_period"))
}

pub fn transfer_ownership(deps: DepsMut, sender: Addr, new_owner: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state
        .new_owner
        .save(deps.storage, &deps.api.addr_validate(&new_owner)?)?;

    Ok(Response::new().add_attribute("action", "steakhub/transfer_ownership"))
}

pub fn accept_ownership(deps: DepsMut, sender: Addr) -> StdResult<Response> {
    let state = State::default();

    let previous_owner = state.owner.load(deps.storage)?;
    let new_owner = state.new_owner.load(deps.storage)?;

    if sender != new_owner {
        return Err(StdError::generic_err(
            "unauthorized: sender is not new owner",
        ));
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

pub fn transfer_fee_account(
    deps: DepsMut,
    sender: Addr,
    fee_account_type: String,
    new_fee_account: String,
) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    let fee_type = FeeType::from_str(&fee_account_type)
        .map_err(|_| StdError::generic_err("Invalid Fee type: Wallet or FeeSplit only"))?;

    state.fee_account_type.save(deps.storage, &fee_type)?;

    state
        .fee_account
        .save(deps.storage, &deps.api.addr_validate(&new_fee_account)?)?;

    Ok(Response::new().add_attribute("action", "steakhub/transfer_fee_account"))
}

pub fn change_denom(deps: DepsMut, sender: Addr, new_denom: String) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    state.denom.save(deps.storage, &new_denom)?;

    Ok(Response::new().add_attribute("action", "steakhub/change_denom"))
}

pub fn update_fee(deps: DepsMut, sender: Addr, new_fee: Decimal) -> StdResult<Response> {
    let state = State::default();

    state.assert_owner(deps.storage, &sender)?;
    if new_fee > state.max_fee_rate.load(deps.storage)? {
        return Err(StdError::generic_err(
            "refusing to set fee above maximum set",
        ));
    }
    state.fee_rate.save(deps.storage, &new_fee)?;

    Ok(Response::new().add_attribute("action", "steakhub/update_fee"))
}

pub fn set_dust_collector(
    deps: DepsMut,
    _env: Env,
    sender: Addr,
    dust_collector: Option<String>,
) -> StdResult<Response> {
    let state = State::default();


    state.assert_owner(deps.storage, &sender)?;
    if let Some(ref dust_addr) = dust_collector {
        state.dust_collector.save(deps.storage, &Some(deps.api.addr_validate(dust_addr)?))?;
    } else {
        state.dust_collector.save(deps.storage, &None)?;
    };

    let event = Event::new("steak/set_dust_collector")
        .add_attribute("dust_collector",  dust_collector.unwrap_or("-cleared-".into()));

    Ok(Response::new()
        .add_event(event)
        .add_attribute("action", "steakhub/set_dust_collector"))
}

pub fn collect_dust(deps: DepsMut, _env: Env) -> StdResult<Response> {
    let state = State::default();

    if let Some(_dust_addr) = state.dust_collector.load(deps.storage)? {
        Ok(Response::new().add_attribute("dust", "tbd"))
    } else {
        Ok(Response::new().add_attribute("dust", "dust-collector-called"))
    }
}

pub fn return_denom(deps: DepsMut, _env: Env, _funds: Vec<Coin>) -> StdResult<Response> {
    let state = State::default();

    if let Some(_dust_addr) = state.dust_collector.load(deps.storage)? {
        Ok(Response::new().add_attribute("dust", "returned"))
    } else {
       Err(StdError::generic_err("No Dust collector set"))
    }
}