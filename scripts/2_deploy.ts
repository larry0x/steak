import * as fs from "fs";
import * as path from "path";
import yargs from "yargs/yargs";
import * as keystore from "./keystore";
import {
  createLCDClient,
  createWallet,
  waitForConfirm,
  storeCodeWithConfirm,
  instantiateWithConfirm,
} from "./helpers";
import { Wallet } from "@terra-money/terra.js";

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
    admin: {
      type: "string",
      demandOption: false,
    },
    msg: {
      type: "string",
      demandOption: true,
    },
    "hub-code-id": {
      type: "number",
      demandOption: false,
    },
    "token-code-id": {
      type: "number",
      demandOption: false,
    },
    "hub-binary": {
      type: "string",
      demandOption: false,
      default: "../artifacts/steak_hub.wasm",
    },
    "token-binary": {
      type: "string",
      demandOption: false,
      default: "../artifacts/steak_token.wasm",
    },
  })
  .parseSync();

async function uploadCode(deployer: Wallet, path: string) {
  await waitForConfirm(`Upload code ${path}?`);
  const codeId = await storeCodeWithConfirm(deployer, path);
  console.log(`Code uploaded! ID: ${codeId}`);
  return codeId;
}

(async function () {
  const terra = createLCDClient(argv["network"]);
  const deployer = await createWallet(terra, argv["key"], argv["key-dir"]);

  const hubCodeId = argv["hub-code-id"] ?? await uploadCode(deployer, path.resolve(argv["hub-binary"]));
  const tokenCodeId = argv["token-code-id"] ?? await uploadCode(deployer, path.resolve(argv["token-binary"]));

  const msg = JSON.parse(fs.readFileSync(path.resolve(argv["msg"]), "utf8"));
  msg["cw20_code_id"] = tokenCodeId;

  await waitForConfirm("Proceed to deploy contracts?");
  const result = await instantiateWithConfirm(
    deployer,
    argv["admin"] ? argv["admin"] : deployer.key.accAddress,
    hubCodeId,
    msg
  );
  const address =
    result.logs[0].eventsByType.instantiate_contract.contract_address[0];
  console.log(`Contract instantiated! Address: ${address}`);
})();
