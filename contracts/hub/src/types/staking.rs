use cosmwasm_std::{Coin, CosmosMsg, StakingMsg};
use osmo_bindings::OsmosisMsg;

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Delegation {
    pub validator: String,
    pub amount: u128,
}

impl Delegation {
    pub fn new(validator: &str, amount: u128) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, "uosmo"),
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Undelegation {
    pub validator: String,
    pub amount: u128,
}

impl Undelegation {
    pub fn new(validator: &str, amount: u128) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, "uosmo"),
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Redelegation {
    pub src: String,
    pub dst: String,
    pub amount: u128,
}

impl Redelegation {
    pub fn new(src: &str, dst: &str, amount: u128) -> Self {
        Self {
            src: src.to_string(),
            dst: dst.to_string(),
            amount,
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg<OsmosisMsg> {
        CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: self.src.clone(),
            dst_validator: self.dst.clone(),
            amount: Coin::new(self.amount, "uosmo"),
        })
    }
}
