import { useState } from 'react'

import { JUNO_DENOM } from '../chain/juno'
import type { ConnectedWallet } from '../chain/keplr'
import { askQuestion } from '../contract/client'

export function AskPage({ wallet }: { wallet: ConnectedWallet | null }) {
  const [text, setText] = useState('')
  const [initialBond, setInitialBond] = useState('1000000')
  const [bondDenom, setBondDenom] = useState(JUNO_DENOM)
  const [answerTimeout, setAnswerTimeout] = useState('86400')
  const [bounty, setBounty] = useState('0')
  const [arbitrator, setArbitrator] = useState('')
  const [nonce, setNonce] = useState(Math.floor(Date.now() / 1000))
  const [busy, setBusy] = useState(false)
  const [err, setErr] = useState<string | null>(null)
  const [txHash, setTxHash] = useState<string | null>(null)

  async function submit() {
    if (!wallet) {
      setErr('Connect wallet first.')
      return
    }
    if (!text.trim()) {
      setErr('Question text required.')
      return
    }
    setBusy(true)
    setErr(null)
    setTxHash(null)
    try {
      const tx = await askQuestion(
        {
          signer: wallet.signer as Parameters<typeof askQuestion>[0]['signer'],
          sender: wallet.address,
          funds:
            bounty === '0' || bounty === ''
              ? []
              : [{ denom: bondDenom, amount: bounty }],
        },
        {
          text,
          answer_type: 'bool',
          bond_denom: bondDenom,
          initial_bond: initialBond,
          answer_timeout_secs: Number(answerTimeout),
          arbitrator: arbitrator.trim() || null,
          arbitration_timeout_secs: null,
          opening_ts: null,
          nonce,
        },
      )
      setTxHash(tx.transactionHash)
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <section className="my-8 max-w-2xl space-y-6">
      <header>
        <h1 className="text-3xl">Ask a question</h1>
        <p className="text-muted text-sm mt-2">
          Questions are immutable post-creation. Pick parameters carefully —
          the bond denom is the economic-security anchor.
        </p>
      </header>

      <div className="space-y-4">
        <Field label="Question text">
          <textarea
            className="input min-h-[80px]"
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Did event X happen?"
          />
        </Field>
        <Field label="Bond denom" hint="ujuno or an IBC voucher; cw20 via Receive path">
          <input
            className="input"
            value={bondDenom}
            onChange={(e) => setBondDenom(e.target.value)}
          />
        </Field>
        <Field
          label="Initial bond floor"
          hint="micro-units; must be ≥ contract min_initial_bond_floor"
        >
          <input
            className="input"
            value={initialBond}
            onChange={(e) => setInitialBond(e.target.value)}
          />
        </Field>
        <Field
          label="Answer timeout (seconds)"
          hint="default 24h = 86400; must be ≥ contract floor"
        >
          <input
            className="input"
            value={answerTimeout}
            onChange={(e) => setAnswerTimeout(e.target.value)}
          />
        </Field>
        <Field label="Initial bounty" hint="micro-units; sent with the message">
          <input
            className="input"
            value={bounty}
            onChange={(e) => setBounty(e.target.value)}
          />
        </Field>
        <Field
          label="Arbitrator (optional)"
          hint="bech32; leave blank for bond-exhaustion-only resolution"
        >
          <input
            className="input"
            value={arbitrator}
            onChange={(e) => setArbitrator(e.target.value)}
            placeholder="juno1…"
          />
        </Field>
        <Field label="Nonce" hint="lets you re-pose the same text; defaults to now">
          <input
            className="input"
            value={String(nonce)}
            onChange={(e) => setNonce(Number(e.target.value || '0'))}
          />
        </Field>
      </div>

      <div className="flex items-center gap-4">
        <button className="btn-primary" disabled={!wallet || busy} onClick={submit}>
          {busy ? 'POSTING…' : 'POST QUESTION'}
        </button>
        {err && <span className="text-accent mono text-xs">{err}</span>}
        {txHash && (
          <span className="text-ink mono text-xs">
            tx <span className="text-accent">{txHash.slice(0, 12)}…</span>
          </span>
        )}
      </div>
    </section>
  )
}

function Field({
  label,
  hint,
  children,
}: {
  label: string
  hint?: string
  children: React.ReactNode
}) {
  return (
    <label className="block">
      <span className="block text-xs uppercase tracking-wider text-muted mb-1">{label}</span>
      {children}
      {hint && <span className="block text-[11px] text-muted mt-1">{hint}</span>}
    </label>
  )
}
