use cosmwasm_std::{Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, U64Key};

use steak::hub::{
    Batch, ConfigResponse, PendingBatch, StateResponse, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem,
};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::state::State;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    Ok(ConfigResponse {
        steak_token: state.steak_token.load(deps.storage)?.into(),
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        validators: state.validators.load(deps.storage)?,
    })
}

pub fn query_state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();

    let steak_token = state.steak_token.load(deps.storage)?;
    let total_usteak = query_cw20_total_supply(&deps.querier, &steak_token)?;

    let validators = state.validators.load(deps.storage)?;
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address)?;
    let total_uluna: Uint128 = delegations.iter().map(|d| d.amount).sum();

    let exchange_rate = if total_usteak.is_zero() {
        Decimal::one()
    } else {
        Decimal::from_ratio(total_uluna, total_usteak)
    };

    Ok(StateResponse {
        total_usteak,
        total_uluna,
        exchange_rate,
        unlocked_coins: state.unlocked_coins.load(deps.storage)?,
    })
}

pub fn query_pending_batch(deps: Deps) -> StdResult<PendingBatch> {
    let state = State::default();
    state.pending_batch.load(deps.storage)
}

pub fn query_previous_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Batch>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|id| Bound::exclusive(U64Key::from(id)));

    state
        .previous_batches
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v)
        })
        .collect()
}

pub fn query_unbond_requests_by_batch(
    deps: Deps,
    id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByBatchResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    state
        .unbond_requests
        .prefix(id.into())
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

pub fn query_unbond_requests_by_user(
    deps: Deps,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByUserResponseItem>> {
    let state = State::default();

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(|id| Bound::exclusive(U64Key::from(id)));

    state
        .unbond_requests
        .idx
        .user
        .prefix(user)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::mock_dependencies;
    use cosmwasm_std::Addr;
    use steak::hub::UnbondRequest;

    #[test]
    fn querying_previous_batches() {
        let mut deps = mock_dependencies(&[]);

        let batches = vec![
            Batch {
                id: 1,
                total_shares: Uint128::new(123),
                uluna_unclaimed: Uint128::new(456),
                est_unbond_end_time: 10000,
            },
            Batch {
                id: 2,
                total_shares: Uint128::new(345),
                uluna_unclaimed: Uint128::new(456),
                est_unbond_end_time: 15000,
            },
        ];

        let state = State::default();
        for batch in &batches {
            state.previous_batches.save(deps.as_mut().storage, batch.id.into(), batch).unwrap();
        }

        let res = query_previous_batches(deps.as_ref(), None, None).unwrap();
        assert_eq!(res, batches.clone());

        let res = query_previous_batches(deps.as_ref(), Some(1), None).unwrap();
        assert_eq!(res, vec![batches[1].clone()]);

        let res = query_previous_batches(deps.as_ref(), Some(2), None).unwrap();
        assert_eq!(res, vec![]);
    }

    #[test]
    fn querying_unbond_shares() {
        let mut deps = mock_dependencies(&[]);

        let unbond_shares = vec![
            UnbondRequest {
                id: 1,
                user: String::from("alice"),
                shares: Uint128::new(123),
            },
            UnbondRequest {
                id: 1,
                user: String::from("bob"),
                shares: Uint128::new(234),
            },
            UnbondRequest {
                id: 1,
                user: String::from("charlie"),
                shares: Uint128::new(345),
            },
            UnbondRequest {
                id: 2,
                user: String::from("alice"),
                shares: Uint128::new(456),
            },
        ];

        let state = State::default();
        for unbond_share in &unbond_shares {
            state
                .unbond_requests
                .save(
                    deps.as_mut().storage,
                    (unbond_share.id.into(), &Addr::unchecked(unbond_share.user.clone())),
                    unbond_share,
                )
                .unwrap();
        }

        let res = query_unbond_requests_by_batch(deps.as_ref(), 1, None, None).unwrap();
        assert_eq!(
            res,
            vec![
                unbond_shares[0].clone().into(),
                unbond_shares[1].clone().into(),
                unbond_shares[2].clone().into()
            ]
        );

        let res = query_unbond_requests_by_batch(deps.as_ref(), 2, None, None).unwrap();
        assert_eq!(res, vec![unbond_shares[3].clone().into()]);

        let res = query_unbond_requests_by_user(deps.as_ref(), String::from("alice"), None, None).unwrap();
        assert_eq!(res, vec![unbond_shares[0].clone().into(), unbond_shares[3].clone().into()]);
    }
}
