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
    "contract-address": {
      type: "string",
      demandOption: true,
    },
    "token-address": {
      type: "string",
      demandOption: true,
    },
    amount: {
      type: "string",
      demandOption: true,
    },
    "minimum-receive": {
      type: "string",
      demandOption: false,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const user = await createWallet(terra, argv["key"], argv["key-dir"]);

  const { txhash } = await sendTxWithConfirm(user, [
    new MsgExecuteContract(user.key.accAddress, argv["token-address"], {
      send: {
        contract: argv["contract-address"],
        amount: argv["amount"],
        msg: encodeBase64({
          zap: {
            minimum_receive: argv["minimum-receive"],
          },
        }),
      },
    }),
  ]);
  console.log(`Success! Txhash: ${txhash}`);
})();
