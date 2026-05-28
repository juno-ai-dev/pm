import type { State } from '../contract/types'

const LABELS: Record<State, string> = {
  not_created: 'NOT CREATED',
  open_unanswered: 'AWAITING ANSWER',
  open_answered: 'ANSWERED',
  pending_arbitration: 'ARBITRATION',
  finalized: 'FINALIZED',
  claimed: 'CLAIMED',
}

const CLASSES: Record<State, string> = {
  not_created: 'chip-muted',
  open_unanswered: 'chip-muted',
  open_answered: 'chip-ink',
  pending_arbitration: 'chip-accent',
  finalized: 'chip-ink',
  claimed: 'chip-muted',
}

export function StateChip({ state }: { state: State }) {
  return <span className={CLASSES[state]}>{LABELS[state]}</span>
}
