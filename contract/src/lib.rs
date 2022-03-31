pub mod contract;
pub mod helpers;
pub mod math;
pub mod msg;
pub mod state;
pub mod types;

#[cfg(not(feature = "library"))]
pub mod entry {
    use crate::state::PendingBatch;

    use super::contract;
    use super::helpers::{parse_received_fund, unwrap_reply};
    use super::msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};
    use super::state::State;

    use cosmwasm_std::{
        entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
        Response, StdError, StdResult, Uint128,
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
        contract::instantiate(deps, env, info, msg)
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
            ExecuteMsg::Bond {} => contract::bond(
                deps,
                env,
                info.sender,
                parse_received_fund(&info.funds, "uluna")?,
            ),
            ExecuteMsg::Harvest {} => contract::harvest(deps, env, info.sender),
            ExecuteMsg::SubmitBatch {} => contract::submit_batch(deps, env),
            ExecuteMsg::WithdrawUnbonded {} => contract::withdraw_unbonded(deps, env, info.sender),
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

                contract::queue_unbond(
                    deps,
                    env,
                    api.addr_validate(&cw20_msg.sender)?,
                    cw20_msg.amount,
                )
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
            return Err(StdError::generic_err(
                "callbacks can only be invoked by the contract itself",
            ));
        }

        match cb_msg {
            CallbackMsg::Swap {} => contract::swap(deps, env),
            CallbackMsg::Reinvest {} => contract::reinvest(deps, env),
        }
    }

    #[entry_point]
    pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
        match reply.id {
            1 => contract::register_steak_token(deps, unwrap_reply(reply)?),
            id => Err(StdError::generic_err(format!("invalid reply id: {}", id))),
        }
    }

    #[entry_point]
    pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Config {} => to_binary(&contract::query_config(deps)?),
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
}
