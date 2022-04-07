use std::collections::HashMap;

use cosmwasm_std::testing::MockQuerier;
use cosmwasm_std::{
    from_binary, from_slice, Addr, Querier, QuerierResult, QueryRequest, SystemError, Uint128,
    WasmQuery,
};
use cw20::Cw20QueryMsg;
use terra_cosmwasm::TerraQueryWrapper;

use super::cw20_querier::Cw20Querier;

pub(super) struct CustomQuerier {
    base: MockQuerier<TerraQueryWrapper>,
    cw20_querier: Cw20Querier,
}

impl Default for CustomQuerier {
    fn default() -> Self {
        Self {
            base: MockQuerier::new(&[]),
            cw20_querier: Cw20Querier::default(),
        }
    }
}

impl Querier for CustomQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        let request: QueryRequest<TerraQueryWrapper> = match from_slice(bin_request) {
            Ok(v) => v,
            Err(e) => {
                return Err(SystemError::InvalidRequest {
                    error: format!("Parsing query request: {}", e),
                    request: bin_request.into(),
                })
                .into()
            },
        };
        self.handle_query(&request)
    }
}

impl CustomQuerier {
    pub fn set_cw20_balance(&mut self, token: &str, user: &str, balance: u128) {
        let token_addr = Addr::unchecked(token);
        let user_addr = Addr::unchecked(user);
        match self.cw20_querier.balances.get_mut(&token_addr) {
            Some(contract_balances) => {
                contract_balances.insert(user_addr, Uint128::new(balance));
            },
            None => {
                let mut contract_balances: HashMap<Addr, Uint128> = HashMap::default();
                contract_balances.insert(user_addr, Uint128::new(balance));
                self.cw20_querier.balances.insert(token_addr, contract_balances);
            },
        };
    }

    pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
        self.cw20_querier
            .total_supplies
            .entry(Addr::unchecked(token))
            .or_insert(Uint128::new(total_supply));
    }

    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                let contract_addr = Addr::unchecked(contract_addr);

                if let Ok(query) = from_binary::<Cw20QueryMsg>(msg) {
                    return self.cw20_querier.handle_query(&contract_addr, query);
                }

                Err(SystemError::InvalidRequest {
                    error: format!("[mock] unsupported wasm query: {:?}", msg),
                    request: Default::default(),
                })
                .into()
            },

            QueryRequest::Custom(TerraQueryWrapper { route, query_data }) => {
                panic!("[mock] custom query is unimplemented");
            }

            _ => self.base.handle_query(request),
        }
    }
}
