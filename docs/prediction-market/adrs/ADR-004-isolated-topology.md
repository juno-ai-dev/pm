# ADR-004 — Immutable factory and isolated markets

**Status:** Accepted 2026-07-16
**Decision:** One noncustodial immutable factory pins one code/tier; it instantiates one adminless contract per market. New versions deploy new factories.

## Alternatives

- one multi-market vault: lower repeated base storage, larger accounting/blast radius;
- migratable governed factory/markets: easier upgrades, additional economic authority;
- isolated immutable instances.

## Evidence

R4 compares cost structure and failure isolation. Exact gas is unavailable in a no-code phase and remains a measurement gate.

## Consequences

Bank balance is directly auditable per market. Indexers enumerate factory versions. No global mutable registry exists. More addresses/instantiate overhead are accepted for containment.

## Revisit

Only with measured storage/gas and a safety proof; never migrate funded v1 markets.
