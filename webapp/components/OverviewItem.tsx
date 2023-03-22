import { FC } from "react";
import { Box, Text } from "@chakra-ui/react";

type Props = {
  primaryText: string;
  secondaryText: string;
  additionalText: string;
};

const OverviewItem: FC<Props> = ({ primaryText, secondaryText, additionalText }) => {
  return (
    <Box
      color="black"
      bg="white"
      p="6"
      pt="5"
      borderRadius="2xl"
      position="relative"
      textAlign="center"
    >
      <Text fontSize="3xl" fontWeight="800">
        {primaryText}
      </Text>
      <Text fontSize="sm" fontWeight="800">
        {secondaryText}
      </Text>
      <Text opacity="0.4" mt="6">
        {additionalText}
      </Text>
    </Box>
  );
};

export default OverviewItem;
