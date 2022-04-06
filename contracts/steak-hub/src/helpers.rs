use std::str::FromStr;

use cosmwasm_std::{Addr, Coin, QuerierWrapper, StdError, StdResult, Uint128};
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
    let token_info: TokenInfoResponse = querier.query_wasm_smart(token_addr, &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

/// Query the amounts of Luna a staker is delegating to a specific validator
pub(crate) fn query_delegation(
    querier: &QuerierWrapper,
    validator: &str,
    delegator_addr: &Addr,
) -> StdResult<Delegation> {
    Ok(Delegation {
        validator: validator.to_string(),
        amount: querier.query_delegation(delegator_addr, validator)?.map(|fd| fd.amount.amount).unwrap_or_else(Uint128::zero),
    })
}

/// Query the amounts of Luna a staker is delegating to each of the validators specified
pub(crate) fn query_delegations(
    querier: &QuerierWrapper,
    validators: &[String],
    delegator_addr: &Addr,
) -> StdResult<Vec<Delegation>> {
    validators
        .iter()
        .map(|validator| query_delegation(querier, validator, delegator_addr))
        .collect()
}

//--------------------------------------------------------------------------------------------------
// Utilities
//--------------------------------------------------------------------------------------------------

/// `cosmwasm_std::Coin` does not implement `FromStr`, so we have do it ourselves
///
/// Parsing the string with regex doesn't work, because the resulting binary would be too big for
/// including the `regex` library. Example:
/// https://github.com/PFC-Validator/terra-rust/blob/v1.1.8/terra-rust-api/src/client/core_types.rs#L34-L55
///
/// We opt for a dirtier solution. Enumerate characters in the string, and break before the first
/// character that is not a number. Split the string at that index.
///
/// This assumes the denom never starts with a number, which is true on Terra.
pub(crate) fn parse_coin(s: &str) -> StdResult<Coin> {
    for (i, c) in s.chars().enumerate() {
        if c.is_alphabetic() {
            let amount = Uint128::from_str(&s[..i])?;
            let denom = &s[i..];
            return Ok(Coin::new(amount.u128(), denom));
        }
    }

    Err(StdError::generic_err(format!("failed to parse coin: {}", s)))
}

/// Find the amount of a denom sent along a message, assert it is non-zero, and no other denom were
/// sent together
pub(crate) fn parse_received_fund(funds: &[Coin], denom: &str) -> StdResult<Uint128> {
    if funds.len() != 1 {
        return Err(StdError::generic_err(
            format!("must deposit exactly one coin; received {}", funds.len())
        ));
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

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_coin() {
        let coin = parse_coin("12345uatom").unwrap();
        assert_eq!(coin, Coin::new(12345, "uatom"));

        let coin = parse_coin("23456ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B").unwrap();
        assert_eq!(coin, Coin::new(23456, "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B"));

        let err = parse_coin("69420").unwrap_err();
        assert_eq!(err, StdError::generic_err("failed to parse coin: 69420"));

        let err = parse_coin("ngmi").unwrap_err();
        assert_eq!(err, StdError::generic_err("Parsing u128: cannot parse integer from empty string"));
    }

    #[test]
    fn parsing_received_fund() {
        let err = parse_received_fund(&[], "uluna").unwrap_err();
        assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 0"));

        let err = parse_received_fund(&[Coin::new(12345, "uatom"), Coin::new(23456, "uluna")], "uluna").unwrap_err();
        assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 2"));

        let err = parse_received_fund(&[Coin::new(12345, "uatom")], "uluna").unwrap_err();
        assert_eq!(err, StdError::generic_err("expected uluna deposit, received uatom"));

        let err = parse_received_fund(&[Coin::new(0, "uluna")], "uluna").unwrap_err();
        assert_eq!(err, StdError::generic_err("deposit amount must be non-zero"));

        let amount = parse_received_fund(&[Coin::new(69420, "uluna")], "uluna").unwrap();
        assert_eq!(amount, Uint128::new(69420));
    }
}