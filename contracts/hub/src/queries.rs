use cosmwasm_std::{Addr, Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::{Bound, CwIntKey};

use steak::hub::{
    Batch, ConfigResponse, PendingBatch, StateResponse, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem,
};

use crate::helpers::{query_cw20_total_supply, query_delegations};
use crate::state::State;

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        new_owner: state
            .new_owner
            .may_load(deps.storage)?
            .map(|addr| addr.into()),
        steak_token: state.steak_token.load(deps.storage)?.into(),
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        denom: state.denom.load(deps.storage)?,
        fee_account: state.fee_account.load(deps.storage)?.to_string(),
        fee_rate: state.fee_rate.load(deps.storage)?,
        max_fee_rate: state.max_fee_rate.load(deps.storage)?,
        validators: state.validators.load(deps.storage)?,
    })
}

pub fn state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();

    let denom = state.denom.load(deps.storage)?;
    let steak_token = state.steak_token.load(deps.storage)?;
    let total_usteak = query_cw20_total_supply(&deps.querier, &steak_token)?;

    let validators = state.validators.load(deps.storage)?;
    let delegations = query_delegations(&deps.querier, &validators, &env.contract.address, &denom)?;
    let total_native: u128 = delegations.iter().map(|d| d.amount).sum();

    let exchange_rate = if total_usteak.is_zero() {
        Decimal::one()
    } else {
        Decimal::from_ratio(total_native, total_usteak)
    };

    Ok(StateResponse {
        total_usteak,
        total_native: Uint128::new(total_native),
        exchange_rate,
        unlocked_coins: state.unlocked_coins.load(deps.storage)?,
    })
}

pub fn pending_batch(deps: Deps) -> StdResult<PendingBatch> {
    let state = State::default();
    state.pending_batch.load(deps.storage)
}

pub fn previous_batch(deps: Deps, id: u64) -> StdResult<Batch> {
    let state = State::default();
    state.previous_batches.load(deps.storage, id)
}

pub fn previous_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Batch>> {
    let state = State::default();

    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

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

pub fn unbond_requests_by_batch(
    deps: Deps,
    id: u64,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByBatchResponseItem>> {
    let state = State::default();

    let addr: Addr;
    let start = match start_after {
        None => None,
        Some(addr_str) => {
            addr = deps.api.addr_validate(&addr_str)?;
            Some(Bound::exclusive(&addr))
        }
    };
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    state
        .unbond_requests
        .prefix(id)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, v) = item?;
            Ok(v.into())
        })
        .collect()
}

pub fn unbond_requests_by_user(
    deps: Deps,
    user: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<UnbondRequestsByUserResponseItem>> {
    let state = State::default();

    let start = start_after.map(|id| {
        let mut key = vec![0u8, 8u8]; // when `u64` are used as keys, they are prefixed with the length, which is [0, 8]
        key.extend(id.to_cw_bytes());
        Bound::exclusive(key)
    });
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

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
