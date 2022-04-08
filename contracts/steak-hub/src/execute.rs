use std::collections::HashSet;
use std::str::FromStr;

use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, DistributionMsg, Env, Event, Order,
    Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper, TerraQuerier};

use steak::hub::{Batch, CallbackMsg, ExecuteMsg, InstantiateMsg, PendingBatch, UnbondRequest};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::math::{
    compute_delegations, compute_mint_amount, compute_unbond_amount, compute_undelegations,
};
use crate::state::State;
use crate::types::Coins;

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(deps: DepsMut, env: Env, msg: InstantiateMsg) -> StdResult<Response> {
    let state = State::default();

    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;
    state.validators.save(deps.storage, &msg.validators)?;
    state.unlocked_coins.save(deps.storage, &vec![])?;

    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: env.block.time.seconds() + msg.epoch_period,
        },
    )?;

    Ok(Response::new().add_submessage(SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(msg.admin),
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
            label: "steak_token".to_string(),
        }),
        1,
    )))
}

pub fn register_steak_token(
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
    state.steak_token.save(deps.storage, &contract_addr)?;

    Ok(Response::new())
}

//--------------------------------------------------------------------------------------------------
// Bonding and harvesting logics
//--------------------------------------------------------------------------------------------------

pub fn bond(
    deps: DepsMut,
    env: Env,
    receiver: Addr,
    uluna_to_bond: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    let usteak_to_mint = compute_mint_amount(usteak_supply, uluna_to_bond, &delegations);
    let new_delegations = compute_delegations(uluna_to_bond, &delegations);

    let delegate_submsgs: Vec<SubMsg<TerraMsgWrapper>> = new_delegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 2))
        .collect();

    let mint_msg: CosmosMsg<TerraMsgWrapper> = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: steak_token.into(),
        msg: to_binary(&Cw20ExecuteMsg::Mint {
            recipient: receiver.to_string(),
            amount: usteak_to_mint,
        })?,
        funds: vec![],
    });

    let event = Event::new("steakhub/bonded")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("receiver", receiver)
        .add_attribute("uluna_bonded", uluna_to_bond)
        .add_attribute("usteak_minted", usteak_to_mint);

    Ok(Response::new()
        .add_submessages(delegate_submsgs)
        .add_message(mint_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/bond"))
}

pub fn harvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let withdraw_submsgs: Vec<SubMsg<TerraMsgWrapper>> = deps
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
        .collect();

    let callback_msgs = vec![CallbackMsg::Swap {}, CallbackMsg::Reinvest {}]
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg<TerraMsgWrapper>>>>()?;

    Ok(Response::new()
        .add_submessages(withdraw_submsgs)
        .add_messages(callback_msgs)
        .add_attribute("action", "steakhub/harvest"))
}

pub fn swap(deps: DepsMut) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let mut unlocked_coins = state.unlocked_coins.load(deps.storage)?;

    let all_denoms: Vec<String> = unlocked_coins
        .iter()
        .cloned()
        .map(|coin| coin.denom)
        .filter(|denom| denom != "uluna")
        .collect();

    let known_denoms: HashSet<String> = TerraQuerier::new(&deps.querier)
        .query_exchange_rates("uluna".to_string(), all_denoms)?
        .exchange_rates
        .into_iter()
        .map(|item| item.quote_denom)
        .collect();

    let swap_submsgs: Vec<SubMsg<TerraMsgWrapper>> = unlocked_coins
        .iter()
        .cloned()
        .filter(|coin| known_denoms.contains(&coin.denom))
        .map(|coin| {
            SubMsg::reply_on_success(
                create_swap_msg(coin, "uluna".to_string()),
                3,
            )
        })
        .collect();

    unlocked_coins.retain(|coin| !known_denoms.contains(&coin.denom));
    state.unlocked_coins.save(deps.storage, &unlocked_coins)?;

    Ok(Response::new()
        .add_submessages(swap_submsgs)
        .add_attribute("action", "steakhub/swap"))
}

/// NOTE: When delegation Luna here, we don't need to use a `SubMsg` to handle the received coins,
/// because we have already withdrawn all claimable staking rewards previously in the same atomic
/// execution.
pub fn reinvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;
    let mut unlocked_coins = state.unlocked_coins.load(deps.storage)?;

    let uluna_to_bond = unlocked_coins
        .iter()
        .find(|coin| coin.denom == "uluna")
        .ok_or_else(|| StdError::generic_err("no uluna available to be bonded"))?
        .amount;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let new_delegations = compute_delegations(uluna_to_bond, &delegations);

    unlocked_coins.retain(|coin| coin.denom != "uluna");
    state.unlocked_coins.save(deps.storage, &unlocked_coins)?;

    let event = Event::new("steakhub/harvested")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("uluna_bonded", uluna_to_bond);

    Ok(Response::new()
        .add_messages(new_delegations.iter().map(|d| d.to_cosmos_msg()))
        .add_event(event)
        .add_attribute("action", "steakhub/reinvest"))
}

