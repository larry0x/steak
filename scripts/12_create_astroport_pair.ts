import yargs from "yargs/yargs";
import { MsgExecuteContract } from "@terra-money/terra.js";
import * as keystore from "./keystore";
import { createLCDClient, createWallet, sendTxWithConfirm, encodeBase64 } from "./helpers";

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
    "steak-token": {
      type: "string",
      demandOption: true,
    },
    "astroport-factory": {
      type: "string",
      demandOption: true,
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const signer = await createWallet(terra, argv["key"], argv["key-dir"]);

  const { txhash } = await sendTxWithConfirm(signer, [
    new MsgExecuteContract(signer.key.accAddress, argv["astroport-factory"], {
      create_pair: {
        pair_type: {
          stable: {},
        },
        asset_infos: [
          {
            token: {
              contract_addr: argv["steak-token"],
            },
          },
          {
            native_token: {
              denom: "uluna",
            },
          },
        ],
        init_params: encodeBase64({
          amp: 50,
        }),
      },
    }),
  ]);
  console.log(`Success! Tx hash: ${txhash}`);
})();
