use cosmwasm_std::{
    entry_point, from_binary, to_binary, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Reply, Response, StdError, StdResult, WasmMsg,
};
use cw20::Cw20ReceiveMsg;
use osmo_bindings::OsmosisMsg;
use steak::hub::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};
use steak::vault_token::Token;

use crate::error::ContractError;
use crate::helpers::{parse_received_fund, unwrap_reply};
use crate::state::State;
use crate::{execute, queries};

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    match msg.token_init_info {
        steak::vault_token::TokenInitInfo::Osmosis { subdenom } => {
            Token::Osmosis { denom: msg.name }.instantiate(deps, env, info, msg)
        }
        steak::vault_token::TokenInitInfo::Cw20 {
            label,
            admin,
            code_id,
            cw20_init_msg,
        } => todo!(),
    }
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
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
            None,
        ),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
    }
}

fn receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response<OsmosisMsg>, ContractError> {
    let api = deps.api;
    match from_binary(&cw20_msg.msg)? {
        ReceiveMsg::QueueUnbond { receiver } => {
            let state = State::default();

            let steak_token = state.steak_token.load(deps.storage)?;
            if (Token::Cw20 {
                address: info.sender,
            } != steak_token)
            {
                return Err(StdError::generic_err(format!(
                    "expecting Steak token, received {}",
                    info.sender
                ))
                .into());
            }

            execute::queue_unbond(
                deps,
                env,
                info,
                api.addr_validate(&receiver.unwrap_or(cw20_msg.sender))?,
                Some(cw20_msg.amount),
            )
        }
    }
}

fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> Result<Response, ContractError> {
    if env.contract.address != info.sender {
        return Err(ContractError::InvalidCallbackSender {});
    }

    match callback_msg {
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> Result<Response, ContractError> {
    match reply.id {
        1 => execute::register_received_coins(deps, env, unwrap_reply(reply)?.events),
        // TODO: Call vault token reply function
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
