import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import * as keystore from "./keystore";
import { createLCDClient, createWallet, encodeBase64, sendTxWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    key: {
      type: "string",
      demandOption: true,
    },
    "key-dir": {
      type: "string",
      demandOption: false,
      default: keystore.DEFAULT_KEY_DIR,
    },
    "hub-address": {
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
  const worker = await createWallet(terra, argv["key"], argv["key-dir"]);

  const config: { stake_token: string } = await terra.wasm.contractQuery(argv["hub-address"], {
    config: {},
  });

  const { txhash } = await sendTxWithConfirm(worker, [
    new MsgExecuteContract(worker.key.accAddress, config["stake_token"], {
      send: {
        contract: argv["hub-address"],
        amount: argv["amount"],
        msg: encodeBase64({
          queue_unbond: {},
        }),
      },
    }),
  ]);
  console.log(`Success! Txhash: ${txhash}`);
})();
