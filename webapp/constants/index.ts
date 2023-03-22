export const CONTRACTS = {
  mainnet: {
    multiquery: "terra1swrywzkphty20e2uzpy582xu836luw0e5yp25m",
    steakHub: "terra15qr8ev2c0a0jswjtfrhfaj5ucgkhjd7la2shlg",
    steakToken: "terra1rl4zyexjphwgx6v3ytyljkkc4mrje2pyznaclv",
  },
  testnet: {
    multiquery: "terra1t5twwglq9vlmf0pz8yadmd6gr6es2gfc4fkjww",
    steakHub: "terra1xshrfs3lp7nwkdfh3067vfsf3kmweygfsc3hzy",
    steakToken: "terra1awhvtkm553rszxtvnuda4fe2r6rjjj7hjwzv0w",
  },
};

export const NETWORKS = {
  mainnet: {
    name: "mainnet",
    chainID: "columbus-5",
    lcd: "https://lcd.terra.dev",
  },
  testnet: {
    name: "testnet",
    chainID: "bombay-12",
    lcd: "https://bombay-lcd.terra.dev",
  },
};

export const GAS_OPTIONS = {
  gas: undefined, // leave undefined so it is estimated when signing
  gasPrices: "0.15uusd",
  gasAdjustment: 1.2,
};
