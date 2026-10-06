import { CoinLaunchConfig, MAINNET_RAYDIUM_CONFIG } from "./production-config.js"

export function validateLaunchInput(config: CoinLaunchConfig): void {
  if (config.name.length === 0) throw new Error("Missing name")
  if (config.symbol.length === 0) throw new Error("Missing symbol")
  if (config.decimals !== 9) throw new Error("Production decimals must be 9")
  if (config.supply !== "1000000000") {
    throw new Error("Production supply must be exactly 1,000,000,000")
  }

  if (config.mint === null) {
    throw new Error(`${config.symbol}: production mint is not created yet`)
  }

  if (config.imageUri === null || !/^https:\/\/[^\s]+$/i.test(config.imageUri)) {
    throw new Error(`${config.symbol}: valid HTTPS image URI required`)
  }

  if (
    config.metadataUri === null ||
    !/^https:\/\/[^\s]+\.json$/i.test(config.metadataUri)
  ) {
    throw new Error(`${config.symbol}: valid HTTPS JSON metadata URI required`)
  }

  if (config.solLiquidity === null) {
    throw new Error(`${config.symbol}: SOL liquidity not approved`)
  }

  if (config.tokenLiquidity === null) {
    throw new Error(`${config.symbol}: token liquidity not approved`)
  }

  if (config.feeBps === null) {
    throw new Error(`${config.symbol}: Raydium fee tier not approved`)
  }

  if (![1, 25, 100].includes(config.feeBps)) {
    throw new Error(
      `${config.symbol}: fee must be one of 1, 25 or 100 bps`
    )
  }

  if (config.initialPriceSolPerToken === null) {
    throw new Error(`${config.symbol}: initial price not approved`)
  }

  if (Number(config.solLiquidity) <= 0) {
    throw new Error(`${config.symbol}: SOL liquidity must be > 0`)
  }

  if (Number(config.tokenLiquidity) <= 0) {
    throw new Error(`${config.symbol}: token liquidity must be > 0`)
  }

  if (Number(config.initialPriceSolPerToken) <= 0) {
    throw new Error(`${config.symbol}: initial price must be > 0`)
  }

  console.log(`PASS: ${config.name} launch input`)
  console.log(`  Network:       ${MAINNET_RAYDIUM_CONFIG.network}`)
  console.log(`  Pool:          ${MAINNET_RAYDIUM_CONFIG.poolType}`)
  console.log(`  Quote:         SOL`)
  console.log(`  Mint:          ${config.mint}`)
  console.log(`  SOL liquidity: ${config.solLiquidity}`)
  console.log(`  Token amount:  ${config.tokenLiquidity}`)
  console.log(`  Fee:           ${config.feeBps} bps`)
  console.log(`  Initial price: ${config.initialPriceSolPerToken} SOL/token`)
}
