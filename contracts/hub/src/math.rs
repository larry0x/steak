use cosmwasm_std::Uint128;

use crate::types::Delegation;

//--------------------------------------------------------------------------------------------------
// Minting/burning logics
//--------------------------------------------------------------------------------------------------

/// Compute the amount of Steak token to mint for a specific Luna stake amount. If current total
/// staked amount is zero, we use 1 usteak = 1 uluna; otherwise, we calculate base on the current
/// uluna per ustake ratio.
pub(crate) fn compute_mint_amount(
    usteak_supply: Uint128,
    uluna_to_bond: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let uluna_bonded: Uint128 = current_delegations.iter().map(|d| d.amount).sum();
    if uluna_bonded.is_zero() {
        uluna_to_bond
    } else {
        usteak_supply.multiply_ratio(uluna_to_bond, uluna_bonded)
    }
}

/// Compute the amount of `uluna` to unbond for a specific `usteak` burn amount
///
/// There is no way `usteak` total supply is zero when the user is senting a non-zero amount of `usteak`
/// to burn, so we don't need to handle division-by-zero here
pub(crate) fn compute_unbond_amount(
    usteak_supply: Uint128,
    usteak_to_burn: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let uluna_bonded: Uint128 = current_delegations.iter().map(|d| d.amount).sum();
    uluna_bonded.multiply_ratio(usteak_to_burn, usteak_supply)
}

//--------------------------------------------------------------------------------------------------
// Delegating/undelegating logics
//--------------------------------------------------------------------------------------------------

/// Given the current delegations made to validators, and a specific amount of `uluna` to stake,
/// compute the new delegations to make such that the delegated amount to each validator is as even
/// as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L19-L53
pub(crate) fn compute_delegations(
    uluna_to_bond: Uint128,
    current_delegations: &[Delegation],
) -> Vec<Delegation> {
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount.u128()).sum();
    let validator_count = current_delegations.len() as u128;

    let uluna_to_distribute = uluna_staked + uluna_to_bond.u128();
    let uluna_per_validator = uluna_to_distribute / validator_count;
    let remainder = uluna_to_distribute % validator_count;

    let mut new_delegations: Vec<Delegation> = vec![];
    let mut uluna_available = uluna_to_bond.u128();
    for (i, d) in current_delegations.iter().enumerate() {
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let uluna_for_validator = uluna_per_validator + remainder_for_validator;

        let mut uluna_to_delegate = if d.amount.u128() > uluna_for_validator {
            0
        } else {
            uluna_for_validator - d.amount.u128()
        };

        uluna_to_delegate = std::cmp::min(uluna_to_delegate, uluna_available);
        uluna_available -= uluna_to_delegate;

        if uluna_to_delegate > 0 {
            new_delegations.push(Delegation::new(&d.validator, uluna_to_delegate));
        }

        if uluna_available == 0 {
            break;
        }
    }

    new_delegations
}

/// Given the current delegations made to validators, and a specific amount of `uluna` to unstake,
/// compute the undelegations to make such that the delegated amount to each validator is as even
/// as possible.
///
/// This function is based on Lido's implementation:
/// https://github.com/lidofinance/lido-terra-contracts/blob/v1.0.2/contracts/lido_terra_validators_registry/src/common.rs#L55-102
pub(crate) fn compute_undelegations(
    uluna_to_unbond: Uint128,
    current_delegations: &[Delegation],
) -> Vec<Delegation> {
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount.u128()).sum();
    let validator_count = current_delegations.len() as u128;

    let uluna_to_distribute = uluna_staked - uluna_to_unbond.u128();
    let uluna_per_validator = uluna_to_distribute / validator_count;
    let remainder = uluna_to_distribute % validator_count;

    let mut new_undelegations: Vec<Delegation> = vec![];
    let mut uluna_available = uluna_to_unbond.u128();
    for (i, d) in current_delegations.iter().enumerate() {
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let uluna_for_validator = uluna_per_validator + remainder_for_validator;

        let mut uluna_to_undelegate = if d.amount.u128() < uluna_for_validator {
            0
        } else {
            d.amount.u128() - uluna_for_validator
        };

        uluna_to_undelegate = std::cmp::min(uluna_to_undelegate, uluna_available);
        uluna_available -= uluna_to_undelegate;

        if uluna_to_undelegate > 0 {
            new_undelegations.push(Delegation::new(&d.validator, uluna_to_undelegate));
        }

        if uluna_available == 0 {
            break;
        }
    }

    new_undelegations
}
