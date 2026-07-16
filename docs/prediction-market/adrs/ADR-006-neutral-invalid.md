# ADR-006 — Neutral invalid and unrecognized results

**Status:** Accepted 2026-07-16
**Decision:** INVALID, UNRESOLVED, and every finalized byte string other than exact 32-byte 0/1 pay YES and NO one-half each.

## Alternatives

- stall/re-question: can lock forever and cw-reality has no reopen path;
- creator/governance refund override: mutable payout authority;
- neutral finality.

## Evidence

Reality.eth defines invalid/unresolved sentinels. Polymarket documents unknown 50/50. Neutral makes a complete set worth one under every terminal result.

## Consequences

Ambiguity does not brick funds, but governance can force neutral with an unknown value. Integer half-dust follows ADR-010 and accrues to LP in pairs.

## Revisit

If a bounded, oracle-preserving re-question mechanism is independently specified and audited in a new version.
