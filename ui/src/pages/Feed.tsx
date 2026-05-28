import { useEffect, useState } from 'react'
import { Link } from 'wouter'

import { CW_REALITY_ADDRESS } from '../chain/juno'
import { queryList } from '../contract/client'
import type { QuestionResponse } from '../contract/types'
import { StateChip } from '../components/StateChip'
import { fmtCoin, fmtDuration } from '../format'

export function FeedPage() {
  const [questions, setQuestions] = useState<QuestionResponse[]>([])
  const [err, setErr] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    queryList({ limit: 30 })
      .then((res) => {
        if (!cancelled) setQuestions(res.questions)
      })
      .catch((e) => {
        if (!cancelled) setErr(e instanceof Error ? e.message : String(e))
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => {
      cancelled = true
    }
  }, [])

  if (!CW_REALITY_ADDRESS) {
    return (
      <section className="my-12">
        <h2 className="text-2xl">Not yet deployed</h2>
        <p className="text-muted max-w-2xl my-4">
          The <span className="mono">cw-reality</span> contract has not yet been uploaded to{' '}
          <span className="mono">juno-1</span>. Once the code is stored and an instance is
          instantiated (stage 5 of <span className="mono">junoreality/PLAN.md</span>), this
          feed will populate.
        </p>
      </section>
    )
  }

  if (loading) return <p className="text-muted my-12">Loading questions from juno-1…</p>
  if (err) return <p className="text-accent mono my-12">{err}</p>
  if (questions.length === 0)
    return <p className="text-muted my-12">No questions on chain yet — ask the first one.</p>

  return (
    <section className="my-8 space-y-0">
      {questions.map((q) => (
        <Link key={q.question_id} href={`/q/${encodeURIComponent(q.question_id)}`}>
          <a className="block py-5 border-b border-ink/10 hover:bg-ink/5 -mx-2 px-2 transition-colors">
            <div className="flex items-start justify-between gap-4">
              <div className="flex-1 min-w-0">
                <p className="text-lg leading-snug">{q.question.text}</p>
                <p className="mt-2 text-xs text-muted mono">
                  bond {fmtCoin(q.question.current_bond, q.question.bond_denom)} · round{' '}
                  {q.question.round_count} · timeout{' '}
                  {fmtDuration(q.question.answer_timeout_secs)}
                </p>
              </div>
              <StateChip state={q.state} />
            </div>
          </a>
        </Link>
      ))}
    </section>
  )
}
