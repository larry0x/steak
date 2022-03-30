use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, StdResult, WasmMsg};
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_cosmwasm::TerraMsgWrapper;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Code ID of the CW20 token contract
    pub cw20_code_id: u64,
    /// Name of the liquid staking token
    pub name: String,
    /// Symbol of the liquid staking token
    pub symbol: String,
    /// Number of decimals of the liquid staking token
    pub decimals: u8,
    /// Accounts who can call the harvest function
    pub workers: Vec<String>,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Implements the Cw20 receiver interface
    Receive(Cw20ReceiveMsg),
    /// Bond specified amount of Luna
    Bond {},
    /// Claim staking rewards, swap all for Luna, and restake
    ///
    /// Currently set to permissioned to deter sandwich attacks. Will explore other options
    Harvest {},
    /// Callbacks; can only be invoked by the contract itself
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Unstake Steak received
    Unstake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    /// Swap Terra stablecoins held by the contract to Luna
    Swap {},
    /// Following the swaps, stake the Luna acquired to the whitelisted validators
    Restake {},
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg<TerraMsgWrapper>> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: String::from(contract_addr),
            msg: to_binary(&ExecuteMsg::Callback(self.clone()))?,
            funds: vec![],
        }))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// The contract's configurations. Response: `ConfigResponse`
    Config {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub steak_token: String,
    pub workers: Vec<String>,
    pub validators: Vec<String>,
}

/// We currently don't take any input parameter for migration
pub type MigrateMsg = Empty;
