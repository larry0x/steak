use cosmwasm_std::{Addr, Coin, CosmosMsg, QuerierWrapper, StakingMsg, StdResult, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub(crate) struct Delegation {
    pub validator: String,
    pub amount: Uint128,
}

impl Delegation {
    /// Create a new `Delegation` instance of the specified validator address and delegated amount
    pub fn new<T: Into<Uint128>>(validator: &String, amount: T) -> Self {
        Self {
            validator: validator.clone(),
            amount: amount.into(),
        }
    }

    /// Create a `CosmosMsg` to make the delegation
    pub fn into_cosmos_msg(&self) -> CosmosMsg {
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: self.validator.clone(),
            amount: Coin::new(self.amount.u128(), "uluna"),
        })
    }

    /// Query the current delegation of the specified validator-delegator pair
    pub fn query(
        querier: &QuerierWrapper,
        validator: &String,
        delegator_addr: &Addr,
    ) -> StdResult<Self> {
        Ok(Self {
            validator: validator.clone(),
            amount: querier
                .query_delegation(delegator_addr, validator)?
                .map(|d| d.amount.amount)
                .unwrap_or_else(Uint128::zero),
        })
    }
}
