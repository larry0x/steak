import { Box, Button, Flex, Link, Spinner, Text } from "@chakra-ui/react";
import { Msg } from "@terra-money/terra.js";
import {
  useConnectedWallet,
  CreateTxFailed,
  Timeout,
  TxFailed,
  TxUnspecifiedError,
  UserDenied,
} from "@terra-money/wallet-provider";
import axios from "axios";
import { FC, useState, useEffect } from "react";

import ModalWrapper from "./ModalWrapper";
import SuccessIcon from "./SuccessIcon";
import FailedIcon from "./FailedIcon";
import ExternalLinkIcon from "./ExternalLinkIcon";
import { useConstants } from "../hooks";
import { truncateString } from "../helpers";
import { useStore } from "../store";

/**
 * If tx is confirmed, should return an response in the following format:
 *
 * ```typescript
 * {
 *   tx: object;
 *   tx_response: object;
 * }
 * ```
 *
 * If not confirmed, the query either fail with error code 400, or return a response in the following format:
 *
 * ```typescript
 * {
 *   code: number;
 *   message: string;
 *   details: any[];
 * }
 * ```
 */
async function checkTxIsConfirmed(grpcGatewayUrl: string, txhash: string): Promise<boolean> {
  try {
    const { data } = await axios.get(`${grpcGatewayUrl}/cosmos/tx/v1beta1/txs/${txhash}`);
    if ("tx" in data) {
      return true;
    }
  } catch {
    return false;
  }
  return false;
}

function SpinnerWrapper() {
  return (
    <Spinner thickness="6px" speed="1s" emptyColor="transparent" color="brand.red" size="xl" />
  );
}

function TxHashText(network: string, txhash: string) {
  return (
    <Flex>
      <Text variant="dimmed" ml="auto" mr="3">
        Tx Hash
      </Text>
      <Link
        isExternal
        href={`https://terrasco.pe/${network}/tx/${txhash}`}
        ml="3"
        mr="auto"
        my="auto"
        textUnderlineOffset="0.3rem"
      >
        {truncateString(txhash, 6, 6)}
        <ExternalLinkIcon
          ml="2"
          style={{
            transform: "translateY(-2.4px)",
          }}
        />
      </Link>
    </Flex>
  );
}

function TxFailedText(error: any) {
  return (
    <Flex>
      <Text variant="dimmed" ml="auto" mr="3">
        Reason
      </Text>
      <Text ml="3" mr="auto">
        {error instanceof CreateTxFailed
          ? "Failed to create tx"
          : error instanceof Timeout
          ? "Timeout"
          : error instanceof TxFailed
          ? "Tx failed"
          : error instanceof TxUnspecifiedError
          ? "Unspecified"
          : error instanceof UserDenied
          ? "User denied"
          : "Unknown"}
      </Text>
    </Flex>
  );
}

function CloseButton(showCloseBtn: boolean, onClick: () => void) {
  return showCloseBtn ? (
    <Button variant="primary" mt="12" onClick={onClick}>
      Close
    </Button>
  ) : null;
}

type Props = {
  msgs: Msg[];
  isOpen: boolean;
  onClose: () => void;
};

const TxModal: FC<Props> = ({ msgs, isOpen, onClose }) => {
  const wallet = useConnectedWallet();
  const store = useStore();
  const { grpcGatewayUrl, gasOptions } = useConstants(wallet?.network.name);
  const [intervalId, setIntervalId] = useState<NodeJS.Timer>();
  const [showCloseBtn, setShowCloseBtn] = useState<boolean>(false);
  const [txConfirmed, setTxConfirmed] = useState<boolean>(false);
  const [txStatusHeader, setTxStatusHeader] = useState<string>();
  const [txStatusIcon, setTxStatusIcon] = useState<JSX.Element>();
  const [txStatusDetail, setTxStatusDetail] = useState<JSX.Element>();

  useEffect(() => {
    setTxConfirmed(false);
    setTxStatusHeader("Transaction Pending");
    setTxStatusIcon(SpinnerWrapper());
    setTxStatusDetail(<Text>Please confirm tx in wallet popup</Text>);
    setIntervalId(undefined);
    setShowCloseBtn(false);
  }, [isOpen]);

  useEffect(() => {
    if (isOpen && wallet) {
      wallet
        .post({ msgs, ...gasOptions })
        .then((result) => {
          setTxStatusHeader("Transaction Broadcasted");
          setTxStatusDetail(TxHashText(wallet!.network.name, result.result.txhash));
          setIntervalId(
            setInterval(() => {
              checkTxIsConfirmed(grpcGatewayUrl!, result.result.txhash).then((txIsConfirmed) => {
                setTxConfirmed(txIsConfirmed);
              });
            }, 1000)
          );
        })
        .catch((error) => {
          setTxStatusHeader("Transaction Failed");
          setTxStatusIcon(<FailedIcon h="80px" w="80px" />);
          setTxStatusDetail(TxFailedText(error));
          setShowCloseBtn(true);
        });
    }
  }, [isOpen]);

  useEffect(() => {
    if (txConfirmed) {
      setTxStatusHeader("Transaction Confirmed");
      setTxStatusIcon(<SuccessIcon h="80px" w="80px" />);
      setShowCloseBtn(true);
      clearInterval(intervalId!);
      store.update(wallet);
    }
  }, [txConfirmed]);

  return (
    <ModalWrapper showHeader={false} isOpen={isOpen} onClose={onClose}>
      <Box w="100%" textAlign="center">
        <Text fontSize="xl" textStyle="minibutton" mt="10">
          {txStatusHeader}
        </Text>
        <Flex w="100%" h="150px" align="center" justify="center">
          {txStatusIcon}
        </Flex>
        <Box mt="3" mb="10">
          {txStatusDetail}
          {CloseButton(showCloseBtn, onClose)}
        </Box>
      </Box>
    </ModalWrapper>
  );
};

export default TxModal;
