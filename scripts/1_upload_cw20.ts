import * as path from "path";
import yargs from "yargs/yargs";
import { createLCDClient, createWallet, storeCodeWithConfirm } from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
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
  const deployer = createWallet(terra);

  const codeId = await storeCodeWithConfirm(deployer, path.resolve(argv["binary"]));
  console.log(`Success! Code ID: ${codeId}`);
})();
