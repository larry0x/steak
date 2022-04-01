import * as fs from "fs";
import * as path from "path";
import yargs from "yargs/yargs";
import {
  createLCDClient,
  createWallet,
  waitForConfirm,
  storeCodeWithConfirm,
  instantiateWithConfirm,
} from "./helpers";

const argv = yargs(process.argv)
  .options({
    network: {
      type: "string",
      demandOption: true,
    },
    admin: {
      type: "string",
      demandOption: false,
    },
    msg: {
      type: "string",
      demandOption: false,
    },
    "code-id": {
      type: "number",
      demandOption: false,
    },
    binary: {
      type: "string",
      demandOption: false,
      default: "../artifacts/steak_hub.wasm",
    },
  })
  .parseSync();

(async function () {
  const terra = createLCDClient(argv["network"]);
  const deployer = createWallet(terra);
  const msg = argv["msg"] ? JSON.parse(fs.readFileSync(path.resolve(argv["msg"]), "utf8")) : {};

  let codeId = argv["code-id"];
  if (!codeId) {
    codeId = await storeCodeWithConfirm(deployer, path.resolve(argv["binary"]));
    console.log(`Code uploaded! Code ID: ${codeId}`);
    await waitForConfirm("Proceed to deploy contract?");
  }

  const result = await instantiateWithConfirm(
    deployer,
    argv["admin"] ? argv["admin"] : deployer.key.accAddress,
    codeId,
    msg
  );
  const address = result.logs[0].eventsByType.instantiate_contract.contract_address[0];
  console.log(`Contract instantiated! Address: ${address}`);
})();
