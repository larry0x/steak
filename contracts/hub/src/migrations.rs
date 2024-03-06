use cosmwasm_std::{Addr, Order, QuerierWrapper, StdError, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};
use pfc_steak::hub::Batch;
use serde::{Deserialize, Serialize};

use crate::{
    helpers::get_denom_balance,
    state::{State, BATCH_KEY_V101},
    types::BooleanKey,
};

const BATCH_KEY_V100: &str = "previous_batches";
const BATCH_KEY_RECONCILED_V100: &str = "previous_batches__reconciled";

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct BatchV100 {
    /// ID of this batch
    pub id: u64,
    /// Whether this batch has already been reconciled
    pub reconciled: bool,
    /// Total amount of shares remaining this batch. Each `usteak` burned = 1 share
    pub total_shares: Uint128,
    /// Amount of `denom` in this batch that have not been claimed
    pub uluna_unclaimed: Uint128,
    /// Estimated time when this batch will finish unbonding
    pub est_unbond_end_time: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct ConfigV100 {}

impl ConfigV100 {
    pub fn upgrade_stores(
        storage: &mut dyn Storage,
        querier: &QuerierWrapper,
        contract_addr: Addr,
    ) -> StdResult<Self> {
        if BATCH_KEY_V101 == BATCH_KEY_V100 {
            Err(StdError::generic_err("STEAK: Migration Failed. Config keys are the same"))
        } else {
            let pb_indexes_v100 = PreviousBatchesIndexesV100 {
                reconciled: MultiIndex::new(
                    |d: &BatchV100| d.reconciled.into(),
                    BATCH_KEY_V100,
                    BATCH_KEY_RECONCILED_V100,
                ),
            };

            let old: IndexedMap<'_, u64, BatchV100, PreviousBatchesIndexesV100<'_>> =
                IndexedMap::new(BATCH_KEY_V100, pb_indexes_v100);
            let state = State::default();
            let denom = state.denom.load(storage)?;
            state.prev_denom.save(storage, &get_denom_balance(querier, contract_addr, denom)?)?;

            let old_batches = old
                .range(storage, None, None, Order::Ascending)
                .map(|item| {
                    let (_, v) = item?;
                    Ok(v)
                    //  Ok(v)
                })
                .collect::<StdResult<Vec<BatchV100>>>()?;

            {
                old_batches.into_iter().for_each(|v| {
                    {
                        let batch: Batch = Batch {
                            id: v.id,
                            reconciled: v.reconciled,
                            total_shares: v.total_shares,
                            amount_unclaimed: v.uluna_unclaimed,
                            est_unbond_end_time: v.est_unbond_end_time,
                        };
                        state.previous_batches.save(storage, v.id, &batch).unwrap();
                    }
                    //  Ok(v)
                });
                let validators = state.validators.load(storage)?;
                state.validators_active.save(storage, &validators)?;
                Ok(ConfigV100 {})
            }
        }
    }
}

pub(crate) struct PreviousBatchesIndexesV100<'a> {
    // pk goes to second tuple element
    pub reconciled: MultiIndex<'a, BooleanKey, BatchV100, Vec<u8>>,
}

impl<'a> IndexList<BatchV100> for PreviousBatchesIndexesV100<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<BatchV100>> + '_> {
        let v: Vec<&dyn Index<BatchV100>> = vec![&self.reconciled];
        Box::new(v.into_iter())
    }
}
