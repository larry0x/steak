import { Box, Table, Thead, Tbody, Tr, Th } from "@chakra-ui/react";
import { useUnbondRequests } from "hooks";
import { FC } from "react";

import Header from "./Header";
import { UnbondQueueItem, UnbondQueueEmpty } from "./UnbondQueueItem";

const UnbondQueue: FC = () => {
  const unbondRequests = useUnbondRequests();

  const items = unbondRequests.length > 0
    ? (
      unbondRequests.map((unbondRequest, index) => <UnbondQueueItem key={index} {...unbondRequest} />)
    )
    : (
      <UnbondQueueEmpty />
    );

  return (
    <>
      <Header text="My Unbonding Requests" pb="1" />
      <Box overflowX="auto">
        <Table style={{ borderCollapse: "separate", borderSpacing: "0 0.6rem" }}>
          <Thead>
            <Tr>
              <Th borderBottom="none" bg="brand.darkBrown" color="white" borderLeftRadius="2xl">
                Status
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white">
                Amount
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white">
                Start Time
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white" borderRightRadius="2xl">
                Est. Finish Time
              </Th>
            </Tr>
          </Thead>
          <Tbody>{items}</Tbody>
        </Table>
      </Box>
    </>
  );
};

export default UnbondQueue;
