import { Box, Flex, Spacer, Text } from "@chakra-ui/react";
import { FC, ReactNode } from "react";

type Props = {
  text: string;
  pb?: string;
  children?: ReactNode;
};

const Header: FC<Props> = ({ text, pb = 4, children }) => {
  return (
    <Box px="0" py="4" pb={pb}>
      <Flex>
        <Text fontSize="2xl" fontWeight="800" opacity={0.4}>
          {text}
        </Text>
        <Spacer />
        {children}
      </Flex>
    </Box>
  );
};

export default Header;
