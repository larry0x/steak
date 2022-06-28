use cosmwasm_std::{to_binary, Addr, Coin, CosmosMsg, Decimal, Empty, StdResult, Uint128, WasmMsg};
use osmo_bindings::OsmosisMsg;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    /// Account who can call certain privileged functions
    pub owner: String,
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
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,
    /// Contract where reward funds are sent
    pub distribution_contract: String,
    /// Fee that is awarded to distribution contract when harvesting rewards
    pub performance_fee: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Bond specified amount of osmo
    Bond { receiver: Option<String> },
    /// Withdraw osmo that have finished unbonding in previous batches
    WithdrawUnbonded { receiver: Option<String> },
    /// Add a validator to the whitelist; callable by the owner
    AddValidator { validator: String },
    /// Remove a validator from the whitelist; callable by the owner
    RemoveValidator { validator: String },
    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership { new_owner: String },
    /// Accept an ownership transfer
    AcceptOwnership {},
    /// Claim staking rewards, swap all for osmo, and restake
    Harvest {},
    /// Use redelegations to balance the amounts of osmo delegated to validators
    Rebalance {},
    /// Update osmo amounts in unbonding batches to reflect any slashing or rounding errors
    Reconcile {},
    /// Submit the current pending batch of unbonding requests to be unbonded
    SubmitBatch {},
    /// Submit an unbonding request to the current unbonding queue; automatically invokes `unbond`
    /// if `epoch_time` has elapsed since when the last unbonding queue was executed.
    QueueUnbond { receiver: Option<String> },
    /// Callbacks; can only be invoked by the contract itself
    Callback(CallbackMsg),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CallbackMsg {
    /// Following the swaps, stake the osmo acquired to the whitelisted validators
    Reinvest {},
}

impl CallbackMsg {
    pub fn into_cosmos_msg(&self, contract_addr: &Addr) -> StdResult<CosmosMsg<OsmosisMsg>> {
        Ok(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: contract_addr.to_string(),
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
    /// The contract's current state. Response: `StateResponse`
    State {},
    /// The current batch on unbonding requests pending submission. Response: `PendingBatch`
    PendingBatch {},
    /// Query an individual batch that has previously been submitted for unbonding but have not yet
    /// fully withdrawn. Response: `Batch`
    PreviousBatch(u64),
    /// Enumerate all previous batches that have previously been submitted for unbonding but have not
    /// yet fully withdrawn. Response: `Vec<Batch>`
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
    /// Account who can call certain privileged functions
    pub owner: String,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Option<String>,
    /// Address of the Steak denom
    pub steak_denom: String,
    /// How often the unbonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: u64,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,
    /// Contract where reward funds are sent
    pub distribution_contract: Addr,
    /// Fee that is awarded to distribution contract when harvesting rewards
    pub performance_fee: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// Total supply to the Steak token
    pub total_usteak: Uint128,
    /// Total amount of uosmo staked
    pub total_uosmo: Uint128,
    /// The exchange rate between usteak and uosmo, in terms of uosmo per usteak
    pub exchange_rate: Decimal,
    /// Staking rewards currently held by the contract that are ready to be reinvested
    pub unlocked_coins: Vec<Coin>,
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
    /// Whether this batch has already been reconciled
    pub reconciled: bool,
    /// Total amount of shares remaining this batch. Each `usteak` burned = 1 share
    pub total_shares: Uint128,
    /// Amount of `uosmo` in this batch that have not been claimed
    pub uosmo_unclaimed: Uint128,
    /// Estimated time when this batch will finish unbonding
    pub est_unbond_end_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UnbondRequest {
    /// ID of the batch
    pub id: u64,
    /// The user's address
    pub user: Addr,
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
            user: s.user.into(),
            shares: s.shares,
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
            shares: s.shares,
        }
    }
}

pub type MigrateMsg = Empty;
