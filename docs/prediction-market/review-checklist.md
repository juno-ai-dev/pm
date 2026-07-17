# Prediction-market phase review

**Decision review date:** 2026-07-16
**Verdict:** issue #2 decision gate closed; scoped milestone implementation authorized
**Authorization boundary:** deployment, fund movement, and mainnet governance-rehearsal transaction execution are not authorized

Jake Hartnell accepted the complete decision packet on 2026-07-16 and delegated architecture, economic-security, and license/provenance decisions to Juno AI. Dissent: none recorded. The accepted architecture and parameters retain every documented residual risk; acceptance does not turn missing evidence into completed audits, legal advice, deployed checksums, funded transactions, or operational readiness.

**Machine-readable authorization:** [`authorization.json`](authorization.json) records `implementation_authorized: true`, `critical_parameters_accepted: true`, the accepted clean-room license route, and fail-closed execution gates. The [issue #2 decision packet](issue-2-decision-packet.md) records reviewer identities, dates, evidence, residual risks, and exact scope.

## Mission questions

| Question | Accepted answer | Evidence | Review result |
| --- | --- | --- | --- |
| What is traded/backed? | Internal YES/NO complete sets; Y=N=P; native ujuno | [R1](mechanism.md), [A1](architecture.md) | Accepted |
| How are prices/liquidity formed? | Integer FPMM, locked creator LP, exact 200-bps LP fee | [R1](mechanism.md), [ADR-002](adrs/ADR-002-fpmm.md), [ADR-009](adrs/ADR-009-locked-initial-liquidity.md) | Accepted with residual model/liquidity risk |
| What exact oracle bytes settle? | Exact 32-byte 0/1; every other result neutral | [R3 compatibility](cw-reality-compatibility.md), [question spec](question-specification.md), [ADR-014](adrs/ADR-014-answer-bytes-and-template.md) | Accepted |
| What happens in resolution failures? | Counter clocks reset; challenge/stall paths are bounded; unanswered is explicitly unbounded | [R3](cw-reality-compatibility.md), [A3](user-journeys.md), [ADR-013](adrs/ADR-013-resolution-liveness.md) | Accepted architecture; #45 implementation evidence open |
| Who is trusted? | Immutable code/chain; each market pins an immutable verdict authority, initially the Juno Agents DAO core; off-chain systems have no settlement authority | [A1](architecture.md), [ADR-017](adrs/ADR-017-juno-governance-arbitration.md), [R5](product-legal-operations.md) | Accepted architecture; DAO governance/upgrades and deployment readiness remain external risks |

## Deliverable and decision audit

| ID | Artifact | Decision review | Remaining evidence boundary |
| --- | --- | --- | --- |
| R1 | [mechanism.md](mechanism.md) | Accepted 2026-07-16 | Implementation model, property, gas, and economic observations |
| R2 | [prior-art.md](prior-art.md) | Accepted provenance baseline | Maintain clean-room contributor provenance; no qualified legal opinion claimed |
| R3 | [cw-reality compatibility](cw-reality-compatibility.md), [question specification](question-specification.md) | Accepted integration specification | Independent audit/reproducible build and issue #4 rehearsal |
| R4 | [juno-and-topology.md](juno-and-topology.md) | Accepted topology | Refresh addresses/checksums and measure gas before deployment |
| R5 | [product-legal-operations.md](product-legal-operations.md) | Architecture/product posture accepted | Issue #26 qualified legal and operational-readiness evidence |
| A1 | [architecture.md](architecture.md) | Accepted | Implementation verification and audit |
| A2 | [security-and-economics.md](security-and-economics.md) | Exact canary tier and residual risks accepted | Tests, audit, measurements, monitoring, and scaling review |
| A3 | [user-journeys.md](user-journeys.md) | Accepted | Executable acceptance tests and operational exercises |
| ADRs | [ADR index](adrs/README.md) | ADR-001–018 Accepted | Revisit triggers and residual evidence remain as recorded |
| Decision | [decision packet](issue-2-decision-packet.md), [authorization](authorization.json) | Accepted and implementation authorized | Deployment/funds/governance transactions remain false |

## GOAL.md section 13 gate audit

| Gate | Current result | Evidence / boundary |
| --- | --- | --- |
| R1–R5 and A1–A3 reviewed/linked | **Decision gate closed** | Juno AI delegated review and Jake Hartnell owner acceptance, 2026-07-16 |
| ADRs 001–018 accepted | **Closed** | All accepted; ADR-017 architecture is settled, with issue #4 retaining rehearsal evidence |
| cw-reality source/schema and selected deployment independently verified | **Open deployment evidence** | [Source baseline](evidence/source-baseline.md), local tests, [height snapshot](evidence/2026-07-15-juno.md), and [wasm attempt](evidence/oracle-wasm-reproducibility.md); build mismatch/no independent audit remain |
| Live evidence archived | **Archived; refresh before deployment** | [Two-provider archive](evidence/raw/39830878/README.md) at height 39,830,878 |
| Result bytes/payout fixed in writing | **Closed** | R3 and ADR-014 accepted |
| Settlement sequences terminate or disclose nontermination | **Closed as specification** | Unanswered nontermination and all documented paths accepted; execution tests remain |
| Financial invariants/specification reviewed | **Closed as decision** | R1/A2/A3 accepted; implementation models/tests remain required |
| Market cap/oracle tier approved | **Closed** | Every exact §4 value accepted 2026-07-16 with residual risk |
| ujuno conventions accepted | **Closed as decision** | R4 accepted; venue/long-window evidence remains a deployment/scaling input |
| Permissions enumerated | **Closed as architecture** | A1/R4 accepted; future deployed addresses/checksums remain absent |
| Challenge accounting specified | **Closed** | A2/ADR-018 accepted |
| Verdict-authority architecture | **Amended; issue #45 implementation open** | Immutable address boundary accepted; Juno Agents DAO core is the v1 profile; no authority rotation or generic relay |
| Live authority rehearsal | **Launch/canary gate** | No DAO proposal, transaction, funding, or end-to-end evidence is claimed or authorized here; #4/#13 preserve deferred x/gov compatibility evidence |
| Implementation test plan | **Closed as specification** | A2 trace accepted; execution evidence belongs to implementation issues |
| License strategy approved | **Closed as project authorization** | Clean-room independent implementation from repository formulas/specifications under Apache-2.0; do not copy/adapt LGPL source; preserve citations/notices as provenance |
| Product/legal/content posture | **Architecture accepted; issue #26 readiness open** | No qualified legal advice or operational readiness is claimed |

## Authorized implementation traceability

| Invariants/risks | Required implementation evidence |
| --- | --- |
| Complete sets, resolved liabilities, product/fees/principal | Arbitrary-precision action-sequence model and exact R1 vectors |
| Close boundary | One second before/at/after plus same-block ordering |
| Resolution binding | Every oracle/question field mismatch and checksum/admin deployment failure |
| Redemption/arithmetic/dust | Partition fuzzing, neutral odd-address permutations, overflow boundaries |
| Challenge/verdict | Multi-contract challenge, spoof, direct cancel, and every proposal outcome; mainnet execution separately authorized under issue #4 |
| Immutability | Chain admin queries and failed migrate attempts before deployment |
| Cap and forced funds | Exact-boundary/overflow tests, random bank excess, and abandoned positions |
| Operations | Tooling may be built; operational readiness requires issue #26 evidence and a separately authorized canary/rehearsal |

## Residual gates carried forward

1. Independently audit cw-reality and reproduce the selected frozen wasm checksum from pinned source; do not represent the recorded best-effort mismatch as provenance.
2. Refresh chain/oracle evidence, deployed addresses, code IDs, checksums, admins, gas, and storage before any deployment authorization.
3. Maintain clean-room implementation provenance. Do not copy or adapt LGPL source; preserve source notices/citations as research provenance rather than code derivation.
4. Issue #45 must produce authoritative DAO-core contract tests and a reviewable non-broadcast Juno Agents DAO proposal packet. Any live DAO proposal/funded canary needs separate authorization. Issues #4 and #13 are deferred x/gov compatibility work, not v1 blockers.
5. Issue #26 must collect qualified, actor- and jurisdiction-specific legal advice and exercised operational controls. This packet contains no legal advice.
6. No deployment or fund movement is authorized by issue #2 acceptance.
