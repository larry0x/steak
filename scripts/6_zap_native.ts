import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import * as keystore from "./keystore";
import { createLCDClient, createWallet, sendTxWithConfirm } from "./helpers";

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
    denom: {
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
    new MsgExecuteContract(
      user.key.accAddress,
      argv["contract-address"],
      {
        zap: {
          minimum_receive: argv["minimum-receive"],
        },
      },
      {
        [argv["denom"]]: argv["amount"],
      }
    ),
  ]);
  console.log(`Success! Txhash: ${txhash}`);
})();
