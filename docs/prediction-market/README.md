# Prediction-market architecture phase

**Snapshot:** 2026-07-15  
**Scope:** research and documentation only  
**Review status:** candidate architecture; human acceptance, legal advice, parameter risk acceptance, and an authorized Juno governance rehearsal remain open

This directory is the review packet required by [GOAL.md](../../GOAL.md). The documents deliberately distinguish observed facts from recommendations and accepted owner decisions. Nothing here authorizes contract code, generated schemas, deployment, or fund movement.

## Deliverables

| ID | Artifact | Status |
| --- | --- | --- |
| R1 | [Mechanism and market microstructure](mechanism.md) | Candidate |
| R2 | [Prior art and incidents](prior-art.md) | Candidate |
| R3 | [cw-reality compatibility](cw-reality-compatibility.md) and [question specification](question-specification.md) | Candidate |
| R4 | [Juno, collateral, and topology](juno-and-topology.md) | Candidate |
| R5 | [Product, legal, content, and operations](product-legal-operations.md) | Candidate; counsel input open |
| A1 | [Architecture](architecture.md) | Candidate |
| A2 | [Security and economics](security-and-economics.md) | Candidate; quantitative risk acceptance open |
| A3 | [User journeys](user-journeys.md) | Candidate |
| ADR | [Decision-record index](adrs/README.md) | Candidate |
| Decision | [Issue #2 decision packet](issue-2-decision-packet.md) | Proposed for sign-off; no approval recorded |
| Authorization | [Machine-readable authorization](authorization.json) | `implementation_authorized: false` |
| Review | [Phase review checklist](review-checklist.md) | Open gates are explicit |

## Evidence

- [Pinned local and upstream source baseline](evidence/source-baseline.md)
- [Height-pinned Juno and oracle snapshot](evidence/2026-07-15-juno.md)
- [Byte-exact two-provider Juno archive](evidence/raw/39830878/README.md)
- [Deployed-oracle wasm reproducibility attempt](evidence/oracle-wasm-reproducibility.md)
- [Height-pinned Osmosis JUNO liquidity and one-day volatility](evidence/2026-07-15-osmosis-juno-liquidity.md)

Every memo uses these labels:

- **Observed fact:** directly established by source, schema, test output, or a height-pinned query.
- **Author claim:** a paper or project describes its own mechanism or policy.
- **Inference:** a conclusion derived from cited evidence; it is not directly asserted by the source.
- **Recommendation:** the architecture proposed for review.
- **Owner decision:** an input already accepted in GOAL.md section 14.
- **Open gate:** evidence or authority is still missing; implementation must not fill it in.

## Load-bearing conclusions

1. The v1 product is a fixed-expiry binary market backed by complete sets in native ujuno.
2. Trading uses one FPMM per market. Initial liquidity is supplied once and locked through resolution; later LP entry and pre-resolution exit are deferred.
3. A market instance atomically asks and verifies its own question. It computes the source-defined question ID, then queries every stored field in the oracle reply before activation.
4. Oracle values are exactly 32 bytes: unsigned big-endian 0 is NO, 1 is YES, all 0xff is INVALID, and 31 bytes of 0xff followed by 0xfe is UNRESOLVED. Every other finalized byte string settles neutrally.
5. Each funded market is non-migratable. The factory can register later code versions but cannot rewrite a live market.
6. The existing production cw-reality address is not acceptable as an immutable dependency because both its chain admin and stored admin are non-empty. A fresh frozen instance using an independently reproduced and audited checksum is the recommended safe default; the current source-to-deployed-byte match remains open.
7. Juno x/gov is the owner-selected verdict authority. Passed proposals 357 and 363 prove generic governance-originated `MsgExecuteContract` precedent, but the exact market → cw-reality verdict/payee path and its failures have not been rehearsed. The architecture therefore remains gated.
8. No protocol admin can pause trading, change a payout, seize collateral, sweep forced funds, choose a verdict, or censor creation. Independent interfaces may apply their own discoverability policies without settlement authority.

## What remains genuinely open

The packet does not manufacture evidence where none exists. The following prevent phase closure:

- an authorized, funded Juno governance rehearsal, including failed execution;
- qualified legal advice for the actual contributors and interface/indexer operators;
- human review and acceptance of all ADRs;
- empirical gas/storage measurements for the proposed contracts, which cannot exist in this no-code phase;
- explicit owner risk acceptance for the candidate fee, liquidity, challenge-bond, oracle-tier, and market-cap values;
- an independent audit or reproducible-build match between cw-reality source commit ee64153 and on-chain checksum e25473…f3e2.

These appear as open evidence in the review checklist rather than being silently converted into defaults.

The [issue #2 decision packet](issue-2-decision-packet.md) consolidates the
proposed ADR dispositions, dated parameter recommendations, schema choices,
license routes, and required sign-off fields. It is decision-ready but not a
decision: its empty sign-off fields and the fail-closed
[`authorization.json`](authorization.json) preserve the current authorization
state. The `blocked: decision` label remains appropriate.
