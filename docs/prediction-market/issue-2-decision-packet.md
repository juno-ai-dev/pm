# Issue #2 — v1 architecture decision packet

**Packet date:** 2026-07-15

**Packet status:** Proposed for review; no approval is recorded

**Authorization:** Contract implementation is **not authorized**

**Scope:** decision preparation only; no contract code, deployment, funds, legal advice, or governance rehearsal

This packet advances [issue #2](https://github.com/juno-ai-dev/pm/issues/2) by putting the remaining choices into one reviewable record. It does not accept architecture, economics, licensing, legal posture, or implementation authority. A recommendation is not a decision. Empty reviewer fields are intentionally not signatures.

The machine-readable companion is [`authorization.json`](authorization.json). It must remain fail-closed until the required reviewers and owner actually sign this packet. The `blocked: decision` issue label must remain while `implementation_authorized` is `false`.

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
| Architecture | _required_ | _required_ | _required_ | _required; write “none” explicitly if none_ | _required_ | _Accept / Replace / Defer / Reject_ |
| Economic security | _required_ | _required_ | _required_ | _required; write “none” explicitly if none_ | _required_ | _Accept / Replace / Defer / Reject_ |
| License/provenance | _required_ | _required_ | _required_ | _required; write “none” explicitly if none_ | _required_ | _Accept / Replace / Defer / Reject_ |
| Owner | _required_ | _required_ | _required_ | _required; write “none” explicitly if none_ | _required_ | _Accept / Replace / Defer / Reject_ |

License review here is a project authorization gate, not legal advice. ADR-016 separately keeps public interface/deployment blocked pending dated advice applicable to actual actors and jurisdictions.

## 3. ADR disposition matrix

All dispositions below preserve the repository's authoritative status: Proposed except ADR-017, which is Deferred. “Recommended disposition” is a request to the named reviewers, not recorded approval.

| ADR | Current status | Recommended disposition | Evidence to review | Dissent / contrary evidence to resolve | Material residual risk | Safe default while open | Objective revisit trigger |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [001](adrs/ADR-001-binary-fixed-expiry.md) | Proposed | Accept binary fixed-expiry only | R3 question/time policy; A1 lifecycle | Broader market types are excluded rather than proven unsafe | Fixed timestamps can lock in bad rules; chain time controls | No implementation | New version only after audited binary lifecycle and demand for another shape |
| [002](adrs/ADR-002-fpmm.md) | Proposed | Accept integer FPMM **only with an approved license route** | R1 formulas/examples; pinned Gnosis commit `6814c024`; R2 | FPMM fitness is inferred; LGPL route unresolved | Thin/manipulable quotes, LP adverse selection, implementation/provenance defects | No FPMM implementation | License sign-off plus independent arithmetic review; later mechanism reconsideration after observed flow/depth |
| [003](adrs/ADR-003-internal-positions.md) | Proposed | Accept non-transferable internal balances | R1; A1 accounting | Sacrifices composability and wallet transfer | Lost keys and abandonment lock claims forever | No implementation | Independently audited token standard and concrete router/CLOB need |
| [004](adrs/ADR-004-isolated-topology.md) | Proposed | Accept immutable factory plus isolated market | A1; R4 | Gas/storage is unmeasured | Repeated instantiate cost; bugs cannot be repaired | No implementation/deployment | Implementation-phase gas evidence and safety proof; never migrate funded v1 |
| [005](adrs/ADR-005-native-ujuno.md) | Proposed; owner direction dated 2026-07-15 | Accept native `ujuno` only | GOAL §14; R4 denom evidence | Owner direction does not prove economic suitability | JUNO volatility and dual collateral/governance concentration | No implementation until whole packet accepted | New audited collateral version after explicit owner decision |
| [006](adrs/ADR-006-neutral-invalid.md) | Proposed | Accept deterministic neutral fallback | R1 dust accounting; R3 byte table | Neutral can reward ambiguous markets and lets governance escape | Canonical wrong 0/1 still redirects value; half-dust goes to LP | No implementation | Audited bounded re-question design in a new version |
| [007](adrs/ADR-007-market-owned-question.md) | Proposed | Accept atomic market-owned question | R3 source matrix/ID derivation; A1 activation | Local ID compatibility is brittle and build provenance is open | Omitted oracle fields or source changes could bind the wrong guarantees | No implementation; creation must fail closed | Audited oracle response/prediction API in a new frozen dependency |
| [008](adrs/ADR-008-oracle-tiers-and-caps.md) | Proposed; risk acceptance open | Accept the canary recommendation only if economic-security and owner reviewers explicitly accept every dated value in §4 | A2; Juno/Osmosis evidence snapshots | Oracle/governance corruption cost is unquantified; external liquidity does not secure resolution | Up to capped principal can follow a wrong canonical result | No deployment; no uncapped factory | Rehearsal, named monitor capital, current governance/concentration evidence, and explicit risk acceptance |
| [009](adrs/ADR-009-locked-initial-liquidity.md) | Proposed | Accept one creator=LP, fixed and locked | R1 lifecycle/payoffs; A1 | LP capital can be locked indefinitely and cannot rebalance | LP loss, unanswered lock, lost LP key | No implementation | Audited fee accumulator/withdrawal design plus operating data |
| [010](adrs/ADR-010-fees-and-dust.md) | Proposed; fee acceptance open | Accept only if exact 200 bps and dust rules are explicitly accepted | R1 worked reconciliation; A2 | Omen precedent and one-day collateral movement do not establish Juno fitness | Fee may overcharge flow or fail to compensate LP; forced excess is stranded | No launch | Measured volume/trade size/LP loss/routing; changed fee requires new factory |
| [011](adrs/ADR-011-permissionless-creation.md) | Proposed; owner direction dated 2026-07-15 | Accept objective bounds with no allowlist | GOAL §14; R5 | Permissionless scope creates spam/content/legal exposure | Harmful/illegal markets remain directly accessible | No public interface/deployment pending ADR-016 gates | New explicit owner decision; existing factory remains immutable |
| [012](adrs/ADR-012-no-admin-or-pause.md) | Proposed | Accept no admin/migration/pause/recovery/sweep | A1 authority matrix; R3 deployed-oracle admin evidence | Immutability prevents emergency repair | Live defects, typo, abandoned funds, and lost keys cannot be repaired | No deployment until frozen checksums/admin state are verified | New version with explicit authority analysis; never mutate funded v1 |
| [013](adrs/ADR-013-resolution-liveness.md) | Proposed | Accept disclosed unbounded unanswered state; accept numeric bounty/timeouts only via §4 | R3 sequences; A2 failure analysis | Bounty does not guarantee an answer; repeated counters remain possible | Funds may remain locked indefinitely | No launch; Merge remains the only unresolved liability-reducing exit | Audited oracle-preserving bounded re-question design |
| [014](adrs/ADR-014-answer-bytes-and-template.md) | Proposed | Accept exact bytes/JCS only with §5 bounds | R3 byte table and typed document | Semantic clarity cannot be enforced; storage/gas unmeasured | Unknown bytes settle neutral; malformed prose can still be binding | No implementation | Versioned template with golden vectors and measured gas; never reinterpret live bytes |
| [015](adrs/ADR-015-offchain-trust.md) | Proposed | Accept off-chain convenience only | A1 query/event surface; R5 | Direct queries can still be unavailable or misrendered | RPC/UI/indexer can lie, lag, censor, or omit | No implementation until query/event contract in §5 is accepted | Future CLOB requires separate signing/availability ADR |
| [016](adrs/ADR-016-product-posture.md) | Proposed; owner direction dated 2026-07-15; counsel open | Defer public interface/deployment; preserve owner-selected candidate posture | GOAL §14; R5 actor/risk matrix | No dated counsel advice or named operators | Actor- and jurisdiction-specific legal/content/operational exposure | No public interface or deployment | Dated applicable advice, named roles, exercised runbooks, and owner acceptance |
| [017](adrs/ADR-017-juno-governance-arbitration.md) | Deferred | Remain Deferred; do not authorize dependent implementation | R3 source/sequence matrix; proposals 357/363; height snapshot | Exact verdict/payee path, failures, gas, and deadline are unrehearsed | Governance can choose wrong answer/payee or fail to execute | No dependent implementation/launch; do not substitute another authority | Separately authorized end-to-end rehearsal, or replacement owner authority decision |
| [018](adrs/ADR-018-challenge-bond.md) | Proposed; economic acceptance open | Accept only if exact bond and all refund/slash paths in §4 are explicitly accepted | A2 path table; R3 sequences | Legitimate challenger loses bond when governance/deposit/execution fails | Challenge can become inaccessible; governance freeze incentives remain | No launch | Rehearsed paths plus accessibility/spam analysis; changed rule requires new tier |

### Owner disposition record

The owner completes this only after the four reviews above.

- **Owner identity:** _required_
- **Decision date:** _required (YYYY-MM-DD)_
- **ADRs accepted:** _list exact IDs, or “none”_
- **ADRs replaced:** _list IDs and link exact replacement text, or “none”_
- **ADRs deferred:** _list IDs, safe default, and trigger, or “none”_
- **ADRs rejected:** _list exact IDs, or “none”_
- **Dissent preserved:** _link/comment or “none”_
- **Implementation authorization:** _AUTHORIZED / NOT AUTHORIZED_
- **Authorized scope:** _required if authorized; approval of docs is not approval of code, deployment, funds, legal posture, or rehearsal_

## 4. Critical numeric decision register

**Recommendation date for every row:** 2026-07-15. **Acceptance date:** unset. These are candidate canary values, not accepted parameters.

| Parameter | Exact recommendation | Rationale / limitation | Required decision | Acceptance date / reviewer |
| --- | ---: | --- | --- | --- |
| Initial liquidity/principal | minimum `100,000,000 ujuno`; even ujuno | Keeps a representative 1-JUNO balanced-pool buy under about 0.5 quote points in R1; usage/gas unmeasured | Accept or replace | _unset_ |
| Locked principal `P` | maximum `200,000,000 ujuno` | Containment only; 20× initial oracle bond is not a corruption-cost proof | Accept or replace | _unset_ |
| Per-address outcome exposure | maximum `20,000,000` units per side | Accidental concentration control; Sybil-bypassable | Accept, replace, or remove explicitly | _unset_ |
| LP fee | exactly `200 bps` (`2%`) | Worked and precedent-backed, not empirically fit for Juno event flow | Accept or replace | _unset_ |
| Protocol fee | exactly `0 bps` | Avoids protocol recipient/sweep surface | Accept or replace | _unset_ |
| Minimum buy / requested sell / Split | `10,000 ujuno` | Bounds dust/spam; gas unmeasured | Accept or replace | _unset_ |
| Per-call trade bound | net split or merge `<= floor(min(reserve_yes,reserve_no)/4)`; result must leave both reserves `>=1` | Limits one-call reserve movement, not cumulative trading | Accept or replace | _unset_ |
| Oracle initial bond floor | `10,000,000 ujuno` | 5% of cap; no proof of adequate deterrence | Accept or replace | _unset_ |
| Oracle bounty | `1,000,000 ujuno` funded separately at creation | Incentive only; no service guarantee | Accept or replace | _unset_ |
| Answer timeout | exactly `86,400 seconds` | Current production floor and documented precedent; resets after each accepted later answer | Accept or replace | _unset_ |
| Challenge bond | `max(10,000,000 ujuno, current_oracle_bond)` | Prevents free freeze but may be inaccessible | Accept or replace | _unset_ |
| Arbitration timeout | exactly `1,814,400 seconds` (21 days) | 10-day deposit + 5-day vote + 6-day margin; exact flow unrehearsed | Accept only after ADR-017 trigger, or replace | _unset_ |
| Creation-to-close lead | minimum `86,400 seconds` | Monitoring/review window, not semantic proof | Accept or replace | _unset_ |
| Maximum creation-to-close duration | `7,776,000 seconds` (90 days) | Bounds pre-close LP/operations burden; unanswered remains unbounded | Accept or replace | _unset_ |
| Opening delay after close | `0..2,592,000 seconds` (30 days), with `opening_ts >= close_ts` | Event/source-specific; longer markets require another tier | Accept or replace | _unset_ |
| Canonical question bytes | maximum `16,384 bytes` UTF-8 after JCS | Candidate storage/gas bound; not measured | Accept or replace | _unset_ |
| Discovery metadata bytes | maximum `4,096 bytes` UTF-8 | Candidate index/storage bound; non-authoritative | Accept or replace | _unset_ |
| Factory pagination | default `50`, maximum `100` records | Bounded query work; gas unmeasured | Accept or replace | _unset_ |

No value may be inferred accepted from the owner directions already recorded in GOAL §14. Any replacement must state raw `ujuno`/seconds/bytes as applicable, date, reviewer, rationale, and impact on all dependent rows.

## 5. Consensus/schema choices to freeze

Everything in this section is a **recommendation awaiting architecture and owner acceptance**.

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

Whether `publication_revision_policy` becomes an explicit source-entry field rather than prose is an **owner architecture decision** because the current candidate JSON example does not include it. If not accepted, the owner must identify the exact existing field that carries publication/revision timing without ambiguity.

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

No route is approved today. The license/provenance reviewer and owner must choose exactly one before ADR-002 or implementation authorization can become Accepted.

| Route | Decision text if selected | Required evidence | Current disposition |
| --- | --- | --- | --- |
| LGPL compliance | “The project will treat source-derived FPMM implementation as LGPL-3.0 and comply with all notice, source, relinking/modification, distribution, and dependency obligations identified by qualified review.” | Reviewer identity/date; distribution model; notices/source plan; dependency inventory; counsel advice where required | Proposed; not approved |
| Independent expression | “The project will implement only the public mathematical mechanism without copying Gnosis expression, structure, comments, tests, or source-derived pseudocode.” | Reviewer identity/date; clean provenance plan; allowed/blocked source list; contributor attestations; independent formula/test derivation; counsel approval requested by R2 | **Recommended**, but not approved |
| Replacement mechanism | Record new mechanism ADR and its license/provenance | Full architecture/economic/license review | Not selected |

Recommended provenance controls for the independent-expression route:

1. pin and cite mathematical/public behavior sources separately from source repositories;
2. prohibit copying or translating Gnosis implementation code, structure, comments, names, and tests;
3. record every implementation contributor, materials consulted, dates, and attestations;
4. derive formulas/tests from R1 and an independently authored reference model;
5. scan commits for copied expression and preserve third-party notices;
6. separately resolve `cw-reality`/Reality.eth provenance and reproducible build mismatch; Apache-2.0 labels do not settle upstream analysis.

### License disposition record

- **Selected route:** _required; LGPL compliance / Independent expression / Replacement_
- **Reviewer identity and role:** _required_
- **Review date:** _required_
- **Evidence/provenance record:** _required link_
- **Dissent/conditions:** _required; “none” if none_
- **Residual risk:** _required_
- **Owner acceptance/date:** _required_

## 7. Authorization truth and label policy

Current truth:

- ADR-001–016 and ADR-018 remain Proposed; ADR-017 remains Deferred.
- No critical numeric parameter has a human acceptance date.
- No FPMM license strategy is approved.
- Architecture, economic-security, license, and owner sign-offs are absent.
- Contract implementation, generated production schema, deployment, fund movement, and governance rehearsal are not authorized.
- `blocked: decision` must not be removed from issue #2 or dependent work.

After real decisions are recorded, update the ADR files, this packet, GOAL checklist, review checklist, and `authorization.json` in the same owner-approved change. Removing a blocked label is a separate owner action and only applies to work whose exact scope is authorized. Never infer authorization from merge, PR approval, issue assignment, or absence of dissent.

## 8. Irreducible owner decisions

Jake must explicitly decide:

1. accept/replace/defer/reject each ADR-001–018, preserving a safe default and trigger for every deferral;
2. accept or replace every row in §4, including economic loss/oracle/governance residual risk;
3. accept or replace the §5 source bounds, nonce, creator=LP, neutral exhaustion, deadline boundary, solvency shortfall, rational quote, event, and pagination contracts;
4. select and sign one FPMM license/provenance route, after the required review;
5. decide whether ADR-017 remains blocking until rehearsal or replace the owner-selected authority—without authorizing rehearsal funds here;
6. accept or replace ADR-016 only after the applicable advice/operations evidence, while acknowledging this packet is not legal advice;
7. state the exact implementation scope, if any, that is authorized. Until then the only truthful value is **NOT AUTHORIZED**.
