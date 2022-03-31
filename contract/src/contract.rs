use crate::helpers::{parse_received_fund, unwrap_reply};
use crate::msg::{
    CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, PendingBatch, QueryMsg, ReceiveMsg,
};
use crate::state::State;
use crate::{execute, queries};

use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult, Uint128,
};
use cw20::Cw20ReceiveMsg;
use terra_cosmwasm::TerraMsgWrapper;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    execute::instantiate(deps, env, info, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response<TerraMsgWrapper>> {
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
        ExecuteMsg::Bond {} => {
            execute::bond(deps, env, info.sender, parse_received_fund(&info.funds, "uluna")?)
        },
        ExecuteMsg::Harvest {} => execute::harvest(deps, env, info.sender),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::WithdrawUnbonded {} => execute::withdraw_unbonded(deps, env, info.sender),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
    }
}

fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response<TerraMsgWrapper>> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::QueueUnbond {} => {
            let state = State::default();

            let steak_token = state.steak_token.load(deps.storage)?;
            if info.sender != steak_token {
                return Err(StdError::generic_err(format!(
                    "expecting STEAK token, received {}",
                    info.sender
                )));
            }

            execute::queue_unbond(deps, env, api.addr_validate(&cw20_msg.sender)?, cw20_msg.amount)
        },
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cb_msg: CallbackMsg,
) -> StdResult<Response<TerraMsgWrapper>> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("callbacks can only be invoked by the contract itself"));
    }

    match cb_msg {
        CallbackMsg::Swap {} => execute::swap(deps, env),
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => execute::register_steak_token(deps, unwrap_reply(reply)?),
        id => Err(StdError::generic_err(format!("invalid reply id: {}", id))),
    }
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::PendingBatch {} => to_binary(&queries::query_pending_batch(deps)?),
        QueryMsg::PreviousBatches {
            start_after,
            limit,
        } => to_binary(&queries::query_previous_batches(deps, start_after, limit)?),
        QueryMsg::UnbondRequestsByBatch {
            id,
            start_after,
            limit,
        } => to_binary(&queries::query_unbond_requests_by_batch(deps, id, start_after, limit)?),
        QueryMsg::UnbondRequestsByUser {
            user,
            start_after,
            limit,
        } => to_binary(&queries::query_unbond_requests_by_user(deps, user, start_after, limit)?),
    }
}

#[entry_point]
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let state = State::default();
    state.epoch_period.save(deps.storage, &(4 * 60 * 60))?; // 4 hours; for testing
    state.unbond_period.save(deps.storage, &(24 * 60 * 60))?; // 24 hrs; for testing
    state.pending_batch.save(
        deps.storage,
        &PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: env.block.time.seconds(),
        },
    )?;

    Ok(Response::new())
}
