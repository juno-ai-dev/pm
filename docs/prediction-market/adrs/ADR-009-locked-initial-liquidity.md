# ADR-009 — One locked initial liquidity position

**Status:** Accepted 2026-07-16
**Decision:** Creator supplies all activation liquidity, receives fixed non-transferable LP supply, and cannot add/remove or claim until resolution.

## Alternatives

- free entry/exit: fee sniping, asymmetric inventory withdrawal, pre-close run;
- pre-close exit with minimum lock: more formulas and weakened depth;
- initial-only locked LP.

## Evidence

R1 shows directional terminal LP payoff and enumerates dynamic-LP accounting risks. Permissionless creators can each sponsor a pool.

## Consequences

Smallest audit surface and stable depth, but capital can remain locked indefinitely if unanswered. No promise of LP profitability.

## Revisit

After audited fee accumulators/withdrawal formulas and operational data justify multi-LP lifecycle.
