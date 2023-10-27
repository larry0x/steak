use cosmwasm_std::{Binary, Deps, DepsMut, entry_point, Env, MessageInfo, Reply, Response, StdError, StdResult, to_binary};
use cw2::{ContractVersion, get_contract_version, set_contract_version};

use pfc_steak::hub::{CallbackMsg, MigrateMsg, QueryMsg};
use pfc_steak::hub_tf::{ExecuteMsg, InstantiateMsg, TokenFactoryType};

use crate::{execute, queries};
use crate::state::State;

//use crate::helpers::{ unwrap_reply};

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "steak-hub-tf";
/// Contract version that is used for migration.
pub const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPLY_INSTANTIATE_TOKEN: u64 = 1;
pub const REPLY_REGISTER_RECEIVED_COINS: u64 = 2;
pub const SPECIAL_SEND_MESSAGE_TO_TRANSFER: &str = "PFC_TRANSFER_NOT_SEND";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, StdError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Bond { receiver, exec_msg } => execute::bond(
            deps,
            env,
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender),
            info.funds,
            exec_msg,
        ),
        ExecuteMsg::Unbond { receiver } => execute::queue_unbond(
            deps,
            env,
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender), info.funds,
        ),
        ExecuteMsg::WithdrawUnbonded { receiver } => execute::withdraw_unbonded(
            deps,
            env,
            info.sender.clone(),
            receiver
                .map(|s| api.addr_validate(&s))
                .transpose()?
                .unwrap_or(info.sender),
        ),
        ExecuteMsg::WithdrawUnbondedAdmin { address } => {
            execute::withdraw_unbonded_admin(deps, env, info.sender, api.addr_validate(&address)?)
        }
        ExecuteMsg::AddValidator { validator } => {
            execute::add_validator(deps, info.sender, validator)
        }
        ExecuteMsg::RemoveValidator { validator } => {
            execute::remove_validator(deps, env, info.sender, validator)
        }
        ExecuteMsg::RemoveValidatorEx { validator } => {
            execute::remove_validator_ex(deps, env, info.sender, validator)
        }
        ExecuteMsg::Redelegate { validator_from,validator_to } => {
            execute::redelegate(deps, env, info.sender, validator_from,validator_to)
        }        

        ExecuteMsg::TransferOwnership { new_owner } => {
            execute::transfer_ownership(deps, info.sender, new_owner)
        }
        ExecuteMsg::AcceptOwnership {} => execute::accept_ownership(deps, info.sender),
        ExecuteMsg::Harvest {} => execute::harvest(deps, env),
        ExecuteMsg::Rebalance { minimum } => execute::rebalance(deps, env, minimum),
        ExecuteMsg::Reconcile {} => execute::reconcile(deps, env),
        ExecuteMsg::SubmitBatch {} => execute::submit_batch(deps, env),
        ExecuteMsg::TransferFeeAccount { fee_account_type, new_fee_account } => {
            execute::transfer_fee_account(deps, info.sender, fee_account_type, new_fee_account)
        }
        ExecuteMsg::UpdateFee { new_fee } => execute::update_fee(deps, info.sender, new_fee),
        ExecuteMsg::Callback(callback_msg) => callback(deps, env, info, callback_msg),
        ExecuteMsg::PauseValidator { validator } => {
            execute::pause_validator(deps, env, info.sender, validator)
        }
        ExecuteMsg::UnPauseValidator { validator } => {
            execute::unpause_validator(deps, env, info.sender, validator)
        }
        ExecuteMsg::SetUnbondPeriod { unbond_period } => execute::set_unbond_period(deps, env, info.sender, unbond_period),

        ExecuteMsg::SetDustCollector { dust_collector } => { execute::set_dust_collector(deps, env, info.sender, dust_collector) }
        ExecuteMsg::CollectDust { max_tokens } => { execute::collect_dust(deps, env, max_tokens) }
        ExecuteMsg::ReturnDenom {} => { execute::return_denom(deps, env, info.funds) }
    }
}


fn callback(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    callback_msg: CallbackMsg,
) -> StdResult<Response> {
    if env.contract.address != info.sender {
        return Err(StdError::generic_err(
            "callbacks can only be invoked by the contract itself",
        ));
    }

    match callback_msg {
        CallbackMsg::Reinvest {} => execute::reinvest(deps, env),
    }
}

#[entry_point]
pub fn reply(deps: DepsMut, env: Env, reply: Reply) -> StdResult<Response> {
    match reply.id {
        REPLY_REGISTER_RECEIVED_COINS => {
            execute::collect_dust(deps, env,10)
        }
        _ => {
            Err(StdError::generic_err(format!(
                "invalid reply id: {}  {:?}",
                reply.id,
                reply.result
            ))
            )
        }
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
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let contract_version = match get_contract_version(deps.storage) {
        Ok(version) => version,
        Err(_) => ContractVersion {
            contract: "steak-hub-tf".to_string(),
            version: "0".to_string(),
        },
    };
    match contract_version.contract.as_ref() {
        #[allow(clippy::single_match)]
        "steak-hub-tf" => match contract_version.version.as_ref() {
            #[allow(clippy::single_match)]
            "0" => {}
            "3.0.1" | "3.0.2" => {
                let  state = State::default();
                let kuji = state.kuji_token_factory.load(deps.storage)?;
                if kuji {
                    state.token_factory_type.save(deps.storage,&TokenFactoryType::Kujira)?
                } else {
                    state.token_factory_type.save(deps.storage,&TokenFactoryType::CosmWasm)?
                }

            }
                _ => {}
        },
        _ => {
            return Err(StdError::generic_err(
                "contract name is not the same. aborting {}",
            ));
        }
    }
    /*
    let state = State::default();

    state.max_fee_rate.save(deps.storage,&Decimal::from_ratio(10u32,100u32))?;
    state.fee_rate.save(deps.storage,&Decimal::from_ratio(10u32,100u32))?;

     */
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("previous_contract_name", &contract_version.contract)
        .add_attribute("previous_contract_version", &contract_version.version)
        .add_attribute("new_contract_name", CONTRACT_NAME)
        .add_attribute("new_contract_version", CONTRACT_VERSION))
}
