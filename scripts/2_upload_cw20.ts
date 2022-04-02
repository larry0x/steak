import * as path from "path";
import yargs from "yargs/yargs";
import * as keystore from "./keystore";
import { createLCDClient, createWallet, storeCodeWithConfirm } from "./helpers";

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
    binary: {
      type: "string",
      demandOption: false,
      default: "../../cw-plus/artifacts/cw20_base.wasm",
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const deployer = await createWallet(terra, argv["key"], argv["key-dir"]);

  const codeId = await storeCodeWithConfirm(deployer, path.resolve(argv["binary"]));
  console.log(`Success! Code ID: ${codeId}`);
})();
