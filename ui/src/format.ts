// Tiny formatting utilities. Bond amounts are always shown with full
// precision *and* humanized form (PLAN.md §3 — explicit constraint).

import { JUNO_DECIMALS, JUNO_DENOM } from './chain/juno'

export function fmtCoin(amount: string, denom: string): string {
  // Always render the raw micro-unit string; append the humanized form when
  // the denom is `ujuno`.
  if (denom === JUNO_DENOM) {
    const human = humanize(amount, JUNO_DECIMALS)
    return `${amount} ${denom} (${human} JUNO)`
  }
  return `${amount} ${denom}`
}

function humanize(microAmount: string, decimals: number): string {
  // Avoid floating-point conversion for accuracy on big values.
  const padded = microAmount.padStart(decimals + 1, '0')
  const whole = padded.slice(0, padded.length - decimals).replace(/^0+(?=\d)/, '')
  const frac = padded.slice(padded.length - decimals).replace(/0+$/, '')
  return frac.length === 0 ? whole : `${whole}.${frac}`
}

export function fmtAddress(addr: string): string {
  if (addr.length <= 14) return addr
  return `${addr.slice(0, 8)}…${addr.slice(-4)}`
}

export function fmtDuration(secs: number): string {
  if (secs < 60) return `${secs}s`
  if (secs < 3600) return `${Math.round(secs / 60)}m`
  if (secs < 86400) return `${(secs / 3600).toFixed(1)}h`
  return `${(secs / 86400).toFixed(1)}d`
}

export function fmtTimestamp(unixSec: number): string {
  return new Date(unixSec * 1000).toISOString().replace('T', ' ').slice(0, 19) + ' UTC'
}

export function timeUntil(unixSec: number, now: number): string {
  const delta = unixSec - now
  if (delta <= 0) return `${fmtDuration(-delta)} ago`
  return `in ${fmtDuration(delta)}`
}
