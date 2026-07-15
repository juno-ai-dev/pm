# Prediction-market phase review

**Audit date:** 2026-07-15  
**Verdict:** phase not closed; implementation is not authorized  
**Reason:** the candidate architecture is documented, but human acceptance, parameter risk acceptance, legal advice, reproducible oracle build/audit, gas evidence, and an authorized Juno governance rehearsal are missing

This is an evidence audit, not a progress summary. “Documented” means the candidate answer exists. It does not mean accepted or externally verified.

**Machine-readable authorization:** [`authorization.json`](authorization.json)
records `implementation_authorized: false`. The
[issue #2 decision packet](issue-2-decision-packet.md) supplies proposed
dispositions and blank reviewer sign-off fields; it does not close a gate.

## Mission questions

| Question | Candidate answer | Evidence | Verdict |
| --- | --- | --- | --- |
| What is traded/backed? | Internal YES/NO complete sets; Y=N=P; native ujuno | [R1](mechanism.md), [A1](architecture.md) | Documented; review open |
| How are prices/liquidity formed? | Integer FPMM, locked creator LP, candidate 2% fee | [R1](mechanism.md), [ADR-002](adrs/ADR-002-fpmm.md), [ADR-009](adrs/ADR-009-locked-initial-liquidity.md) | Formulas/examples proven on paper; fee/LP policy acceptance open |
| What exact oracle bytes settle? | Exact 32-byte 0/1; every other result neutral | [R3 compatibility](cw-reality-compatibility.md), [question spec](question-specification.md), [ADR-014](adrs/ADR-014-answer-bytes-and-template.md) | Documented; human acceptance open |
| What happens in every resolution failure? | Counter clocks reset; challenge/governance/stall paths bounded; unanswered explicitly unbounded | [R3](cw-reality-compatibility.md), [A3](user-journeys.md), [ADR-013](adrs/ADR-013-resolution-liveness.md) | Documented; governance path unproven |
| Who is trusted? | Immutable code/chain; x/gov only for challenged answer/payee; off-chain no authority | [A1](architecture.md), [R4](juno-and-topology.md), [R5](product-legal-operations.md) | Documented; frozen deployment/counsel/rehearsal open |

## Deliverable audit

| ID | Artifact | Scope evidence | Review evidence | Verdict |
| --- | --- | --- | --- | --- |
| R1 | [mechanism.md](mechanism.md) | Formulas, rounding, multi-trade conservation, impact, LP payoff, dust | No human sign-off | Candidate complete; open |
| R2 | [prior-art.md](prior-art.md) | Required systems, changes/failures, dispositions, licenses | License counsel absent | Candidate complete; open |
| R3 | [cw-reality-compatibility.md](cw-reality-compatibility.md), [question-specification.md](question-specification.md) | Source matrix, bytes, governance, sequences, discrepancies | Governance rehearsal absent | Candidate complete; open |
| R4 | [juno-and-topology.md](juno-and-topology.md) | Chain profile, units, topology, admin/version/deployment | Market gas/storage cannot be measured in no-code phase | Candidate complete; open |
| R5 | [product-legal-operations.md](product-legal-operations.md) | No-entity posture, roles, content/discovery, runbooks | Qualified counsel and named operators absent | Candidate complete; open |
| A1 | [architecture.md](architecture.md) | Components, lifecycle, storage, actions/queries/events, failures, upgrades | No architecture reviewer | Candidate complete; open |
| A2 | [security-and-economics.md](security-and-economics.md) | Invariants, threats, tier, params, audit/tests | Parameter acceptance absent | Candidate complete; open |
| A3 | [user-journeys.md](user-journeys.md) | All named roles and normal/adverse cases | No acceptance-test reviewer | Candidate complete; open |
| ADRs | [ADR index](adrs/README.md) | 001–018 each has alternatives/evidence/consequences/revisit | All Proposed except ADR-017 deferred | Not accepted |
| Decision packet | [issue-2-decision-packet.md](issue-2-decision-packet.md), [authorization.json](authorization.json) | ADR matrix, dated numeric register, schema/license choices, sign-off fields, fail-closed authorization | No required reviewer or owner has signed | Decision-ready; not accepted |

## GOAL.md section 13 gate audit

| Gate | Authoritative evidence required | Current evidence | Verdict |
| --- | --- | --- | --- |
| R1–R5 and A1–A3 reviewed/linked | Files plus reviewer acceptance | Files linked above; no reviewer record | **Open** |
| ADRs 001–018 accepted or safely deferred | Status, reviewer/date, evidence, dissent, residual risk, safe default/revisit | [Decision packet](issue-2-decision-packet.md) contains a complete proposed matrix and sign-off fields; ADR-017 safely blocks dependent implementation; no sign-off exists | **Open** |
| cw-reality source/schema and production instance independently verified | Source/schema pin, tests, on-chain state, reproducible wasm/audit | [Source baseline](evidence/source-baseline.md), 57 tests, [height snapshot](evidence/2026-07-15-juno.md), and [wasm attempt](evidence/oracle-wasm-reproducibility.md); deployed bytes agree across providers, but the best-effort source build mismatched and no independent audit exists | **Open** |
| Live evidence archived with raw values/height/authorities | Untouched bodies+headers at one height and sign-off refresh | [Byte-exact two-provider archive](evidence/raw/39830878/README.md) at 39,830,878 now satisfies the collection format; sign-off refresh and reviewer attestation remain | **Evidence archived; sign-off open** |
| Result bytes/payout fixed in writing | Accepted immutable spec | Exact table exists in R3/ADR-014; Proposed | **Open** |
| Every settlement sequence terminates or discloses nontermination | Reviewed sequence/state analysis | Normal/counter/challenge/stall/neutral and unanswered nontermination documented | **Documented; review open** |
| Invariants balance for buys/sells/LP/fees/rounding/payouts/partial/forced funds | Worked examples plus independent calculation | R1/A3 exact 105-JUNO reconciliation, dust/partial/forced rules; no independent reviewer/model | **Open** |
| Market cap/oracle tier approved | Dated quantitative risk acceptance | Candidate values are consolidated with unset acceptance dates in the [decision packet](issue-2-decision-packet.md); no approval | **Open** |
| ujuno/display/liquidity conversions verified | Chain denom facts and reviewed conversions | R4 conversions plus [single-venue Osmosis measurement](evidence/2026-07-15-osmosis-juno-liquidity.md); venue-complete/long-window evidence and human acceptance remain | **Open** |
| Permissions enumerated address by address | Accepted matrix with refreshed addresses | A1/R4 matrix; x/gov height-pinned; future frozen oracle/factory/market addresses absent | **Open** |
| Challenge accounting all paths | Accepted state/accounting table | A2/ADR-018 cover changed/same/rejected/failed/stale/timeout/direct cancel | **Documented; review open** |
| Juno governance verdict rehearsed or path rejected | Authorized on-chain rehearsal evidence or replacement owner decision | Passed proposals 357/363 establish generic x/gov `MsgExecuteContract` precedent; exact market verdict/payee, cw-reality effects, gas, stale/failure paths remain unrehearsed | **Open—blocking** |
| Implementation test plan covers required classes | Traceable test/audit plan | A2 includes unit/property/multi/adversarial/migration/gas/on-chain | **Documented; review open** |
| License strategy approved | License/owner approval and provenance; counsel input where required | Two candidate routes and exact provenance controls are decision-ready in the [decision packet](issue-2-decision-packet.md); no route approved | **Open** |
| Product/legal/content posture documented | Memo plus applicable legal advice | R5 documented; counsel absent | **Open** |
| Human can trace one unit end to end | Reviewer demonstration | A3 cross-journey and R1 amounts exist; no human attestation | **Open** |

## Definition-of-success audit

| Requirement | Candidate control | Evidence quality |
| --- | --- | --- |
| No action creates excess claims | P/Y/N/F/C equations, checked actions | Paper specification and local oracle tests only; future property/model tests required |
| No trading at/after close | Derived state and execute guard | Specified for every price-changing action; future boundary test required |
| Finalized exact oracle/question only | dual query plus full-field checks | Matches inspected source; future multi-contract test/reproducible oracle required |
| Deterministic all-answer payout | exact-byte total mapping | Fully specified; not accepted |
| Immutable/governed policies explicit | no admins; x/gov narrow verdict | Explicit; frozen addresses do not yet exist |
| Rounding/fees/dust/forced/partial/last behavior | R1 cumulative and dust rules | Worked evidence; independent review/model missing |
| Oracle security relative to value | fixed candidate tier/cap | Relationship explicit but not proven/accepted |
| Permissionless Internet consequences | R5 actor/content/ops matrix | Documented; counsel/owners missing |
| Test/audit plan from invariants | A2 trace | Documented; no implementation execution authorized |

## Source claim classification audit

- **Observed facts:** local source behavior, schema hash, 57-test result, source commits, two-provider height-pinned chain/oracle state, deployed wasm retrieval, governance precedents, and single-venue Osmosis reserves/TWAP are labeled.
- **Author claims/project policy:** mechanism papers and official project docs are cited as such.
- **Inferences/recommendations:** FPMM choice, locked LP, frozen oracle, canary values, and content controls are labeled candidate.
- **Owner decisions:** native JUNO, permissionless creation, Juno governance, experimental/no-entity posture are traced to GOAL.md section 14.
- **Missing evidence:** no document converts rehearsal, counsel, review, gas, audit, or risk acceptance into a passing claim.

## Implementation-test traceability

| Invariants/risks | Planned evidence |
| --- | --- |
| 1–8 pre-resolution complete sets, resolved T2 coverage, product/fees/principal | arbitrary-precision action-sequence model and exact R1 vectors |
| 9 close | one second before/at/after plus same-block ordering |
| 10 resolution binding | every oracle/question field mismatch and checksum/admin deployment failure |
| 11–15 redemption/arithmetic/dust/path | partition fuzzing, neutral odd-address permutations, overflow boundaries |
| 16–17 challenge/verdict | multi-contract challenge, spoof, direct cancel, every proposal outcome |
| 18 immutability | chain admin queries and failed migrate attempts |
| 19 cap | split/buy exact boundary and overflow |
| 20 no sweep/forced funds | random bank excess and abandoned positions |
| Threat model | adversarial suite in A2 plus interface/indexer fault exercises |
| Operations | monitored frozen-oracle/canary/governance rehearsal with incident timeline |

## Blocking evidence/actions

1. Human reviewers must accept/reject every deliverable and ADR, record dissent, and approve or replace quantitative parameters.
2. Independently audit cw-reality and reproduce the selected on-chain/future frozen wasm checksum from pinned source; the recorded best-effort mismatch is not provenance.
3. Have a reviewer verify the byte-exact archive and repeat the two-provider height-pinned capture at sign-off.
4. Obtain qualified legal and license advice for actual participants/operators.
5. In a separately authorized phase, measure market/factory gas/storage once an implementation candidate exists.
6. In a separately authorized, funded rehearsal, execute the Juno x/gov verdict path including failure cases; otherwise obtain a replacement owner decision.
7. Name and exercise content-review, monitoring, incident, deposit-sponsor, and keeper roles.

Until those are satisfied, unchecked gates in GOAL.md must remain unchecked and no implementation plan is authorized.
