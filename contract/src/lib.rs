pub mod contract;
pub mod helpers;
pub mod math;
pub mod msg;
pub mod state;
pub mod types;

#[cfg(not(feature = "library"))]
pub mod entry {
    use super::helpers::{parse_received_fund, unwrap_reply};
    use super::msg::*;
    use super::*;

    use cosmwasm_std::{
        entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdError,
        StdResult,
    };

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
    ) -> StdResult<Response> {
        match msg {
            ExecuteMsg::Receive(cw20_msg) => contract::execute_receive(deps, env, info, cw20_msg),
            ExecuteMsg::Stake {} => contract::execute_stake(
                deps,
                env,
                info.sender,
                parse_received_fund(&info.funds, "uluna")?,
            ),
            ExecuteMsg::Harvest {} => contract::execute_harvest(deps, env, info.sender),
        }
    }

    #[entry_point]
    pub fn reply(deps: DepsMut, _env: Env, reply: Reply) -> StdResult<Response> {
        match reply.id {
            1 => contract::after_deploying_token(deps, unwrap_reply(reply)?),
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
