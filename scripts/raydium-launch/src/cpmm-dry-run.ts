import fs from "node:fs"
import path from "node:path"
import { MAINNET_RAYDIUM_CONFIG } from "./production-config.js"

const root = path.resolve(process.cwd(), "../..")

const coins = [
  {
    config: "production/coins/rocket-rat.json",
    metadata: "production/metadata/rocket-rat.json",
    name: "Rocket Rat",
    symbol: "RKTROT"
  },
  {
    config: "production/coins/degen-rat.json",
    metadata: "production/metadata/degen-rat.json",
    name: "Degen Rat",
    symbol: "DGRAT"
  }
]

console.log("==================================================")
console.log(" MEMELAB V0.9 — RAYDIUM CPMM PREPARATION")
console.log("==================================================")
console.log("")
console.log("EXECUTION MODE: DISABLED")
console.log("SIGNER: NOT LOADED")
console.log("MAINNET SUBMISSION: DISABLED")
console.log("")

for (const coin of coins) {
  const configPath = path.join(root, coin.config)
  const metadataPath = path.join(root, coin.metadata)

  const config = JSON.parse(fs.readFileSync(configPath, "utf8"))
  const metadata = JSON.parse(fs.readFileSync(metadataPath, "utf8"))

  if (config.name !== coin.name) throw new Error(`${coin.symbol}: name mismatch`)
  if (config.symbol !== coin.symbol) throw new Error(`${coin.symbol}: symbol mismatch`)
  if (config.token_standard !== "Token-2022") {
    throw new Error(`${coin.symbol}: Token-2022 required`)
  }
  if (config.network !== "mainnet-beta") {
    throw new Error(`${coin.symbol}: mainnet-beta required`)
  }
  if (config.raydium.pool_type !== "CPMM") {
    throw new Error(`${coin.symbol}: CPMM required`)
  }

  if (metadata.name !== coin.name) {
    throw new Error(`${coin.symbol}: metadata name mismatch`)
  }

  if (metadata.symbol !== coin.symbol) {
    throw new Error(`${coin.symbol}: metadata symbol mismatch`)
  }

  console.log(`--- ${coin.name} (${coin.symbol}) ---`)
  console.log("PASS: Token-2022")
  console.log("PASS: Mainnet configuration")
  console.log("PASS: Raydium CPMM target")
  console.log("PASS: metadata identity")
  console.log(`Mint: ${config.mint ?? "PENDING"}`)
  console.log(`Metadata URI: ${config.metadata_uri ?? "PENDING"}`)
  console.log(`Pool: ${config.raydium.pool_id ?? "PENDING"}`)
  console.log("")
}

console.log("==================================================")
console.log(" RESULT: V0.9 CPMM PREPARATION GREEN")
console.log("==================================================")
console.log("")
console.log("No mint created.")
console.log("No wallet loaded.")
console.log("No signer loaded.")
console.log("No pool created.")
console.log("No liquidity deposited.")
console.log("No mainnet transaction submitted.")
