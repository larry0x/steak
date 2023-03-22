import * as constants from "../constants";
import { useStore } from "../store";

export const useConstants = (network?: string) => {
  if (!network) {
    return {};
  }

  if (network !== "mainnet" && network !== "testnet") {
    throw new Error(`Invalid network ${network}; must be mainnet|testnet`);
  }

  return {
    grpcGatewayUrl: constants.NETWORKS[network]["lcd"],
    gasOptions: constants.GAS_OPTIONS,
    contracts: constants.CONTRACTS[network],
  };
};

export const usePrices = () => {
  return useStore((state) => {
    const { priceLunaUsd, hubState } = state;
    return {
      luna: priceLunaUsd,
      steak: priceLunaUsd && hubState ? priceLunaUsd * hubState.exchangeRate : undefined,
    };
  });
};

export const useBalances = () => useStore((state) => state.balances);

export const useValidators = () => useStore((state) => state.validators ?? []);

export const useExchangeRate = () => useStore((state) => state.hubState?.exchangeRate);

export const useNextBatchTime = () => useStore((state) => state.pendingBatch?.startTime);

export const useUnbondRequests = () => useStore((state) => state.unbondRequests ?? []);
