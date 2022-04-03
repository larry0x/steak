use cosmwasm_std::{Reply, StdError, StdResult, SubMsgExecutionResponse};

/// Unwrap a `Reply` object to extract the response
pub fn unwrap_reply(reply: Reply) -> StdResult<SubMsgExecutionResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}
