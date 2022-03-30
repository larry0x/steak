use cosmwasm_std::{
    Addr, QuerierWrapper, Reply, StdError, StdResult, SubMsgExecutionResponse, Uint128, Coin
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

use crate::types::Delegation;

//--------------------------------------------------------------------------------------------------
// Queriers
//--------------------------------------------------------------------------------------------------

/// Query the total supply of a CW20 token
pub(crate) fn query_cw20_total_supply(
    querier: &QuerierWrapper,
    token_addr: &Addr,
) -> StdResult<Uint128> {
    let token_info: TokenInfoResponse =
        querier.query_wasm_smart(token_addr, &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

/// Query the amounts of Luna a staker is delegating to each of the validators specified
pub(crate) fn query_delegations(
    querier: &QuerierWrapper,
    validators: &[String],
    delegator_addr: &Addr,
) -> StdResult<Vec<Delegation>> {
    validators
        .iter()
        .map(|validator| Delegation::query(querier, validator, delegator_addr))
        .collect()
}

//--------------------------------------------------------------------------------------------------
// Utilities
//--------------------------------------------------------------------------------------------------

/// Find the amount of a denom sent along a message, assert it is non-zero, and no other denom were
/// sent together
pub(crate) fn parse_received_fund(funds: &[Coin], denom: &str) -> StdResult<Uint128> {
    // Deposit must contain only 1 coin; this coin must be Luna; the amount must be non-zero
    if funds.len() > 1 {
        return Err(StdError::generic_err("more than one coins deposited"));
    }

    let fund = &funds[0];
    if fund.denom != denom {
        return Err(StdError::generic_err(
            format!("expected {} deposit, received {}", denom, fund.denom)
        ));
    }

    if fund.amount.is_zero() {
        return Err(StdError::generic_err("deposit amount must be non-zero"));
    }

    Ok(fund.amount)
}

/// Unwrap a `Reply` object to extract the response
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgExecutionResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}
