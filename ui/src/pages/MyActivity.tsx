import { useEffect, useState } from 'react'

import { JUNO_DENOM } from '../chain/juno'
import type { ConnectedWallet } from '../chain/keplr'
import { queryBalance, withdraw } from '../contract/client'
import { fmtCoin } from '../format'

export function MyActivityPage({ wallet }: { wallet: ConnectedWallet | null }) {
  const [balance, setBalance] = useState<string | null>(null)
  const [denom, setDenom] = useState(JUNO_DENOM)
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const [tick, setTick] = useState(0)

  useEffect(() => {
    if (!wallet) return
    queryBalance(wallet.address, denom)
      .then((b) => setBalance(b.amount))
      .catch((e) => setErr(e instanceof Error ? e.message : String(e)))
  }, [wallet, denom, tick])

  if (!wallet) {
    return <p className="text-muted my-12">Connect your wallet to see claimable balances.</p>
  }

  async function doWithdraw() {
    if (!wallet) return
    setBusy(true)
    setErr(null)
    try {
      await withdraw(
        { signer: wallet.signer as Parameters<typeof withdraw>[0]['signer'], sender: wallet.address },
        denom,
      )
      setTick((t) => t + 1)
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="my-8 space-y-6 max-w-2xl">
      <header>
        <h1 className="text-3xl">My activity</h1>
        <p className="text-muted text-sm mt-2 mono break-all">{wallet.address}</p>
      </header>

      <section className="rule pt-6">
        <h2 className="text-xs uppercase tracking-wider text-muted">Claimable balance</h2>
        <div className="flex items-center gap-3 mt-3">
          <input
            className="input w-40"
            value={denom}
            onChange={(e) => setDenom(e.target.value)}
          />
          <span className="text-2xl mono">
            {balance === null ? '—' : fmtCoin(balance, denom)}
          </span>
          <button
            className="btn-primary text-xs"
            disabled={busy || !balance || balance === '0'}
            onClick={doWithdraw}
          >
            {busy ? 'WITHDRAWING…' : 'WITHDRAW'}
          </button>
        </div>
        {err && <p className="text-accent mono text-xs mt-2">{err}</p>}
      </section>

      <section className="rule pt-6">
        <h2 className="text-xs uppercase tracking-wider text-muted">Claim flow</h2>
        <p className="text-sm text-muted mt-3 max-w-prose">
          Once a question is finalized, anyone can drive the claim walk by supplying
          the answer history in reverse order. The contract verifies each entry against
          its persisted history hash and credits the right answerer(s) per Reality.eth's
          right-answer-redistribution rule. Credits land in your pull-payment balance —
          this withdraw step is the only outbound bank transfer the contract makes.
        </p>
      </section>
    </section>
  )
}
