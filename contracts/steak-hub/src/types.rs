use cosmwasm_std::{Addr, Coin, CosmosMsg, QuerierWrapper, StakingMsg, StdResult, Uint128};
use terra_cosmwasm::TerraMsgWrapper;

pub(crate) struct Delegation {
    pub validator: String,
    pub amount: Uint128,
}

impl Delegation {
    /// Create a new `Delegation` instance of the specified validator address and delegation amount
    pub fn new<T: Into<Uint128>>(validator: &str, amount: T) -> Self {
        Self {
            validator: String::from(validator),
            amount: amount.into(),
        }
    }

    /// Create a `CosmosMsg` to make the delegation
    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }

    /// Query the current delegation of the specified validator-delegator pair
    pub fn query(
        querier: &QuerierWrapper,
        validator: &str,
        delegator_addr: &Addr,
    ) -> StdResult<Self> {
        Ok(Self {
            validator: String::from(validator),
            amount: querier
                .query_delegation(delegator_addr, validator)?
                .map(|fd| fd.amount.amount)
                .unwrap_or_else(Uint128::zero),
        })
    }
}

pub(crate) struct Undelegation {
    pub validator: String,
    pub amount: Uint128,
}

impl Undelegation {
    /// Create a new `Undelegation` instance of the specified validator address and undelegation amount
    pub fn new<T: Into<Uint128>>(validator: &str, amount: T) -> Self {
        Self {
            validator: String::from(validator),
            amount: amount.into(),
        }
    }

    /// Create a `CosmosMsg` to make the delegation
    pub fn to_cosmos_msg(&self) -> CosmosMsg<TerraMsgWrapper> {
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }
}