# ADR-001 — Binary fixed-expiry markets

**Status:** Accepted 2026-07-16
**Decision:** V1 supports exactly one YES/NO proposition with immutable close_ts and opening_ts. Trading rejects at block.time >= close_ts.

## Alternatives

- categorical/scalar/combinatorial markets: larger payout, liquidity, and question surfaces;
- rolling or admin-closed markets: mutable boundary and privileged timing;
- binary fixed expiry: smallest complete lifecycle.

## Evidence

cw-reality stores Bool metadata and an opening timestamp. Injective documents separate expiration and settlement times. Complete-set sources give the simplest two-outcome conservation model.

## Consequences

Close and expected resolution are separate. Postponement/cancellation behavior belongs in immutable rules. No admin extends trading. Other shapes require new contracts and ADRs.

## Revisit

After v1 has audited accounting and real liquidity data; never by migrating a live market.
