use cosmwasm_std::{Addr, Coin, Decimal, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

use pfc_steak::hub::{Batch, FeeType, PendingBatch, UnbondRequest};

use crate::types::BooleanKey;
pub(crate) const BATCH_KEY_V101: &str = "previous_batches_101";
pub(crate) const BATCH_KEY_RECONCILED_V101: &str = "previous_batches__reconciled_101";

pub(crate) struct State<'a> {
    /// Account who can call certain privileged functions
    pub owner: Item<'a, Addr>,
    /// Pending ownership transfer, awaiting acceptance by the new owner
    pub new_owner: Item<'a, Addr>,
    pub fee_account_type: Item<'a,FeeType>,
    /// Account to send fees to
    pub fee_account: Item<'a, Addr>,
    /// Current fee rate
    pub fee_rate: Item<'a, Decimal>,
    /// Maximum fee rate
    pub max_fee_rate: Item<'a, Decimal>,
    /// denom to accept
    pub denom: Item<'a, String>,
    /// Address of the Steak token
    pub steak_token: Item<'a, Addr>,
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
    pub validators_active: Item<'a, Vec<String>>,
    /// coins in 'denom' held before reinvest was called.
    pub prev_denom: Item<'a, Uint128>,
    /// Dust Collector contract
    pub dust_collector: Item<'a, Option<Addr>>,
}

impl Default for State<'static> {
    fn default() -> Self {
        let pb_indexes = PreviousBatchesIndexes {
            reconciled: MultiIndex::new(
                |d: &Batch| d.reconciled.into(),
                BATCH_KEY_V101,
                BATCH_KEY_RECONCILED_V101,
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
            fee_account: Item::new("fee_account"),
            fee_rate: Item::new("fee_rate"),
            max_fee_rate: Item::new("max_fee_rate"),
            denom: Item::new("denom"),
            steak_token: Item::new("steak_token"),
            epoch_period: Item::new("epoch_period"),
            unbond_period: Item::new("unbond_period"),
            validators: Item::new("validators"),
            unlocked_coins: Item::new("unlocked_coins"),
            pending_batch: Item::new("pending_batch"),
            previous_batches: IndexedMap::new(BATCH_KEY_V101, pb_indexes),
            unbond_requests: IndexedMap::new("unbond_requests", ubr_indexes),
            validators_active: Item::new("validators_active"),
            prev_denom: Item::new("prev_denom"),
            fee_account_type: Item::new("fee_account_type"),
            dust_collector: Item::new("dust_collector")
        }
    }
}

impl<'a> State<'a> {
    pub fn assert_owner(&self, storage: &dyn Storage, sender: &Addr) -> StdResult<()> {
        let owner = self.owner.load(storage)?;
        if *sender == owner {
            Ok(())
        } else {
            Err(StdError::generic_err("unauthorized: sender is not owner"))
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
