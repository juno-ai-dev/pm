// juno-1 mainnet configuration. No testnet selector — mainnet only per GOAL.md.

export const JUNO_CHAIN_ID = 'juno-1'
export const JUNO_BECH32_PREFIX = 'juno'
export const JUNO_RPC = 'https://juno-rpc.polkachu.com'
export const JUNO_REST = 'https://juno-api.polkachu.com'
export const JUNO_DENOM = 'ujuno'
export const JUNO_DECIMALS = 6
export const JUNO_GAS_PRICE = '0.075ujuno'

/**
 * The deployed `cw-reality` contract on juno-1.
 * - Code ID: 5121 (sha256 e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2)
 * - Stored: 2026-05-28, tx C04EFAD45CAC8A7A5CB0B301EA994791A1D8F7A74FA0C92C92892DF993C15B96, block 38338310
 * - Instantiated: tx CD888CAB75F3C2660433DFF92410CA0277B89528D649BC09E156FB6B17CE6370, block 38338323
 */
export const CW_REALITY_CODE_ID = 5121
export const CW_REALITY_ADDRESS: string =
  'juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur'
