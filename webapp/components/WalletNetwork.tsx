import { Box } from "@chakra-ui/react";
import { FC } from "react";

type Props = {
  network?: string;
};

const WalletNetwork: FC<Props> = ({ network }) => {
  return network === "mainnet" ? null : (
    <Box
      color="white"
      bg="brand.red"
      px="10px"
      py="4px"
      borderRadius="full"
      fontSize="sm"
      fontWeight="800"
      textTransform="uppercase"
      minW="0"
      position="absolute"
      top="-1rem"
      right="-0.5rem"
      zIndex="1"
    >
      {network ? network : "network unknown"}
    </Box>
  );
};

export default WalletNetwork;
