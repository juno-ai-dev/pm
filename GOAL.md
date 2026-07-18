# GOAL — Juno prediction market research, planning, and architecture

**Artifact status:** issue #2 decision packet accepted 2026-07-16; scoped implementation authorized; external evidence, deployment, funds, legal/operational readiness, and governance-transaction gates remain open
**Research snapshot:** 2026-07-15
**Authorization boundary:** the accepted packet authorizes milestone implementation code and tooling. It does not authorize deployment, fund movement, mainnet governance-rehearsal transactions, or claims of legal/operational readiness.

Unchecked items in this document are gates for executing and closing the research phase. They do not mean this charter is incomplete, and they must not be marked complete merely because implementation is ready to start.

## 1. Mission

Produce an evidence-backed, reviewable architecture for a prediction market on Juno Network that consumes finalized answers from `cw-reality` without weakening either the market's collateral solvency or the oracle's trust model.

The phase ends with enough precision that implementation can begin without inventing economic rules, settlement behavior, permissions, or accounting invariants inside a pull request. It does **not** end merely because a contract diagram exists.

The architecture must answer five questions:

1. What exactly is traded, and how is every position collateralized?
2. How are prices formed and liquidity providers compensated?
3. What exact oracle question and answer bytes settle a market?
4. What happens for invalid, unknown, premature, unanswered, disputed, and stalled questions?
5. Which on-chain and off-chain actors are trusted, and what can each actor do?

## 2. Definition of success

This phase is successful when reviewers can trace every accepted state transition from market creation through redemption and verify all of the following:

- No valid action can create more collateral claims than the contract holds.
- A market cannot trade at or after its close boundary.
- Settlement can only consume a finalized answer from the configured `cw-reality` instance and question.
- YES, NO, invalid/unknown, and every non-canonical oracle answer have deterministic payout behavior.
- Market rules, timestamps, answer encoding, oracle, arbitrator, collateral denom, fees, and upgrade authority are immutable or governed by an explicitly documented policy.
- Rounding direction, fee ownership, dust, forced funds, partial redemptions, and last-redeemer behavior are specified.
- Oracle economic security is considered relative to market value at risk rather than treated as secure merely because a contract call succeeds.
- The permissionless Internet launch posture has explicit legal-risk, content-safety, discoverability, and operations consequences. The absence of an operating entity does not make those consequences disappear for contributors, frontend/indexer hosts, market creators, or users.
- The implementation phase has a prioritized test and audit plan derived from written invariants.

### 2.1 Evidence and decision protocol

- Technical claims must cite primary sources: pinned source commits and compiled schemas for code behavior, papers for mechanisms, official project documentation for intended behavior, audits and post-mortems for failures, and direct chain queries for live state.
- Every research memo records source version or commit, access date, and whether a statement is observed fact, author claim, project policy, inference, provisional recommendation, or accepted decision.
- Live-chain facts record chain ID, block height, block time, endpoint, raw value, and human-unit conversion. They are queried again at architecture sign-off and immediately before deployment planning.
- When prose, schema, tests, and executable source disagree, compiled behavior and source control the technical design; the discrepancy remains visible until separately corrected.
- Quantitative parameters remain visibly open until supported by worked examples, measurements, or explicit risk acceptance. A convenient default is not evidence.
- Legal and regulatory conclusions require advice from qualified counsel for the relevant contributors and interface operators. The architecture may identify risks and controls but must not represent itself as legal advice.
- No phase deliverable may quietly introduce executable code, generated schema, a deployment transaction, or a collateral transfer. Numerical analysis belongs in reviewable tables and hand-worked examples during this phase.

## 3. Accepted product architecture

The following architecture was accepted with ADR-001–018 on 2026-07-16. Historical evidence limitations and revisit triggers remain applicable.

| Dimension | Accepted v1 direction | Reason |
| --- | --- | --- |
| Market shape | One fixed-expiry binary YES/NO question | Smallest complete market with unambiguous collateral accounting and the best fit for `cw-reality`'s `Bool` question metadata. |
| Collateral | Native Juno Network `ujuno` only initially | Matches the owner-selected product and removes IBC trace, bridge, issuer, and CW20 callback risks from v1. Displayed prices are fractions of the one-JUNO terminal payout, while all accounting uses integer `ujuno`. |
| Backing | Fully collateralized complete sets: one unit of collateral can become one YES plus one NO | Winning claims remain solvent without liquidations, leverage, or a counterparty credit model. This is the core pattern in Gnosis Conditional Tokens and current Polymarket position accounting. |
| Trading | Fixed-product market maker (FPMM/CPMM) funded at creation | Continuous quotes suit a new, thin venue better than an empty order book and use integer-friendly arithmetic. |
| Positions | Internal per-market balances in v1 | Avoids deploying two CW20 contracts per market or depending immediately on the older CW1155 stack. Tokenized/transferable positions remain a future compatibility layer. |
| Contract topology | Governed factory/registry plus one isolated market contract per market | Isolates collateral and failures, makes a market's bank balance auditable, and permits new code IDs without migrating every live market. |
| Oracle ownership | The market creates or atomically binds its own `cw-reality` question | Prevents substitution of a look-alike question and binds the complete resolution policy at creation. |
| Optimistic oracle answer window | 24 hours after the latest answer | Uses the production `cw-reality` minimum. With no counter-answer and no arbitration challenge, the answer can finalize after roughly one day rather than waiting through a governance cycle. |
| Challenged arbitration window | Accepted 21 days for implementation | Applies only after a bonded challenge freezes the oracle. It covers the current Juno standard governance lifecycle—up to 10 days for deposit plus five days for voting—with six days of operational margin. Issue #4 retains rehearsal evidence before transaction execution or deployment. |
| Invalid/unknown payout | Neutral binary payout: YES and NO each redeem for 0.5 collateral per share | Guarantees a terminal settlement for invalid and non-canonical answers without fragmenting liquidity into a third outcome. Polymarket documents the same 50/50 terminal case for unknown outcomes. |
| Fees | Immutable LP trading fee; no protocol skim in the first audited release | Keeps the first accounting model narrow. The fee value remains an empirical decision, not a guess. |
| Leverage | None | Complete-set solvency is the v1 safety boundary. |
| Creation policy | Permissionless from the first release | Any address may create a market. Objective contract-level safety bounds may still cap duration, market collateral, fees, question size, and approved oracle/arbitrator settings; permissionless does not mean parameterless. |

This hypothesis intentionally optimizes for correctness and a credible first liquidity experience. It does not attempt to reproduce every feature of a mature exchange.

