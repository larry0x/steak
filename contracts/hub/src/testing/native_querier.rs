use std::collections::HashMap;

use cosmwasm_std::{to_binary, Decimal, QuerierResult, SystemError, SystemResult};
use terra_cosmwasm::{ExchangeRateItem, ExchangeRatesResponse, TerraQuery};

use super::helpers::err_unsupported_query;

#[derive(Default)]
pub struct NativeQuerier {
    /// Maps (base_denom, quote_denom) pair to exchange rate
    pub exchange_rates: HashMap<(String, String), Decimal>,
}

impl NativeQuerier {
    /// We only implement the `exchange_rates` query as that is the only one we need in the unit tests
    ///
    /// NOTE: When querying exchange rates, Terra's oracle module behaves in the following way:
    /// - If `quote_denoms` contains _at least one_ known denom (meaning a denom that has exchange
    ///   rate defined), the query will be successful, and the response will contain the exchange
    ///   rates of only known denoms and omit denoms that are not known;
    /// - If `quote_denoms` only contains unknown denoms, the query fails.
    ///
    /// Examples:
    /// - [Success](https://bombay-fcd.terra.dev/wasm/contracts/terra1xf8kh2r7n06wk0mdhq0tgcrcyv90snjzfxfacg/store?query_msg=%7B%22ExchangeRates%22:[%22uusd%22,%22ukrw%22,%22ibc%2F0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B%22]%7D),
    ///   where the unknown denom (`ibc/...`) is omitted from the response
    /// - [Fail](https://bombay-fcd.terra.dev/wasm/contracts/terra1xf8kh2r7n06wk0mdhq0tgcrcyv90snjzfxfacg/store?query_msg=%7B%22ExchangeRates%22:[%22ibc%2F0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B%22]%7D),
    ///   if the query only contains the unknown denom
    ///
    /// We emulate this behaviour in our mock querier.
    pub fn handle_query(&self, query: &TerraQuery) -> QuerierResult {
        if let TerraQuery::ExchangeRates {
            base_denom,
            quote_denoms,
        } = query
        {
            let exchange_rates: Vec<ExchangeRateItem> = quote_denoms
                .iter()
                .map(|quote_denom| {
                    self.exchange_rates.get(&(base_denom.clone(), quote_denom.clone())).map(
                        |rate| ExchangeRateItem {
                            quote_denom: quote_denom.clone(),
                            exchange_rate: *rate,
                        },
                    )
                })
                .filter(|item| item.is_some())
                .map(|item| item.unwrap())
                .collect();

            if exchange_rates.is_empty() {
                return SystemResult::Err(SystemError::InvalidRequest {
                    error: format!("[mock] quote_denoms are all unknown"),
                    request: Default::default(),
                });
            }

            return Ok(to_binary(&ExchangeRatesResponse {
                base_denom: base_denom.into(),
                exchange_rates,
            })
            .into())
            .into();
        }

        err_unsupported_query(query)
    }
}
