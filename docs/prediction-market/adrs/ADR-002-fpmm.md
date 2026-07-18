# ADR-002 — Integer fixed-product market maker

**Status:** Accepted 2026-07-16; clean-room independent implementation required
**Decision:** Use the R1 binary FPMM formulas with checked Uint256 intermediates and caller-adverse ceiling rules.

## Alternatives

- CLOB: efficient when populated but empty at bootstrap and infrastructure-heavy;
- LMSR: bounded sponsor loss but exp/log math and different subsidy owner;
- pm-AMM: outcome-specific but transcendental/time-dependent math;
- parimutuel: no continuous secondary exit.

## Evidence

Pinned Gnosis FPMM commit 6814c024 supplies production formula precedent. R1 quantifies Juno-sized price impact and reconciles a buy/sell sequence.

## Consequences

Creator capital supplies continuous quotes; LP bears informed flow. Price is not promised as probability. Gnosis code is LGPL-3.0, so implementation needs approved license strategy or independent expression.

## Revisit

After observed v1 flow/depth or audited transferable positions justify a CLOB/router or pm-AMM.
