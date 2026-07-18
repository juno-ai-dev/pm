# ADR-003 — Internal outcome positions

**Status:** Accepted 2026-07-16
**Decision:** Store YES/NO balances inside each market. Expose Split, Merge, Buy, Sell, and Redeem; do not expose transfer.

## Alternatives

- two CW20 contracts per market;
- CW1155/conditional-token dependency;
- internal ledgers.

## Evidence

Complete-set semantics do not require token transferability. Token contracts add callbacks, allowance, metadata, deployment, and cross-contract audit scope.

## Consequences

No external exchange/composability in v1 and no position transfer between wallets. Every balance is directly scoped to its collateral vault. A later token layer cannot rewrite old balances.

## Revisit

When a CLOB/router has a concrete demand and independently audited position-token standard.
