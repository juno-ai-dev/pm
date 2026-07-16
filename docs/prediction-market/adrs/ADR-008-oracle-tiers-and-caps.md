# ADR-008 — Oracle tiers and market caps

**Status:** Accepted 2026-07-16; documented economic residual risks retained
**Decision:** One immutable factory represents one tier. Accepted canary: P cap 200 JUNO, 10-JUNO initial oracle bond, 10-JUNO challenge floor, 1-JUNO bounty, and named 20-JUNO first-counter monitoring capacity.

## Alternatives

- uncapped markets: can outgrow resolution security;
- creator-selected arbitrary parameters: adverse selection toward weak security;
- fixed reviewed tiers.

## Evidence

Wrong canonical results can redirect up to P. Existing production oracle floors are only 0.1 JUNO/24h and do not scale with value. At the measured Osmosis snapshot, 200 JUNO was below 0.057% of each of the two largest visible equal-weight JUNO reserves, so the canary is small relative to that external exit inventory. Governance corruption cost is unquantified, external depth does not secure the oracle, and the [single-venue measurement](../evidence/2026-07-15-osmosis-juno-liquidity.md) is not venue-complete.

## Consequences

Canary cap is containment, not proof. P <= 20× initial bond is an accepted canary ratio with documented residual risk. Scaling requires current governance/concentration and acquisition analysis plus a new decision.

## Safe default and revisit

No deployment until accepted. Revisit with observed volume, committed monitoring capital, governance rehearsal, and current chain evidence; do not copy the ratio automatically.
