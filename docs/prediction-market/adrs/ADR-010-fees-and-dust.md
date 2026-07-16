# ADR-010 — Fees, rounding, and dust

**Status:** Accepted 2026-07-16; documented fee residual risks retained
**Decision:** Immutable LP fee 200 bps, protocol fee zero. All divisions use the R1 caller-adverse/cumulative rules; neutral half-dust pairs accrue to LP; forced excess has no claimant.

## Alternatives

- no fee: no LP flow compensation;
- mutable/dynamic fee: governance/admin and quote complexity;
- protocol skim: additional owner/legal/accounting surface.

## Evidence

Omen documents 2% precedent, not Juno fitness. R1 balances exact examples and demonstrates rounding directions. A [one-day JUNO/ATOM TWAP sample](../evidence/2026-07-15-osmosis-juno-liquidity.md) had a 1.886% high/low range, but collateral movement does not measure event-driven informed flow and cannot validate the fee.

## Consequences

Fees never back positions or guarantee LP profit. Address splitting cannot increase neutral payout. There is no sweep or last-user vault windfall.

## Safe default and revisit

Acceptance authorizes implementing the 2% value only. Launch remains separately unauthorized until deployment/readiness gates and an explicit deployment decision exist. Revisit from measured volume, LP loss, trade size, and routing; a new value means a new factory.
