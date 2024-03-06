use cosmwasm_std::{
    from_binary,
    testing::{mock_env, MockApi, MockStorage, MOCK_CONTRACT_ADDR},
    Addr, BlockInfo, ContractInfo, Deps, Env, OwnedDeps, QuerierResult, SystemError, SystemResult,
    Timestamp,
};
use pfc_steak::hub::QueryMsg;
use serde::de::DeserializeOwned;

use super::custom_querier::CustomQuerier;
use crate::contract::query;

pub(super) fn err_unsupported_query<T: std::fmt::Debug>(request: T) -> QuerierResult {
    SystemResult::Err(SystemError::InvalidRequest {
        error: format!("[mock] unsupported query: {:?}", request),
        request: Default::default(),
    })
}

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: CustomQuerier::default(),
        custom_query_type: Default::default(),
    }
}

pub(super) fn mock_env_at_timestamp(timestamp: u64) -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: Timestamp::from_seconds(timestamp),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        contract: ContractInfo {
            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
        },
        transaction: None,
    }
}

pub(super) fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    let bin = query(deps, mock_env(), msg).unwrap();
    //eprintln!("Query Response {:?}",&bin);
    from_binary(&bin).unwrap()
}
