# ADR-015 — Indexer and frontend are non-authoritative

**Status:** Accepted 2026-07-16
**Decision:** Every financial fact is directly queryable. Indexers/UIs provide discovery, history, and warnings but cannot create balances, close, resolve, or alter rules.

## Alternatives

- signed off-chain orders/indexer-authoritative state;
- privileged resolution adapter;
- read-only convenience.

## Evidence

The v1 internal ledger and AMM need no relayer. Threat model includes stale/malicious display and event reprocessing.

## Consequences

Reference UI disables signing on direct-query mismatch and displays heights/hashes. Independent interfaces may unlist differently. Event consumers reconcile against state.

## Revisit

A future CLOB needs its own signature, cancellation, availability, and sequencing ADR; settlement remains on-chain.
