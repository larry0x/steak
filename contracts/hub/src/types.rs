use std::str::FromStr;

use cosmwasm_std::{Coin, CosmosMsg, StakingMsg, StdError, StdResult, Uint128};
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
    pub fn add(mut self, coin_to_add: &Coin) -> StdResult<Self> {
        match self.0.iter_mut().find(|coin| coin.denom == coin_to_add.denom) {
            Some(coin) => {
                coin.amount = coin.amount.checked_add(coin_to_add.amount)?;
            },
            None => {
                self.0.push(coin_to_add.clone());
            },
        }
        Ok(self)
    }

    pub fn add_many(mut self, coins_to_add: &Coins) -> StdResult<Self> {
        for coin_to_add in &coins_to_add.0 {
            self = self.add(coin_to_add)?;
        }
        Ok(self)
    }
}

//--------------------------------------------------------------------------------------------------
// Delegation
//--------------------------------------------------------------------------------------------------

#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Delegation {
    pub validator: String,
    pub amount: Uint128,
}

impl Delegation {
    pub fn new<T: Into<Uint128>>(validator: &str, amount: T) -> Self {
        Self {
            validator: validator.to_string(),
            amount: amount.into(),
        }
    }

    pub fn to_delegate_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }

    pub fn to_undelegate_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }

    pub fn to_redelegate_msg(&self, src_validator: &str) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Redelegate {
            src_validator: src_validator.to_string(),
            dst_validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }
}
