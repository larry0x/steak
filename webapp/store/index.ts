import { ConnectedWallet } from "@terra-money/wallet-provider";
import axios from "axios";
import create from "zustand";

import { NETWORKS, CONTRACTS } from "../constants";
import { encodeBase64, decodeBase64 } from "../helpers";
import {
  ContractStoreResponse,
  Cw20BalanceResponse,
  ExchangeRateResponse,
  MultiqueryResponse,
  ValidatorsResponse,
  NativeBalanceResponse,
  Batch,
  PendingBatch,
  StateResponse,
  ConfigResponse,
  UnbondRequestsByUserResponse,
} from "../types";

export type ValidatorParsed = {
  operatorAddress: string;
  isActive: boolean;
  moniker: string;
  identity: string;
  tokens: number;
  commissionRate: number;
};

export type UnbondRequestParsed = {
  status: "pending" | "unbonding" | "completed";
  amount: number; // means `usteak` amount if the batch has not been submitted, or `uluna` if already submitted
  startTime: Date;
  finishTime: Date;
  batchIsReconciled: boolean;
};

export type State = {
  priceLunaUsd?: number;
  balances?: {
    uusd: number;
    uluna: number;
    usteak: number;
  };
  hubState?: {
    totalLunaLocked: number;
    exchangeRate: number;
  };
  pendingBatch?: {
    id: number;
    startTime: Date;
  };
  validators?: ValidatorParsed[];
  unbondRequests?: UnbondRequestParsed[];

  update: (wallet?: ConnectedWallet) => Promise<void>;
};

