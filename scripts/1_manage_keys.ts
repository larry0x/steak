import * as fs from "fs";
import * as path from "path";
import * as promptly from "promptly";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import * as keystore from "./keystore";

async function addKey(keyName: string, keyDir: string, coinType: number) {
  if (!fs.existsSync(keyDir)) {
    fs.mkdirSync(keyDir, { recursive: true });
  }

  const mnemonic = await promptly.prompt("Enter BIP-39 seed phrase:");

  const password = await promptly.password("Enter a password to encrypt the key:");
  const repeat = await promptly.password("Repeat the password:");
  if (password != repeat) {
    throw new Error("Passwords don't match!");
  }

  const accAddress = keystore.save(keyName, keyDir, mnemonic, coinType, password);
  console.log("Success! Address:", accAddress);
}

function listKeys(keyDir: string) {
  fs.readdirSync(keyDir)
    .filter((fn) => {
      return fn.endsWith(".json");
    })
    .sort()
    .forEach((fn) => {
      const entity: keystore.Entity = JSON.parse(fs.readFileSync(path.join(keyDir, fn), "utf8"));
      console.log(`- name: ${entity.name}`);
      console.log(`  address: ${entity.address}`);
    });
}

function removeKey(keyName: string, keyDir: string) {
  keystore.remove(keyName, keyDir);
  console.log("Success!");
}

yargs(hideBin(process.argv))
  .command(
    "add <key>",
    "Add a key with the given name",
    (yargs) => {
      return yargs
        .positional("key", {
          type: "string",
          describe: "name of the key",
          demandOption: true,
        })
        .option("key-dir", {
          type: "string",
          describe: "path to the directory where encrypted key files are stored",
          demandOption: false,
          default: keystore.DEFAULT_KEY_DIR,
        })
        .option("coin-type", {
          type: "number",
          describe: "SLIP-0044 coin type for use in derivation of the private key",
          demandOption: false,
          default: 330, // Terra = 330, Cosmos = 118
        });
    },
    (argv) => addKey(argv["key"], argv["key-dir"], argv["coin-type"]).catch(console.log)
  )
  .command(
    "rm <key>",
    "Remove a key of the given name",
    (yargs) => {
      return yargs
        .positional("key", {
          type: "string",
          describe: "name of the key",
          demandOption: true,
        })
        .option("key-dir", {
          type: "string",
          describe: "path to the directory where encrypted key files are stored",
          demandOption: false,
          default: keystore.DEFAULT_KEY_DIR,
        });
    },
    (argv) => removeKey(argv["key"], argv["key-dir"])
  )
  .command(
    "ls",
    "List all keys",
    (yargs) => {
      return yargs.option("key-dir", {
        type: "string",
        describe: "path to the directory where encrypted key files are stored",
        demandOption: false,
        default: keystore.DEFAULT_KEY_DIR,
      });
    },
    (argv) => listKeys(argv["key-dir"])
  )
  .wrap(100)
  .parse();
