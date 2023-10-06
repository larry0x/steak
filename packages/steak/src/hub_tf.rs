use std::str::FromStr;
use cosmwasm_schema::cw_serde;
//
use cosmwasm_std::{Decimal, Uint128};

use crate::hub::CallbackMsg;

#[cw_serde]
pub enum TokenFactoryType {
    CosmWasm =1,
    Kujira =2,
    Injective =3
}
impl ToString for TokenFactoryType {
    fn to_string(&self) -> String {
        match &self {
            TokenFactoryType::CosmWasm => String::from("CosmWasm"),
            TokenFactoryType::Kujira => String::from("Kujira"),
            TokenFactoryType::Injective => String::from("Injective"),
        }
    }
}
impl FromStr for TokenFactoryType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CosmWasm" => Ok(TokenFactoryType::CosmWasm),
            "Kujira" => Ok(TokenFactoryType::Kujira),
            "Injective" => Ok(TokenFactoryType::Injective),
            _ => Err(()),
        }
    }
}

#[cw_serde]
pub struct InstantiateMsg {
    /// Account who can call certain privileged functions
    pub owner: String,
    /// How often the un-bonding queue is to be executed, in seconds
    pub epoch_period: u64,
    /// The staking module's un-bonding time, in seconds
    pub unbond_period: u64,
    /// Initial set of validators who will receive the delegations
    pub validators: Vec<String>,
    /// denomination of coins to steak (uXXXX)
    pub denom: String,
    /// denomination of the steak token (eg steakLuna)
    pub steak_denom: String,
    /// type of fee account
    pub fee_account_type: String,
    /// Fee Account to send fees too
    pub fee_account: String,
    /// Fee "1.00 = 100%"
    pub fee_amount: Decimal,
    /// Max Fee "1.00 = 100%"
    pub max_fee_amount: Decimal,
    // different chains have different token factory implementations
    pub token_factory: String,
    /// The Dust collector contract
    pub dust_collector: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Bond specified amount of Luna
    Bond { receiver: Option<String>, exec_msg: Option<String> },
    /// Bond specified amount of Luna
    Unbond { receiver: Option<String> },

    /// Withdraw Luna that have finished un-bonding in previous batches
    WithdrawUnbonded { receiver: Option<String> },
    /// Withdraw Luna that has finished unbonding in previous batches, for given address
    WithdrawUnbondedAdmin { address: String },
    /// Add a validator to the whitelist; callable by the owner
    AddValidator { validator: String },
    /// Remove a validator from the whitelist; callable by the owner
    RemoveValidator { validator: String },
    /// Remove a validator from the whitelist; callable by the owner. Does not undelegate. use for typos
    RemoveValidatorEx { validator: String },

    /// Pause a validator from accepting new delegations
    PauseValidator { validator: String },
    /// Unpause a validator from accepting new delegations
    UnPauseValidator { validator: String },

    /// Transfer ownership to another account; will not take effect unless the new owner accepts
    TransferOwnership { new_owner: String },
    /// Accept an ownership transfer
    AcceptOwnership {},
    /// Claim staking rewards, swap all for Luna, and restake
    Harvest {},
    /// Use redelegations to balance the amounts of Luna delegated to validators
    Rebalance { minimum: Uint128 },
    /// redelegate stake from one validator to another
    Redelegate { validator_from: String, validator_to: String },
    /// Update Luna amounts in unbonding batches to reflect any slashing or rounding errors
    Reconcile {},
    /// Submit the current pending batch of unbonding requests to be unbonded
    SubmitBatch {},
    /// Set unbond period
    SetUnbondPeriod { unbond_period: u64 },

    /// Transfer Fee collection account to another account
    TransferFeeAccount {
        fee_account_type: String,
        new_fee_account: String,
    },
    /// Update fee collection amount
    UpdateFee { new_fee: Decimal },
    /// Callbacks; can only be invoked by the contract itself
    Callback(CallbackMsg),
    /// Set Dust Collector Contract
    SetDustCollector { dust_collector: Option<String> },
    /// Collect the Dust
    CollectDust {},
    /// Return the Dust in shiny 'base denom'
    ReturnDenom {},
}

