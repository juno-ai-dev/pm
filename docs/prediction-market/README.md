# Prediction-market architecture phase

**Snapshot:** accepted 2026-07-16
**Scope:** accepted architecture and authorized milestone implementation specification
**Review status:** decision packet, ADRs, and exact canary parameters accepted; deployment, funds, legal/operational readiness, audit/build evidence, and governance-rehearsal transactions remain gated

This directory is the accepted review packet required by [GOAL.md](../../GOAL.md). The documents distinguish observed facts, accepted decisions, and still-missing evidence. Contract code, tests/models, SDK, frontend, indexer, and operations tooling are authorized; deployment, fund movement, and mainnet governance-rehearsal transaction execution are not.

## Deliverables

| ID | Artifact | Status |
| --- | --- | --- |
| R1 | [Mechanism and market microstructure](mechanism.md) | Accepted architecture baseline |
| R2 | [Prior art and incidents](prior-art.md) | Accepted provenance baseline |
| R3 | [cw-reality compatibility](cw-reality-compatibility.md) and [question specification](question-specification.md) | Accepted integration baseline; audit/build/rehearsal evidence open |
| R4 | [Juno, collateral, and topology](juno-and-topology.md) | Accepted topology baseline; deployment evidence open |
| R5 | [Product, legal, content, and operations](product-legal-operations.md) | Architecture accepted; issue #26 readiness evidence open |
| Policy | [Reference-interface discovery, review, report, and appeal](interface-discovery-policy.md) | Implementation specification; named-reviewer and counsel gates remain open |
| A1 | [Architecture](architecture.md) | Accepted |
| A2 | [Security and economics](security-and-economics.md) | Accepted with documented residual risks |
| A3 | [User journeys](user-journeys.md) | Accepted |
| ADR | [Decision-record index](adrs/README.md) | ADR-001–018 Accepted |
| Decision | [Issue #2 decision packet](issue-2-decision-packet.md) | Accepted 2026-07-16 |
| Authorization | [Machine-readable authorization](authorization.json) | `implementation_authorized: true` |
| Review | [Phase review checklist](review-checklist.md) | Decision gates closed; evidence gates explicit |

## Evidence

- [Pinned local and upstream source baseline](evidence/source-baseline.md)
- [Height-pinned Juno and oracle snapshot](evidence/2026-07-15-juno.md)
- [Byte-exact two-provider Juno archive](evidence/raw/39830878/README.md)
- [Deployed-oracle wasm reproducibility attempt](evidence/oracle-wasm-reproducibility.md)
- [Frozen oracle artifact/deployment runbook and verifier](oracle-deployment/README.md)
- [Height-pinned Osmosis JUNO liquidity and one-day volatility](evidence/2026-07-15-osmosis-juno-liquidity.md)

Every memo uses these labels:

- **Observed fact:** directly established by source, schema, test output, or a height-pinned query.
- **Author claim:** a paper or project describes its own mechanism or policy.
- **Inference:** a conclusion derived from cited evidence; it is not directly asserted by the source.
- **Recommendation:** an implementation detail or future evidence action supported by the analysis; it does not reopen an accepted owner decision unless explicitly marked **Open gate**.
- **Owner decision:** an input already accepted in GOAL.md section 14.
- **Open gate:** evidence or authority is still missing; implementation must not fill it in.

## Load-bearing conclusions

1. The v1 product is a fixed-expiry binary market backed by complete sets in native ujuno.
2. Trading uses one FPMM per market. Initial liquidity is supplied once and locked through resolution; later LP entry and pre-resolution exit are deferred.
3. A market instance atomically asks and verifies its own question. It computes the source-defined question ID, then queries every stored field in the oracle reply before activation.
4. Oracle values are exactly 32 bytes: unsigned big-endian 0 is NO, 1 is YES, all 0xff is INVALID, and 31 bytes of 0xff followed by 0xfe is UNRESOLVED. Every other finalized byte string settles neutrally.
5. Each funded market is non-migratable. The factory can register later code versions but cannot rewrite a live market.
6. The existing production cw-reality address is not acceptable as an immutable dependency because both its chain admin and stored admin are non-empty. A fresh frozen instance using an independently reproduced and audited checksum is the recommended safe default; the current source-to-deployed-byte match remains open.
7. Each market pins one immutable address-based verdict authority. V1 uses the Juno Agents DAO core `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`; issue #45 owns implementation and a non-broadcast proposal packet. Juno x/gov remains a future compatible authority profile under deferred issues #4 and #13 and does not block DAO-based v1.
8. No protocol admin can pause trading, change a payout, seize collateral, sweep forced funds, choose a verdict, or censor creation. Independent interfaces may apply their own discoverability policies without settlement authority.

## What remains genuinely open after acceptance

The packet does not manufacture evidence where none exists. The following prevent deployment/operational readiness, not authorized implementation:

- an authorized live Juno Agents DAO rehearsal before a funded canary, if required by the launch gate; implementation itself requires authoritative contract tests and a reviewable non-broadcast proposal packet;
- qualified legal advice for the actual contributors and interface/indexer operators;
- empirical gas/storage measurements for the implementation contracts;
- an independent audit or reproducible-build match for the selected frozen cw-reality artifact.

These appear as open evidence in the review checklist rather than being silently converted into defaults.

The [issue #2 decision packet](issue-2-decision-packet.md) records accepted ADR dispositions, exact dated parameters, schema choices, the clean-room license/provenance route, reviewers, residual risks, and owner authorization. [`authorization.json`](authorization.json) is the fail-closed execution boundary: implementation is true while deployment, funds, and governance-rehearsal transaction execution remain false. Its policy permits removal of `blocked: decision`; label changes remain a separate action.
