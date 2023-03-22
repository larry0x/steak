import { SimpleGrid } from "@chakra-ui/react";
import { FC } from "react";

import Header from "./Header";
import OverviewItem from "./OverviewItem";
import { useStore } from "../store";
import { formatNumber } from "helpers";

const Overview: FC = () => {
  const store = useStore();

  const exchangeRate = store.hubState ? store.hubState.exchangeRate : 1;
  const totalLunaLocked = store.hubState ? store.hubState.totalLunaLocked : 0;
  const totalValueLocked = store.priceLunaUsd ? totalLunaLocked * store.priceLunaUsd : 0;

  return (
    <>
      <Header text="Overview" />
      <SimpleGrid minChildWidth="250px" spacing="10px" mb="4">
        <OverviewItem
          primaryText={"$" + formatNumber(totalValueLocked, 0)}
          secondaryText={`(${formatNumber(totalLunaLocked, 0)} LUNA)`}
          additionalText="Total value locked"
        />
        <OverviewItem
          primaryText={formatNumber(exchangeRate, 6)}
          secondaryText="LUNA per STEAK"
          additionalText="Exchange ratio"
        />
        <OverviewItem primaryText="TBD" secondaryText="_" additionalText="Current APY" />
      </SimpleGrid>
    </>
  );
};

export default Overview;
