# Issue #2 — v1 architecture decision packet

**Packet date:** 2026-07-15
**Acceptance date:** 2026-07-16

**Packet status:** Accepted

**Authorization:*** Milestone implementation is **authorized** for contract code, tests/models, SDK, frontend, indexer, and operations tooling

**Safety boundary:** deployment, fund movement, mainnet governance-rehearsal transaction execution, and claims of legal or operational readiness remain unauthorized

On 2026-07-16 Jake Hartnell accepted the complete packet and delegated architecture, economic-security, and license/provenance decisions to Juno AI: “Everything is accepted. You are empowered to make decisions and are not blocked. I will review when done.” This records architecture/product acceptance and implementation authority; it does not manufacture an audit, qualified legal advice, deployed checksum, funded/mainnet governance transaction, or operational-readiness evidence.

The machine-readable companion is [`authorization.json`](authorization.json). It authorizes implementation tooling while retaining separate fail-closed gates for deployment, funds, and governance-rehearsal transaction execution. Its label policy permits removal of `blocked: decision`; this packet and PR do not themselves change labels.

## 1. How to decide

Each required reviewer records identity, review date, evidence considered, dissent, residual risk, and disposition. Permitted dispositions are:

- **Accept** — accepts the proposed decision and listed residual risk;
- **Replace** — supplies exact replacement text/value and rationale;
- **Defer** — retains the safe default and supplies an objective revisit trigger;
- **Reject** — rejects the proposal without authorizing implementation that depends on it.

No row becomes Accepted merely because a reviewer approves this PR. The owner must record a final disposition after all required reviews. The owner may not mark implementation authorized while any required ADR or critical parameter is Proposed/Rejected, or Deferred without a safe default and trigger.

## 2. Required review and sign-off

| Review | Reviewer identity | Date (YYYY-MM-DD) | Evidence considered | Dissent / conditions | Residual risk accepted | Disposition |
| --- | --- | --- | --- | --- | --- | --- |
| Architecture | Juno AI (delegated by Jake Hartnell) | 2026-07-16 | R1–R5, A1–A3, ADR-001–018, §§4–5 | none recorded | Residual risks in the ADR matrix and evidence gates | Accept |
| Economic security | Juno AI (delegated by Jake Hartnell) | 2026-07-16 | R1, A2, ADR-008/010/013/017/018, exact §4 register | none recorded | Capped loss, oracle/governance failure, thin liquidity, liveness and parameter-model risk | Accept |
| License/provenance | Juno AI (delegated by Jake Hartnell) | 2026-07-16 | R2, repository Apache-2.0 policy, §6 provenance controls | none recorded | Independent-expression provenance must be maintained; this is not qualified legal advice | Accept independent-expression route |
| Owner | Jake Hartnell | 2026-07-16 | Complete decision packet and delegated reviews | none recorded | All documented residual risks; evidence gates remain | Accept and authorize scoped implementation |

License review here is a project authorization gate, not legal advice. ADR-016 separately keeps public interface/deployment blocked pending dated advice applicable to actual actors and jurisdictions.

## 3. ADR disposition matrix

All ADR-001–018 dispositions were accepted on 2026-07-16. ADR-017 was subsequently amended on 2026-07-17 by issue #45: the immutable address-based authority boundary remains accepted, but the Juno Agents DAO core is the v1 profile and Juno `x/gov` is deferred compatibility work under #4/#13. The table below records the original decision packet; the amended ADR and current roadmap govern implementation.

