use cosmwasm_std::{
    Binary, Decimal, Deps, DepsMut, entry_point, Env, from_binary, MessageInfo, Reply, Response,
    StdError, StdResult, to_binary,
};
use cw2::{ContractVersion, get_contract_version, set_contract_version};
use cw20::Cw20ReceiveMsg;

use pfc_steak::hub::{CallbackMsg, ExecuteMsg, FeeType, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};

use crate::{execute, queries};
use crate::helpers::{get_denom_balance, unwrap_reply};
use crate::migrations::ConfigV100;
use crate::state::State;

/// Contract name that is used for migration.
pub const CONTRACT_NAME: &str = "steak-hub";
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
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    execute::instantiate(deps, env, msg)
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let api = deps.api;
    match msg {
        ExecuteMsg::Receive(cw20_msg) => receive(deps, env, info, cw20_msg),
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
        ExecuteMsg::CollectDust {} => { execute::collect_dust(deps, env) }
        ExecuteMsg::ReturnDenom {} => { execute::return_denom(deps, env, info.funds) }
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
        ReceiveMsg::QueueUnbond { receiver } => {
            let state = State::default();

            let steak_token = state.steak_token.load(deps.storage)?;
            if info.sender != steak_token {
                return Err(StdError::generic_err(format!(
                    "expecting Steak token, received {}",
                    info.sender
                )));
            }

            execute::queue_unbond(
                deps,
                env,
                api.addr_validate(&receiver.unwrap_or(cw20_msg.sender))?,
                cw20_msg.amount,
            )
        }
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
        1 => execute::register_steak_token(deps, unwrap_reply(reply)?),
        REPLY_REGISTER_RECEIVED_COINS => {
            execute::register_received_coins(deps, env, unwrap_reply(reply)?.events)
        }
        id => Err(StdError::generic_err(format!(
            "invalid reply id: {}; must be 1-2",
            id
        ))),
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
pub fn migrate(deps: DepsMut, env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    let contract_version = match get_contract_version(deps.storage) {
        Ok(version) => version,
        Err(_) => ContractVersion {
            contract: "steak-hub".to_string(),
            version: "0".to_string(),
        },
    };
    match contract_version.contract.as_ref() {
        #[allow(clippy::single_match)]
        "steak-hub" => match contract_version.version.as_ref() {
            #[allow(clippy::single_match)]
            "0" => {
                let state = State::default();
                let owner = state.owner.load(deps.storage)?;
                state.denom.save(deps.storage, &"uluna".to_string())?;
                state.fee_account.save(deps.storage, &owner)?;
                state.max_fee_rate.save(deps.storage, &Decimal::zero())?;
                state.fee_rate.save(deps.storage, &Decimal::zero())?;
                state.fee_account_type.save(deps.storage, &FeeType::Wallet)?;
                ConfigV100::upgrade_stores(deps.storage, &deps.querier, env.contract.address)?;
                state.dust_collector.save(deps.storage, &None)?;
            }
            "2.1.4" => {
                let state = State::default();
                ConfigV100::upgrade_stores(deps.storage, &deps.querier, env.contract.address)?;
                state.fee_account_type.save(deps.storage, &FeeType::Wallet)?;
                state.dust_collector.save(deps.storage, &None)?;
            }
            "2.1.5" => {
                ConfigV100::upgrade_stores(deps.storage, &deps.querier, env.contract.address)?;
                let state = State::default();
                state.fee_account_type.save(deps.storage, &FeeType::Wallet)?;
                state.dust_collector.save(deps.storage, &None)?;
            }
            "2.1.6" | "2.1.7" => {
                let state = State::default();
                // note: this is also done in ConfigV100::upgrade
                let denom = state.denom.load(deps.storage)?;
                state.prev_denom.save(
                    deps.storage,
                    &get_denom_balance(&deps.querier, env.contract.address, denom)?,
                )?;

                state.fee_account_type.save(deps.storage, &FeeType::Wallet)?;
                state.dust_collector.save(deps.storage, &None)?;
            }
            "2.1.8" |"2.1.16"=> {
                let state = State::default();
                state.fee_account_type.save(deps.storage, &FeeType::Wallet)?;
                state.dust_collector.save(deps.storage, &None)?;
            }
            "3.0.1" => {
                let state = State::default();
                state.dust_collector.save(deps.storage, &None)?;
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
