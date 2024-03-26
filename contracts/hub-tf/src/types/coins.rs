use std::str::FromStr;

use cosmwasm_std::{Coin, StdError, StdResult};

use crate::helpers::parse_coin;

pub struct Coins(pub Vec<Coin>);

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

    pub fn find(&self, denom: &str) -> Coin {
        self.0
            .iter()
            .find(|coin| coin.denom == denom)
            .cloned()
            .unwrap_or_else(|| Coin::new(0, denom))
    }
}
