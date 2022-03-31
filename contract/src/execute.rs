use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, DistributionMsg, Env, MessageInfo, Order,
    Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::math::{
    compute_delegations, compute_mint_amount, compute_unbond_amount, compute_undelegations,
};
use crate::msg::{Batch, CallbackMsg, ExecuteMsg, InstantiateMsg, PendingBatch, UnbondShare};
use crate::state::State;

//--------------------------------------------------------------------------------------------------
// Instantiation
//--------------------------------------------------------------------------------------------------

pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let state = State::default();

    let worker_addrs =
        msg.workers.iter().map(|s| deps.api.addr_validate(s)).collect::<StdResult<Vec<Addr>>>()?;

    state.workers.save(deps.storage, &worker_addrs)?;
    state.validators.save(deps.storage, &msg.validators)?;
    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;

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
            admin: Some(info.sender.into()), // for now we use the deployer as the admin
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
            label: String::from("steak_token"),
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
    staker_addr: Addr,
    uluna_to_bond: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;

    // Query the delegations made by Steak Hub to validators, as well as the total supply of Steak
    // token, which we will use to compute stuff
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    // Compute the amount of `usteak` to mint
    let usteak_to_mint = compute_mint_amount(usteak_supply, uluna_to_bond, &delegations);

    // Compute the amount of `uluna` to be delegated to each validator
    let new_delegations = compute_delegations(uluna_to_bond, &delegations);

    Ok(Response::new()
        .add_messages(new_delegations.iter().map(|d| d.to_cosmos_msg()))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: steak_token.into(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: staker_addr.clone().into(),
                amount: usteak_to_mint,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "steak_hub/bond")
        .add_attribute("staker", staker_addr)
        .add_attribute("uluna_bonded", uluna_to_bond))
}

pub fn harvest(deps: DepsMut, env: Env, worker_addr: Addr) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    // Only whitelisted workers can harvest
    let worker_addrs = state.workers.load(deps.storage)?;
    if !worker_addrs.contains(&worker_addr) {
        return Err(StdError::generic_err("sender is not a whitelisted worker"));
    }

    // For each of the whitelisted validators, create a message to withdraw delegation reward
    let msgs: Vec<CosmosMsg<TerraMsgWrapper>> = deps
        .querier
        .query_all_delegations(&env.contract.address)?
        .into_iter()
        .map(|d| {
            CosmosMsg::Distribution(DistributionMsg::WithdrawDelegatorReward {
                validator: d.validator,
            })
        })
        .collect();

    // Following the reward withdrawal, we dispatch two callbacks: to swap all rewards to Luna, and
    // to stake these Luna to the whitelisted validators
    let callback_msgs = vec![CallbackMsg::Swap {}, CallbackMsg::Reinvest {}]
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg<TerraMsgWrapper>>>>()?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_messages(callback_msgs)
        .add_attribute("action", "steak_hub/harvest"))
}

pub fn swap(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    // Query the amounts of Terra stablecoins available to be swapped
    let coins = deps.querier.query_all_balances(&env.contract.address)?;

    // For each of denom that is not `uluna`, create a message to swap it into Luna
    let msgs: Vec<CosmosMsg<TerraMsgWrapper>> = coins
        .into_iter()
        .filter(|coin| coin.denom != "uluna")
        .map(|coin| create_swap_msg(coin, String::from("uluna")))
        .collect();

    Ok(Response::new().add_messages(msgs).add_attribute("action", "steak_hub/swap"))
}

pub fn reinvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;

    // Query the amount of `uluna` available to be staked
    let uluna_to_bond = deps.querier.query_balance(&env.contract.address, "uluna")?.amount;

    // Compute the amount of `uluna` to be delegated to each validator, based on the amounts of
    // delegations they currently have
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let new_delegations = compute_delegations(uluna_to_bond, &delegations);

    Ok(Response::new()
        .add_messages(new_delegations.iter().map(|d| d.to_cosmos_msg()))
        .add_attribute("action", "steak_hub/reinvest"))
}

//--------------------------------------------------------------------------------------------------
// Unbonding logics
//--------------------------------------------------------------------------------------------------

