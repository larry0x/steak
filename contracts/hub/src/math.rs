use std::{cmp, cmp::Ordering};

use cosmwasm_std::Uint128;

use steak::hub::Batch;

use crate::types::{Delegation, Redelegation, Undelegation};

//--------------------------------------------------------------------------------------------------
// Minting/burning logics
//--------------------------------------------------------------------------------------------------

/// Compute the amount of Steak token to mint for a specific Luna stake amount. If current total
/// staked amount is zero, we use 1 usteak = 1 native; otherwise, we calculate base on the current
/// native per ustake ratio.
pub(crate) fn compute_mint_amount(
    usteak_supply: Uint128,
    native_to_bond: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let native_bonded: u128 = current_delegations.iter().map(|d| d.amount).sum();
    if native_bonded == 0 {
        native_to_bond
    } else {
        usteak_supply.multiply_ratio(native_to_bond, native_bonded)
    }
}

/// Compute the amount of `native` to unbond for a specific `usteak` burn amount
///
/// There is no way `usteak` total supply is zero when the user is senting a non-zero amount of `usteak`
/// to burn, so we don't need to handle division-by-zero here
pub(crate) fn compute_unbond_amount(
    usteak_supply: Uint128,
    usteak_to_burn: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let native_bonded: u128 = current_delegations.iter().map(|d| d.amount).sum();
    Uint128::new(native_bonded).multiply_ratio(usteak_to_burn, usteak_supply)
}

//--------------------------------------------------------------------------------------------------
// Delegation logics
//--------------------------------------------------------------------------------------------------

/// Given the current delegations made to validators, and a specific amount of `native` to unstake,
/// compute the undelegations to make such that the delegated amount to each validator is as even
/// as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L55-102
pub(crate) fn compute_undelegations(
    native_to_unbond: Uint128,
    current_delegations: &[Delegation],
    denom: &str,
) -> Vec<Undelegation> {
    let native_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let validator_count = current_delegations.len() as u128;

    let native_to_distribute = native_staked - native_to_unbond.u128();
    let native_per_validator = native_to_distribute / validator_count;
    let remainder = native_to_distribute % validator_count;

    let mut new_undelegations: Vec<Undelegation> = vec![];
    let mut native_available = native_to_unbond.u128();
    for (i, d) in current_delegations.iter().enumerate() {
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let native_for_validator = native_per_validator + remainder_for_validator;

        let mut native_to_undelegate = if d.amount < native_for_validator {
            0
        } else {
            d.amount - native_for_validator
        };

        native_to_undelegate = cmp::min(native_to_undelegate, native_available);
        native_available -= native_to_undelegate;

        if native_to_undelegate > 0 {
            new_undelegations.push(Undelegation::new(&d.validator, native_to_undelegate, denom));
        }

        if native_available == 0 {
            break;
        }
    }

    new_undelegations
}

/// Given a validator who is to be removed from the whitelist, and current delegations made to other
/// validators, compute the new delegations to make such that the delegated amount to each validator
// is as even as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L19-L53
pub(crate) fn compute_redelegations_for_removal(
    delegation_to_remove: &Delegation,
    current_delegations: &[Delegation],
    denom: &str,
) -> Vec<Redelegation> {
    let native_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let validator_count = current_delegations.len() as u128;

    let native_to_distribute = native_staked + delegation_to_remove.amount;
    let native_per_validator = native_to_distribute / validator_count;
    let remainder = native_to_distribute % validator_count;

    let mut new_redelegations: Vec<Redelegation> = vec![];
    let mut native_available = delegation_to_remove.amount;
    for (i, d) in current_delegations.iter().enumerate() {
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let native_for_validator = native_per_validator + remainder_for_validator;

        let mut native_to_redelegate = if d.amount > native_for_validator {
            0
        } else {
            native_for_validator - d.amount
        };

        native_to_redelegate = cmp::min(native_to_redelegate, native_available);
        native_available -= native_to_redelegate;

        if native_to_redelegate > 0 {
            new_redelegations.push(Redelegation::new(
                &delegation_to_remove.validator,
                &d.validator,
                native_to_redelegate,
                denom,
            ));
        }

        if native_available == 0 {
            break;
        }
    }

    new_redelegations
}

/// Compute redelegation moves that will make each validator's delegation the targeted amount (hopefully
/// this sentence makes sense)
///
/// This algorithm does not guarantee the minimal number of moves, but is the best I can some up with...
pub(crate) fn compute_redelegations_for_rebalancing(
    current_delegations: &[Delegation],
) -> Vec<Redelegation> {
    let native_staked: u128 = current_delegations.iter().map(|d| d.amount).sum();
    let validator_count = current_delegations.len() as u128;

    let native_per_validator = native_staked / validator_count;
    let remainder = native_staked % validator_count;

    // If a validator's current delegated amount is greater than the target amount, native will be
    // redelegated _from_ them. They will be put in `src_validators` vector
    // If a validator's current delegated amount is smaller than the target amount, native will be
    // redelegated _to_ them. They will be put in `dst_validators` vector
    let mut src_delegations: Vec<Delegation> = vec![];
    let mut dst_delegations: Vec<Delegation> = vec![];
    for (i, d) in current_delegations.iter().enumerate() {
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let native_for_validator = native_per_validator + remainder_for_validator;

        match d.amount.cmp(&native_for_validator) {
            Ordering::Greater => {
                src_delegations.push(Delegation::new(
                    &d.validator,
                    d.amount - native_for_validator,
                    &d.denom,
                ));
            }
            Ordering::Less => {
                dst_delegations.push(Delegation::new(
                    &d.validator,
                    native_for_validator - d.amount,
                    &d.denom,
                ));
            }
            Ordering::Equal => (),
        }
    }

    let mut new_redelegations: Vec<Redelegation> = vec![];
    while !src_delegations.is_empty() && !dst_delegations.is_empty() {
        let src_delegation = src_delegations[0].clone();
        let dst_delegation = dst_delegations[0].clone();
        let native_to_redelegate = cmp::min(src_delegation.amount, dst_delegation.amount);

        if src_delegation.amount == native_to_redelegate {
            src_delegations.remove(0);
        } else {
            src_delegations[0].amount -= native_to_redelegate;
        }

        if dst_delegation.amount == native_to_redelegate {
            dst_delegations.remove(0);
        } else {
            dst_delegations[0].amount -= native_to_redelegate;
        }

        new_redelegations.push(Redelegation::new(
            &src_delegation.validator,
            &dst_delegation.validator,
            native_to_redelegate,
            &src_delegation.denom,
        ));
    }

    new_redelegations
}

//--------------------------------------------------------------------------------------------------
// Batch logics
//--------------------------------------------------------------------------------------------------

/// If the received native amount after the unbonding period is less than expected, e.g. due to rounding
/// error or the validator(s) being slashed, then deduct the difference in amount evenly from each
/// unreconciled batch.
///
/// The idea of "reconciling" is based on Stader's implementation:
/// https://github.com/stader-labs/stader-liquid-token/blob/v0.2.1/contracts/staking/src/contract.rs#L968-L1048
pub(crate) fn reconcile_batches(batches: &mut [Batch], native_to_deduct: Uint128) {
    let batch_count = batches.len() as u128;
    let native_per_batch = native_to_deduct.u128() / batch_count;
    let remainder = native_to_deduct.u128() % batch_count;

    for (i, batch) in batches.iter_mut().enumerate() {
        let remainder_for_batch: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let native_for_batch = native_per_batch + remainder_for_batch;

        batch.amount_unclaimed -= Uint128::new(native_for_batch);
        batch.reconciled = true;
    }
}
