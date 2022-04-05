use astroport::asset::{Asset, AssetInfo};
use cosmwasm_std::{Addr, Coin, StdError, StdResult, Uint128};

pub(crate) fn new_native_from_funds(funds: &[Coin]) -> StdResult<Asset> {
    if funds.len() > 1 {
        return Err(StdError::generic_err("more than one coins deposited"));
    }

    let fund = &funds[0];

    Ok(Asset {
        info: AssetInfo::NativeToken { denom: fund.denom.clone() },
        amount: fund.amount,
    })
}

pub(crate) fn new_cw20(contract_addr: Addr, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::Token { contract_addr },
        amount,
    }
}
