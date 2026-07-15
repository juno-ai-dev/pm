# Architecture decision records

These records are candidates until a human reviewer records acceptance. “Owner direction fixed” means GOAL.md section 14 constrains the answer; it does not waive the evidence gates.

The proposed cross-ADR disposition, evidence, dissent, residual-risk, parameter,
license, and reviewer fields are assembled in the
[issue #2 decision packet](../issue-2-decision-packet.md). That packet does not
change the statuses below.

| ADR | Decision | Status |
| --- | --- | --- |
| [001](ADR-001-binary-fixed-expiry.md) | Binary fixed-expiry only | Proposed |
| [002](ADR-002-fpmm.md) | Integer FPMM | Proposed |
| [003](ADR-003-internal-positions.md) | Internal positions | Proposed |
| [004](ADR-004-isolated-topology.md) | Immutable factory + isolated market | Proposed |
| [005](ADR-005-native-ujuno.md) | Native ujuno only | Proposed; owner direction fixed |
| [006](ADR-006-neutral-invalid.md) | Neutral unknown/invalid | Proposed |
| [007](ADR-007-market-owned-question.md) | Atomic market-owned question | Proposed |
| [008](ADR-008-oracle-tiers-and-caps.md) | Security tiers and caps | Proposed; risk acceptance open |
| [009](ADR-009-locked-initial-liquidity.md) | One locked initial LP | Proposed |
| [010](ADR-010-fees-and-dust.md) | 2% LP fee, no protocol fee, explicit dust | Proposed; fee acceptance open |
| [011](ADR-011-permissionless-creation.md) | Permissionless creation | Proposed; owner direction fixed |
| [012](ADR-012-no-admin-or-pause.md) | No live admin, migration, pause, or sweep | Proposed |
| [013](ADR-013-resolution-liveness.md) | Bounty/keepers; bounded stall, unbounded unanswered | Proposed |
| [014](ADR-014-answer-bytes-and-template.md) | Exact bytes and canonical question | Proposed |
| [015](ADR-015-offchain-trust.md) | Off-chain convenience only | Proposed |
| [016](ADR-016-product-posture.md) | Experimental, value-bearing, permissionless/no-entity | Proposed; owner direction fixed; counsel open |
| [017](ADR-017-juno-governance-arbitration.md) | Market controller relays Juno x/gov verdict | Deferred at rehearsal gate; owner authority fixed |
| [018](ADR-018-challenge-bond.md) | Bonded one-shot challenge | Proposed; economic acceptance open |

Acceptance requires reviewer identity/date, considered evidence, dissent/residual risk, and any parameter approval. Editing “Proposed” to “Accepted” without that review is not phase closure.
