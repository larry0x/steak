use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, Deps, DepsMut, DistributionMsg, Env, MessageInfo,
    Order, Response, StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use cw_storage_plus::U64Key;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::math::{
    compute_delegations, compute_mint_amount, compute_unbond_amount, compute_undelegations,
};
use crate::msg::{CallbackMsg, ConfigResponse, ExecuteMsg, InstantiateMsg};
use crate::state::{Batch, PendingBatch, State};

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

    let worker_addrs = msg
        .workers
        .iter()
        .map(|s| deps.api.addr_validate(s))
        .collect::<StdResult<Vec<Addr>>>()?;

    state.workers.save(deps.storage, &worker_addrs)?;
    state.validators.save(deps.storage, &msg.validators)?;
    state.epoch_period.save(deps.storage, &msg.epoch_period)?;
    state.unbond_period.save(deps.storage, &msg.unbond_period)?;

    state.current_batch.save(
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

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "steak_hub/swap"))
}

pub fn reinvest(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;

    // Query the amount of `uluna` available to be staked
    let uluna_to_bond = deps
        .querier
        .query_balance(&env.contract.address, "uluna")?
        .amount;

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
    let mut current_batch = state.current_batch.load(deps.storage)?;
    current_batch.usteak_to_burn = current_batch.usteak_to_burn.checked_add(usteak_to_burn)?;
    state.current_batch.save(deps.storage, &current_batch)?;

    // Update the user's requested unbonding amount
    state
        .active_requests
        .update(deps.storage, (&staker_addr, current_batch.id.into()), |x| {
            x.unwrap_or_else(Uint128::zero)
                .checked_add(usteak_to_burn)
                .map_err(StdError::overflow)
        })?;

    // If the current batch's estimated unbonding start time is reached, then submit it for unbonding
    let mut msgs: Vec<CosmosMsg<TerraMsgWrapper>> = vec![];
    if env.block.time.seconds() >= current_batch.est_unbond_start_time {
        msgs.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: env.contract.address.into(),
            msg: to_binary(&ExecuteMsg::Unbond {})?,
            funds: vec![],
        }));
    }

    Ok(Response::new()
        .add_messages(msgs)
        .add_attribute("action", "steak_hub/queue_unbond")
        .add_attribute("staker", staker_addr)
        .add_attribute("usteak_to_burn", usteak_to_burn))
}

pub fn unbond(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();

    // The current batch can only be unbonded once the estimated unbonding time has been reached
    let current_time = env.block.time.seconds();
    let current_batch = state.current_batch.load(deps.storage)?;
    if current_time < current_batch.est_unbond_start_time {
        return Err(StdError::generic_err(format!(
            "batch can only be submitted for unbonding after {}",
            current_batch.est_unbond_start_time
        )));
    }

    // Query the delegations made by Steak Hub to validators, as well as the total supply of Steak
    // token, which we will use to compute stuff
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;

    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    // Compute the amount of `uluna` to unbond
    let uluna_to_unbond =
        compute_unbond_amount(usteak_supply, current_batch.usteak_to_burn, &delegations);

    // Compute the amount of `uluna` to undelegate from each validator
    let new_undelegations = compute_undelegations(uluna_to_unbond, &delegations);

    // Save the current pending batch to the previous batches map
    state.previous_batches.save(
        deps.storage,
        current_batch.id.into(),
        &Batch {
            uluna_unbonded: uluna_to_unbond,
            usteak_burned: current_batch.usteak_to_burn,
            unbond_start_time: current_time,
        },
    )?;

    // Create the next pending batch
    let epoch_period = state.epoch_period.load(deps.storage)?;
    state.current_batch.save(
        deps.storage,
        &PendingBatch {
            id: current_batch.id + 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: current_time + epoch_period,
        },
    )?;

    Ok(Response::new()
        .add_messages(new_undelegations.iter().map(|d| d.to_cosmos_msg()))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: steak_token.into(),
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: current_batch.usteak_to_burn,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "steak_hub/unbond")
        .add_attribute("batch_id", current_batch.id.to_string())
        .add_attribute("usteak_burned", current_batch.usteak_to_burn)
        .add_attribute("uluna_unbonded", uluna_to_unbond))
}

pub fn withdraw_unbonded(
    deps: DepsMut,
    env: Env,
    staker_addr: Addr,
) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let current_time = env.block.time.seconds();
    let unbond_period = state.unbond_period.load(deps.storage)?;

    // Grab the IDs of all previous batches where the user has an active request in, ordered
    // ascendingly, i.e. the oldest batch first
    let ids: Vec<U64Key> = state
        .active_requests
        .prefix(&staker_addr)
        .keys(deps.storage, None, None, Order::Ascending)
        .map(U64Key::from)
        .collect();

    // For each batch, check whether it has finished unbonding. It yes, increment the amount of `uluna`
    // to refund the user, and remove this unbonding request from the active queue
    let mut uluna_to_refund = Uint128::zero();
    for id in ids {
        let batch = state.previous_batches.load(deps.storage, id.clone())?;
        let key = (&staker_addr, id);
        let request = state.active_requests.load(deps.storage, key.clone())?;
        if batch.unbond_start_time + unbond_period <= current_time {
            uluna_to_refund = uluna_to_refund.checked_add(
                batch
                    .uluna_unbonded
                    .multiply_ratio(request, batch.usteak_burned),
            )?;
            state.active_requests.remove(deps.storage, key);
        }
    }

    Ok(Response::new()
        .add_message(CosmosMsg::Bank(BankMsg::Send {
            to_address: staker_addr.clone().into(),
            amount: vec![Coin::new(uluna_to_refund.u128(), "uluna")],
        }))
        .add_attribute("action", "steak_hub/withdraw_unbonded")
        .add_attribute("staker", staker_addr)
        .add_attribute("uluna_refunded", uluna_to_refund))
}

//--------------------------------------------------------------------------------------------------
// Queries
//--------------------------------------------------------------------------------------------------

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    let worker_addrs = state.workers.load(deps.storage)?;
    Ok(ConfigResponse {
        steak_token: state.steak_token.load(deps.storage)?.into(),
        workers: worker_addrs.iter().map(|addr| addr.to_string()).collect(),
        validators: state.validators.load(deps.storage)?,
    })
}
