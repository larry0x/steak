import { chakra, Link, Box, Flex, Text } from "@chakra-ui/react";
import { useConnectedWallet } from "@terra-money/wallet-provider";
import NextLink from "next/link";
import { FC } from "react";

import Header from "./Header";
import AstroportIcon from "./AstroportIcon";
import { usePrices, useBalances, useConstants } from "../hooks";
import { formatNumber } from "../helpers";

const bondOrUnbondStyle = {
  transition: "0.2s all",
  outline: "none",
  borderRadius: "md",
  color: "brand.red",
  bg: "white",
  px: "10",
  py: "2",
  m: "1",
  _hover: {
    color: "brand.black",
    bg: "brand.lightBrown",
    textDecoration: "none",
  },
};

const MySteak: FC = () => {
  const wallet = useConnectedWallet();
  const prices = usePrices();
  const balances = useBalances();
  const { contracts } = useConstants(wallet?.network.name);

  const steakBalance = balances ? balances.usteak / 1e6 : undefined;
  const steakValue = steakBalance && prices.steak ? steakBalance * prices.steak : undefined;

  return (
    <>
      <Header text="My Steak">
        <Link
          variant="submit"
          isExternal={true}
          href={`https://app.astroport.fi/swap?from=uluna&to=${contracts?.steakToken}`}
        >
          <Flex
            display={["none", "flex", null, null]}
            w="100%"
            h="100%"
            justify="center"
            align="center"
          >
            Trade STEAK on <AstroportIcon w="1.6rem" h="1.6rem" ml="2" mr="1" /> Astroport
          </Flex>
          <Flex
            display={["flex", "none", null, null]}
            w="100%"
            h="100%"
            justify="center"
            align="center"
          >
            Trade STEAK
          </Flex>
        </Link>
      </Header>
      <Box color="white" bg="brand.red" p="12" mb="4" borderRadius="2xl" textAlign="center">
        <Text fontSize="6xl" fontWeight="800">
          {steakBalance ? formatNumber(steakBalance, 3) : "0.000"}
        </Text>
        <Text fontWeight="800">
          {"($" + (steakValue ? formatNumber(steakValue, 2) : "0.00") + ")"}
        </Text>
        <Text color="brand.lightBrown" mt="5">
          STEAK balance in wallet
        </Text>
        <Flex direction={["column", "row", null, null]} justify="center" mt="10">
          <NextLink href="/bond" passHref>
            <chakra.a {...bondOrUnbondStyle}>Stake LUNA</chakra.a>
          </NextLink>
          <NextLink href="/unbond" passHref>
            <chakra.a {...bondOrUnbondStyle}>Unstake STEAK</chakra.a>
          </NextLink>
        </Flex>
      </Box>
    </>
  );
};

export default MySteak;
