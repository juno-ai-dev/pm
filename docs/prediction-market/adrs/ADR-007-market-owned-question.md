# ADR-007 — Atomic market-owned oracle question

**Status:** Accepted 2026-07-16
**Decision:** Market instantiate asks the question, locally derives the source-defined ID, and full-field verifies it in reply before activation.

## Alternatives

- user-supplied/precreated ID: substitution risk;
- event scraping: untyped and not return data;
- oracle modification/prediction query: violates unchanged-oracle owner constraint;
- local derivation plus query.

## Evidence

cw-reality id.rs includes market asker/address-bound fields but omits answer type, schema, arbitration timeout, and bounty. AskQuestion sets no response data.

## Consequences

Creation is atomic and market is both asker and arbitrator. ID algorithm becomes a pinned compatibility dependency with golden vectors. Every omitted field is queried.

## Revisit

If a future audited oracle exposes typed response data/prediction query; old markets remain pinned.