pub fn register_received_coins(
    deps: DepsMut,
    env: Env,
    response: SubMsgExecutionResponse,
    event_type: &str,
    receiver_key: &str,
    received_coins_key: &str
) -> StdResult<Response> {
    let event = response
        .events
        .iter()
        .find(|event| event.ty == event_type)
        .ok_or_else(|| StdError::generic_err(format!("cannot find `{}` event", event_type)))?;

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
        .ok_or_else(|| StdError::generic_err(format!("cannot find `{}` attribute", received_coins_key)))?
        .value;

    let received_coins = if *receiver == env.contract.address {
        Coins::from_str(received_coins_str)?
    } else {
        Coins(vec![])
    };

    let state = State::default();
    state.unlocked_coins.update(deps.storage, |coins| -> StdResult<_> {
        let coins = Coins(coins).add_many(&received_coins)?;
        Ok(coins.0)
    })?;

    Ok(Response::new()
        .add_attribute("action", "steakhub/register_received_coins"))
}

//--------------------------------------------------------------------------------------------------
// Unbonding logics
//--------------------------------------------------------------------------------------------------

pub fn queue_unbond(
    deps: DepsMut,
    env: Env,
    receiver: Addr,
    usteak_to_burn: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    let mut pending_batch = state.pending_batch.load(deps.storage)?;
    pending_batch.usteak_to_burn += usteak_to_burn;
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
            request.shares += usteak_to_burn;
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

pub fn submit_batch(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;
    let unbond_period = state.unbond_period.load(deps.storage)?;
    let pending_batch = state.pending_batch.load(deps.storage)?;

    let current_time = env.block.time.seconds();
    if current_time < pending_batch.est_unbond_start_time {
        return Err(StdError::generic_err(
            format!("batch can only be submitted for unbonding after {}", pending_batch.est_unbond_start_time)
        ));
    }

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    let uluna_to_unbond = compute_unbond_amount(usteak_supply, pending_batch.usteak_to_burn, &delegations);
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
            total_shares: pending_batch.usteak_to_burn,
            uluna_unclaimed: uluna_to_unbond,
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

    let undelegate_submsgs: Vec<SubMsg<TerraMsgWrapper>> = new_undelegations
        .iter()
        .map(|d| SubMsg::reply_on_success(d.to_cosmos_msg(), 2))
        .collect();

    let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: steak_token.into(),
        msg: to_binary(&Cw20ExecuteMsg::Burn {
            amount: pending_batch.usteak_to_burn,
        })?,
        funds: vec![],
    });

    let event = Event::new("steakhub/unbond_submitted")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("id", pending_batch.id.to_string())
        .add_attribute("uluna_unbonded", uluna_to_unbond)
        .add_attribute("usteak_burned", pending_batch.usteak_to_burn);

    Ok(Response::new()
        .add_submessages(undelegate_submsgs)
        .add_message(burn_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/unbond"))
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
        .collect::<StdResult<Vec<UnbondRequest>>>()?;

    let mut total_uluna_to_refund = Uint128::zero();
    let mut ids: Vec<String> = vec![];
    for request in &requests {
        let mut batch = state.previous_batches.load(deps.storage, request.id.into())?;
        if batch.est_unbond_end_time < current_time {
            let uluna_to_refund = batch.uluna_unclaimed.multiply_ratio(request.shares, batch.total_shares);

            ids.push(request.id.to_string());

            total_uluna_to_refund += uluna_to_refund;
            batch.total_shares -= request.shares;
            batch.uluna_unclaimed -= uluna_to_refund;

            if batch.total_shares.is_zero() {
                state.previous_batches.remove(deps.storage, request.id.into());
            }

            state.unbond_requests.remove(deps.storage, (request.id.into(), &user))?;
        }
    }

    let refund_msg = CosmosMsg::Bank(BankMsg::Send {
        to_address: receiver.clone().into(),
        amount: vec![Coin::new(total_uluna_to_refund.u128(), "uluna")],
    });

    let event = Event::new("steakhub/unbonded_withdrawn")
        .add_attribute("time", env.block.time.seconds().to_string())
        .add_attribute("height", env.block.height.to_string())
        .add_attribute("ids", ids.join(","))
        .add_attribute("user", user)
        .add_attribute("receiver", receiver)
        .add_attribute("uluna_refunded", total_uluna_to_refund);

    Ok(Response::new()
        .add_message(refund_msg)
        .add_event(event)
        .add_attribute("action", "steakhub/withdraw_unbonded"))
}
