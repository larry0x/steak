use std::collections::{BTreeSet, HashSet};
use std::iter::FromIterator;

use cosmwasm_std::{ Decimal, Deps, Env, Order, StdResult, Uint128};
use cw_storage_plus::Bound;

use pfc_steak::hub::{
    Batch, ConfigResponse, PendingBatch, StateResponse, UnbondRequestsByBatchResponseItem,
    UnbondRequestsByUserResponseItem,
};

use crate::helpers::query_delegations;
use crate::state;
use crate::state::{State, VALIDATORS, VALIDATORS_ACTIVE};

const MAX_LIMIT: u32 = 30;
const DEFAULT_LIMIT: u32 = 10;

pub fn config(deps: Deps) -> StdResult<ConfigResponse> {
    let state = State::default();
    let mut validators: BTreeSet<String> = BTreeSet::new();
    for res in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators.insert(res?);
    }

    let mut validators_active: BTreeSet<String> = BTreeSet::new();
    for res in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        let validator = res?;
        validators.remove(&validator);
        validators_active.insert(validator);
    }


    let validator_active_vec: Vec<String> = Vec::from_iter(validators_active.into_iter());
    let paused_validators: Vec<String> = Vec::from_iter(validators.into_iter());

    Ok(ConfigResponse {
        owner: state.owner.load(deps.storage)?.into(),
        new_owner: state
            .new_owner
            .may_load(deps.storage)?
            .map(|addr| addr.into()),
        steak_token: state.steak_denom.load(deps.storage)?,
        epoch_period: state.epoch_period.load(deps.storage)?,
        unbond_period: state.unbond_period.load(deps.storage)?,
        denom: state.denom.load(deps.storage)?,
        fee_type: state.fee_account_type.load(deps.storage)?.to_string(),
        fee_account: state.fee_account.load(deps.storage)?.to_string(),
        fee_rate: state.fee_rate.load(deps.storage)?,
        max_fee_rate: state.max_fee_rate.load(deps.storage)?,
        validators: validator_active_vec,
        paused_validators,
        dust_collector: state.dust_collector.load(deps.storage)?.map( |a| a.to_string())
    })
}

pub fn state(deps: Deps, env: Env) -> StdResult<StateResponse> {
    let state = State::default();
    let denom = state.denom.load(deps.storage)?;
    let total_usteak = state.steak_minted.load(deps.storage)?;// query_cw20_total_supply(&deps.querier, &steak_token)?;
    let mut validators: HashSet<String> = Default::default();
    for res in VALIDATORS.items(deps.storage, None, None, Order::Ascending) {
        validators.insert(res?);
    }
    let mut validators_active: HashSet<String> = Default::default();
    for res in VALIDATORS_ACTIVE.items(deps.storage, None, None, Order::Ascending) {
        validators_active.insert(res?);
    }
    validators.extend(validators_active);
    let validator_vec: Vec<String> = Vec::from_iter(validators.into_iter());
    let delegations = query_delegations(&deps.querier, &validator_vec, &env.contract.address, &denom)?;
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
    state::previous_batches().load(deps.storage, id)
}

pub fn previous_batches(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Vec<Batch>> {
    let start = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    state::previous_batches()
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
    //let addr: Addr;
    let addr_clone;
    let start = match start_after {
        None => None,
        Some(addr_str) => {
            deps.api.addr_validate(&addr_str)?;
            addr_clone = addr_str;
            Some(Bound::exclusive(addr_clone.as_str()))
        }
    };
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    state::unbond_requests()
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
    let user_addr = deps.api.addr_validate(&user)?;

    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    if let Some(start_id) = start_after {
        state::unbond_requests()
            .idx
            .user
            .prefix(user_addr.to_string())
            .range(deps.storage, None, None, Order::Ascending)
            .filter(|r| {
                let x = r.as_ref().unwrap();

                x.1.id > start_id
            })
            .take(limit)
            .map(|item| {
                let (_, v) = item?;
                Ok(v.into())
            })
            .collect()
    } else {
        state::unbond_requests()
            .idx
            .user
            .prefix(user_addr.to_string())
            .range(deps.storage, None, None, Order::Ascending)
            .take(limit)
            .map(|item| {
                let (_, v) = item?;
                Ok(v.into())
            })
            .collect()
    }
}
