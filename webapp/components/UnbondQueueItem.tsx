import { chakra, Tr, Td, Text } from "@chakra-ui/react";
import NextLink from "next/link";
import { FC } from "react";

import { capitalizeFirstLetter, formatNumber } from "../helpers";
import { UnbondRequestParsed } from "../store";

const UnbondQueueEmpty: FC = () => {
  return (
    <Tr bg="white" mb="2">
      <Td colSpan={4} py="6" textAlign="center" borderBottom="none" borderRadius="2xl">
        No active unbonding request
      </Td>
    </Tr>
  );
};

const UnbondQueueItem: FC<UnbondRequestParsed> = ({ status, amount, startTime, finishTime }) => {
  const finishTimeItem =
    status === "completed" ? (
      <NextLink href="/withdraw" passHref>
        <chakra.a
          transition="0.2s all"
          outline="none"
          border="solid 2px #d9474b"
          borderRadius="md"
          color="white"
          bg="brand.red"
          px="10"
          py="2"
          _hover={{
            color: "brand.red",
            bg: "white",
            textDecoration: "none",
          }}
        >
          Claim LUNA
        </chakra.a>
      </NextLink>
    ) : (
      <Text>{finishTime.toLocaleString()}</Text>
    );

  return (
    <Tr transition="0.25s all" bg="white" mb="2" _hover={{ bg: "gray.100" }}>
      <Td borderBottom="none" py="6" borderLeftRadius="2xl">
        {capitalizeFirstLetter(status)}
      </Td>
      <Td borderBottom="none" py="6" minW="200px">
        {formatNumber(amount / 1e6, 6) + (status === "pending" ? " STEAK" : " LUNA")}
      </Td>
      <Td borderBottom="none" py="6" minW="230px">
        {startTime.toLocaleString()}
      </Td>
      <Td borderBottom="none" py="6" minW="230px" borderRightRadius="2xl">
        {finishTimeItem}
      </Td>
    </Tr>
  );
};

export { UnbondQueueItem, UnbondQueueEmpty };
