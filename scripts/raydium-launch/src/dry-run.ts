import fs from "node:fs";
import path from "node:path";

type CoinConfig = {
  name: string;
  symbol: string;
  supply: string;
  decimals: number;
  token_standard: string;
  network: string;
  metadata_uri: string | null;
  image_uri: string | null;
  initial_liquidity: {
    quote: string;
    sol: number | null;
    token: number | null;
  };
  raydium: {
    pool_type: string;
    pool_id: string | null;
    lp_mint: string | null;
  };
  status: string;
};

const root = path.resolve(process.cwd(), "../..");

const files = [
  "production/coins/rocket-rat.json",
  "production/coins/degen-rat.json"
];

function fail(message: string): never {
  console.error(`FAIL: ${message}`);
  process.exit(1);
}

function validateUrl(label: string, value: string | null) {
  if (value === null) {
    console.log(`PENDING: ${label}`);
    return;
  }

  if (!/^https:\/\/[^\s]+\.json$/i.test(value)) {
    fail(`${label} must be a plain HTTPS .json URL`);
  }
}

console.log("==================================================");
console.log(" MEMELAB V0.5 — RAYDIUM DEX DRY RUN");
console.log("==================================================");
console.log("");
console.log("MAINNET TRANSACTION SUBMISSION: DISABLED");
console.log("");

for (const relative of files) {
  const file = path.join(root, relative);
  const config = JSON.parse(
    fs.readFileSync(file, "utf8")
  ) as CoinConfig;

  console.log(`--- ${config.name} (${config.symbol}) ---`);

  if (config.token_standard !== "Token-2022") {
    fail(`${config.symbol}: wrong token standard`);
  }

  if (config.network !== "mainnet-beta") {
    fail(`${config.symbol}: wrong network`);
  }

  if (config.supply !== "1000000000") {
    fail(`${config.symbol}: supply mismatch`);
  }

  if (config.decimals !== 9) {
    fail(`${config.symbol}: decimals mismatch`);
  }

  if (config.raydium.pool_type !== "CPMM") {
    fail(`${config.symbol}: Raydium pool type must be CPMM`);
  }

  validateUrl(`${config.symbol} metadata URI`, config.metadata_uri);
  validateUrl(`${config.symbol} image URI`, config.image_uri);

  if (config.mint !== null) {
    console.log(`Mint: ${config.mint}`);
  } else {
    console.log("PENDING: production mint");
  }

  if (
    config.initial_liquidity.sol === null ||
    config.initial_liquidity.token === null
  ) {
    console.log("PENDING: initial liquidity decision");
  }

  console.log(`STATUS: ${config.status}`);
  console.log("");
}

console.log("==================================================");
console.log(" RESULT: DRY-RUN GREEN");
console.log("==================================================");
console.log("");
console.log("No wallet loaded.");
console.log("No signer loaded.");
console.log("No mint created.");
console.log("No pool created.");
console.log("No liquidity deposited.");
console.log("No mainnet transaction submitted.");
