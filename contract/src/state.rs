use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map, U64Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Represents a batch of unbonding requests that has not yet been submitted for unbonding
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub(crate) struct PendingBatch {
    /// ID of this batch
    pub id: u64,
    /// Total amount of `usteak` to be burned in this batch
    pub usteak_to_burn: Uint128,
    /// Estimated time that this batch will be submitted for unbonding
    pub est_unbond_start_time: u64,
}

/// Represents a batch that has already been submitted for unbonding
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub(crate) struct Batch {
    /// Total amount of `usteak` burned in this batch
    pub usteak_burned: Uint128,
    /// Total amount of `uluna` unbonded in this batch
    pub uluna_unbonded: Uint128,
    /// The time when this batch started unbonding
    pub unbond_start_time: u64,
}

/// Represents the contract's storage
pub(crate) struct State<'a> {
    /// Address of the Steak token
    pub steak_token: Item<'a, Addr>,
    /// Accounts who can call the harvest function
    pub workers: Item<'a, Vec<Addr>>,
    /// Validators who will receive the delegations
    pub validators: Item<'a, Vec<String>>,
    /// How often the unbonding queue is to be executed
    pub epoch_period: Item<'a, u64>,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: Item<'a, u64>,
    /// The current batch of unbonding requests queded to be executed
    pub pending_batch: Item<'a, PendingBatch>,
    /// Previous batches that have started unbonding but not yet finished
    pub previous_batches: Map<'a, U64Key, Batch>,
    /// Unbonding requests that have not been finalized
    pub active_requests: Map<'a, (&'a Addr, U64Key), Uint128>,
}

impl Default for State<'static> {
    fn default() -> Self {
        Self {
            steak_token: Item::new("steak_token"),
            workers: Item::new("workers"),
            validators: Item::new("validators"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: Map::new("previous_batches"),
            active_requests: Map::new("active_requests"),
        }
    }
}
