use std::collections::HashMap;

use cosmwasm_std::{to_binary, QuerierResult, SystemError, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

use super::helpers::err_unsupported_query;

#[derive(Default)]
pub(super) struct Cw20Querier {
    /// Mapping token address to its total supply
    pub total_supplies: HashMap<String, u128>,
    /// Mapping token address and user address to the user's token balance
    pub balances: HashMap<String, HashMap<String, u128>>,
}

impl Cw20Querier {
    pub fn handle_query(&self, contract_addr: &str, query: Cw20QueryMsg) -> QuerierResult {
        match &query {
            Cw20QueryMsg::TokenInfo {} => {
                let total_supply = self
                    .total_supplies
                    .get(contract_addr)
                    .ok_or_else(|| SystemError::InvalidRequest {
                        error: format!("[mock] total supply not set for cw20 `{}`", contract_addr),
                        request: Default::default(),
                    })
                    .unwrap();

                Ok(to_binary(&TokenInfoResponse {
                    name: "".to_string(),
                    symbol: "".to_string(),
                    decimals: 0,
                    total_supply: Uint128::new(*total_supply),
                })
                .into())
                .into()
            },

            Cw20QueryMsg::Balance {
                address,
            } => {
                let contract_balances = self
                    .balances
                    .get(contract_addr)
                    .ok_or_else(|| SystemError::InvalidRequest {
                        error: format!("[mock] balances not set for cw20 `{}`", contract_addr),
                        request: Default::default(),
                    })
                    .unwrap();

                let balance = contract_balances
                    .get(address)
                    .ok_or_else(|| SystemError::InvalidRequest {
                        error: format!(
                            "[mock] balance not set for cw20 `{}` and user `{}`",
                            contract_addr, address
                        ),
                        request: Default::default(),
                    })
                    .unwrap();

                Ok(to_binary(&BalanceResponse {
                    balance: Uint128::new(*balance),
                })
                .into())
                .into()
            },

            other_query => err_unsupported_query(other_query),
        }
    }
}
