use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult,
};

use osmo_bindings::OsmosisMsg;
use steak::hub::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::error::ContractError;
use crate::helpers::{parse_received_fund, unwrap_reply};
use crate::{execute, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Bond { receiver } => execute::bond(
            deps,
            env,
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender),
            parse_received_fund(&info.funds, "uosmo")?,
        ),
        ExecuteMsg::WithdrawUnbonded { receiver } => execute::withdraw_unbonded(
            deps,
            env,
            info.sender.clone(),
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender.clone()),
        ),
        ExecuteMsg::AddValidator { validator } => {
            execute::add_validator(deps, info.sender, validator)
        }
        ExecuteMsg::RemoveValidator { validator } => {
            execute::remove_validator(deps, env, info.sender, validator)
        }
        ExecuteMsg::TransferOwnership { new_owner } => {
            execute::transfer_ownership(deps, info.sender.clone(), new_owner)
        }
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender.clone()),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env),
        ExecuteMsg::Rebalance {} => execute::rebalance(deps, env),
        ExecuteMsg::Reconcile {} => execute::reconcile(deps, env),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::QueueUnbond { receiver } => execute::queue_unbond(
            deps,
            env,
            info.clone(),
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender.clone()),
        ),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    if env.contract.address != info.sender {
        return Err(ContractError::InvalidCallbackSender {});
    }

    match callback_msg {
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response<OsmosisMsg>, ContractError> {
    match reply.id {
        1 => execute::register_received_coins(deps, env, unwrap_reply(reply)?.events),
        id => Err(ContractError::InvalidReplyId { id }),
    }
}

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::config(deps)?),
        QueryMsg::State {} => to_binary(&queries::state(deps, env)?),
        QueryMsg::PendingBatch {} => to_binary(&queries::pending_batch(deps)?),
        QueryMsg::PreviousBatch(id) => to_binary(&queries::previous_batch(deps, id)?),
        QueryMsg::PreviousBatches { start_after, limit } => {
            to_binary(&queries::previous_batches(deps, start_after, limit)?)
        }
        QueryMsg::UnbondRequestsByBatch {
            id,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_batch(
            deps,
            id,
            start_after,
            limit,
        )?),
        QueryMsg::UnbondRequestsByUser {
            user,
            start_after,
            limit,
        } => to_binary(&queries::unbond_requests_by_user(
            deps,
            user,
            start_after,
            limit,
        )?),
    }
}

#[entry_point]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::new())
}
