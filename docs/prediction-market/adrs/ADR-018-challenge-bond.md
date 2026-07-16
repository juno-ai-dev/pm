# ADR-018 — One-shot challenge bond

**Status:** Accepted 2026-07-16; documented economic residual risks retained
**Decision:** One pre-finality challenge per market escrows max(10 JUNO, current oracle bond). Governance changes snapshot: refund. Same answer or no executed pre-deadline verdict: full LP slash.

## Alternatives

- free freeze: cheap mass griefing;
- always refund: timeout freeze remains nearly free;
- always slash: no reward for correcting;
- subjective reviewer allocation: new authority.

## Evidence

cw-reality collects no public arbitration fee and only market can request. Objective byte comparison and execution/no-execution are available on-chain; rejection reasons are not reliably available.

## Consequences

Legitimate challenger can lose due to governance/deposit failure. C is segregated from P/F/bounty. Noncanonical changed verdict refunds and settles neutral. One-shot rule bounds repeated freezes.

## Safe default and revisit

The amount/payee rule is accepted for implementation only. Launch remains separately unauthorized until governance flows are rehearsed, readiness gates close, and an explicit deployment decision exists. Revisit from challenge accessibility, spam cost, cap, and observed governance response in a new tier/version.
