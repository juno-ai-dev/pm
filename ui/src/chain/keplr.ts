// Minimal Keplr / Leap wallet integration. We deliberately avoid `@cosmos-kit/react`
// here to keep the bundle small and the surface easy to audit. Both Keplr and Leap
// expose the same `window.keplr`-shaped API for our purposes.

import {
  JUNO_BECH32_PREFIX,
  JUNO_CHAIN_ID,
  JUNO_DECIMALS,
  JUNO_DENOM,
  JUNO_GAS_PRICE,
  JUNO_REST,
  JUNO_RPC,
} from './juno'

export type WalletKind = 'keplr' | 'leap'

interface KeplrLike {
  enable(chainId: string): Promise<void>
  getOfflineSigner(chainId: string): unknown
  getKey(chainId: string): Promise<{ name: string; bech32Address: string }>
  experimentalSuggestChain?(chain: unknown): Promise<void>
}

declare global {
  interface Window {
    keplr?: KeplrLike
    leap?: KeplrLike
  }
}

const JUNO_CHAIN_SUGGESTION = {
  chainId: JUNO_CHAIN_ID,
  chainName: 'Juno',
  rpc: JUNO_RPC,
  rest: JUNO_REST,
  bip44: { coinType: 118 },
  bech32Config: {
    bech32PrefixAccAddr: JUNO_BECH32_PREFIX,
    bech32PrefixAccPub: `${JUNO_BECH32_PREFIX}pub`,
    bech32PrefixValAddr: `${JUNO_BECH32_PREFIX}valoper`,
    bech32PrefixValPub: `${JUNO_BECH32_PREFIX}valoperpub`,
    bech32PrefixConsAddr: `${JUNO_BECH32_PREFIX}valcons`,
    bech32PrefixConsPub: `${JUNO_BECH32_PREFIX}valconspub`,
  },
  currencies: [
    { coinDenom: 'JUNO', coinMinimalDenom: JUNO_DENOM, coinDecimals: JUNO_DECIMALS },
  ],
  feeCurrencies: [
    {
      coinDenom: 'JUNO',
      coinMinimalDenom: JUNO_DENOM,
      coinDecimals: JUNO_DECIMALS,
      gasPriceStep: { low: 0.03, average: 0.075, high: 0.1 },
    },
  ],
  stakeCurrency: { coinDenom: 'JUNO', coinMinimalDenom: JUNO_DENOM, coinDecimals: JUNO_DECIMALS },
  features: ['cosmwasm'],
}

function pickProvider(kind: WalletKind): KeplrLike {
  if (kind === 'leap') {
    if (!window.leap) throw new Error('Leap wallet not detected — install from leapwallet.io')
    return window.leap
  }
  if (!window.keplr) throw new Error('Keplr wallet not detected — install from keplr.app')
  return window.keplr
}

export interface ConnectedWallet {
  kind: WalletKind
  name: string
  address: string
  signer: unknown
  gasPrice: string
}

export async function connect(kind: WalletKind): Promise<ConnectedWallet> {
  const provider = pickProvider(kind)
  // Best-effort chain suggestion; safe to call repeatedly.
  try {
    await provider.experimentalSuggestChain?.(JUNO_CHAIN_SUGGESTION)
  } catch {
    /* ignore — chain may already be known to the wallet */
  }
  await provider.enable(JUNO_CHAIN_ID)
  const key = await provider.getKey(JUNO_CHAIN_ID)
  const signer = provider.getOfflineSigner(JUNO_CHAIN_ID)
  return {
    kind,
    name: key.name,
    address: key.bech32Address,
    signer,
    gasPrice: JUNO_GAS_PRICE,
  }
}
