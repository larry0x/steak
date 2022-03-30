use cosmwasm_std::Empty;
use cw20::Cw20ReceiveMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Stake specified amount of Luna
    Stake {},
    /// Claim staking rewards, swap all for Luna, and restake
    ///
    /// Currently set to permissioned to deter sandwich attacks. Will explore other options
    Harvest {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Unstake Steak received
    Unstake {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {}

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
