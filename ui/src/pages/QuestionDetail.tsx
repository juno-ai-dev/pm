import { useEffect, useState } from 'react'

import type { ConnectedWallet } from '../chain/keplr'
import {
  cancelArbitration,
  disputeAnswer,
  fundBounty,
  queryQuestion,
  requestArbitration,
  submitAnswer,
} from '../contract/client'
import type { QuestionResponse } from '../contract/types'
import { StateChip } from '../components/StateChip'
import { StateMachineTimeline } from '../components/StateMachineTimeline'
import { fmtAddress, fmtCoin, fmtDuration, fmtTimestamp } from '../format'

interface Props {
  qid: string
  wallet: ConnectedWallet | null
}

export function QuestionDetailPage({ qid, wallet }: Props) {
  const [data, setData] = useState<QuestionResponse | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [refreshTick, setRefreshTick] = useState(0)

  useEffect(() => {
    let cancelled = false
    queryQuestion(qid)
      .then((r) => {
        if (!cancelled) setData(r)
      })
      .catch((e) => {
        if (!cancelled) setErr(e instanceof Error ? e.message : String(e))
      })
    return () => {
      cancelled = true
    }
  }, [qid, refreshTick])

  if (err) return <p className="text-accent mono my-12">{err}</p>
  if (!data) return <p className="text-muted my-12">Loading…</p>

  const q = data.question

  async function withWallet<T>(action: (w: ConnectedWallet) => Promise<T>) {
    if (!wallet) {
      setErr('connect wallet first')
      return
    }
    setBusy(true)
    setErr(null)
    try {
      await action(wallet)
      setRefreshTick((t) => t + 1)
    } catch (e) {
      setErr(e instanceof Error ? e.message : String(e))
    } finally {
      setBusy(false)
    }
  }

  return (
    <article className="my-8 space-y-6">
      <header>
        <div className="flex items-center gap-3 mb-3">
          <StateChip state={data.state} />
          <span className="text-xs text-muted mono">round {q.round_count}</span>
          {q.arbitrator && (
            <span className="text-xs text-muted mono">arb {fmtAddress(q.arbitrator)}</span>
          )}
        </div>
        <h1 className="text-3xl leading-tight">{q.text}</h1>
        <p className="mt-3 text-xs text-muted mono break-all">
          <span className="text-ink/70">question_id</span> {data.question_id}
        </p>
      </header>

      <StateMachineTimeline current={data.state} />

      <section className="grid grid-cols-2 gap-y-2 gap-x-8 text-sm border-t border-b border-ink/10 py-4">
        <Row k="Asker" v={fmtAddress(q.asker)} />
        <Row k="Answer type" v={q.answer_type} />
        <Row k="Bond denom" v={q.bond_denom} />
        <Row k="Initial / current bond" v={`${q.initial_bond} → ${q.current_bond}`} mono />
        <Row k="Answer timeout" v={fmtDuration(q.answer_timeout_secs)} />
        <Row k="Bounty" v={fmtCoin(q.bounty, q.bond_denom)} />
        <Row k="Finalize at" v={q.finalize_ts ? fmtTimestamp(q.finalize_ts) : '—'} />
        <Row
          k="Arbitration deadline"
          v={q.arbitration_deadline ? fmtTimestamp(q.arbitration_deadline) : '—'}
        />
      </section>

      <section className="space-y-3">
        <h2 className="text-xs uppercase tracking-wider text-muted">Actions</h2>
        <div className="flex flex-wrap gap-3">
          <ActionForm
            label={q.round_count === 0 ? 'Submit answer' : 'Counter-answer'}
            disabled={!wallet || busy || data.state !== 'open_answered' && data.state !== 'open_unanswered'}
            fields={[
              { name: 'answer', placeholder: 'answer bytes (hex or text)' },
              { name: 'bond', placeholder: `bond in ${q.bond_denom}` },
            ]}
            onSubmit={(values) =>
              withWallet((w) => {
                const params = {
                  question_id: data.question_id,
                  answer: toBase64Bytes(values.answer),
                  current_bond_seen: q.current_bond,
                }
                const opts = {
                  signer: w.signer as Parameters<typeof submitAnswer>[0]['signer'],
                  sender: w.address,
                  funds: [{ denom: q.bond_denom, amount: values.bond }],
                }
                return q.round_count === 0
                  ? submitAnswer(opts, params)
                  : disputeAnswer(opts, { ...params, new_answer: params.answer })
              })
            }
          />
          <ActionForm
            label="Fund bounty"
            disabled={!wallet || busy || data.state === 'finalized' || data.state === 'claimed'}
            fields={[{ name: 'bounty', placeholder: `top-up in ${q.bond_denom}` }]}
            onSubmit={(values) =>
              withWallet((w) =>
                fundBounty(
                  {
                    signer: w.signer as Parameters<typeof fundBounty>[0]['signer'],
                    sender: w.address,
                    funds: [{ denom: q.bond_denom, amount: values.bounty }],
                  },
                  data.question_id,
                ),
              )
            }
          />
          {q.arbitrator && wallet?.address === q.arbitrator && (
            <>
              <button
                disabled={busy || data.state !== 'open_answered'}
                className="btn-primary text-xs"
                onClick={() =>
                  withWallet((w) =>
                    requestArbitration(
                      {
                        signer: w.signer as Parameters<typeof requestArbitration>[0]['signer'],
                        sender: w.address,
                      },
                      data.question_id,
                    ),
                  )
                }
              >
                REQUEST ARBITRATION
              </button>
              <button
                disabled={busy || data.state !== 'pending_arbitration'}
                className="btn-ghost text-xs"
                onClick={() =>
                  withWallet((w) =>
                    cancelArbitration(
                      {
                        signer: w.signer as Parameters<typeof cancelArbitration>[0]['signer'],
                        sender: w.address,
                      },
                      data.question_id,
                    ),
                  )
                }
              >
                CANCEL ARBITRATION
              </button>
            </>
          )}
        </div>
        {err && <p className="text-accent mono text-xs mt-2">{err}</p>}
      </section>
    </article>
  )
}