| ADR | Status | Accepted disposition | Evidence considered | Contrary evidence / limitation | Material residual risk | Implementation/deployment control | Objective revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [001](adrs/ADR-001-binary-fixed-expiry.md) | Accepted | Binary fixed-expiry only | R3 question/time policy; A1 lifecycle | Broader market types are excluded rather than proven unsafe | Fixed timestamps can lock in bad rules; chain time controls | Implement; no deployment authority | New version only after audited binary lifecycle and demand for another shape |
| [002](adrs/ADR-002-fpmm.md) | Accepted | Integer FPMM under the §6 clean-room route | R1 formulas/examples; pinned Gnosis commit `6814c024`; R2 | FPMM fitness is inferred; LGPL source is excluded | Thin/manipulable quotes, LP adverse selection, implementation/provenance defects | Clean-room implementation only; no deployment authority | License sign-off plus independent arithmetic review; later mechanism reconsideration after observed flow/depth |
| [003](adrs/ADR-003-internal-positions.md) | Accepted | Non-transferable internal balances | R1; A1 accounting | Sacrifices composability and wallet transfer | Lost keys and abandonment lock claims forever | Implement; no deployment authority | Independently audited token standard and concrete router/CLOB need |
| [004](adrs/ADR-004-isolated-topology.md) | Accepted | Immutable factory plus isolated market | A1; R4 | Gas/storage is unmeasured | Repeated instantiate cost; bugs cannot be repaired | Implement and measure; no deployment authority | Implementation-phase gas evidence and safety proof; never migrate funded v1 |
| [005](adrs/ADR-005-native-ujuno.md) | Accepted | Native `ujuno` only | GOAL §14; R4 denom evidence | Owner direction does not prove economic suitability | JUNO volatility and dual collateral/governance concentration | Implement; no deployment authority | New audited collateral version after explicit owner decision |
| [006](adrs/ADR-006-neutral-invalid.md) | Accepted | Deterministic neutral fallback | R1 dust accounting; R3 byte table | Neutral can reward ambiguous markets and lets governance escape | Canonical wrong 0/1 still redirects value; half-dust goes to LP | Implement and test; no deployment authority | Audited bounded re-question design in a new version |
| [007](adrs/ADR-007-market-owned-question.md) | Accepted | Atomic market-owned question | R3 source matrix/ID derivation; A1 activation | Local ID compatibility is brittle and build provenance is open | Omitted oracle fields or source changes could bind the wrong guarantees | Implement fail-closed creation; no deployment authority | Audited oracle response/prediction API in a new frozen dependency |
| [008](adrs/ADR-008-oracle-tiers-and-caps.md) | Accepted | Exact canary recommendation and every dated value in §4 | A2; Juno/Osmosis evidence snapshots | Oracle/governance corruption cost is unquantified; external liquidity does not secure resolution | Up to capped principal can follow a wrong canonical result | No deployment; no uncapped factory | Rehearsal, named monitor capital, current governance/concentration evidence, and explicit risk acceptance |
| [009](adrs/ADR-009-locked-initial-liquidity.md) | Accepted | One creator=LP, fixed and locked | R1 lifecycle/payoffs; A1 | LP capital can be locked indefinitely and cannot rebalance | LP loss, unanswered lock, lost LP key | Implement and test; no deployment authority | Audited fee accumulator/withdrawal design plus operating data |
| [010](adrs/ADR-010-fees-and-dust.md) | Accepted | Exact 200 bps and documented dust rules | R1 worked reconciliation; A2 | Omen precedent and one-day collateral movement do not establish Juno fitness | Fee may overcharge flow or fail to compensate LP; forced excess is stranded | No launch | Measured volume/trade size/LP loss/routing; changed fee requires new factory |
| [011](adrs/ADR-011-permissionless-creation.md) | Accepted | Objective bounds with no allowlist | GOAL §14; R5 | Permissionless scope creates spam/content/legal exposure | Harmful/illegal markets remain directly accessible | No public interface/deployment pending ADR-016 gates | New explicit owner decision; existing factory remains immutable |
| [012](adrs/ADR-012-no-admin-or-pause.md) | Accepted | No admin/migration/pause/recovery/sweep | A1 authority matrix; R3 deployed-oracle admin evidence | Immutability prevents emergency repair | Live defects, typo, abandoned funds, and lost keys cannot be repaired | No deployment until frozen checksums/admin state are verified | New version with explicit authority analysis; never mutate funded v1 |
| [013](adrs/ADR-013-resolution-liveness.md) | Accepted | Disclosed unbounded unanswered state and §4 bounty/timeouts | R3 sequences; A2 failure analysis | Bounty does not guarantee an answer; repeated counters remain possible | Funds may remain locked indefinitely | No launch; Merge remains the only unresolved liability-reducing exit | Audited oracle-preserving bounded re-question design |
| [014](adrs/ADR-014-answer-bytes-and-template.md) | Accepted | Exact bytes/JCS with §5 bounds | R3 byte table and typed document | Semantic clarity cannot be enforced; storage/gas unmeasured | Unknown bytes settle neutral; malformed prose can still be binding | Implement and measure; no deployment authority | Versioned template with golden vectors and measured gas; never reinterpret live bytes |
| [015](adrs/ADR-015-offchain-trust.md) | Accepted | Off-chain convenience only | A1 query/event surface; R5 | Direct queries can still be unavailable or misrendered | RPC/UI/indexer can lie, lag, censor, or omit | Implement the accepted §5 query/event contract; no deployment authority | Future CLOB requires separate signing/availability ADR |
| [016](adrs/ADR-016-product-posture.md) | Accepted; legal/operational readiness evidence remains open | Experimental, value-bearing, permissionless/no-entity architecture; public operation/deployment remains gated by issue #26 | GOAL §14; R5 actor/risk matrix | No dated counsel advice or named operators | Actor- and jurisdiction-specific legal/content/operational exposure | No public interface or deployment | Dated applicable advice, named roles, exercised runbooks, and owner acceptance |
| [017](adrs/ADR-017-juno-governance-arbitration.md) | Amended 2026-07-17; issue #45 | Market controller relays verdicts only from an immutable authority; Juno Agents DAO core is v1, x/gov later | R3 source/sequence matrix; DAO core profile; historical proposals 357/363 | Exact live DAO verdict/payee path, failures, gas, and deadline are unrehearsed | DAO governance/upgrades can choose wrong answer/payee or fail to execute | Implement/test and produce non-broadcast packet only; no live proposal or deployment authority | DAO profile changes, live rehearsal evidence, or later x/gov work under #4/#13 |
| [018](adrs/ADR-018-challenge-bond.md) | Accepted | Exact bond and all documented refund/slash paths in §4 | A2 path table; R3 sequences | Legitimate challenger loses bond when governance/deposit/execution fails | Challenge can become inaccessible; governance freeze incentives remain | No launch | Rehearsed paths plus accessibility/spam analysis; changed rule requires new tier |

