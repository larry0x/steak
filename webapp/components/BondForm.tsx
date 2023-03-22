import { useDisclosure, Box, Flex, Button, Text } from "@chakra-ui/react";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { useConnectedWallet } from "@terra-money/wallet-provider";
import { FC, useEffect, useState } from "react";

import Header from "./Header";
import AssetInput from "./AssetInput";
import ArrowDownIcon from "./ArrowDownIcon";
import TxModal from "./TxModal";
import { truncateDecimals } from "../helpers";
import { usePrices, useBalances, useExchangeRate, useConstants } from "../hooks";

const BondForm: FC = () => {
  const wallet = useConnectedWallet();
  const prices = usePrices();
  const balances = useBalances();
  const exchangeRate = useExchangeRate();
  const [offerAmount, setOfferAmount] = useState<number>(0);
  const [returnAmount, setReturnAmount] = useState<number>(0);
  const [msgs, setMsgs] = useState<MsgExecuteContract[]>([]);
  const { contracts } = useConstants(wallet?.network.name);
  const { isOpen, onOpen, onClose } = useDisclosure();

  useEffect(() => {
    if (wallet && contracts) {
      setMsgs([
        new MsgExecuteContract(
          wallet.terraAddress,
          contracts.steakHub,
          {
            bond: {},
          },
          {
            uluna: offerAmount * 1e6,
          }
        ),
      ]);
    } else {
      setMsgs([]);
    }
  }, [wallet?.network.name, wallet?.terraAddress, offerAmount]);

  const handleOfferAmountChange = (newOfferAmount: number) => {
    setOfferAmount(newOfferAmount);
    setReturnAmount(exchangeRate ? truncateDecimals(newOfferAmount / exchangeRate) : 0);
  };

  return (
    <Box maxW="container.sm" mx="auto">
      <Header text="Stake LUNA" />
      <Box position="relative">
        <AssetInput
          assetSymbol="LUNA"
          assetLogo="/luna.png"
          price={prices.luna}
          balance={balances ? balances.uluna / 1e6 : 0}
          isEditable={true}
          onAmountChange={handleOfferAmountChange}
        />
        <Flex
          justify="center"
          align="center"
          position="absolute"
          top="50%"
          left="50%"
          transform="translate(-50%, -50%)"
        >
          <ArrowDownIcon
            w="3rem"
            h="3rem"
            fill="brand.red"
            bg="white"
            border="solid 6px white"
            borderRadius="full"
          />
        </Flex>
        <AssetInput
          assetSymbol="STEAK"
          assetLogo="/steak.png"
          price={prices.steak}
          balance={balances ? balances.usteak / 1e6 : 0}
          isEditable={false}
          fixedAmount={returnAmount}
        />
      </Box>
      <Box textAlign="center">
        <Button
          type="button"
          variant="primary"
          mt="6"
          onClick={onOpen}
          isLoading={false}
          isDisabled={!wallet || offerAmount == 0}
        >
          Stake
        </Button>
        <Text mt="3" textStyle="small" variant="dimmed" textAlign="center">
          {""}
        </Text>
        <TxModal isOpen={isOpen} onClose={onClose} msgs={msgs} />
      </Box>
    </Box>
  );
};

export default BondForm;
