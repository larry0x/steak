import { Box, Table, Thead, Tbody, Tr, Th } from "@chakra-ui/react";
import { useValidators } from "hooks";
import { FC } from "react";

import Header from "./Header";
import ValidatorsItem from "./ValidatorsItem";

const UnbondQueue: FC = () => {
  const validators = useValidators();

  const items = validators.map((validator, index) => <ValidatorsItem key={index} {...validator} />);

  return (
    <>
      <Header text="Whitelisted Validators" pb="1" />
      <Box overflowX="auto">
        <Table style={{ borderCollapse: "separate", borderSpacing: "0 0.6rem" }}>
          <Thead>
            <Tr>
              <Th borderBottom="none" bg="brand.darkBrown" color="white" borderLeftRadius="2xl">
                Validator
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white">
                Status
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white">
                Voting Power
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white">
                Commission
              </Th>
              <Th borderBottom="none" bg="brand.darkBrown" color="white" borderRightRadius="2xl">
                APR
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