The owner describes the product as “play money” because it uses JUNO in an experimental Internet protocol with no operating entity. Native JUNO is nevertheless transferable and value-bearing. The architecture therefore treats it as real collateral for solvency, oracle incentives, adversarial analysis, and contributor/user risk disclosures. “Play money” is a product posture, not permission to weaken financial invariants.

## 4. Initial research synthesis

### 4.1 Market mechanisms

| Mechanism | Strengths | Costs and failure modes | Phase disposition |
| --- | --- | --- | --- |
| Continuous limit order book (CLOB) | Capital-efficient quotes, familiar maker/taker model, good routing, and precise limit orders. Current Polymarket and Injective binary markets demonstrate the mature form. | An empty book is not a market. It needs active makers, cancellation/indexing infrastructure, signed-order rules, and careful partial-fill accounting. A hybrid/off-chain book also adds availability and sequencing dependencies. | Document as the likely scale-up path, not the bootstrap v1. Preserve the ability to add a separate exchange over transferable positions later. |
| Logarithmic Market Scoring Rule (LMSR) | Always quotes; sponsor loss is bounded. Hanson's cost is determined by the movement from the initial to final report rather than the number of trades. | Requires exponent/logarithm math and a calibrated liquidity parameter `b`. Fixed-point approximation is a large consensus-critical surface in CosmWasm. The market sponsor, not LPs, deliberately subsidizes information. | Retain as a research benchmark. Do not make it the default v1 without a compelling subsidy model and an independently reviewed fixed-point design. |
| Fixed Product Market Maker (FPMM/CPMM) | Battle-tested prediction-market pattern; supports user-supplied liquidity; complete-set split/merge preserves backing; straightforward integer arithmetic. Omen deployed one AMM per market. | LPs are structurally exposed to informed order flow and final 0/1 convergence. Thin pools have high slippage. Fees may not cover adverse selection, especially near resolution. | Recommended v1 baseline, subject to worked accounting examples and parameter analysis. Disclose LP risk plainly. |
| `pm-AMM` | Designed specifically for outcome-token dynamics and concentrates liquidity near 50%, where uncertainty is highest. The dynamic form explicitly models increasing risk near expiry. | Newer design, normal CDF/PDF/inverse math, materially larger numerical and audit surface, and less production history. Dynamic liquidity intentionally falls toward expiry, which may conflict with user demand. | Research and compare; do not lead with it. A later version may justify the extra math after FPMM data exists. |
| Parimutuel pool | Very simple pooled accounting and no continuous maker inventory. | No true secondary exit before close; displayed odds and execution price are path-dependent; weak fit for users expecting a tradable probability. | Out of scope for v1 except as a benchmark. |

The key conclusion is not that FPMM is universally optimal. It is that an FPMM is the smallest mechanism that combines continuous availability, outside liquidity contribution, fully collateralized outcome shares, and tractable on-chain arithmetic. Research must still quantify how poor its quotes become at the liquidity levels Juno can realistically attract.

### 4.2 Implementation lessons to carry forward

| System | What to study | Transferable lesson |
| --- | --- | --- |
| Gnosis Conditional Tokens + FPMM | Complete-set split/merge/redeem, payout vectors, LP fee accounting, buy/sell rounding | Separate collateral semantics from the exchange mechanism. A YES/NO pair is one fully backed claim set; the AMM is only an inventory manager. The FPMM source is LGPL-3.0, so implementation must either comply with that license or independently implement the public mechanism with license review. |
| Omen | One FPMM per market, Reality.eth resolution, invalid-market rules, initial funding | The closest architectural precedent to Juno plus `cw-reality`. Clear question rules and an invalid path are part of financial safety, not merely UI copy. |
| Polymarket | Fully backed binary tokens, CLOB over conditional positions, exact question IDs, rules-first resolution, neutral 50/50 result | Tokenization and exchange can be separate layers. Mature liquidity may favor a CLOB, but the backing and resolution primitives remain on-chain. |
| Augur | Dispute economics, invalid-market history, reporting-token security budget, current Lituus reboot | Oracle cost must exceed profitable corruption. A bespoke security token and fork are far beyond this project's scope; do not invent one casually. |
| Manifold | User-funded CPMM and thin-market bootstrap | Creator subsidies make long-tail markets possible, while LP risk and parameter selection remain product problems. |
| Injective | Binary order-book lifecycle with separate expiration and settlement timestamps | Trading close and expected settlement are distinct times. State names and UI must not blur them. |
| Zeitgeist | Evolution from CPMM/Rikiddo toward newer AMM and hybrid routing; documented exponential-math hotfix | Advanced math and mechanism migrations create real operational risk. Preserve versioned market instances and route later rather than forcing live-market migration. |
| `cw-reality` | Bond doubling, final-answer constraints, reader-side guarantees, arbitration, stalled-arbitrator timeout | Treat the oracle as a separately secured dependency. The market must validate and pin the question's guarantees rather than trusting user-supplied identifiers. |

### 4.3 Research conclusion on “price equals probability”

The UI may display the current marginal YES price as an implied probability, but documentation must not promise that it is a calibrated forecast. Price also contains liquidity, fee, inventory, risk-preference, manipulation, and market-participation effects. The architecture will distinguish:

- **marginal quote**: the price for an infinitesimal next trade;
- **execution price**: collateral paid or received divided by shares;
- **displayed implied probability**: the marginal quote normalized to `[0, 1]`;
- **resolution payout**: 1, 0, or 0.5 per share, independent of the final trading price.

## 5. Accepted architecture to implement and verify

### 5.1 Trust boundaries and components

```text
market creator / trader / LP
            |
            v
  market factory + registry ------> indexer / API / UI
            |                         (read-only convenience)
            v
   one binary-market instance <----- immutable verdict authority
   - collateral vault                (Juno Agents DAO core for v1)
   - YES/NO balances
   - FPMM reserves
   - LP shares + fee accounting
   - lifecycle + redemption
   - bonded challenge gate
   - governance-only verdict relay
            |
            | configured as the question's arbitrator
            v
    pinned cw-reality instance
```

