use cosmwasm_std::{Storage, Order, StdResult, Uint128, Event};
use cw_storage_plus::{Map, U64Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use steak::hub::Batch;

use crate::state::State;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LegacyBatch {
    pub id: u64,
    pub total_shares: Uint128,
    pub uluna_unclaimed: Uint128,
    pub est_unbond_end_time: u64,
}

const LEGACY_BATCHES: Map<U64Key, LegacyBatch> = Map::new("previous_batches");

pub(crate) fn migrate_batches(storage: &mut dyn Storage) -> StdResult<Event> {
    let state = State::default();

    // Find all previous batches
    let legacy_batches = LEGACY_BATCHES
        .range(storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect::<StdResult<Vec<_>>>()?;

    // Cast legacy data to the new type
    let batches = legacy_batches
        .iter()
        .map(|lb| Batch {
            id: lb.id,
            reconciled: false,
            total_shares: lb.total_shares,
            uluna_unclaimed: lb.uluna_unclaimed,
            est_unbond_end_time: lb.est_unbond_end_time,
        })
        .collect::<Vec<_>>();

    // Delete the legacy data
    legacy_batches
        .iter()
        .for_each(|lb| {
            LEGACY_BATCHES.remove(storage, lb.id.into())
        });

    // Save the new type data
    // We use unwrap here, which is undesired, but it's ok with me since this code will only be in
    // the contract temporarily
    batches
        .iter()
        .for_each(|b| {
            state.previous_batches.save(storage, b.id.into(), b).unwrap()
        });

    let ids = batches.iter().map(|b| b.id.to_string()).collect::<Vec<_>>();

    Ok(Event::new("steakhub/batches_migrated")
        .add_attribute("ids", ids.join(",")))
}

#[cfg(test)]
mod tests {
    use cosmwasm_std::testing::mock_dependencies;

    use super::*;
    use crate::queries;

    #[test]
    fn migrating_batches() {
        let mut deps = mock_dependencies(&[]);

        let legacy_batches = vec![
            LegacyBatch {
                id: 1,
                total_shares: Uint128::new(123),
                uluna_unclaimed: Uint128::new(678),
                est_unbond_end_time: 10000,
            },
            LegacyBatch {
                id: 2,
                total_shares: Uint128::new(234),
                uluna_unclaimed: Uint128::new(789),
                est_unbond_end_time: 15000,
            },
            LegacyBatch {
                id: 3,
                total_shares: Uint128::new(345),
                uluna_unclaimed: Uint128::new(890),
                est_unbond_end_time: 20000,
            },
            LegacyBatch {
                id: 4,
                total_shares: Uint128::new(456),
                uluna_unclaimed: Uint128::new(999),
                est_unbond_end_time: 25000,
            },
        ];

        for legacy_batch in &legacy_batches {
            LEGACY_BATCHES.save(deps.as_mut().storage, legacy_batch.id.into(), legacy_batch).unwrap();
        }

        let event = migrate_batches(deps.as_mut().storage).unwrap();
        assert_eq!(
            event,
            Event::new("steakhub/batches_migrated").add_attribute("ids", "1,2,3,4")
        );

        let batches = queries::previous_batches(deps.as_ref(), None, None).unwrap();

        let expected = legacy_batches
            .iter()
            .map(|lb| Batch {
                id: lb.id,
                reconciled: false,
                total_shares: lb.total_shares,
                uluna_unclaimed: lb.uluna_unclaimed,
                est_unbond_end_time: lb.est_unbond_end_time,
            })
            .collect::<Vec<_>>();

        assert_eq!(batches, expected);
    }
}