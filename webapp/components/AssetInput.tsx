import {
  Box,
  Button,
  Flex,
  HStack,
  Image,
  NumberInput,
  NumberInputField,
  Text,
} from "@chakra-ui/react";
import { formatNumber } from "helpers";
import { FC, useState } from "react";

type Props = {
  assetSymbol: string;
  assetLogo: string;
  price?: number;
  balance?: number;
  isEditable?: boolean; // important
  fixedAmount?: number; // must supply if `isEditable` is set to false
  onAmountChange?: (newAmount: number) => void;
};

const AssetInput: FC<Props> = ({
  assetSymbol,
  assetLogo,
  price = 0,
  balance = 0,
  isEditable = true,
  fixedAmount,
  onAmountChange = () => {},
}) => {
  const [amount, setAmount] = useState<number>(0);

  const maxBtn = isEditable ? (
    <Button
      type="button"
      variant="mini"
      onClick={() => {
        setAmount(balance);
        onAmountChange(balance);
      }}
      isDisabled={false}
    >
      Max
    </Button>
  ) : null;

  return (
    <Box bg="white" borderRadius="2xl" p="6" mb="2">
      <Flex direction={["column", null, "row"]}>
        <Box flex="1">
          <Box
            bg="brand.red"
            color="white"
            display="flex"
            borderRadius="full"
            textAlign="left"
            px="4"
            h="16"
            lineHeight="1.2"
          >
            <Flex align="center">
              <Box>
                <Image src={assetLogo} alt="Logo" width="10" height="10" />
              </Box>
              <Box ml="3" flex="1">
                <Text fontSize="2xl">{assetSymbol}</Text>
                <Text fontSize="sm">Price: ${formatNumber(price, 2)}</Text>
              </Box>
            </Flex>
          </Box>
        </Box>
        <Box flex="1" ml={[null, null, "8"]} mt={["4", null, "0"]}>
          <NumberInput
            defaultValue={0}
            value={fixedAmount ?? amount} // if no external fixed amount is set, then use the internal amount
            min={0}
            max={isEditable ? balance : undefined} // if not editable, then do not set a max
            precision={6}
            onChange={(value: string) => {
              setAmount(Number(value));
              onAmountChange(Number(value));
            }}
            isDisabled={!isEditable}
            clampValueOnBlur={true}
          >
            <NumberInputField
              h="16"
              bg="brand.darkBrown"
              fontSize="2xl"
              textAlign="right"
              p="4"
              pt="0"
              placeholder="0.0"
              _disabled={{
                bg: "brand.lighterBrown",
                opacity: "1.0",
                cursor: "not-allowed",
              }}
            />
            <Box position="absolute" bottom="2" right="1.1rem">
              <Text fontSize="small">${formatNumber(price * (fixedAmount ?? amount), 2)}</Text>
            </Box>
          </NumberInput>
          <Flex align="center" justify="space-between" mt="1">
            <HStack spacing="4">
              <Text variant="dimmed" fontSize="sm">
                In Wallet:
              </Text>
              <Text fontSize="sm" ml="2">
                {formatNumber(balance, 3)}
              </Text>
            </HStack>
            <Box>{maxBtn}</Box>
          </Flex>
        </Box>
      </Flex>
    </Box>
  );
};

export default AssetInput;