- **Factory/registry:** permissionlessly instantiates a versioned market code ID, records discoverability metadata, and enforces objective protocol bounds such as `ujuno` collateral, market caps, question limits, oracle address, the pinned verdict authority, and parameter ranges. It must never custody trader collateral or require a creator allowlist.
- **Binary market:** owns one market's collateral, positions, pool reserves, fee state, oracle binding, settlement state, and arbitration-control path. It is configured as its question's `cw-reality` arbitrator so that it can freeze the answer immediately after a valid bonded challenge. It may forward an arbitration verdict only when called by its exact immutable `verdict_authority`.
- **`cw-reality`:** owns question answering, bond escalation, finalization, and arbitration state. It never determines market payouts; it returns answer bytes.
- **Verdict authority:** is an immutable address selected in the deployment profile. V1 pins the active Juno Agents DAO core `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`. Only that core—not a member, proposal module, voting module, EOA, or other contract—may call the verdict entrypoint. The DAO's code, modules, membership, voting rules, and response timing are external trust assumptions even though the per-market address cannot rotate. `cw-reality` sees the market address as arbitrator. A future profile may pin Juno `x/gov`; #4/#13 are deferred and non-blocking for DAO-based v1.
- **Indexer/API/UI:** reconstructs trades, positions, candles, activity, and searchable metadata. It has no authority to create balances, choose answers, resolve early, or alter rules.

One contract per market is the accepted default because it makes fund isolation and incident containment easy to reason about. Deployment evidence must measure creation/storage cost before scaling, without reopening the implementation architecture decision.

### 5.2 Position and collateral model

The accepted v1 emulates the economically useful subset of conditional tokens inside one market contract:

1. **Split:** lock `x` units of collateral and create `x` YES plus `x` NO shares.
2. **Merge:** burn `x` YES plus `x` NO and release `x` units of collateral.
3. **Redeem after resolution:** burn a user's shares and pay according to the immutable payout vector.

For valid YES, the payout vector is `[1, 0]`; for valid NO, `[0, 1]`; for invalid, unknown, unresolved, or any non-canonical answer, `[1/2, 1/2]`.

FPMM trades are composed from those primitives:

- A buy takes collateral, separates the LP fee, splits the net amount into a complete set, adds it to pool inventory, and removes enough of the requested outcome to restore the product invariant.
- A sell adds the user's requested outcome to pool inventory, removes equal complete sets from pool inventory, merges them into collateral, separates the LP fee, and returns the quoted collateral.
- For binary reserves `R_yes` and `R_no`, the no-fee invariant is `R_yes * R_no = k`. The indicative YES marginal price is `R_no / (R_yes + R_no)`; fewer YES tokens in the pool means a higher YES price.

All formulas, operation ordering, and rounding rules require a separate mechanism memo with hand-worked examples before implementation. “Like Uniswap” is not a specification.

### 5.3 Market lifecycle

| State | Entry | Permitted financial actions | Exit |
| --- | --- | --- | --- |
| `Trading` | Market instantiated, initial liquidity funded, oracle question bound | Buy, sell, view quotes; add/remove liquidity only if the accepted LP policy allows it | First block time at or after `close_ts` |
| `AwaitingResolution` | `now >= close_ts` | No trading; no price-changing liquidity action; users may answer or counter-answer in `cw-reality`; once an answer exists and before it finalizes, a user may post the market's challenge bond to request arbitration | The oracle finalizes normally, or a challenge moves it to `PendingArbitration` |
| `PendingArbitration` | The market accepts a bonded challenge and forwards `RequestArbitration` | No trading; challenge bond remains a separately tracked liability; the pinned verdict authority may submit the verdict through the market | An accepted verdict permits resolution; cancellation after the deadline returns the oracle to its answer window |
| `Resolved` | Anyone successfully imports and validates the final answer once | Redeem positions and LP claims; read final payout | Terminal |

The market should derive closure from block time even if no one calls a “close” transaction. The first resolution call stores the payout vector permanently; later calls are idempotent or reject cleanly.

Required time ordering:

```text
creation_ts < close_ts <= oracle.opening_ts < expected answer finality
```

`close_ts` is the last trading boundary. `oracle.opening_ts` is when answers become admissible. The architecture must state whether these are equal or separated by a safety delay. The UI must display both trading close and expected resolution: the optimistic case is 24 hours after the latest answer, while counter-answers reset that clock and arbitration can extend it by at least the governance window.

### 5.4 Market question specification

The title alone is not the contract. Every market must bind an immutable resolution document containing at least:

- one objectively decidable YES/NO proposition;
- an absolute UTC cutoff and the observation period;
- the earliest answer time;
- primary resolution source(s) and precedence if sources disagree;
- definitions for named entities, measurements, inclusivity, rounding, revisions, cancellations, postponements, ties, and source outages;
- explicit invalid/unknown conditions;
- canonical YES, NO, invalid, and unresolved answer encodings;
- collateral denom, oracle address, market arbitration-controller address, immutable verdict authority, oracle bond floor, challenge bond, answer timeout, and arbitration timeout;
- a content hash stored alongside any human-readable text or URI.

Relative dates, subjective language, mutually compatible outcomes, and events directly manipulable by traders are invalid creation candidates. Omen's rules are useful prior art: they treat premature dates, subjective claims, non-exclusive outcomes, and markets that directly incentivize violence as invalid.

## 6. `cw-reality` compatibility gate

`cw-reality` is a substantial resolution foundation, but its source must be treated as canonical over surrounding prose. The following items were resolved by the accepted architecture and must now be implemented and verified.

