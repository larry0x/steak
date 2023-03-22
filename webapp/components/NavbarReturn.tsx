import { chakra, HStack, Text } from "@chakra-ui/react";
import NextLink from "next/link";
import { FC, MouseEventHandler } from "react";

import ArrowLeftIcon from "./ArrowLeftIcon";

type Props = {
  onClick?: MouseEventHandler<HTMLAnchorElement>;
};

const NavbarReturn: FC<Props> = ({ onClick }) => {
  return (
    <NextLink href="/" passHref>
      <chakra.a
        color="brand.darkerBrown"
        fill="brand.darkerBrown"
        _hover={{
          color: "brand.red",
          fill: "brand.red",
        }}
        transition="0.2s all"
        whiteSpace="nowrap"
        onClick={onClick}
      >
        <HStack>
          <ArrowLeftIcon w="3rem" h="3rem" />
          <Text fontSize="1.5rem" fontWeight="800">
            Back
          </Text>
        </HStack>
      </chakra.a>
    </NextLink>
  );
};

export default NavbarReturn;
