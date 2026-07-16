# ADR-012 — No admin, migration, pause, recovery, or sweep

**Status:** Accepted 2026-07-16
**Decision:** Factory, funded markets, and recommended oracle have empty chain admins; oracle stored admin is None. No pause/recovery/sweep execute exists.

## Alternatives

- DAO migration with delay;
- emergency guardian/pause;
- immutable instances and version replacement.

## Evidence

cw-reality authenticates arbitrator address, not code. Existing production oracle has a migration admin, making it another resolution authority. Pause/recovery can block or seize valid claims.

## Consequences

No key can fix a live typo/bug, recover lost keys, or sweep abandoned/forced funds. Incident response is warnings/routing/new versions. Chain consensus upgrades remain foundational trust.

## Revisit

Only for a new version with explicit authority/risk analysis; never add hidden authority to existing markets.
