use cosmwasm_std::testing::{mock_env, MOCK_CONTRACT_ADDR, MockStorage, MockApi};
use cosmwasm_std::{from_binary, Addr, BlockInfo, ContractInfo, Deps, Env, Timestamp, OwnedDeps};
use serde::de::DeserializeOwned;

use steak::hub::QueryMsg;

use crate::contract::query;

use super::custom_querier::CustomQuerier;

pub(super) fn mock_dependencies() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    OwnedDeps {
        storage: MockStorage::default(),
        api: MockApi::default(),
        querier: CustomQuerier::default(),
    }
}

pub(super) fn mock_env_with_timestamp(timestamp: u64) -> Env {
    Env {
        block: BlockInfo {
            height: 12_345,
            time: Timestamp::from_seconds(timestamp),
            chain_id: "cosmos-testnet-14002".to_string(),
        },
        contract: ContractInfo {
            address: Addr::unchecked(MOCK_CONTRACT_ADDR),
        },
    }
}

pub(super) fn query_helper<T: DeserializeOwned>(deps: Deps, msg: QueryMsg) -> T {
    from_binary(&query(deps, mock_env(), msg).unwrap()).unwrap()
}