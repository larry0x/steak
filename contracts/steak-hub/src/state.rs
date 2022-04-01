use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key};

use crate::msg::{Batch, PendingBatch, UnbondRequest};

pub(crate) struct State<'a> {
    /// Address of the Steak token
    pub steak_token: Item<'a, Addr>,
    /// How often the unbonding queue is to be executed
    pub epoch_period: Item<'a, u64>,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: Item<'a, u64>,
    /// Accounts who can call the harvest function
    pub workers: Item<'a, Vec<Addr>>,
    /// Validators who will receive the delegations
    pub validators: Item<'a, Vec<String>>,
    /// Coins that can be reinvested
    pub unlocked_coins: Item<'a, Vec<Coin>>,
    /// The current batch of unbonding requests queded to be executed
    pub pending_batch: Item<'a, PendingBatch>,
    /// Previous batches that have started unbonding but not yet finished
    pub previous_batches: Map<'a, U64Key, Batch>,
    /// Shares in an unbonding batch, with the batch ID and the user address as composite key,
    /// additionally indexed by the user address
    pub unbond_requests: IndexedMap<'a, (U64Key, &'a Addr), UnbondRequest, UnbondRequestsIndexes<'a>>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let indexes = UnbondRequestsIndexes {
            user: MultiIndex::new(
                |d: &UnbondRequest, k: Vec<u8>| (d.user.clone(), k),
                "unbond_requests",
                "unbond_requests__user",
            ),
        };
        Self {
            steak_token: Item::new("steak_token"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            workers: Item::new("workers"),
            validators: Item::new("validators"),
            unlocked_coins: Item::new("unlocked_coins"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: Map::new("previous_batches"),
            unbond_requests: IndexedMap::new("unbond_requests", indexes),
        }
    }
}

pub(crate) struct UnbondRequestsIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, (String, Vec<u8>), UnbondRequest>,
}

impl<'a> IndexList<UnbondRequest> for UnbondRequestsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRequest>> + '_> {
        let v: Vec<&dyn Index<UnbondRequest>> = vec![&self.user];
        Box::new(v.into_iter())
    }
}
