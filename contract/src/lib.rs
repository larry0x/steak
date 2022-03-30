pub mod contract;
pub mod helpers;
pub mod math;
pub mod msg;
pub mod state;
pub mod types;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::contract;
    use super::helpers::{parse_received_fund, unwrap_reply};
    use super::msg::{CallbackMsg, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ReceiveMsg};
    use super::state::State;

    use cosmwasm_std::{
        entry_point, from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply,
        Response, StdError, StdResult,
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
            ExecuteMsg::Stake {} => contract::stake(
                deps,
                env,
                info.sender,
                parse_received_fund(&info.funds, "uluna")?,
            ),
            ExecuteMsg::Harvest {} => contract::harvest(deps, env, info.sender),
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
            ReceiveMsg::Unstake {} => {
                let state = State::default();

                let steak_token = state.steak_token.load(deps.storage)?;
                if info.sender != steak_token {
                    return Err(StdError::generic_err(format!(
                        "expecting STEAK token, received {}",
                        info.sender
                    )));
                }

                contract::unstake(
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
            CallbackMsg::Restake {} => contract::restake(deps, env),
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
    pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
        Ok(Response::new())
    }
}