### Owner disposition record

Completed after the four reviews above.

- **Owner identity:** Jake Hartnell
- **Decision date:** 2026-07-16
- **ADRs accepted:** ADR-001 through ADR-018
- **ADRs replaced:** none
- **ADRs deferred:** none; ADR-017 implementation is accepted while rehearsal evidence remains issue #4 work
- **ADRs rejected:** none
- **Dissent preserved:** none recorded
- **Implementation authorization:** **AUTHORIZED**
- **Authorized scope:** contract code, tests/models, SDK, frontend, indexer, and operations tooling for the milestone. Deployment, fund movement, legal-readiness claims, and mainnet governance-rehearsal transaction execution are not authorized.

## 4. Critical numeric decision register

**Recommendation date for every row:** 2026-07-15. **Acceptance date:** 2026-07-16. Jake Hartnell and Juno AI (delegated by Jake Hartnell) accepted every value exactly as recommended, together with the listed limitations and residual risk. Acceptance authorizes implementation, not deployment.

| Parameter | Accepted value | Rationale / limitation | Recorded decision | Acceptance date / reviewer |
| --- | ---: | --- | --- | --- |
| Initial liquidity/principal | minimum `100,000,000 ujuno`; even ujuno | Keeps a representative 1-JUNO balanced-pool buy under about 0.5 quote points in R1; usage/gas unmeasured | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Locked principal `P` | maximum `200,000,000 ujuno` | Containment only; 20× initial oracle bond is not a corruption-cost proof | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Per-address outcome exposure | maximum `20,000,000` units per side | Accidental concentration control; Sybil-bypassable | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| LP fee | exactly `200 bps` (`2%`) | Worked and precedent-backed, not empirically fit for Juno event flow | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Protocol fee | exactly `0 bps` | Avoids protocol recipient/sweep surface | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Minimum buy / requested sell / Split | `10,000 ujuno` | Bounds dust/spam; gas unmeasured | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Per-call trade bound | net split or merge `<= floor(min(reserve_yes,reserve_no)/4)`; result must leave both reserves `>=1` | Limits one-call reserve movement, not cumulative trading | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Oracle initial bond floor | `10,000,000 ujuno` | 5% of cap; no proof of adequate deterrence | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Oracle bounty | `1,000,000 ujuno` funded separately at creation | Incentive only; no service guarantee | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| First-counter monitoring capacity | named monitor able to post at least `20,000,000 ujuno` | Accepted operational pre-deployment commitment, not a contract parameter or guaranteed service | Accepted for implementation planning; deployment evidence remains open | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Answer timeout | exactly `86,400 seconds` | Current production floor and documented precedent; resets after each accepted later answer | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Challenge bond | `max(10,000,000 ujuno, current_oracle_bond)` | Prevents free freeze but may be inaccessible | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Arbitration timeout | exactly `1,814,400 seconds` (21 days) | 10-day deposit + 5-day vote + 6-day margin; exact flow unrehearsed | Accepted for implementation; rehearsal evidence remains issue #4 | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Creation-to-close lead | minimum `86,400 seconds` | Monitoring/review window, not semantic proof | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Maximum creation-to-close duration | `7,776,000 seconds` (90 days) | Bounds pre-close LP/operations burden; unanswered remains unbounded | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Opening delay after close | `0..2,592,000 seconds` (30 days), with `opening_ts >= close_ts` | Event/source-specific; longer markets require another tier | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Canonical question bytes | maximum `16,384 bytes` UTF-8 after JCS | Accepted storage/gas bound; not yet measured | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Discovery metadata bytes | maximum `4,096 bytes` UTF-8 | Accepted index/storage bound; non-authoritative and not yet measured | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |
| Factory pagination | default `50`, maximum `100` records | Bounded query work; gas unmeasured | Accepted | 2026-07-16; Jake Hartnell / delegated Juno AI |

