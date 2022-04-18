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
    pair: {
      type: "string",
      demandOption: true,
    },
    "steak-token": {
      type: "string",
      demandOption: true,
    },
    "steak-amount": {
      type: "string",
      demandOption: true,
    },
    "uluna-amount": {
      type: "string",
      demandOption: false,
    },
    "slippage-tolerance": {
      type: "string",
      demandOption: false,
      default: "0.005", // 0.5%
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = await createWallet(terra, argv["key"], argv["key-dir"]);

  // If uluna amount is not specified, by default we provide the same amount as Steak
  argv["uluna-amount"] = argv["uluna-amount"] || argv["steak-amount"];

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, argv["steak-token"], {
      increase_allowance: {
        spender: argv["pair"],
        amount: argv["steak-amount"],
      },
    }),
    new MsgExecuteContract(
      signer.key.accAddress,
      argv["pair"],
      {
        provide_liquidity: {
          assets: [
            {
              info: {
                token: {
                  contract_addr: argv["steak-token"],
                },
              },
              amount: argv["steak-amount"],
            },
            {
              info: {
                native_token: {
                  denom: "uluna",
                },
              },
              amount: argv["uluna-amount"],
            },
          ],
          slippage_tolerance: argv["slippage-tolerance"],
        },
      },
      {
        uluna: argv["uluna-amount"],
      }
    ),
  ]);
  console.log(`Success! Tx hash: ${txhash}`);
})();