| Topic | Repository evidence | Required disposition |
| --- | --- | --- |
| Canonical bool bytes | `AnswerType::Bool` is stored, but `SubmitAnswer` accepts opaque `Binary`; the tests do not establish a public YES/NO encoding standard. | Implement ADR-014's accepted encoding: exact 32-byte unsigned big-endian `0` and `1`. The market maps every other result to neutral payout; it must not parse loose strings. |
| Invalid versus unresolved | The source has Reality.eth's `UNRESOLVED_ANSWER` (`0xff…fe`) but no market-level invalid policy and no implemented reopen path. | v1 maps the invalid convention (`0xff…ff`), unresolved, and unknown bytes to accepted neutral finality `[1/2, 1/2]`. |
| Guarantee query | `FinalAnswerIfMatches` can enforce final bond, minimum answer timeout, arbitrator, and denom. | Use it at resolution, but also validate the full `Question` at creation because the guarantee query does not check text, opening time, answer type/schema, asker, arbitration timeout, or contract checksum. |
| Question creation and ID | The question ID binds the oracle contract address, asker, nonce, content hash, arbitrator, bond denom, initial bond, answer timeout, and opening time. It does **not** bind `answer_type`, `answer_schema`, or `arbitration_timeout_secs`. `AskQuestion` emits the ID but does not return it as response data or expose a prediction query. | Implement ADR-007's accepted atomic market-owned question creation with deterministic local ID derivation. Query and compare every omitted field. Do not scrape untyped event attributes as the sole binding. |
| Arbitration controller | `RequestArbitration` is callable only by the configured arbitrator, unlike a trader-paid public escalation flow. | Keep `cw-reality` unchanged and configure the binary-market contract itself as the question's arbitrator. This controller role is narrow: the market may request arbitration after a valid public challenge, but it cannot choose the verdict. `FinalAnswerIfMatches` must require the market address as arbitrator. |
| Optimistic finality | The production oracle permits a 24-hour answer timeout. Each later bonded answer restarts the timeout. | Configure the production 24-hour window. If nobody counter-answers or posts an arbitration challenge, the latest answer can finalize after roughly 24 hours. Never use the fast-demo oracle for production markets. |
| Challenge trigger | A market contract configured as arbitrator can forward `RequestArbitration` while the oracle is `OpenAnswered`; `cw-reality` does not itself collect a public arbitration-request bond. | Expose a permissionless market `Challenge` operation after an answer and before finalization. It must escrow the separate anti-griefing bond accepted in ADR-018, snapshot the challenged answer/current oracle bond and challenger, pass the current-bond front-run guard, and atomically forward `RequestArbitration`. Apply ADR-018's objective refund/slash rule; a free freeze is unacceptable. |
| Verdict execution | `SubmitArbitration` requires `PendingArbitration` and the configured-arbitrator sender. DAO DAO proposals execute wasm messages from the DAO core. Historical Cosmos SDK/Juno evidence also supports a future x/gov profile. | V1 authenticates the immutable Juno Agents DAO core and forwards `SubmitArbitration`, causing `cw-reality` to see the configured market sender. No other caller or market entrypoint may author or relay a verdict. Issue #45 requires exact-sender/failure tests and a non-broadcast DAO proposal packet. Live proposal execution requires separate authorization; #4/#13 preserve future x/gov evidence without blocking v1. |
| Controller mutability | `cw-reality` authenticates the arbitrator address, not the code currently running at that address. | Implement ADR-012's accepted non-migratable policy: funded market instances have no admin or migration path, so the arbitration-control path, pinned verdict-authority address, and live market code cannot become replaceable after question creation. DAO governance/upgrades remain a disclosed external trust risk. |
| Stalled arbitration | After `arbitration_deadline`, anyone may cancel arbitration, after which the answer window restarts and bond resolution may finalize. | Surface this in risk disclosures and calculate the maximum expected settlement delay. |
| Unanswered questions | A question with no answer never reaches `Finalized`. | Require a creation-funded oracle bounty and a keeper/answerer plan, with operational alerts before and after opening. Under ADR-013, v1 has no privileged emergency settlement path; unanswered questions remain disclosed nonterminal behavior. |
| Arbitrary arbitrator answer | The source and reconciled `ARBITRATION.md` state that the arbitrator may author any `Binary` and choose a validated payee, with no submitted-history membership proof. | The market recognizes only exact canonical bytes and maps everything else to neutral. This limits market-payout harm but cannot prevent redirection of the oracle bounty and bond winnings to the arbitrator-selected payee. |
| Denom support | Native and CW20 funding paths exist, but v1 `Withdraw` only sends native bank coins according to the self-audit notes. | Use native bank collateral and native oracle bonds for the first market version. Treat CW20 market collateral as out of scope until end-to-end withdrawal is supported and audited. |
| Answer filter | `answer_schema` delegates to `cw-filter`, but ordinary answer validity must not be the market's only safety boundary, and arbitrator answers remain trusted bytes. | Optionally use a filter for user feedback and spam reduction. The market's exact-byte payout mapping remains authoritative. |
| Deployed instance and mutability | The README lists production and fast-demo `juno-1` instances and code ID/checksum values. The 2026-07-15 live query confirms production address `juno1g0p…uceur`, code ID `5121`, checksum `e254…f3e2`, 0.1 JUNO minimum initial bond, and 24-hour timeout floor. It also shows a non-empty chain migration admin, `juno1mtz…xvzwd`; the same address appears in the contract's stored `Config.admin`. Pinning an address therefore does not pin immutable behavior. | Before deployment authorization, independently re-query chain state at a recorded height, code checksum, chain admin, config floors, and liveness. Implement ADR-012 with a fresh frozen oracle instance and treat any checksum/admin mismatch as a deployment blocker. Never configure production markets against the fast demo instance. |
| Documentation consistency | The contract README and `ARBITRATION.md` now consistently state that arbitration is an address permission and no adapter contract ships in v1. | Keep the compatibility memo, schema, and public contract documentation aligned with compiled source behavior as implementation proceeds. |

### 6.1 Verified discovery snapshot

The following observations were independently queried on 2026-07-15. They establish the research baseline, not deployment approval; the final memos must preserve raw responses and block-height evidence.

