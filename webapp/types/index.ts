export type Asset = {
  logo: string;
  name: string;
  symbol: string;
};

/**
 * Corresponding to Go type `sdk.Coin` or Rust type `cosmwasm_std::Coin`
 */
export type Coin = {
  denom: string;
  amount: string;
};

/**
 * Response of GRPC Gateway `/cosmos/bank/v1beta1/balances/{address}` API, with pagination paramters omitted
 */
export interface NativeBalancesResponse {
  balances: Coin[];
}

/**
 * Response of GRPC Gateway `/cosmos/staking/v1beta1/validators` API, with pagination parameters omitted
 */
export interface ValidatorsResponse {
  validators: {
    operator_address: string;
    consensus_pubkey: {
      "@type": string;
      key: string;
    };
    jailed: boolean;
    status: "BOND_STATUS_BONDED" | "BOND_STATUS_UNBONDING" | "BOND_STATUS_UNBONDED";
    tokens: string;
    delegator_shares: string;
    description: {
      moniker: string;
      identity: string;
      website: string;
      security_contact: string;
      details: string;
    };
    unbonding_height: string;
    unbonding_time: string;
    commission: {
      commission_rates: {
        rate: string;
        max_rate: string;
        max_change_rate: string;
      };
      update_time: string;
    };
    min_self_delegation: string;
  }[];
}

/**
 * Response of GRPC Gateway `/wasm/v1beta1/contracts/{contractAddress}/store` API
 */
export type ContractStoreResponse<T> = {
  query_result: T;
};

/**
 * Response of `cosmwasm_std::BankQuery::Balance`; corresponding to Rust struct `cosmwasm_std::BalanceResponse`
 */
export type NativeBalanceResponse = {
  amount: Coin;
};

/**
 * Corresponding to Rust struct `cw20::BalanceResponse`
 */
export type Cw20BalanceResponse = {
  balance: string;
};

/**
 * Corresponding to Rust strust [`terra_cosmwasm::ExchangeRateResponseItem`](https://github.com/terra-money/terra-cosmwasm/blob/v2.2.0/packages/terra-cosmwasm/src/query.rs#L57-L62)
 */
export type ExchangeRateItem = {
  quote_denom: string;
  exchange_rate: string;
};

/**
 * Corresponding to Rust strust [`terra_cosmwasm::ExchangeRateResponse`](https://github.com/terra-money/terra-cosmwasm/blob/v2.2.0/packages/terra-cosmwasm/src/query.rs#L64-L69)
 */
export type ExchangeRateResponse = {
  base_denom: string;
  exchange_rates: ExchangeRateItem[];
};

/**
 * Response type of the [`multiquery`](https://github.com/st4k3h0us3/multiquery) contract
 */
export type MultiqueryResponse = {
  success: boolean;
  data: string;
}[];

/**
 * Response type of [`steak::hub::QueryMsg::Config`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L106-L116)
 */
export type ConfigResponse = {
  steak_token: string;
  epoch_period: number;
  unbond_period: number;
  validators: string[];
};

/**
 * Response type of [`steak::hub::QueryMsg::State`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L118-L128)
 */
export type StateResponse = {
  total_usteak: string;
  total_uluna: string;
  exchange_rate: string;
  unlocked_coins: Coin[];
};

/**
 * Response type of [`steak::hub::QueryMsg::PendingBatch`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L130-L138)
 */
export type PendingBatch = {
  id: number;
  usteak_to_burn: string;
  est_unbond_start_time: number;
};

/**
 * Corresponding to Rust struct [`steak::hub::Batch`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L140-L150)
 */
export type Batch = {
  id: number;
  reconciled: boolean;
  total_shares: string;
  uluna_unclaimed: string;
  est_unbond_end_time: number;
};

/**
 * Corresponding to Rust struct [`steak::hub::UnbondRequestsByUserResponseItem`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L179-L185)
 */
export type UnbondRequestsByUserResponseItem = {
  id: number;
  shares: string;
};

/**
 * Response type of [`steak::hub::QueryMsg::UnbondRequestsByUser`](https://github.com/st4k3h0us3/steak-contracts/blob/v1.0.0-rc0/packages/steak/src/hub.rs#L98-L103)
 */
export type UnbondRequestsByUserResponse = UnbondRequestsByUserResponseItem[];
