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

    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }
}

//--------------------------------------------------------------------------------------------------
// Undelegation
//--------------------------------------------------------------------------------------------------

#[cfg_attr(test, derive(Debug, PartialEq))]
pub(crate) struct Undelegation {
    pub validator: String,
    pub amount: Uint128,
}

impl Undelegation {
    pub fn new<T: Into<Uint128>>(validator: &str, amount: T) -> Self {
        Self {
            validator: validator.to_string(),
            amount: amount.into(),
        }
    }

    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }
}

//--------------------------------------------------------------------------------------------------
// Undelegation
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsing_coins() {
        let coins = Coins::from_str("").unwrap();
        assert_eq!(coins.0, vec![]);

        let coins = Coins::from_str("12345uatom").unwrap();
        assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

        let coins = Coins::from_str("12345uatom,23456uluna").unwrap();
        assert_eq!(coins.0, vec![Coin::new(12345, "uatom"), Coin::new(23456, "uluna")]);
    }

    #[test]
    fn adding_coins() {
        let mut coins = Coins(vec![]);

        coins = coins.add(&Coin::new(12345, "uatom")).unwrap();
        assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

        coins = coins.add(&Coin::new(23456, "uluna")).unwrap();
        assert_eq!(coins.0, vec![Coin::new(12345, "uatom"), Coin::new(23456, "uluna")]);

        coins = coins.add_many(&Coins::from_str("76543uatom,69420uusd").unwrap()).unwrap();
        assert_eq!(
            coins.0,
            vec![Coin::new(88888, "uatom"), Coin::new(23456, "uluna"), Coin::new(69420, "uusd")]
        );
    }

    #[test]
    fn casting_delegation_to_msg() {
        let d = Delegation::new("alice", 12345u128);
        assert_eq!(
            d.to_cosmos_msg(),
            CosmosMsg::Staking(StakingMsg::Delegate {
                validator: String::from("alice"),
                amount: Coin::new(12345, "uluna"),
            }),
        );
    }

    #[test]
    fn casting_undelegation_to_msg() {
        let ud = Undelegation::new("bob", 23456u128);
        assert_eq!(
            ud.to_cosmos_msg(),
            CosmosMsg::Staking(StakingMsg::Undelegate {
                validator: String::from("bob"),
                amount: Coin::new(23456, "uluna"),
            }),
        );
    }
}
