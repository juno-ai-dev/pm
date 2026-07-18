# ADR-011 — Permissionless creation

**Status:** Accepted 2026-07-16
**Decision:** Any address may create under objective immutable tier bounds. No creator allowlist or contract content moderator exists.

## Alternatives

- allowlist/curation at factory;
- governance approval;
- permissionless contracts with independent discovery policy.

## Evidence

Owner selected permissionless first release. On-chain code cannot reliably decide truth, legality, harm, identity, or geography.

## Consequences

Spam, duplicates, unsafe and illegal content can exist on-chain. Reference interfaces quarantine/filter their own catalogs without settlement power. Seed liquidity/gas provide friction, not eligibility.

## Revisit

Owner direction would require a new explicit decision. Existing factory remains permissionless and immutable.
