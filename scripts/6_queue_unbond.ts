import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import { createLCDClient, createWallet, encodeBase64, sendTxWithConfirm } from "./helpers";

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
  const worker = createWallet(terra);

  const config: { steak_token: string } = await terra.wasm.contractQuery(argv["steak-hub"], {
    config: {},
  });

  const { txhash } = await sendTxWithConfirm(worker, [
    new MsgExecuteContract(worker.key.accAddress, config["steak_token"], {
      send: {
        contract: argv["steak-hub"],
        amount: argv["amount"],
        msg: encodeBase64({
          queue_unbond: {},
        }),
      },
    }),
  ]);
  console.log(`Success! Txhash: ${txhash}`);
})();