export const useStore = create<State>((set) => ({
  priceLunaUsd: undefined,
  balances: undefined,
  hubState: undefined,
  pendingBatch: undefined,
  unbondRequests: undefined,

  update: async (wallet?: ConnectedWallet) => {
    // Display mainnet stats by default
    const network = wallet ? (wallet.network.name as "mainnet" | "testnet") : "mainnet";

    const grpcGatewayUrl = NETWORKS[network]["lcd"];
    const { multiquery, steakHub, steakToken } = CONTRACTS[network];

    // These are user-independent queries; we query them regardless of whether a wallet is connected
    let queries: object[] = [
      {
        custom: {
          route: "oracle",
          query_data: {
            exchange_rates: {
              base_denom: "uluna",
              quote_denoms: ["uusd"],
            },
          },
        },
      },
      {
        wasm: {
          smart: {
            contract_addr: steakHub,
            msg: encodeBase64({
              state: {},
            }),
          },
        },
      },
      {
        wasm: {
          smart: {
            contract_addr: steakHub,
            msg: encodeBase64({
              config: {},
            }),
          },
        },
      },
      {
        wasm: {
          smart: {
            contract_addr: steakHub,
            msg: encodeBase64({
              pending_batch: {},
            }),
          },
        },
      },
    ];

    // These are user-dependent queries; we query them only if a wallet is connected
    if (wallet) {
      queries = queries.concat([
        {
          bank: {
            balance: {
              address: wallet.terraAddress,
              denom: "uusd",
            },
          },
        },
        {
          bank: {
            balance: {
              address: wallet.terraAddress,
              denom: "uluna",
            },
          },
        },
        {
          wasm: {
            smart: {
              contract_addr: steakToken,
              msg: encodeBase64({
                balance: {
                  address: wallet.terraAddress,
                },
              }),
            },
          },
        },
        {
          wasm: {
            smart: {
              contract_addr: steakHub,
              msg: encodeBase64({
                unbond_requests_by_user: {
                  user: wallet.terraAddress,
                  limit: 30, // we assume the user doesn't have more than 30 outstanding unbonding requests
                },
              }),
            },
          },
        },
      ]);
    }

    const axiosResponse1 = await axios.get<ContractStoreResponse<MultiqueryResponse>>(
      `${grpcGatewayUrl}/terra/wasm/v1beta1/contracts/${multiquery}/store?query_msg=${encodeBase64(queries)}`
    );

    // --------------------------- Process user-independent query result ---------------------------

    const [
      lunaPriceResult,
      hubStateResult,
      hubConfigResult,
      pendingBatchResult,
    ] = axiosResponse1["data"]["query_result"].slice(0, 4);

    if (!lunaPriceResult || !lunaPriceResult.success) {
      throw new Error("Failed to query luna price");
    }
    if (!hubStateResult || !hubStateResult.success) {
      throw new Error("Failed to query hub state");
    }
    if (!hubConfigResult || !hubConfigResult.success) {
      throw new Error("Failed to query hub config");
    }
    if (!pendingBatchResult || !pendingBatchResult.success) {
      throw new Error("Failed to query pending batch");
    }

    const lunaPriceResponse: ExchangeRateResponse = decodeBase64(lunaPriceResult.data);
    const config: ConfigResponse = decodeBase64(hubConfigResult.data);
    const pendingBatch: PendingBatch = decodeBase64(pendingBatchResult.data);
    const hubStateResponse: StateResponse = decodeBase64(hubStateResult.data);

    set({
      priceLunaUsd: Number(lunaPriceResponse["exchange_rates"][0]!["exchange_rate"]),
      hubState: {
        totalLunaLocked: Number(hubStateResponse["total_uluna"]) / 1e6,
        exchangeRate: Number(hubStateResponse["exchange_rate"]),
      },
      pendingBatch: {
        id: pendingBatch.id,
        startTime: new Date(pendingBatch["est_unbond_start_time"] * 1000),
      },
    });

    //----------------------------------- Query validator status -----------------------------------

    const axiosResponse3 = await axios.get<ValidatorsResponse>(
      `${grpcGatewayUrl}/cosmos/staking/v1beta1/validators?status=BOND_STATUS_BONDED&pagination.limit=150`
    );

    const validators = axiosResponse3["data"]["validators"]
      .filter((v) => config.validators.includes(v["operator_address"]))
      .map((v) => ({
        operatorAddress: v["operator_address"],
        isActive: v["jailed"] === false && v["status"] === "BOND_STATUS_BONDED",
        moniker: v["description"]["moniker"],
        identity: v["description"]["identity"],
        tokens: Number(v["tokens"]),
        commissionRate: Number(v["commission"]["commission_rates"]["rate"]),
      }));

    validators.sort((a, b) => {
      if (a.tokens < b.tokens) {
        return 1;
      } else if (a.tokens > b.tokens) {
        return -1;
      } else {
        return 0;
      }
    });

    set({ validators });

    // ---------------------------- Process user-dependent query result ----------------------------

    if (!wallet) { return; }

    const [
      uusdBalanceResult,
      ulunaBalanceResult,
      usteakBalanceResult,
      unbondRequestsByUserResult,
    ] = axiosResponse1["data"]["query_result"].slice(4, 8);

    if (!uusdBalanceResult || !uusdBalanceResult.success) {
      throw new Error("Failed to query uusd balance");
    }
    if (!ulunaBalanceResult || !ulunaBalanceResult.success) {
      throw new Error("Failed to query uluna balance");
    }
    if (!usteakBalanceResult || !usteakBalanceResult.success) {
      throw new Error("Failed to query usteak balance");
    }
    if (!unbondRequestsByUserResult || !unbondRequestsByUserResult.success) {
      throw new Error(`Failed to query unbonding requests by user ${wallet.terraAddress}`);
    }

    const uusdBalanceResponse: NativeBalanceResponse = decodeBase64(uusdBalanceResult.data);
    const ulunaBalanceResponse: NativeBalanceResponse = decodeBase64(ulunaBalanceResult.data);
    const usteakBalanceResponse: Cw20BalanceResponse = decodeBase64(usteakBalanceResult.data);
    const unbondRequests: UnbondRequestsByUserResponse = decodeBase64(unbondRequestsByUserResult.data);

    const ids: number[] = [];
    for (const unbondRequest of unbondRequests) {
      if (unbondRequest.id !== pendingBatch.id) {
        ids.push(unbondRequest.id);
      }
    }

    const batchesById: { [key: number]: Batch } = {};
    if (ids.length > 0) {
      const queries2 = encodeBase64(
        ids.map((id) => ({
          wasm: {
            smart: {
              contract_addr: steakHub,
              msg: encodeBase64({
                previous_batch: id,
              }),
            },
          },
        }))
      );

      const axiosResponse2 = await axios.get<ContractStoreResponse<MultiqueryResponse>>(
        `${grpcGatewayUrl}/terra/wasm/v1beta1/contracts/${multiquery}/store?query_msg=${queries2}`
      );

      for (const result of axiosResponse2["data"]["query_result"]) {
        if (result.success) {
          const batch: Batch = decodeBase64(result.data);
          batchesById[batch.id] = batch;
        } else {
          throw new Error("Fail to query one of the previous batches");
        }
      }
    }

    const currentTime = new Date();
    const unbondRequestsParsed: UnbondRequestParsed[] = [];
    for (const unbondRequest of unbondRequests) {
      if (unbondRequest.id === pendingBatch.id) {
        unbondRequestsParsed.push({
          status: "pending",
          amount: Number(unbondRequest.shares),
          startTime: new Date(pendingBatch["est_unbond_start_time"] * 1000),
          finishTime: new Date(
            (pendingBatch["est_unbond_start_time"] + config["unbond_period"]) * 1000
          ),
          batchIsReconciled: false,
        });
      } else {
        const batch = batchesById[unbondRequest.id]!;
        const finishTime = new Date(batch["est_unbond_end_time"] * 1000);
        unbondRequestsParsed.push({
          status: currentTime < finishTime ? "unbonding" : "completed",
          amount:
            (Number(batch["uluna_unclaimed"]) * Number(unbondRequest.shares)) /
            Number(batch["total_shares"]),
          startTime: new Date((batch["est_unbond_end_time"] - config["unbond_period"]) * 1000),
          finishTime,
          batchIsReconciled: batch["reconciled"],
        });
      }
    }

    unbondRequestsParsed.sort((a, b) => {
      if (a.finishTime < b.finishTime) {
        return 1;
      } else if (a.finishTime > b.finishTime) {
        return -1;
      } else {
        return 0;
      }
    });

    set({
      balances: {
        uusd: Number(uusdBalanceResponse.amount.amount),
        uluna: Number(ulunaBalanceResponse.amount.amount),
        usteak: Number(usteakBalanceResponse.balance),
      },
      unbondRequests: unbondRequestsParsed,
    });
  },
}));