function Row({ k, v, mono }: { k: string; v: string; mono?: boolean }) {
  return (
    <div className="flex justify-between gap-4">
      <span className="text-muted text-xs uppercase tracking-wider">{k}</span>
      <span className={mono ? 'mono text-sm' : 'text-sm'}>{v}</span>
    </div>
  )
}

interface ActionFormProps {
  label: string
  disabled: boolean
  fields: { name: string; placeholder: string }[]
  onSubmit: (values: Record<string, string>) => Promise<void> | void
}

function ActionForm({ label, disabled, fields, onSubmit }: ActionFormProps) {
  const [values, setValues] = useState<Record<string, string>>({})
  return (
    <form
      className="flex items-center gap-2"
      onSubmit={(e) => {
        e.preventDefault()
        void onSubmit(values)
      }}
    >
      {fields.map((f) => (
        <input
          key={f.name}
          className="input w-40"
          placeholder={f.placeholder}
          value={values[f.name] ?? ''}
          onChange={(e) => setValues({ ...values, [f.name]: e.target.value })}
        />
      ))}
      <button className="btn-primary text-xs" type="submit" disabled={disabled}>
        {label.toUpperCase()}
      </button>
    </form>
  )
}

function toBase64Bytes(input: string): string {
  // If input looks like hex (0x…), strip and convert; otherwise treat as UTF-8.
  let bytes: Uint8Array
  if (input.startsWith('0x') && /^0x[0-9a-fA-F]+$/.test(input)) {
    const hex = input.slice(2)
    bytes = new Uint8Array(hex.length / 2)
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16)
    }
  } else {
    bytes = new TextEncoder().encode(input)
  }
  // Pad/truncate to 32 bytes — Reality.eth's bytes32 convention.
  const out = new Uint8Array(32)
  out.set(bytes.slice(0, 32))
  let bin = ''
  for (const b of out) bin += String.fromCharCode(b)
  return btoa(bin)
}
