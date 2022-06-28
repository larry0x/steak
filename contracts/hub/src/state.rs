use cosmwasm_std::{Addr, Coin, Decimal, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};
use steak::hub::{Batch, PendingBatch, UnbondRequest};

use crate::{error::ContractError, types::BooleanKey};

pub(crate) struct State<'a> {
    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,
    /// Denom of the Steak coin
    pub steak_denom: Item<'a, String>,
    /// How often the unbonding queue is to be executed
    pub epoch_period: Item<'a, u64>,
    /// The staking module's unbonding time, in seconds
    pub unbond_period: Item<'a, u64>,
    /// Validators who will receive the delegations
    pub validators: Item<'a, Vec<String>>,
    /// Coins that can be reinvested
    pub unlocked_coins: Item<'a, Vec<Coin>>,
    /// The current batch of unbonding requests queded to be executed
    pub pending_batch: Item<'a, PendingBatch>,
    /// Previous batches that have started unbonding but not yet finished
    pub previous_batches: IndexedMap<'a, u64, Batch, PreviousBatchesIndexes<'a>>,
    /// Users' shares in unbonding batches
    pub unbond_requests: IndexedMap<'a, (u64, &'a Addr), UnbondRequest, UnbondRequestsIndexes<'a>>,
    /// The total supply of the steak coin
    pub total_usteak_supply: Item<'a, Uint128>,
    /// Contract where reward funds are sent
    pub distribution_contract: Item<'a, Addr>,
    /// Fee that is awarded to distribution contract when harvesting rewards
    pub performance_fee: Item<'a, Decimal>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let pb_indexes = PreviousBatchesIndexes {
            reconciled: MultiIndex::new(
                |d: &Batch| d.reconciled.into(),
                "previous_batches",
                "previous_batches__reconciled",
            ),
        };
        let ubr_indexes = UnbondRequestsIndexes {
            user: MultiIndex::new(
                |d: &UnbondRequest| d.user.clone().into(),
                "unbond_requests",
                "unbond_requests__user",
            ),
        };
        Self {
            owner: Item::new("owner"),
            new_owner: Item::new("new_owner"),
            steak_denom: Item::new("steak_denom"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            validators: Item::new("validators"),
            unlocked_coins: Item::new("unlocked_coins"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: IndexedMap::new("previous_batches", pb_indexes),
            unbond_requests: IndexedMap::new("unbond_requests", ubr_indexes),
            total_usteak_supply: Item::new("total_usteak_supply"),
            distribution_contract: Item::new("distribution_contract"),
            performance_fee: Item::new("performance_fee"),
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> Result<(), ContractError> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(())
        } else {
            Err(ContractError::Unauthorized {})
        }
    }
}

pub(crate) struct PreviousBatchesIndexes<'a> {
    // pk goes to second tuple element
    pub reconciled: MultiIndex<'a, BooleanKey, Batch, Vec<u8>>,
}

impl<'a> IndexList<Batch> for PreviousBatchesIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Batch>> + '_> {
        let v: Vec<&dyn Index<Batch>> = vec![&self.reconciled];
        Box::new(v.into_iter())
    }
}

pub(crate) struct UnbondRequestsIndexes<'a> {
    // pk goes to second tuple element
    pub user: MultiIndex<'a, String, UnbondRequest, Vec<u8>>,
}

impl<'a> IndexList<UnbondRequest> for UnbondRequestsIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<UnbondRequest>> + '_> {
        let v: Vec<&dyn Index<UnbondRequest>> = vec![&self.user];
        Box::new(v.into_iter())
    }
}
