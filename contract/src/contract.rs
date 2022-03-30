use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, Deps, DepsMut, DistributionMsg, Env, MessageInfo, Response,
    StdError, StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;
use terra_cosmwasm::{create_swap_msg, TerraMsgWrapper};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::math::{compute_delegations, compute_mint_amount};
use crate::msg::{CallbackMsg, ConfigResponse, InstantiateMsg};
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

    let worker_addrs = msg
        .workers
        .iter()
        .map(|s| deps.api.addr_validate(s))
        .collect::<StdResult<Vec<Addr>>>()?;

    state.workers.save(deps.storage, &worker_addrs)?;
    state.validators.save(deps.storage, &msg.validators)?;

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
// Execution
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

    // Compute the amount of `uluna` to be delegated to each validator
    let new_delegations = compute_delegations(uluna_to_bond, &delegations);

    // Compute the amount of `usteak` to mint
    let usteak_to_mint = compute_mint_amount(usteak_supply, uluna_to_bond, &delegations);

    Ok(Response::new()
        .add_messages(new_delegations.iter().map(|d| d.into_cosmos_msg()))
        .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: steak_token.into(),
            msg: to_binary(&Cw20ExecuteMsg::Mint {
                recipient: staker_addr.clone().into(),
                amount: usteak_to_mint,
            })?,
            funds: vec![],
        }))
        .add_attribute("action", "steak_hub/execute/bond")
        .add_attribute("staker", staker_addr)
        .add_attribute("amount_bonded", uluna_to_bond))
}

pub fn unstake(
    _deps: DepsMut,
    _env: Env,
    _staker_addr: Addr,
    _usteak_to_burn: Uint128,
) -> StdResult<Response<TerraMsgWrapper>> {
    Err(StdError::generic_err("WIP"))
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
    let callback_msgs = vec![CallbackMsg::Swap {}, CallbackMsg::Restake {}]
        .iter()
        .map(|callback| callback.into_cosmos_msg(&env.contract.address))
        .collect::<StdResult<Vec<CosmosMsg<TerraMsgWrapper>>>>()?;

    Ok(Response::new()
        .add_messages(msgs)
        .add_messages(callback_msgs)
        .add_attribute("action", "steak_hub/execute/harvest"))
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
        .add_attribute("action", "steak_hub/callback/swap"))
}

pub fn restake(deps: DepsMut, env: Env) -> StdResult<Response<TerraMsgWrapper>> {
    let state = State::default();
    let validators = state.validators.load(deps.storage)?;

    // Query the amount of `uluna` available to be staked
    let uluna_to_stake = deps
        .querier
        .query_balance(&env.contract.address, "uluna")?
        .amount;

    // Compute the amount of `uluna` to be delegated to each validator, based on the amounts of
    // delegations they currently have
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let new_delegations = compute_delegations(uluna_to_stake, &delegations);

    Ok(Response::new()
        .add_messages(new_delegations.iter().map(|d| d.into_cosmos_msg()))
        .add_attribute("action", "steak_hub/callback/restake"))
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
        validators: state.validators.load(deps.storage)?.into(),
    })
}
