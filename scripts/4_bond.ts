import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { createLCDClient, createWallet, sendTxWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    "steak-hub": {
      type: "string",
      demandOption: true,
    },
    amount: {
      type: "string",
      demandOption: true,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const user = createWallet(terra);

  const { txhash } = await sendTxWithConfirm(user, [
    new MsgExecuteContract(
      user.key.accAddress,
      argv["steak-hub"],
      {
        bond: {},
      },
      {
        uluna: argv["amount"],
      }
    ),
  ]);
  console.log(`Success! Txhash: ${txhash}`);
})();
