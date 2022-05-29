# Stake Token

Stake Token is based the [vanilla CW20 contract](https://github.com/CosmWasm/cw-plus/tree/v0.9.1/contracts/cw20-base) with two changes to prevent manipulations of token prices:

- `ExecuteMsg::Burn` can only be executed by the minter, i.e. Eris Staking Hub contract;
- `ExecuteMsg::BurnFrom` is disabled.
