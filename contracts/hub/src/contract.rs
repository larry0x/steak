use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response,
    StdError, StdResult,
};
use cw20::Cw20ReceiveMsg;

use steak::hub::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::helpers::{parse_received_fund, unwrap_reply};
use crate::state::State;
use crate::{execute, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
        ExecuteMsg::Bond {
            receiver,
        } => execute::bond(
            deps,
            env,
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
            parse_received_fund(&info.funds, "uluna")?,
        ),
        ExecuteMsg::WithdrawUnbonded {
            receiver,
        } => execute::withdraw_unbonded(
            deps,
            env,
            info.sender.clone(),
            receiver.map(|s| api.addr_validate(&s)).transpose()?.unwrap_or(info.sender),
        ),
        ExecuteMsg::WithdrawUnbondedAdmin {
            address,
        } => execute::withdraw_unbonded_admin(
            deps,
            env,
            info.sender.clone(),
             api.addr_validate(&address)?,
        ),
        ExecuteMsg::AddValidator {
            validator,
        } => execute::add_validator(deps, info.sender, validator),
        ExecuteMsg::RemoveValidator {
            validator,
        } => execute::remove_validator(deps, env, info.sender, validator),
        ExecuteMsg::TransferOwnership {
            new_owner,
        } => execute::transfer_ownership(deps, info.sender, new_owner),
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env),
        ExecuteMsg::Rebalance {} => execute::rebalance(deps, env),
        ExecuteMsg::Reconcile {} => execute::reconcile(deps, env),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
    }
}

fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> StdResult<Response> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::QueueUnbond {
            receiver,
        } => {
            let state = State::default();

            let steak_token = state.steak_token.load(deps.storage)?;
            if info.sender != steak_token {
                return Err(StdError::generic_err(
                    format!("expecting Steak token, received {}", info.sender),
                ));
            }

            execute::queue_unbond(
                deps,
                env,
                api.addr_validate(&receiver.unwrap_or(cw20_msg.sender))?,
                cw20_msg.amount,
            )
        },
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err("callbacks can only be invoked by the contract itself"));
    }

    match callback_msg {
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        1 => execute::register_steak_token(deps, unwrap_reply(reply)?),
        2 => execute::register_received_coins(deps, env, unwrap_reply(reply)?.events),
        id => Err(StdError::generic_err(format!("invalid reply id: {}; must be 1-2", id))),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps, env)?),
        QueryMsg::PendingBatch {} => to_binary(&queries::pending_batch(deps)?),
        QueryMsg::PreviousBatch(id) => to_binary(&queries::previous_batch(deps, id)?),
        QueryMsg::PreviousBatches {
            start_after,
            limit,
        } => to_binary(&queries::previous_batches(deps, start_after, limit)?),
        QueryMsg::UnbondRequestsByBatch {
            id,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_batch(deps, id, start_after, limit)?),
        QueryMsg::UnbondRequestsByUser {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user(deps, user, start_after, limit)?),
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new())
}
