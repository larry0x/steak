use std::collections::HashMap;

use cosmwasm_std::{to_binary, Addr, QuerierResult, SystemError, Uint128};
use cw20::{BalanceResponse, Cw20QueryMsg, TokenInfoResponse};

#[derive(Default)]
pub(super) struct Cw20Querier {
    /// Mapping token address to its total supply
    pub total_supplies: HashMap<Addr, Uint128>,
    /// Mapping token address and user address to the user's token balance
    pub balances: HashMap<Addr, HashMap<Addr, Uint128>>,
}

impl Cw20Querier {
    pub fn handle_query(&self, contract_addr: &Addr, query: Cw20QueryMsg) -> QuerierResult {
        match query {
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
                    total_supply: *total_supply,
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

                let user_balance = contract_balances
                    .get(&Addr::unchecked(&address))
                    .ok_or_else(|| SystemError::InvalidRequest {
                        error: format!("[mock] balance not set for cw20 `{}` and user `{}`", contract_addr, address),
                        request: Default::default(),
                    })
                    .unwrap();

                Ok(to_binary(&BalanceResponse {
                    balance: *user_balance,
                })
                .into())
                .into()
            },

            other_query => Err(SystemError::InvalidRequest {
                error: format!("[mock] unsupported cw20 query: {:?}", other_query),
                request: Default::default(),
            })
            .into(),
        }
    }
}
