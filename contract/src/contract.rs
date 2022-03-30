use cosmwasm_std::{
    from_binary, to_binary, Addr, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response, StdError,
    StdResult, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::math::{compute_delegations, compute_mint_amount};
use crate::msg::{ConfigResponse, InstantiateMsg, ReceiveMsg};
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

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

pub fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::Unstake {} => {
            let state = State::default();

            let steak_token = state.steak_token.load(deps.storage)?;
            if info.sender != steak_token {
                return Err(StdError::generic_err(format!(
                    "expecting STEAK token, received {}",
                    info.sender
                )));
            }

            execute_unstake(
                deps,
                env,
                api.addr_validate(&cw20_msg.sender)?,
                cw20_msg.amount,
            )
        },
    }
}

pub fn execute_stake(
    deps: DepsMut,
    env: Env,
    staker_addr: Addr,
    uluna_to_stake: Uint128,
) -> StdResult<Response> {
    let state = State::default();
    let steak_token = state.steak_token.load(deps.storage)?;
    let validators = state.validators.load(deps.storage)?;

    // Query the delegations made by Steak Hub to validators, as well as the total supply of Steak
    // token, which we will use to compute stuff
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let usteak_supply = query_cw20_total_supply(&deps.querier, &steak_token)?;

    // Compute the amount of `uluna` to be delegated to each validator
    let new_delegations = compute_delegations(uluna_to_stake, &delegations);

    // Compute the amount of `usteak` to mint
    let usteak_to_mint = compute_mint_amount(usteak_supply, uluna_to_stake, &delegations);

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
        .add_attribute("action", "steak_hub/execute/stake")
        .add_attribute("staker", staker_addr)
        .add_attribute("amount_staked", uluna_to_stake))
}

pub fn execute_unstake(
    _deps: DepsMut,
    _env: Env,
    _staker_addr: Addr,
    _usteak_to_burn: Uint128,
) -> StdResult<Response> {
    Err(StdError::generic_err("WIP"))
}

pub fn execute_harvest(deps: DepsMut, _env: Env, worker_addr: Addr) -> StdResult<Response> {
    let state = State::default();

    let worker_addrs = state.workers.load(deps.storage)?;
    if !worker_addrs.contains(&worker_addr) {
        return Err(StdError::generic_err("sender is not a whitelisted worker"));
    }

    Err(StdError::generic_err("WIP"))
}

//--------------------------------------------------------------------------------------------------
// Replies
//--------------------------------------------------------------------------------------------------

pub fn after_deploying_token(
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