| Fact | Observed value | Architecture consequence |
| --- | --- | --- |
| Juno chain software | `juno-1`, application `v29.1.0`, Cosmos SDK `v0.50.13`, `wasmd v0.54.0` via [node info](https://rest.cosmos.directory/juno/cosmos/base/tendermint/v1beta1/node_info) | R4 must test against the actual chain generation and re-query it at sign-off. |
| Governance authority | `juno10d07y265gmmuvt4z0w9aw880jnsr700jvss730` via [module accounts](https://rest.cosmos.directory/juno/cosmos/auth/v1beta1/module_accounts) | This height-pinned address is the accepted ADR-017 input and must be refreshed before deployment; it is not a timeless constant. |
| Standard governance timing and deposit | 5,000 JUNO minimum deposit, ten-day maximum deposit period, five-day voting period via [governance parameters](https://rest.cosmos.directory/juno/cosmos/gov/v1/params/voting) | Arbitration needs a credible deposit sponsor and enough deadline margin. A technically callable verdict path is not operationally live without both. |
| Production oracle instance | Address `juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur`, code ID `5121`, non-empty admin via [contract info](https://rest.cosmos.directory/juno/cosmwasm/wasm/v1/contract/juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur) | Existing production deployment is not code-immutable and cannot be trusted by address alone. |
| Oracle checksum and instantiate permission | SHA-256 `e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2`; instantiate permission `Everybody` via [code info](https://rest.cosmos.directory/juno/cosmwasm/wasm/v1/code-info/5121) | A fresh frozen instance is technically possible, but source-to-byte reproducibility, audit, instance ownership, operations, and compatibility remain open. |
| Production oracle floors | 100,000 `ujuno` initial bond floor and 86,400-second answer-timeout floor via the live [`Config` query](https://rest.cosmos.directory/juno/cosmwasm/wasm/v1/contract/juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur/smart/eyJjb25maWciOnt9fQ==) | These are floors, not proof that a question is adequately secured for a market's value at risk. |

The [Cosmos SDK v0.50 governance specification](https://docs.cosmos.network/sdk/v0.50/build/modules/gov/README) confirms that accepted proposals execute registered messages signed by the governance module account. Proposals 357 and 363 add primary-chain precedent for generic Juno governance-originated wasm execution. Neither proves exact verdict encoding, market and oracle effects, gas/failure behavior, deposit sponsorship, or response time for a prediction-market dispute.

### 6.2 Optimistic and challenged settlement

```text
latest oracle answer
        |
        +-- no counter-answer or challenge --> 24 hours --> finalizable
        |
        +-- later bonded answer -------------> 24-hour clock restarts
        |
        +-- bonded market challenge ----------> arbitration freezes
                                                   |
                                                   +-- pinned authority verdict --> finalizable
                                                   |
                                                   +-- 21-day deadline --> public cancel
                                                                          --> answer clock restarts
```

The 24 hours starts at the latest answer, not necessarily at market close. An unanswered question still cannot finalize, so the oracle bounty and keeper path remain necessary.

### 6.3 Oracle security budget

Oracle correctness is economic security, not only address authentication. For each market tier, the architecture must relate:

- maximum collateral and expected profit from a corrupt answer;
- the question's initial and current bond;
- dispute monitoring time;
- pinned-authority governance security and response time;
- the cost of capturing or bribing the verdict authority;
- the effect of a publicly cancelled, stalled arbitration.

A permissionless market with arbitrary collateral and no value cap can outgrow its oracle security. The factory therefore supports market security tiers that bind an approved controller code/checksum and governance verdict authority, minimum oracle parameters, creation bounty, and a maximum collateral/open-interest cap. Implement the exact accepted canary ratios from the decision register; do not invent alternatives in code. New tiers or scaling require separate evidence and acceptance.

## 7. Financial invariants to specify and later test

These are architecture requirements. The mechanism memo may refine notation but may not weaken them without an accepted decision record.

1. **Complete-set conservation:** total YES supply equals total NO supply equals tracked locked principal after every split or merge, counting both user and AMM balances.
2. **Collateral coverage:** the market's actual bank balance is at least locked principal plus all accrued fee/refund liabilities. Forced transfers are excess funds, never newly recognized liabilities.
3. **Terminal conservation:** for payout vectors `[1,0]`, `[0,1]`, and `[1/2,1/2]`, aggregate position payouts equal tracked locked principal, subject only to explicitly assigned integer dust.
4. **Pool inventory:** both outcome reserves stay positive while trading is enabled; a trade cannot drain either reserve or make a denominator zero.
5. **Invariant direction:** without fees, quote execution preserves the selected FPMM invariant under the specified integer rounding; fees may only move value toward the LP fee account, never the trader.
6. **Adverse rounding:** quote and execution round against the caller consistently. Query and execute use the same math, and callers provide minimum output or maximum input plus a deadline.
7. **Fee conservation:** cumulative LP fee claims cannot exceed fees collected. Adding/removing liquidity cannot claim fees earned before the LP's participation.
8. **Principal separation:** market creation cost, oracle bounty, protocol fees, and LP fees never come out of collateral already backing user positions.
9. **Time boundary:** trades and price-changing liquidity operations fail when `block.time >= close_ts`, regardless of message ordering in the same block.
10. **Resolution binding:** only the pinned oracle address and exact question ID can set the one-time payout vector, after all required guarantees are rechecked.
11. **Idempotent redemption:** a position is burned or its balance reduced before transfer is queued; repeated, reentrant, batched, or partial claims cannot double-pay.
12. **LP solvency at resolution:** LP shares redeem only against the pool's remaining positions and fee entitlement; they have no senior claim over trader redemption collateral.
13. **Bounded arithmetic:** every multiplication, division, reserve product, fee accumulator, and share calculation has a documented numeric bound and overflow behavior.
14. **Dust ownership:** every division remainder has one immutable recipient rule. No caller can profit by splitting one operation into many smaller operations.
15. **Path independence where promised:** adding/removing liquidity or redeeming in batches produces the same final entitlements as the equivalent aggregate operation, apart from the documented dust rule.
16. **Challenge-bond separation:** challenge bonds remain separately tracked liabilities and never count as position collateral, pool reserves, LP fees, oracle bounty, or spendable surplus.
17. **Verdict authorization:** only the exact immutable `verdict_authority` may cause the market to forward `SubmitArbitration`; a challenger, creator, DAO member/module, admin, or arbitrary caller cannot select an answer or payee.

## 8. Threat model and operational risks

The architecture review must cover at least these adversaries and failures:

- a market creator who writes ambiguous rules, selects a spoofed controller or weak verdict authority, uses a spoofed denom, or links the wrong question;
- a trader who sandwiches, front-runs, back-runs, splits orders to exploit rounding, submits at the close boundary, or manipulates the last displayed price;
- an LP who adds just before fee realization, removes after learning private information, or attempts to withdraw collateral backing traders;
- an oracle participant who answers too early, submits non-canonical bytes, griefs through bond escalation, or relies on nobody monitoring the timeout;
- a challenger who cheaply freezes many markets, evades a challenge-bond loss, races finalization, or exploits ambiguous refund/slash criteria;
- captured, conflicted, unavailable, or slow governance;
- an admin who migrates a market or oracle to confiscatory code;
- a malicious or compromised frontend that changes displayed rules, quotes, slippage, timestamps, denom labels, or transaction messages;
- an indexer outage or reprocessing error;
- an IBC asset whose path, issuer, bridge, channel, liquidity, or transfer availability changes;
- chain halt, delayed blocks, RPC disagreement, time drift, transaction spam, or gas exhaustion;
- unsolicited bank transfers and rounding dust that make raw balance differ from internal accounting;
- spam markets, duplicate questions, impersonated events, unsafe incentives, illegal content, and markets whose outcome a participant can cheaply cause;
- low liquidity that produces a visually plausible “probability” from an economically trivial trade;
- a market whose value at risk exceeds the bond and governance security protecting its answer.

The permissionless-release operations design must include monitoring for market close, oracle opening, first answer, disputes, arbitration proposals, arbitration deadline, final answer, failed resolution calls, collateral imbalance, and anomalous reserve changes.

## 9. Research and architecture workstreams

### 9.1 Deliverable index

The phase should create the following reviewable documents and link their accepted revisions back here. Names may change during review, but scope may not disappear.

| Workstream | Planned artifact |
| --- | --- |
| R1 | [mechanism.md](docs/prediction-market/mechanism.md) |
| R2 | [prior-art.md](docs/prediction-market/prior-art.md) |
| R3 | [cw-reality-compatibility.md](docs/prediction-market/cw-reality-compatibility.md) and [question-specification.md](docs/prediction-market/question-specification.md) |
| R4 | [juno-and-topology.md](docs/prediction-market/juno-and-topology.md) |
| R5 | [product-legal-operations.md](docs/prediction-market/product-legal-operations.md) |
| A1 | [architecture.md](docs/prediction-market/architecture.md) |
| A2 | [security-and-economics.md](docs/prediction-market/security-and-economics.md) |
| A3 | [user-journeys.md](docs/prediction-market/user-journeys.md) |
| ADRs | [ADR index](docs/prediction-market/adrs/README.md), ADR-001 through ADR-018 |
| Phase review | [review-checklist.md](docs/prediction-market/review-checklist.md) with evidence for every section 13 gate |
| Evidence | [source baseline](docs/prediction-market/evidence/source-baseline.md), [Juno snapshot and governance precedent](docs/prediction-market/evidence/2026-07-15-juno.md), [exact raw archive](docs/prediction-market/evidence/raw/39830878/README.md), [oracle wasm attempt](docs/prediction-market/evidence/oracle-wasm-reproducibility.md), and [Osmosis liquidity/volatility](docs/prediction-market/evidence/2026-07-15-osmosis-juno-liquidity.md) |

### 9.2 Dependency-ordered phase plan

1. **Freeze the evidence baseline:** pin source commits and schemas; archive live Juno/oracle query results with height and time; reconcile source-versus-doc discrepancies.
2. **Resolve the load-bearing mechanisms:** complete R1–R4; decide trade math, LP lifecycle, question binding, oracle bytes, governance feasibility, challenge economics, topology, and migration posture before drawing a final architecture.
3. **Specify the complete system:** complete R5 and A1–A3 using the accepted mechanism decisions. Every execute path, permission, liability, state transition, and off-chain dependency receives an owner and failure behavior.
4. **Attack the specification:** run hand-worked conservation examples, parameter sensitivity tables, adversarial journeys, legal/content review, and requirement-to-test traceability. Revise decisions when evidence fails.
5. **Hold the phase review:** accept or explicitly defer every ADR, close every section 13 gate with direct evidence, record dissent and residual risks, and only then authorize a separate implementation plan.

The workflow may overlap research, but implementation must conform to the accepted packet and carry forward its residual evidence gates. ADR-009, ADR-010, ADR-012, ADR-013, ADR-017, and ADR-018 are accepted; ADR-017 was amended by issue #45 to use the Juno Agents DAO core for v1 while preserving deferred x/gov compatibility under #4/#13.

### R1 — Mechanism and market microstructure

**Questions**

- At realistic Juno liquidity, what price impact does a binary FPMM create for representative trade sizes?
- How do FPMM, LMSR, `pm-AMM`, parimutuel, CLOB, and hybrid routing compare on bootstrap capital, numerical complexity, LP loss, liveness, and user comprehension?
- What LP fee and initial liquidity are needed for useful quotes without implying LP profitability?
- Should liquidity be locked until resolution, removable only before close, or freely removable with a permanent minimum-liquidity floor?
- Can LP shares be priced and redeemed fairly after a strongly directional market?

**Deliverable:** mechanism decision memo with formulas, rounding table, hand-worked multi-trade examples, liquidity/price-impact tables, LP payoff examples for YES/NO/neutral outcomes, and a clear FPMM recommendation or rejection.

### R2 — Prior-art and incident review

**Questions**

- Which production contracts and audits best define split/merge, FPMM trades, payout vectors, and fee accounting?
- Which real incidents came from ambiguous questions, invalid outcomes, oracle timeouts, numerical approximation, migration, and resolution adapters?
- Which design choices were later removed or replaced, and why?

**Deliverable:** source-linked comparison matrix covering at least Gnosis/Omen, Polymarket, Augur, Manifold, Injective, Zeitgeist, Reality.eth, and `cw-reality`, with a “copy concept / adapt / reject” disposition for each relevant mechanism. License provenance is part of the matrix.

### R3 — Oracle integration and question policy

**Questions**

- What exact 32-byte values mean YES, NO, invalid, and unresolved?
- Who creates the question and how does the market learn its ID atomically?
- Which `Question` fields and contract metadata are pinned and revalidated?
- What challenge-bond amount and refund/slash rule deter free settlement freezes without pricing out legitimate challenges?
- How does a user construct and submit the DAO DAO proposal that supplies a verdict after the market has already requested arbitration, and how is exact DAO-core execution verified?
- What bounty and monitoring arrangement makes “nobody answered” unlikely?
- Is neutral finality sufficient, or is a bounded re-question mechanism required?
- Does a 21-day arbitration timeout provide adequate margin for the Juno Agents DAO under voting, execution, block-inclusion, and failed-proposal scenarios?

**Deliverable:** `cw-reality` compatibility memo, canonical question template, byte-encoding specification, DAO-authority feasibility decision, sequence diagrams for normal/disputed/stalled/unanswered flows, and a source/docs discrepancy list. Preserve Juno x/gov as a deferred compatible profile.

### R4 — Collateral, Juno, and contract topology

**Questions**

- How should `ujuno` amounts, six-decimal display values, minimum trades, liquidity, and oracle bonds be represented without unit ambiguity?
- How do JUNO volatility, liquidity, governance concentration, and its dual role as both collateral and governance power affect the market and oracle threat models?
- What are the current Juno CosmWasm version, transaction/gas limits, code upload/instantiate policy, and `Instantiate2` capabilities?
- What are the cost and indexing implications of one contract per market?
- Should live market instances be immutable, DAO-migratable with delay, or frozen after funding?

**Deliverable:** verified chain/collateral profile, topology decision, admin and migration matrix, market-versioning strategy, and deployment dependency checklist. Chain facts must be re-queried at sign-off rather than copied from stale docs.

### R5 — Product, legal, content, and operations

**Questions**

- What precisely does the experimental/play-money label promise when the collateral is transferable JUNO?
- With no operating entity and a global Internet audience, which responsibilities remain with contributors, frontend/indexer hosts, RPC providers, market creators, and users in their own jurisdictions?
- Which warnings, interface choices, and optional frontend controls are appropriate without pretending the permissionless contracts can enforce geography or identity?
- With permissionless creation, who may filter market discoverability in independent interfaces, and how are unsafe markets reported without granting settlement authority to an indexer?
- Which topics are prohibited because they are illegal, unsafe, manipulable, defamatory, private, or directly incentivize harm?

**Deliverable:** documented permissionless/no-entity product posture, contributor and frontend risk assessment (including independent legal advice where applicable), content/discoverability policy for reference interfaces, operations runbook, incident roles, and an explicit list of assumptions the protocol cannot enforce on-chain.

### A1 — Architecture specification

**Deliverable:** component boundaries, trust diagram, market state machine, storage/accounting model, execute/query/event surface at a conceptual level, time semantics, permissions, failure behavior, indexing contract, and upgrade strategy. This is documentation, not generated code.

### A2 — Economic and security specification

**Deliverable:** final financial invariants, threat model, economic-security tiers, parameter table with rationales and safe bounds, audit scope, and implementation-phase property-test plan.

### A3 — User journeys and acceptance cases

**Deliverable:** worked journeys for creator, trader, LP, resolver, answerer, disputer, arbitrator, and keeper across normal YES, normal NO, neutral/invalid, arbitration, stalled arbitration, unanswered question, chain halt, and failed indexer scenarios.

## 10. Architecture decisions that must be recorded

Each item receives an ADR with alternatives, evidence, decision, consequences, and revisit trigger.

| ADR | Decision | Accepted answer |
| --- | --- | --- |
| 001 | v1 market types | Binary fixed-expiry only |
| 002 | liquidity mechanism | FPMM |
| 003 | position representation | Internal balances; transferable token layer deferred |
| 004 | contract topology | Factory plus one contract per market |
| 005 | collateral policy | Native `ujuno` only initially |
| 006 | invalid/unrecognized result | 50/50 neutral payout |
| 007 | question ownership and ID binding | Market-owned atomic creation with deterministic local ID derivation and full-field reply verification |
| 008 | oracle guarantee tiers and market caps | Exact canary tier accepted in the decision packet §4 |
| 009 | LP entry/exit and minimum liquidity | One creator=LP; fixed initial supply locked through resolution |
| 010 | LP fee, protocol fee, and dust | 200-bps immutable LP fee; zero protocol fee; accepted caller-adverse/cumulative dust rules |
| 011 | factory permission and launch policy | Permissionless creation from first release; objective protocol bounds remain allowed |
| 012 | admin, migration, pause, and recovery | No admin, migration, pause, recovery, or sweep |
| 013 | unanswered and stalled-resolution policy | 1-JUNO bounty + keepers; unanswered disclosed as nonterminal; 21-day challenged stall timeout |
| 014 | canonical oracle answer bytes and question template | Exact 32-byte mapping and JCS `juno-pm-question/1` template |
| 015 | indexer and frontend trust contract | Off-chain convenience only; all financial facts independently queryable on-chain |
| 016 | product and Internet launch posture | Experimental/play-money intent, value-bearing JUNO, no operating entity, global permissionless contracts; participant-specific risks must be documented |
| 017 | Immutable verdict-authority arbitration | Use `cw-reality` unchanged; market is controller; Juno Agents DAO core is the v1 authority; future x/gov remains compatible and deferred. Issue #45 owns implementation. |
| 018 | Challenge-bond economics and accounting | One-shot `max(10 JUNO, current oracle bond)` escrow with accepted refund/slash path table |

## 11. Parameter register

The phase must recommend values and immutable bounds for the following. Empty values are intentional; they require evidence.

| Parameter | Purpose | Decision basis |
| --- | --- | --- |
| Minimum initial liquidity | Avoid unusable or drainable pools | Price impact at representative trade sizes |
| Maximum market collateral/open interest | Keep value at risk within oracle and first-release security | Oracle/arbitrator attack-cost analysis |
| LP fee and allowed range | Compensate inventory and informed-flow risk | FPMM payoff and volume scenarios |
| Minimum trade and liquidity increment | Bound rounding and spam | Collateral decimals, gas, dust analysis |
| Maximum trade relative to reserves | Protect quotes and prevent reserve exhaustion | Invariant and rounding analysis |
| `close_ts` lead time | Give traders a known window | Product policy and oracle source timing |
| Oracle opening delay | Prevent answer-before-event races | Event/source characteristics |
| Oracle initial bond floor | Deter frivolous or corrupt answers | Market security tier |
| Oracle bounty | Incentivize timely first answer | Expected answerer cost and keeper model |
| Answer timeout | Permit counter-answers while bounding optimistic finality | 24 hours, the production `cw-reality` minimum; resets after each later answer |
| Market challenge bond | Deter permissionless arbitration griefing | Value at risk, false-challenge cost, accessibility, refund/slash objectivity, and spam analysis in ADR-018 |
| Arbitration timeout | Bound the freeze after a challenge | 21-day v1 bound retained pending issue #45 implementation and DAO-specific operational validation |
| Arbitration controller and verdict authority | Separate technical sender from social authority | Market contract is the `cw-reality` arbitrator-controller; immutable Juno Agents DAO core is the v1 verdict authority; future x/gov is a later profile |
| Metadata/rules size limits | Bound storage and parsing | Juno gas/storage measurements |
| Maximum market duration | Bound locked LP capital and operational burden | LP policy and monitoring capability |
| Permissionless-release market and wallet caps | Contain first-release failures without choosing who may create | Audit maturity and operations capacity |

## 12. Explicit non-goals for v1 architecture

- Scalar, categorical, combinatorial, conditional, negative-risk, or linked markets.
- A central limit order book, off-chain signed-order relayer, RFQ system, or hybrid router.
- Leverage, borrowing, margin, liquidation, portfolio netting, or cross-margin.
- Short positions other than acquiring the complementary fully backed outcome.
- A new oracle token, juror court, Kleros clone, or standalone arbitrator-adapter contract; the narrow challenge and governance-verdict relay lives in each market instance.
- Cross-chain trading or redemption; collateral must already exist on Juno before entering a market.
- Arbitrary CW20 collateral.
- Yield-bearing or rehypothecated collateral.
- Governance that can rewrite a live market's question or payout.
- Guaranteed LP profitability or claims that market price is a calibrated forecast.
- A production UI, indexer, bot, keeper, or deployment during this phase.

## 13. Phase exit criteria

The checklist distinguishes the closed issue #2 decision gate from evidence and execution gates that remain required before deployment/readiness.

**Current authorization (2026-07-16): IMPLEMENTATION AUTHORIZED.** Jake Hartnell accepted the [issue #2 decision packet](docs/prediction-market/issue-2-decision-packet.md) and delegated architecture, economic-security, and license/provenance review to Juno AI. The [authorization record](docs/prediction-market/authorization.json) authorizes contract code, tests/models, SDK, frontend, indexer, and operations tooling. Deployment, fund movement, and mainnet governance-rehearsal transaction execution remain false. The policy permits removal of `blocked: decision`, but label changes remain a separate action.

- [x] R1–R5 and A1–A3 deliverables are reviewed and linked from this document.
- [x] ADRs 001–018 are accepted; residual risks and revisit triggers remain documented.
- [ ] The `cw-reality` compiled schema/source behavior and deployed production instance are independently verified.
- [x] Live Juno/oracle evidence is archived with chain ID, block height, block time, endpoints, raw responses, unit conversions, code ID/checksum, and all migration authorities; refresh remains required before deployment.
- [x] YES/NO/invalid/unresolved bytes and payout mapping are fixed in writing.
- [x] Normal, counter-answered, challenged, governance-resolved, stalled, unanswered, and neutral settlement sequences terminate or have a clearly disclosed non-termination condition.
- [x] Financial invariants balance in hand-worked examples for buys, sells, LP entry/exit, fees, rounding, all payout vectors, partial redemptions, and forced funds; executable verification remains implementation work.
- [x] The market cap/oracle security-tier relationship and exact canary parameters are accepted with documented residual risk.
- [x] Native `ujuno` denomination, six-decimal display convention, liquidity assumptions, and all JUNO amount conversions are accepted; current measurements remain deployment/scaling inputs.
- [x] Admin, migration, pause, factory, arbitration-controller, governance-verdict, and operations permissions are enumerated conceptually; deployed addresses remain a deployment gate.
- [x] Challenge-bond accounting and refund/slash behavior are specified and accepted for correct, incorrect, rejected, failed, stale, and timed-out arbitration paths.
- [ ] Issue #45 proves exact DAO-core authorization and failures in contract tests and produces a reviewable non-broadcast Juno Agents DAO proposal packet. Any live proposal/funded canary remains separately authorized; #4/#13 x/gov rehearsal is deferred and non-blocking.
- [x] The implementation test plan includes unit, property, multi-contract, adversarial, migration, gas, and on-chain rehearsal coverage derived from the threat model.
- [x] License strategy is approved: clean-room independent implementation from repository specifications/formulas under Apache-2.0; no copying/adapting LGPL source; notices/citations retained as provenance.
- [x] The experimental/play-money label, value-bearing JUNO risk, permissionless/no-entity launch scope, content/discoverability policy, and participant-specific legal-risk posture are documented; issue #26 legal/operational-readiness evidence remains open.
- [ ] A human reviewer can explain exactly how one unit of collateral moves from deposit to trade to each possible terminal payout without referring to unwritten assumptions.

## 14. Owner decisions recorded

Recorded 2026-07-15 and accepted with the complete packet on 2026-07-16:

1. **Economic mode:** experimental/play-money intent. Because the selected asset is transferable JUNO, all financial and adversarial analysis still treats it as value-bearing collateral.
2. **Collateral:** native JUNO initially; on-chain denomination `ujuno`.
3. **Creation:** permissionless from the first release. No creator allowlist is part of the target architecture.
4. **Arbitration:** each market uses `cw-reality` unchanged as its oracle and is configured as the question's narrow arbitrator-controller. An adequately bonded public challenge freezes the answer; only the market's immutable verdict-authority address may make it forward a verdict. V1 pins the active Juno Agents DAO core. Juno `x/gov` remains an eventual compatible profile under deferred #4/#13. Issue #45 owns the DAO implementation; live proposal execution or funds require a separate gate.
5. **Launch context:** no operating entity; a protocol for the Internet. The reference architecture will keep contracts globally permissionless and separately document the responsibilities and risks of contributors, interface/indexer hosts, market creators, and users.

## 15. Primary reading set

These sources anchor the initial synthesis; the research memos should cite exact versions, commits, audits, and access dates.

### Mechanism research

- Robin Hanson, [Logarithmic Market Scoring Rules for Modular Combinatorial Information Aggregation](https://hanson.gmu.edu/mktscore.pdf).
- Gnosis, [Conditional Tokens documentation](https://conditional-tokens-docs.netlify.app/) and [Fixed Product Market Maker source](https://github.com/gnosis/conditional-tokens-market-makers/blob/master/contracts/FixedProductMarketMaker.sol).
- Ciamac Moallemi and Dan Robinson, [pm-AMM: A Uniform AMM for Prediction Markets](https://www.paradigm.xyz/writing/pm-amm).
- Angeris and Chitra, [Improved Price Oracles: Constant Function Market Makers](https://arxiv.org/abs/2003.10001).

### Production and near-production systems

- Omen, [FAQ and FPMM overview](https://omen.eth.link/faq.pdf) and [invalid-market rules](https://omen.eth.link/rules.pdf).
- Polymarket, [markets and identifiers](https://docs.polymarket.com/concepts/markets-events), [Conditional Token Framework](https://docs.polymarket.com/trading/ctf/overview), and [resolution](https://docs.polymarket.com/concepts/resolution).
- Injective, [binary options market lifecycle](https://docs.injective.network/developers-native/injective/exchange/02_binary_options_markets).
- Manifold, [CFMM design discussion](https://manifoldmarkets.notion.site/6-CFMM-6e19db13b4c54d69ac7f9dda6f772bd1) and [multi-CPMM mechanism](https://manifoldmarkets.notion.site/Multi-CPMM-62fe5b99013c4d5a87dfa84e0b8fa642).
- Zeitgeist, [release history](https://github.com/zeitgeistpm/zeitgeist/releases) for CPMM, Rikiddo/AMM changes, hybrid routing, and numerical hotfixes.
- Augur, [v2 whitepaper](https://github.com/AugurProject/whitepaper/releases/latest/download/augur-whitepaper-v2.pdf) and [current reboot/fork material](https://www.augur.net/).

### Juno and local oracle foundation

- Juno Network, [developer documentation](https://docs.junonetwork.io/), [CosmWasm deployment](https://docs.junonetwork.io/developer-guides/cosmwasm-contracts/deploy-a-contract), and [TokenFactory/native asset rationale](https://docs.junonetwork.io/developer-guides/juno-modules/tokenfactory).
- Juno governance, [live governance parameters](https://rest.cosmos.directory/juno/cosmos/gov/v1/params/voting), [current network parameters](https://juno.valopers.com/parameters), [module accounts](https://rest.cosmos.directory/juno/cosmos/auth/v1beta1/module_accounts), and [Cosmos SDK proposal execution semantics](https://docs.cosmos.network/sdk/latest/modules/gov/README).
- Local canonical sources: `contracts/cw-reality/src/msg.rs`, `state.rs`, `query.rs`, `id.rs`, and `execute/*`.
- Local design and research: `ARBITRATION.md` and `docs/juno-reality/*`.

## 16. Final principle

`cw-reality` can tell a contract which answer finalized. It cannot make an ambiguous market clear, an under-collateralized market solvent, an empty market liquid, a slow governance process fit a short dispute window, or a value-bearing permissionless launch risk-free. This phase exists to specify those boundaries before code makes them expensive to change.