No replacement value is implied by this acceptance. A future replacement must state raw `ujuno`/seconds/bytes as applicable, date, reviewer, rationale, and impact on all dependent rows.

## 5. Consensus/schema choices to freeze

Everything in this section is **accepted for implementation** as of 2026-07-16, with measurements and operational evidence still required before deployment.

### 5.1 Typed source-entry bounds

The market constructs JCS; callers do not submit arbitrary JSON. Lengths are UTF-8 byte lengths. Total canonical question bytes remain bounded by §4.

| Field | Recommended bound |
| --- | --- |
| `title` | `1..160` bytes |
| `proposition` | `1..1,024` bytes |
| `definitions` | `0..16` entries, each `1..512` bytes |
| `invalid_conditions` | `1..16` entries, each `1..512` bytes |
| `primary_sources` | `1..5` ordered entries |
| `secondary_sources` | `0..5` ordered entries |
| source `publisher` | `1..128` bytes |
| source `identifier` | `1..256` bytes |
| source `url` | `1..2,048` bytes; absolute `https` URI in the reference template |
| source `retrieval` | `1..128` bytes |
| source `publication_revision_policy` | `1..512` bytes |
| source `fallback_condition` | `1..512` bytes |
| `source_disagreement_policy` | `1..1,024` bytes |
| observation `revision_policy` | `1..512` bytes |
| `language` | exactly `en` for `juno-pm-question/1` |

`publication_revision_policy` is accepted as an explicit source-entry field. The implementation must not hide publication/revision timing in ambiguous prose.

### 5.2 Identity, LP, time, and neutral exhaustion

- **Nonce:** factory-maintained monotonic `u64`, starting at `0`; checked increment; assigned in creation order; never caller-selected or reused. A failed atomic creation rolls back allocation. The nonce and factory address are queryable and emitted.
- **Creator = LP:** the authenticated `Factory.CreateMarket` sender is both immutable `creator` and sole initial LP. There is no creator/LP override, recipient, or transfer field. Fixed LP supply equals initial principal.
- **Neutral exhaustion:** an address's odd half-unit remainder is finalized to global half-dust only after a successful redemption leaves **both** its YES and NO balances at zero. Burning one side to zero is not exhaustion. Once finalized, the address cannot receive positions in v1 because positions are non-transferable and Split/Buy are closed after resolution.
- **Oracle arbitration deadline:** `deadline = arbitration_start_ts + 1,814,400` with checked arithmetic. `GovernanceVerdict` is valid only when `block.time < deadline`; timeout/cancel is valid when `block.time >= deadline`. Equality belongs exclusively to timeout. The deadline is snapshotted and never extended.

