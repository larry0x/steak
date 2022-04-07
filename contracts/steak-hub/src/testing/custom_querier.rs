use std::collections::HashMap;

use cosmwasm_std::testing::{StakingQuerier, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    from_binary, from_slice, Addr, Coin, FullDelegation, Querier, QuerierResult, QueryRequest,
    SystemError, WasmQuery,
};
use cw20::Cw20QueryMsg;
use terra_cosmwasm::TerraQueryWrapper;

use crate::types::Delegation;

use super::cw20_querier::Cw20Querier;
use super::helpers::err_unsupported_query;

#[derive(Default)]
pub(super) struct CustomQuerier {
    pub cw20_querier: Cw20Querier,
    pub staking_querier: StakingQuerier,
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
    #[allow(dead_code)]
    pub fn set_cw20_balance(&mut self, token: &str, user: &str, balance: u128) {
        match self.cw20_querier.balances.get_mut(token) {
            Some(contract_balances) => {
                contract_balances.insert(user.to_string(), balance);
            },
            None => {
                let mut contract_balances: HashMap<String, u128> = HashMap::default();
                contract_balances.insert(user.to_string(), balance);
                self.cw20_querier.balances.insert(token.to_string(), contract_balances);
            },
        };
    }

    pub fn set_cw20_total_supply(&mut self, token: &str, total_supply: u128) {
        self.cw20_querier.total_supplies.insert(token.to_string(), total_supply);
    }

    pub fn set_staking_delegations(&mut self, delegations: &[Delegation]) {
        let fds: Vec<FullDelegation> = delegations
            .iter()
            .map(|d| FullDelegation {
                delegator: Addr::unchecked(MOCK_CONTRACT_ADDR),
                validator: d.validator.clone(),
                amount: Coin::new(d.amount.u128(), "uluna"),
                can_redelegate: Coin::new(0, "uluna"),
                accumulated_rewards: vec![],
            })
            .collect();

        self.staking_querier = StakingQuerier::new("uluna", &[], &fds);
    }

    pub fn handle_query(&self, request: &QueryRequest<TerraQueryWrapper>) -> QuerierResult {
        match request {
            QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr,
                msg,
            }) => {
                if let Ok(query) = from_binary::<Cw20QueryMsg>(msg) {
                    return self.cw20_querier.handle_query(&contract_addr, query);
                }

                err_unsupported_query(msg)
            },

            QueryRequest::Custom(TerraQueryWrapper {
                route,
                query_data,
            }) => {
                panic!("[mock] custom query is unimplemented");
            },

            QueryRequest::Staking(query) => self.staking_querier.query(query),

            _ => err_unsupported_query(request),
        }
    }
}
