use std::str::FromStr;

use cosmwasm_std::{Coin, CosmosMsg, StakingMsg, StdError, StdResult};
use terra_cosmwasm::TerraMsgWrapper;

use crate::helpers::parse_coin;

//--------------------------------------------------------------------------------------------------
// Coins
//--------------------------------------------------------------------------------------------------

pub(crate) struct Coins(pub Vec<Coin>);

impl FromStr for Coins {
    type Err = StdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Ok(Self(vec![]));
        }

        Ok(Self(
            s.split(',')
                .filter(|coin_str| !coin_str.is_empty()) // coin with zero amount may appeat as an empty string in the event log
                .collect::<Vec<&str>>()
                .iter()
                .map(|s| parse_coin(s))
                .collect::<StdResult<Vec<Coin>>>()?,
        ))
    }
}

impl Coins {
    pub fn add(&mut self, coin_to_add: &Coin) -> StdResult<()> {
        match self.0.iter_mut().find(|coin| coin.denom == coin_to_add.denom) {
            Some(coin) => {
                coin.amount = coin.amount.checked_add(coin_to_add.amount)?;
            },
            None => {
                self.0.push(coin_to_add.clone());
            },
        }
        Ok(())
    }

    pub fn add_many(&mut self, coins_to_add: &Coins) -> StdResult<()> {
        for coin_to_add in &coins_to_add.0 {
            self.add(coin_to_add)?;
        }
        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Delegation
//--------------------------------------------------------------------------------------------------

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Delegation {
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

    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, "uluna"),
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Undelegation {
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

    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount, "uluna"),
        })
    }
}

#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Redelegation {
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

    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: self.src.clone(),
            dst_validator: self.dst.clone(),
            amount: Coin::new(self.amount, "uluna"),
        })
    }
}
