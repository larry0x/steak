use cosmwasm_std::{Deps, Order, StdResult, Env, Uint128, Decimal};
use cw_storage_plus::{Bound, U64Key};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::msg::{
    Batch, ConfigResponse, PendingBatch, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem, StateResponse,
};
use crate::state::State;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    let worker_addrs = state.workers.load(deps.storage)?;
    Ok(ConfigResponse {
        steak_token: state.steak_token.load(deps.storage)?.into(),
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        workers: worker_addrs.iter().map(|addr| addr.to_string()).collect(),
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
        unlocked_coins: state.unlocked_coins.load(deps.storage)?
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
