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
    uluna_to_stake: Uint128,
    current_delegations: &[Delegation],
) -> Uint128 {
    let uluna_staked: Uint128 = current_delegations.iter().map(|d| d.amount).sum();
    if uluna_staked.is_zero() {
        uluna_to_stake
    } else {
        usteak_supply.multiply_ratio(uluna_to_stake, uluna_staked)
    }
}

//--------------------------------------------------------------------------------------------------
// Delegating/undelegating logics
//--------------------------------------------------------------------------------------------------

/// Given the current delegations made to validators, and a specific amount of `uluna` to stake,
/// compute the new delegations to make such that the delegated amount to each validator is as even
/// as possible.
pub(crate) fn compute_delegations(
    uluna_to_stake: Uint128,
    current_delegations: &[Delegation],
) -> Vec<Delegation> {
    // The total amount of `uluna` currently staked to validators, and the number of validators
    let uluna_staked: u128 = current_delegations.iter().map(|d| d.amount.u128()).sum();
    let validator_count = current_delegations.len() as u128;

    // The _target_ amount of `uluna` that each validator should receive
    let uluna_to_distribute = uluna_staked + uluna_to_stake.u128();
    let uluna_per_validator = uluna_to_distribute / validator_count;
    let remainder = uluna_to_distribute % validator_count;

    // The new delegations to make such that each validator's delegated amount is as close to the
    // target amount as possible
    let mut new_delegations: Vec<Delegation> = vec![];
    let mut uluna_available = uluna_to_stake.u128();
    for (i, d) in current_delegations.iter().enumerate() {
        // The target amount for this specific validator, with the remainder taken into account
        let remainder_for_validator: u128 = if (i + 1) as u128 <= remainder { 1 } else { 0 };
        let uluna_for_validator = uluna_per_validator + remainder_for_validator;

        // If the validator's actual delegation amount is bigger than the target amount, we do not
        // not delegate to it this time
        //
        // Otherwise, if the actual delegation amount is smaller than the target amount, we attempt
        // to delegate the difference
        let mut uluna_to_delegate = if d.amount.u128() > uluna_for_validator {
            0
        } else {
            uluna_for_validator - d.amount.u128()
        };

        // Also need to check if we have enough `uluna` available to delegate
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

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // WIP
}
