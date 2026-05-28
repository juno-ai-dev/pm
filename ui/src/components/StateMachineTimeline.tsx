import type { State } from '../contract/types'

// Horizontal timeline showing the state machine as the visual anchor on the
// question detail page (PLAN.md §3 explicit constraint).

interface Stop {
  state: State
  label: string
  body: string
}

const STOPS: Stop[] = [
  {
    state: 'open_unanswered',
    label: 'Asked',
    body: 'Question posed; awaiting the first answer.',
  },
  {
    state: 'open_answered',
    label: 'Answered',
    body: 'An answer is in. Counter-answers must bond ≥ 2× the current bond.',
  },
  {
    state: 'pending_arbitration',
    label: 'Arbitration',
    body: 'Frozen pending the arbitrator. Cancellable by anyone after deadline.',
  },
  {
    state: 'finalized',
    label: 'Finalized',
    body: 'Final answer fixed. Claim distributes bonds + bounty.',
  },
  {
    state: 'claimed',
    label: 'Claimed',
    body: 'All bonds + bounty paid out. History remains queryable.',
  },
]

function stateIndex(s: State): number {
  switch (s) {
    case 'not_created':
      return -1
    case 'open_unanswered':
      return 0
    case 'open_answered':
      return 1
    case 'pending_arbitration':
      return 2
    case 'finalized':
      return 3
    case 'claimed':
      return 4
  }
}

export function StateMachineTimeline({ current }: { current: State }) {
  const activeIdx = stateIndex(current)
  return (
    <div className="my-10">
      <div className="grid grid-cols-5 gap-0 relative">
        {/* Connecting rule */}
        <div className="absolute top-3 left-[10%] right-[10%] border-t border-ink/30" />
        {STOPS.map((stop, idx) => {
          const passed = idx <= activeIdx
          const isCurrent = idx === activeIdx
          return (
            <div key={stop.state} className="relative flex flex-col items-center px-2">
              <div
                className={`relative z-10 w-6 h-6 mb-3 ${
                  passed ? 'bg-ink' : 'bg-paper border border-ink/30'
                } ${isCurrent ? 'ring-4 ring-accent/30' : ''}`}
                aria-current={isCurrent || undefined}
              />
              <div className="text-[11px] uppercase tracking-wider text-ink/80">{stop.label}</div>
              <div className="text-[11px] text-muted text-center mt-1 leading-snug">
                {stop.body}
              </div>
            </div>
          )
        })}
      </div>
    </div>
  )
}
