import { ChakraProvider } from "@chakra-ui/react";
import { WalletProvider, StaticWalletProvider, NetworkInfo } from "@terra-money/wallet-provider";
import { AppProps } from "next/app";

import { NETWORKS } from "../constants";
import Layout from "../components/Layout";
import theme from "../theme";

const walletNetoworkChainIds: Record<number, NetworkInfo> = {
  0: NETWORKS["testnet"],
  1: NETWORKS["mainnet"],
};

const SteakApp = ({ Component, pageProps }: AppProps) => {
  const main = (
    <ChakraProvider theme={theme}>
      <Layout>
        <Component {...pageProps} />
      </Layout>
    </ChakraProvider>
  );

  return typeof window !== "undefined" ? (
    <WalletProvider defaultNetwork={NETWORKS["mainnet"]} walletConnectChainIds={walletNetoworkChainIds}>
      {main}
    </WalletProvider>
  ) : (
    <StaticWalletProvider defaultNetwork={NETWORKS["mainnet"]}>
      {main}
    </StaticWalletProvider>
  );
};

export default SteakApp;
