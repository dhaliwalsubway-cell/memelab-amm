export type CoinLaunchConfig = {
  name: string
  symbol: string
  decimals: number
  supply: string
  mint: string | null
  imageUri: string | null
  metadataUri: string | null
  solLiquidity: string | null
  tokenLiquidity: string | null
  feeBps: number | null
  initialPriceSolPerToken: string | null
}

export const ROCKET_RAT: CoinLaunchConfig = {
  name: "Rocket Rat",
  symbol: "RKTROT",
  decimals: 9,
  supply: "1000000000",
  mint: null,
  imageUri: null,
  metadataUri: null,
  solLiquidity: null,
  tokenLiquidity: null,
  feeBps: null,
  initialPriceSolPerToken: null
}

export const DEGEN_RAT: CoinLaunchConfig = {
  name: "Degen Rat",
  symbol: "DGRAT",
  decimals: 9,
  supply: "1000000000",
  mint: null,
  imageUri: null,
  metadataUri: null,
  solLiquidity: null,
  tokenLiquidity: null,
  feeBps: null,
  initialPriceSolPerToken: null
}

export const MAINNET_RAYDIUM_CONFIG = {
  poolType: "CPMM",
  network: "mainnet-beta",
  quoteMint: "So11111111111111111111111111111111111111112",
  transactionVersion: "V0"
} as const