### 5.3 Solvency response and shortfall

The Solvency query returns decimal-string integers and never underflows:

- `bank_balance`;
- `principal_or_terminal_liability` (`P` before resolution; `ceil(T2/2)` after);
- `fee_liability` (`F`);
- `challenge_liability` (`C`);
- `lp_whole_coin_accrual`;
- `accounted_liability` (sum of the preceding liabilities);
- `forced_excess = max(bank_balance - accounted_liability, 0)`;
- `shortfall = max(accounted_liability - bank_balance, 0)`;
- `solvent = (shortfall == 0)`;
- `height` and `block_time`.

A nonzero shortfall is an observable invariant breach, not a claim against an admin or forced excess elsewhere.

### 5.4 Exact rational quotes

All quote ratios use `{ "numerator": "<unsigned decimal>", "denominator": "<positive unsigned decimal>" }`; no float or rounded decimal is consensus output, and reduction by GCD is not required. Components retain their formula meaning:

- marginal YES before/after: `reserve_no / (reserve_yes + reserve_no)`;
- marginal NO before/after: `reserve_yes / (reserve_yes + reserve_no)`;
- average buy price: `gross / outcome_out`;
- average sell price: `net_return / outcome_in`;
- fee rate: `fee / gross` for buy, `fee / complete_sets_merged` for sell;
- absolute impact: `abs(after_num*before_den - before_num*after_den) / (after_den*before_den)`, plus `direction = up|down|flat`.

Quotes also return `height`, `block_time`, reserve snapshot, gross/net/fee/input/output, and the caller constraint (`min_out` or `max_in`) expected by execute. A quote is not reserved and must not be called a probability.

### 5.5 Event schema v1

Every market event uses type `wasm-juno_pm_v1` and includes `action`, `protocol_version`, `factory`, `market`, `height`, and `block_time`. Attribute names are lowercase `snake_case`; amounts are raw unsigned decimal strings; timestamps are Unix seconds; answers include both `answer_hex` (lowercase, no `0x`) and `answer_base64` (RFC 4648 standard alphabet with padding). No attribute changes meaning within v1.

| `action` | Required action-specific attributes |
| --- | --- |
| `market_created` | `creator`, `nonce`, `initial_principal`, `oracle_bounty` |
| `market_activated` | `creator`, `lp`, `question_id`, `question_hash`, `close_ts`, `opening_ts` |
| `split` / `merge` | `account`, `amount`, `principal_after` |
| `trade` | `account`, `side`, `outcome`, `gross`, `net`, `fee`, `input`, `output`, `reserve_yes_before`, `reserve_no_before`, `reserve_yes_after`, `reserve_no_after`, `principal_after`, `fee_liability_after` |
| `challenge_requested` | `challenger`, `question_id`, `answer_hex`, `answer_base64`, `oracle_bond`, `challenge_bond`, `arbitration_deadline` |
| `governance_verdict_forwarded` | `question_id`, `answer_hex`, `answer_base64`, `payee` |
| `challenge_refunded` / `challenge_slashed` | `challenger`, `amount`, `recipient` |
| `arbitration_stalled` | `question_id`, `arbitration_deadline`, `challenge_bond` |
| `market_resolved` | `question_id`, `answer_hex`, `answer_base64`, `payout_yes_num`, `payout_no_num`, `payout_den`, `principal_at_resolution`, `terminal_liability_numerator` |
| `positions_redeemed` | `account`, `yes_burned`, `no_burned`, `paid`, `terminal_liability_numerator_after` |
| `lp_redeemed` | `lp`, `lp_burned`, `position_paid`, `fee_paid`, `lp_supply_remaining` |
| `lp_accrual_claimed` | `lp`, `amount`, `lp_accrual_after` |

Factory registry events use the same type and identity fields. Events are non-authoritative convenience; direct query state controls. Indexers key chain ID + contract address + transaction hash + event index and must tolerate replay/rollback.

