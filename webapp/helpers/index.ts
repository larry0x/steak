/**
 * Encode a JavaScript object into a base64 string
 */
export function encodeBase64(obj: any): string {
  return Buffer.from(JSON.stringify(obj)).toString("base64");
}

/**
 * Decode a base64 string into a JavaScript object
 */
export function decodeBase64<T>(str: string): T {
  return JSON.parse(Buffer.from(str, "base64").toString());
}

/**
 * Truncate the middle portion of a string
 */
export function truncateString(text = "", h = 4, t = 4) {
  const head = text.slice(0, h);
  const tail = text.slice(-1 * t, text.length);
  return text.length > h + t ? [head, tail].join("...") : text;
}

/**
 * Truncate the decimal places of a string
 */
export function truncateDecimals(x: number, decPlaces = 6) {
  const multiplier = Math.pow(10, decPlaces);
  return Math.ceil(x * multiplier) / multiplier;
}

/**
 * Format a number
 */
export function formatNumber(x: number, decPlaces = 2) {
  const integerPart = Math.floor(x);
  const decimalPart = x - integerPart;
  return integerPart.toString().replace(/\B(?=(\d{3})+(?!\d))/g, ",") + decimalPart.toFixed(decPlaces).slice(1);
}

/**
 * Format a percentage number
 */
export function formatPercentage(perc: number) {
  return Math.floor(perc * 100).toString() + "%";
}

/**
 * Make the first letter of a string uppercase
 */
export function capitalizeFirstLetter(str: string) {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
