import * as fs from "fs";
import * as path from "path";
import * as crypto from "crypto";

import { MnemonicKey, RawKey } from "@terra-money/terra.js";

export const DEFAULT_KEY_DIR = path.join(__dirname, "./keys");

const KEY_SIZE = 256;
const ITERATIONS = 100;

export type Entity = {
  name: string;
  address: string;
  cipherText: string;
};

function encrypt(plainText: string, password: string): string {
  const salt = crypto.randomBytes(16);
  const iv = crypto.randomBytes(16);
  const key = crypto.pbkdf2Sync(password, salt, ITERATIONS, KEY_SIZE / 8, "sha1");

  const cipher = crypto.createCipheriv("AES-256-CBC", key, iv);
  const encryptedText = Buffer.concat([cipher.update(plainText), cipher.final()]);

  return salt.toString("hex") + iv.toString("hex") + encryptedText.toString("base64");
}

function decrypt(cipherText: string, password: string): string {
  const salt = Buffer.from(cipherText.slice(0, 32), "hex");
  const iv = Buffer.from(cipherText.slice(32, 64), "hex");
  const key = crypto.pbkdf2Sync(password, salt, ITERATIONS, KEY_SIZE / 8, "sha1");

  const encrypedText = cipherText.slice(64);
  const cipher = crypto.createDecipheriv("AES-256-CBC", key, iv);
  const decryptedText = Buffer.concat([cipher.update(encrypedText, "base64"), cipher.final()]);

  return decryptedText.toString();
}

export function save(
  keyName: string,
  keyDir: string,
  mnemonic: string,
  coinType: number,
  password: string
) {
  const filePath = path.join(keyDir, `${keyName}.json`);
  if (fs.existsSync(filePath)) {
    throw new Error(`file ${filePath} already exists!`);
  }

  const mnemonicKey = new MnemonicKey({ mnemonic, coinType });
  const privateKey = mnemonicKey.privateKey.toString("hex");
  const cipherText = encrypt(privateKey, password);

  const entity: Entity = {
    name: keyName,
    address: mnemonicKey.accAddress,
    cipherText,
  };
  fs.writeFileSync(filePath, JSON.stringify(entity, null, 2));

  return mnemonicKey.accAddress;
}

export function load(keyName: string, keyDir: string, password: string): RawKey {
  const filePath = path.join(keyDir, `${keyName}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`file ${filePath} does not exist!`);
  }

  const entity: Entity = JSON.parse(fs.readFileSync(filePath, "utf8"));
  const privateKey = decrypt(entity.cipherText, password);
  const rawKey = new RawKey(Buffer.from(privateKey, "hex"));

  return rawKey;
}

export function remove(keyName: string, keyDir: string) {
  const filePath = path.join(keyDir, `${keyName}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`file ${filePath} does not exist!`);
  }

  fs.unlinkSync(filePath);
}