pub fn queue_unbond(
    deps: DepsMut,
    env: Env,
    staker_addr: Addr,
    usteak_to_burn: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    // Update the pending batch data
    let mut pending_batch = state.pending_batch.load(deps.storage)?;
    pending_batch.usteak_to_burn += usteak_to_burn;
    state.pending_batch.save(deps.storage, &pending_batch)?;

    // Update the user's requested unbonding amount
    state.unbond_shares.update(
        deps.storage,
        (pending_batch.id.into(), &staker_addr),
        |x| -> StdResult<_> {
            let mut unbond_share = x.unwrap_or_else(|| UnbondShare {
                id: pending_batch.id,
                user: staker_addr.to_string(),
                shares: Uint128::zero(),
            });
            unbond_share.shares += usteak_to_burn;
            Ok(unbond_share)
        },
    )?;

    // If the current batch's estimated unbonding start time is reached, then submit it for unbonding
    let mut msgs: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    if env.block.time.seconds() >= pending_batch.est_unbond_start_time {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.into(),
            msg: to_binary(&ExecuteMsg::SubmitBatch {})?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "steak_hub/queue_unbond")
        .add_attribute("staker", staker_addr)
        .add_attribute("usteak_to_burn", usteak_to_burn))
}

pub fn submit_batch(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;
    let unbond_period = state.unbond_period.load(deps.storage)?;
    let pending_batch = state.pending_batch.load(deps.storage)?;

    // The current batch can only be unbonded once the estimated unbonding time has been reached
    let current_time = env.block.time.seconds();
    if current_time < pending_batch.est_unbond_start_time {
        return Err(StdError::generic_err(format!(
            "batch can only be submitted for unbonding after {}",
            pending_batch.est_unbond_start_time
        )));
    }

    // Query the delegations made by Steak Hub to validators, as well as the total supply of Steak
    // token, which we will use to compute stuff
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    // Compute the amount of `uluna` to unbond
    let uluna_to_unbond =
        compute_unbond_amount(usteak_supply, pending_batch.usteak_to_burn, &delegations);

    // Compute the amount of `uluna` to undelegate from each validator
    let new_undelegations = compute_undelegations(uluna_to_unbond, &delegations);

    // Save the current pending batch to the previous batches map
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

    // Create the next pending batch
    let epoch_period = state.epoch_period.load(deps.storage)?;
    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: pending_batch.id + 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: current_time + epoch_period,
        },
    )?;

    Ok(Response::new()
        .add_messages(new_undelegations.iter().map(|d| d.to_cosmos_msg()))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: steak_token.into(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: pending_batch.usteak_to_burn,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "steak_hub/unbond")
        .add_attribute("batch_id", pending_batch.id.to_string())
        .add_attribute("usteak_burned", pending_batch.usteak_to_burn)
        .add_attribute("uluna_unbonded", uluna_to_unbond))
}

pub fn withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    staker_addr: Addr,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let current_time = env.block.time.seconds();

    // Fetch the user's unclaimed unbonding shares
    //
    // NOTE: If the user has too many unclaimed shares, this may not fit in the WASM memory... But
    // this practically is never going to happen in practice. Who would create hundreds of unbonding
    // requests and never claim them?
    let unbond_shares = state
        .unbond_shares
        .idx
        .user
        .prefix(staker_addr.to_string())
        .range(deps.storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<UnbondShare>>>()?;

    // Enumerate through the user's all unclaimed unbonding shares. For each share, check whether
    // its batch has finished unbonding. It yes, increment the amount of uluna to refund the user,
    // and remove this unbonding request from the active queue
    //
    // If a batch has been completely refunded (i.e. total shares = 0), remove it from storage
    let mut total_uluna_to_refund = Uint128::zero();
    for unbond_share in &unbond_shares {
        let mut batch = state.previous_batches.load(deps.storage, unbond_share.id.into())?;
        if batch.est_unbond_end_time < current_time {
            let uluna_to_refund =
                batch.uluna_unclaimed.multiply_ratio(unbond_share.shares, batch.total_shares);
            
            total_uluna_to_refund += uluna_to_refund;
            batch.total_shares -= unbond_share.shares;
            batch.uluna_unclaimed -= uluna_to_refund;

            if batch.total_shares.is_zero() {
                state.previous_batches.remove(deps.storage, unbond_share.id.into());
            }

            state.unbond_shares.remove(deps.storage, (unbond_share.id.into(), &staker_addr))?;
        }
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: staker_addr.clone().into(),
            amount: vec![Coin::new(total_uluna_to_refund.u128(), "uluna")],
        }))
        .add_attribute("action", "steak_hub/withdraw_unbonded")
        .add_attribute("staker", staker_addr)
        .add_attribute("uluna_refunded", total_uluna_to_refund))
}
