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
import { InstantiateMsg } from "./types/hub/instantiate_msg";

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
      demandOption: false,
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
      default: "../artifacts/eris_staking_hub.wasm",
    },
    "token-binary": {
      type: "string",
      demandOption: false,
      default: "../artifacts/eris_staking_token.wasm",
    },
  })
  .parseSync();

async function uploadCode(deployer: Wallet, path: string) {
  await waitForConfirm(`Upload code ${path}?`);
  const codeId = await storeCodeWithConfirm(deployer, path);
  console.log(`Code uploaded! ID: ${codeId}`);
  return codeId;
}

const templates: Record<string, InstantiateMsg> = {
  testnet: <InstantiateMsg>{
    name: "Amplified LUNA",
    symbol: "ampLUNA",
    cw20_code_id: 0,
    decimals: 6,
    epoch_period: 3 * 24 * 60 * 60,
    unbond_period: 21 * 24 * 60 * 60,
    validators: ["terravaloper1cevf3xwxm8zjhj7yrnjc0qy6y6ng98lxgxp79x"],
    protocol_fee_contract: "terra1pm8wqcrvk2qysf30u4e5mwprjfj9hj87dph3ne",
    protocol_reward_fee: "0.05",
    owner: "",
  },
};

(async function () {
  const terra = createLCDClient(argv["network"]);
  const deployer = await createWallet(terra, argv["key"], argv["key-dir"]);

  const hubCodeId =
    argv["hub-code-id"] ??
    (await uploadCode(deployer, path.resolve(argv["hub-binary"])));
  const tokenCodeId =
    argv["token-code-id"] ??
    (await uploadCode(deployer, path.resolve(argv["token-binary"])));

  let msg: any;
  if (argv["msg"]) {
    msg = JSON.parse(fs.readFileSync(path.resolve(argv["msg"]), "utf8"));
  } else {
    msg = templates[argv["network"]];
  }
  msg["cw20_code_id"] = tokenCodeId;
  msg["owner"] = msg["owner"] || deployer.key.accAddress;

  await waitForConfirm("Proceed to deploy contracts?");
  const result = await instantiateWithConfirm(
    deployer,
    argv["admin"] ? argv["admin"] : deployer.key.accAddress,
    hubCodeId,
    msg
  );
  const address =
    result.logs[0].eventsByType["instantiate_contract"]["contract_address"][0];
  console.log(`Contract instantiated! Address: ${address}`);
})();
