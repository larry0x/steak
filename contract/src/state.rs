use cosmwasm_std::Addr;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, U64Key};

use crate::msg::{Batch, PendingBatch, UnbondShare};

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
    /// The current batch of unbonding requests queded to be executed
    pub pending_batch: Item<'a, PendingBatch>,
    /// Previous batches that have started unbonding but not yet finished
    pub previous_batches: Map<'a, U64Key, Batch>,
    /// Shares in an unbonding batch, with the batch ID and the user address as composite key,
    /// additionally indexed by the user address
    pub unbond_shares: IndexedMap<'a, (U64Key, &'a Addr), UnbondShare, UnbondSharesIndexes<'a>>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let indexes = UnbondSharesIndexes {
            user: MultiIndex::new(
                unbond_shares_user_index,
                "unbond_shares",
                "unbond_shares__user",
            ),
        };
        Self {
            steak_token: Item::new("steak_token"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            workers: Item::new("workers"),
            validators: Item::new("validators"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: Map::new("previous_batches"),
            unbond_shares: IndexedMap::new("unbond_shares", indexes),
        }
    }
}

pub(crate) struct UnbondSharesIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, (String, Vec<u8>), UnbondShare>,
}

impl<'a> IndexList<UnbondShare> for UnbondSharesIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondShare>> + '_> {
        let v: Vec<&dyn Index<UnbondShare>> = vec![&self.user];
        Box::new(v.into_iter())
    }
}

pub(crate) fn unbond_shares_user_index(d: &UnbondShare, k: Vec<u8>) -> (String, Vec<u8>) {
    (d.user.clone(), k)
}
