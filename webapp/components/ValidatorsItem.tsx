import { Link, Tr, Td, Text, HStack, Image } from "@chakra-ui/react";
import { useConnectedWallet } from "@terra-money/wallet-provider";
import { FC } from "react";

import LunaIcon from "./LunaIcon";
import { formatNumber, formatPercentage } from "../helpers";
import { ValidatorParsed } from "../store";

const ValidatorItem: FC<ValidatorParsed> = ({
  operatorAddress,
  identity,
  isActive,
  moniker,
  tokens,
  commissionRate,
}) => {
  const wallet = useConnectedWallet();
  const network = wallet?.network.name ?? "mainnet";

  return (
    <Tr transition="0.25s all" bg="white" mb="2" _hover={{ bg: "gray.100" }}>
      <Td borderBottom="none" py="3" borderLeftRadius="2xl">
        <HStack>
          <Image
            src={`/${identity}.jpeg`}
            alt={identity}
            h="2rem"
            w="2rem"
            mr="1"
            borderRadius="full"
            onError={({ currentTarget }) => {
              currentTarget.src = "/nopic.jpeg";
            }}
          />
          <Link
            href={`https://terrasco.pe/${network}/validators/${operatorAddress}`}
            isExternal={true}
            mr="1"
            whiteSpace="nowrap"
            textUnderlineOffset="0.3rem"
          >
            {moniker}
          </Link>
        </HStack>
      </Td>
      <Td borderBottom="none" py="3">
        <Text
          bg={isActive ? "green.300" : "red"}
          p="1"
          textAlign="center"
          textStyle="minibutton"
          letterSpacing="wider"
          borderRadius="md"
        >
          {isActive ? "active" : "inactive"}
        </Text>
      </Td>
      <Td borderBottom="none" py="3">
        <HStack>
          <Text>{formatNumber(tokens / 1e6, 0)}</Text>
          <LunaIcon w="1rem" h="1rem" />
        </HStack>
      </Td>
      <Td borderBottom="none" py="3">
        {formatPercentage(commissionRate)}
      </Td>
      <Td borderBottom="none" py="3" borderRightRadius="2xl">
        TBD
      </Td>
    </Tr>
  );
};

export default ValidatorItem;
