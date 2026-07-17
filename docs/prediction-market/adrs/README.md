# Architecture decision records

Jake Hartnell accepted ADR-001 through ADR-018 on 2026-07-16 and delegated architecture, economic-security, and license/provenance review to Juno AI. Acceptance authorizes scoped implementation; it does not waive the residual audit, legal, operational, deployment, fund-movement, or governance-rehearsal evidence gates.

The accepted cross-ADR disposition, evidence, dissent, residual-risk, parameter,
license, and reviewer fields are assembled in the
[issue #2 decision packet](../issue-2-decision-packet.md).

| ADR | Decision | Status |
| --- | --- | --- |
| [001](ADR-001-binary-fixed-expiry.md) | Binary fixed-expiry only | Accepted 2026-07-16 |
| [002](ADR-002-fpmm.md) | Integer FPMM | Accepted 2026-07-16; clean-room route |
| [003](ADR-003-internal-positions.md) | Internal positions | Accepted 2026-07-16 |
| [004](ADR-004-isolated-topology.md) | Immutable factory + isolated market | Accepted 2026-07-16 |
| [005](ADR-005-native-ujuno.md) | Native ujuno only | Accepted 2026-07-16 |
| [006](ADR-006-neutral-invalid.md) | Neutral unknown/invalid | Accepted 2026-07-16 |
| [007](ADR-007-market-owned-question.md) | Atomic market-owned question | Accepted 2026-07-16 |
| [008](ADR-008-oracle-tiers-and-caps.md) | Security tiers and caps | Accepted 2026-07-16 |
| [009](ADR-009-locked-initial-liquidity.md) | One locked initial LP | Accepted 2026-07-16 |
| [010](ADR-010-fees-and-dust.md) | 2% LP fee, no protocol fee, explicit dust | Accepted 2026-07-16 |
| [011](ADR-011-permissionless-creation.md) | Permissionless creation | Accepted 2026-07-16 |
| [012](ADR-012-no-admin-or-pause.md) | No live admin, migration, pause, or sweep | Accepted 2026-07-16 |
| [013](ADR-013-resolution-liveness.md) | Bounty/keepers; bounded stall, unbounded unanswered | Accepted 2026-07-16 |
| [014](ADR-014-answer-bytes-and-template.md) | Exact bytes and canonical question | Accepted 2026-07-16 |
| [015](ADR-015-offchain-trust.md) | Off-chain convenience only | Accepted 2026-07-16 |
| [016](ADR-016-product-posture.md) | Experimental, value-bearing, permissionless/no-entity | Accepted; issue #26 evidence gate |
| [017](ADR-017-juno-governance-arbitration.md) | Immutable verdict authority; Juno Agents DAO v1, x/gov later | Amended 2026-07-17; issue #45 |
| [018](ADR-018-challenge-bond.md) | Bonded one-shot challenge | Accepted 2026-07-16 |

Reviewer identities, date, considered evidence, no recorded dissent, residual risks, parameter approval, and authorization boundaries are recorded in the decision packet and machine-readable authorization.