### 5.6 Pagination

`Factory.ListMarkets` is ordered by ascending factory nonce and accepts exclusive `start_after_nonce` plus `limit`; default `50`, maximum `100`, and `limit=0` rejects. The response includes `markets` and nullable `next_start_after_nonce`, which is the last returned nonce only when another page exists. No user-position enumeration query is introduced; `Position(address)` remains direct to avoid privacy and unbounded scans.

## 6. License and provenance decision

The accepted route is a clean-room independent implementation of the public mathematical mechanism from this repository's specifications and formulas under the repository's Apache-2.0 policy. Implementers must not copy, adapt, translate, or derive expression from LGPL source. Notices and citations are preserved as provenance, not as evidence of code derivation. This project authorization is not qualified legal advice.

| Route | Decision text if selected | Required evidence | Current disposition |
| --- | --- | --- | --- |
| LGPL compliance | “The project will treat source-derived FPMM implementation as LGPL-3.0 and comply with all notice, source, relinking/modification, distribution, and dependency obligations identified by qualified review.” | Reviewer identity/date; distribution model; notices/source plan; dependency inventory; counsel advice where required | Not selected; LGPL source must not be copied or adapted |
| Independent expression | “The project will implement only the public mathematical mechanism without copying Gnosis expression, structure, comments, tests, or source-derived pseudocode.” | Reviewer identity/date; clean provenance plan; allowed/blocked source list; contributor attestations; independent formula/test derivation | **Accepted 2026-07-16** |
| Replacement mechanism | Record new mechanism ADR and its license/provenance | Full architecture/economic/license review | Not selected |

Recommended provenance controls for the independent-expression route:

1. pin and cite mathematical/public behavior sources separately from source repositories;
2. prohibit copying or translating Gnosis implementation code, structure, comments, names, and tests;
3. record every implementation contributor, materials consulted, dates, and attestations;
4. derive formulas/tests from R1 and an independently authored reference model;
5. scan commits for copied expression and preserve third-party notices;
6. separately resolve `cw-reality`/Reality.eth provenance and reproducible build mismatch; Apache-2.0 labels do not settle upstream analysis.

### License disposition record

- **Selected route:** Independent expression / clean-room implementation under repository Apache-2.0 policy
- **Reviewer identity and role:** Juno AI (delegated by Jake Hartnell), license/provenance reviewer
- **Review date:** 2026-07-16
- **Evidence/provenance record:** R1 formulas, R2 source/citation matrix, and the controls above
- **Dissent/conditions:** none recorded; do not copy or adapt LGPL source
- **Residual risk:** provenance discipline must be maintained; this approval is not qualified legal advice
- **Owner acceptance/date:** Jake Hartnell, 2026-07-16

## 7. Authorization truth and label policy

Current truth as of 2026-07-16:

- ADR-001–018 are Accepted; ADR-017's architecture is settled while rehearsal evidence remains issue #4 work.
- Every critical numeric parameter is accepted exactly as recommended, dated 2026-07-16.
- The clean-room independent-expression FPMM license/provenance strategy is approved as project policy, not legal advice.
- Architecture, economic-security, license/provenance, and owner sign-offs are recorded above.
- Contract code, tests/models, SDK, frontend, indexer, and operations tooling are authorized for the milestone.
- Deployment, fund movement, and mainnet governance-rehearsal transaction execution remain unauthorized separate safety gates.
- The authorization policy permits removal of `blocked: decision`, but label changes remain a separate owner/repository action and are not made by this PR.

This owner-approved change updates the ADR files, this packet, GOAL checklist, review checklist, and `authorization.json` together. Authorization comes from the recorded owner delegation, never merely from merge, PR approval, issue assignment, or absence of dissent.

## 8. Residual evidence gates

The decision gate is closed, but evidence and execution gates remain. Issue #4 owns end-to-end/mainnet governance-rehearsal evidence and transaction authorization. Issue #26 owns qualified legal and operational-readiness evidence. Implementation issues must carry forward audit, reproducible-build/checksum, gas/storage, test, monitoring, and named-operator requirements. None of those artifacts or transactions is claimed complete by this acceptance.
