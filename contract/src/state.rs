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
    /// Estimated time when this batch will be submitted for unbonding
    pub est_unbond_start_time: u64,
}

/// Represents a batch that has already been submitted for unbonding
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub(crate) struct Batch {
    /// Total amount of shares remaining this batch. Each `usteak` burned = 1 share
    pub total_shares: Uint128,
    /// Amount of `uluna` in this batch that have not been claimed
    pub uluna_unclaimed: Uint128,
    /// Estimated time when this batch will finish unbonding
    pub est_unbond_end_time: u64,
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
    /// The user's unbonding share in a specific batch. 1 usteak burned = 1 share in that batch
    pub unbond_shares: Map<'a, (&'a Addr, U64Key), Uint128>,
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
            unbond_shares: Map::new("unbond_shares"),
        }
    }
}
