use cosmwasm_std::{to_binary, Addr, CosmosMsg, Empty, StdResult, Uint128, WasmMsg};
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
    /// How often the unbonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: u64,
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
    Harvest {},
    /// Submit the current pending batch of unbonding requests to be unbonded
    SubmitBatch {},
    /// Withdraw Luna that have finished unbonding in previous batches
    WithdrawUnbonded {},
    /// Callbacks; can only be invoked by the contract itself
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReceiveMsg {
    /// Submit an unbonding request to the current unbonding queue; automatically invokes `unbond`
    /// if `epoch_time` has elapsed since when the last unbonding queue was executed.
    QueueUnbond {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    /// Swap Terra stablecoins held by the contract to Luna
    Swap {},
    /// Following the swaps, stake the Luna acquired to the whitelisted validators
    Reinvest {},
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
    /// The current batch on unbonding requests pending submission. Response: `crate::state::PendingBatch`
    PendingBatch {},
    /// Enumerate previous batches that have previously been submitted for unbonding but have not yet
    /// been fully withdrawn. Response: `Vec<crate::state::Batch>`
    PreviousBatches {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    /// Enumerate all outstanding unbonding requests in a given batch. Response: `Vec<UnbondRequestsResponseByBatchItem>`
    UnbondRequestsByBatch {
        id: u64,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Enumreate all outstanding unbonding requests from given a user. Response: `Vec<UnbondRequestsByUserResponseItem>`
    UnbondRequestsByUser {
        user: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    /// Address of the Steak token
    pub steak_token: String,
    /// How often the unbonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: u64,
    /// Accounts who can call the harvest function
    pub workers: Vec<String>,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingBatch {
    /// ID of this batch
    pub id: u64,
    /// Total amount of `usteak` to be burned in this batch
    pub usteak_to_burn: Uint128,
    /// Estimated time when this batch will be submitted for unbonding
    pub est_unbond_start_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Batch {
    /// ID of this batch
    pub id: u64,
    /// Total amount of shares remaining this batch. Each `usteak` burned = 1 share
    pub total_shares: Uint128,
    /// Amount of `uluna` in this batch that have not been claimed
    pub uluna_unclaimed: Uint128,
    /// Estimated time when this batch will finish unbonding
    pub est_unbond_end_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnbondRequest {
    /// ID of the batch
    pub id: u64,
    /// The user's address
    pub user: String,
    /// The user's share in the batch
    pub shares: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnbondRequestsByBatchResponseItem {
    /// The user's address
    pub user: String,
    /// The user's share in the batch
    pub shares: Uint128,
}

impl From<UnbondRequest> for UnbondRequestsByBatchResponseItem {
    fn from(s: UnbondRequest) -> Self {
        Self {
            user: s.user,
            shares: s.shares
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnbondRequestsByUserResponseItem {
    /// ID of the batch
    pub id: u64,
    /// The user's share in the batch
    pub shares: Uint128,
}

impl From<UnbondRequest> for UnbondRequestsByUserResponseItem {
    fn from(s: UnbondRequest) -> Self {
        Self {
            id: s.id,
            shares: s.shares
        }
    }
}

/// We currently don't take any input parameter for migration
pub type MigrateMsg = Empty;
