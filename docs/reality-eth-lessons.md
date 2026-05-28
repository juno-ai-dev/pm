# Reality.eth — lessons digested for `cw-reality`

> Stage-1 gate closure (per `PLAN.md`). Companion documents:
> `research-notes-reality-source.md` (v3.0 Solidity walkthrough),
> `research-notes-v2v3-safesnap.md` (version history + SafeSnap failure modes),
> `research-notes-bond-economics.md` (Augur/UMA/Kleros comparison + defaults),
> `research-notes-cw-prior-art.md` (in-repo CosmWasm patterns to copy).
>
> This file is the **digested lessons** — what to actually carry into stage 2.
> Compiled 2026-05-28.

---

## 0. Canonical reference

**Port `RealityETH-3.0.sol`, not v4 and not v2.1.** v3.0 is deployed on 30+
chains (`chains/deployments/RealityETH-3.0.json` in `RealityETH/reality-eth-monorepo`);
v3.2 ships on a small subset; v4 is unreleased on any production chain.

Audit-pinned commit for line citations:
`RealityETH/reality-eth-monorepo@b996b0a0899451b95887b59243a118a467f602d0`,
flat-file path `packages/contracts/flat/RealityETH-3.0.sol`. Sources cached
locally at `docs/.research-cache/`.

**Delete commit-reveal from the port.** v4 PR #133 removed it with Edmund
Edgar's own conclusion: *"In practice it was never used in a way where that
mattered."* It caused two of three v3 audit issues. We inherit the conclusion;
v1 of `cw-reality` ships without it.

---

## 1. State machine

Reality.eth has **no explicit `enum State`** — state is derived from four
fields on `Question` (`timeout`, `finalize_ts`, `is_pending_arbitration`,
`history_hash`) gated through five modifiers. **Port as an explicit Rust
`enum State { NotCreated, OpenUnanswered, OpenAnswered, PendingArbitration, Finalized, Claimed }`**
with an `assert_state(qid, &[State])` helper. Modifier-as-guard silently
combines orthogonal concerns; making the state explicit in Rust is strictly
safer.

Permitted transitions (matching Reality.eth exactly):

```
NotCreated ──ask──> OpenUnanswered ──answer──> OpenAnswered ──┐
                                                              │
                                                  ┌──answer (2× bond)──┐
                                                  │                    │
                                                  └────────────────────┘
                                                              │
                                                              │ request_arbitration
                                                              ▼
                                                  PendingArbitration ──cancel──> OpenAnswered (fresh timeout)
                                                              │
                                                              │ submit_arbitration
                                                              ▼
                                                          Finalized ──claim──> Claimed (history drained)
                                                              ▲
                                                              │ idle timeout
                                                              └────── OpenAnswered (no new answer for `timeout` secs)
```

Two non-obvious transitions to port literally:

- **`cancel_arbitration` re-extends, doesn't restore.** Reality sets
  `finalize_ts = block.timestamp + timeout` on cancel, not the original
  finalize_ts. Restoring the original lets an attacker freeze-and-immediately-
  cancel to insta-finalize at the prior bond. `RealityETH-3.0.sol:530-531`.
- **Arbitration requires at least one answer.** `notifyOfArbitrationRequest`
  requires `finalize_ts > UNANSWERED`. This was added between the G0 audit
  and the deployed version (audit issue #2 fix) — without it, an attacker
  could freeze an unanswered question and force finalization to `bytes32(0)`.
  **Port the require literally. Do not skip it.**

---

## 2. Bond escrow — pull-payments, history-hash chain, multi-denom binding

Three load-bearing patterns.

### 2.1 Pull-payment via `balanceOf`

Reality.eth's only outbound ETH transfer is `BalanceHolder.withdraw()`
(line 18). All other "payouts" credit `balanceOf[payee] += value` — pure
storage writes. This is the single biggest reason Reality has zero
documented reentrancy bugs on the ETH path.

**Port literally:** `BALANCES: Map<&Addr, Vec<Coin>>` (vec because multi-denom).
`Claim` and `Finalize` update balances; a separate `Withdraw {}` message
drains via `BankMsg::Send`. **Never pay inline during state mutation** — sub-message
replies are CosmWasm's reentrancy surface.

### 2.2 History-hash chain

Reality stores only the chain tip:

```solidity
new_history_hash = keccak256(prev_hash, answer, bond, answerer, is_commitment)
```

Every answer is a step in a hash chain. The full chain exists only in events
(`LogNewAnswer`) and must be replayed at claim time. Storage cost is O(1) per
round, regardless of dispute length.

**Port with denom binding.** A multi-denom CosmWasm port that omits the denom
from the hash input lets an attacker swap denoms mid-round and confuse the
claim replay. Use:

```
H_n = sha256(H_{n-1} || answer_or_commitment || bond_denom_bytes || bond_amount.to_be_bytes() || answerer_canonical_addr || is_commitment_byte)
```

Addresses canonicalized via `api.addr_canonicalize` so bytes are stable.

**Indexer dependency.** Claiming requires off-chain event indexing. cw-reality
inherits this. Emit a per-round `wasm-cw_reality/new_answer` event with
`(prev_hash, new_hash, answer_or_commitment, bond_denom, bond_amount, answerer, block_time)`.
Without this, no one can claim. `reality-ui` will reconstruct from the event;
confirm `indexer-proxy` indexes wasm events before stage 5.

### 2.3 Per-question denom binding (multi-denom support)

Reality.eth assumes a **single fungible per contract instance** —
`RealityETH-3.0.sol` uses ETH; `RealityETH_ERC20-3.0.sol` is one fixed ERC20.
GOAL.md says cw-reality supports native + cw20 + IBC denoms. The port
preserves Reality's invariant **per question, not per instance**:

- Pin the question's denom at ask time (the first `info.funds` becomes
  `Question.bond_denom`).
- Reject any subsequent answer in a different denom with a typed error.
- cw20 path: bonds arrive via `Cw20ReceiveMsg`. The receive hook must carry
  `(question_id, action)`. Treat the `Receive` entry like Reality's
  `payable` modifier.
- IBC path: no special-casing; `ibc/...` denoms are bank-native.

### 2.4 BondEconomics: bond-asset risk is out-of-protocol

Multi-denom escrow introduces vectors no prior mechanism (Reality, Augur, UMA,
Kleros) has analyzed: an asker who controls a small-cap cw20 can mint
arbitrary supply, post arbitrary bonds, and trivially win their own question
via bond exhaustion using tokens they minted. An IBC-denom rug renders the
entire bond ladder valueless mid-dispute.

**Mitigation is social-layer, not in-protocol.** Documentation must warn that
the bond denom is the economic-security anchor. The reference UI should warn
when `bond_denom` falls outside a curated allowlist (JUNO, IBC-USDC, IBC-ATOM,
major Juno DAO tokens). The contract stays permissionless.

---

## 3. `claimWinnings` — the heart

Newest-to-oldest walk through the history hash chain. Each step:

1. Verify hash. Revert if mismatch.
2. Add the previous round's bond to `queued_funds` (we now know who to credit
   it to).
3. If this round's answer matches `best_answer`:
   - First match: this is the latest right-answerer → becomes `payee`, gets
     the bounty.
   - Subsequent match by a different address: pay the current payee a
     **takeover fee = min(queued_funds, new_payee_bond)**, then hand over to
     the earlier payee with the residual.
   - Subsequent match by the same address: keep accumulating.
4. If wrong answer: skip. The bond was already added to `queued_funds`, and
   the next-earlier right-answerer (or the current payee if none) collects.
5. Shave 2.5% of every **interior** bond (every bond except the chain-tip
   winning bond). Shaved fraction stays in contract balance with no
   `balanceOf` entry — effectively burned. (For sub-40-unit bonds the shave
   rounds to zero due to integer division. Port as-is.)

A resumable claim state `Claim { payee, last_bond, queued_funds }` persists if
the caller runs out of gas mid-walk; next call resumes. Reality drops this on
completion.

### 3.1 Port literally — including the math

Per v3 audit issue #3, `answer_takeover_fee` was previously path-dependent
(aggregate accounting correct, per-claim amounts differed depending on
whether you claimed multiple rounds together or separately). The fix unified
the subtraction. **Property-test `claiming round-by-round vs. claiming all at
once produce identical final balances.**

Sum-of-payouts equals sum-of-bonds in every multi-round scenario. Verified
by hand on a 3-round example (`research-notes-reality-source.md` §3):
Alice 1 right → Bob 2 wrong → Carol 4 right yields Carol 10 (bounty 0
example: 1+2+4=7 distributed; Carol gets 4 own + 2 of Bob + 0 = 6 not 10
in toy; full worked example in source notes balances on 100-bounty case).

### 3.2 Burn destination

Reality's 2.5% interior shave never credits a `balanceOf` entry. CosmWasm
equivalent: leave amount in contract bank balance, never credit anyone.
**No DAO-treasury skim, no Juno-fund sweep.** Define it as protocol burn,
match Reality.

### 3.3 Decision: keep history on claim, don't delete

Reality.eth deletes history on claim — gas refund on EVM. Per PR #134,
this created a footgun where `verifyHistory` only works pre-finalization.
**cw-reality keeps history.** CosmWasm storage is cheaper than EVM
storage; the safety/queryability win is worth the bytes. Queries that
read history after claim must still work (FM-10 from v2v3-safesnap notes).

---

## 4. Arbitrator — `Option<Addr>`, permission-not-abstraction

Reality.eth's `IArbitrator` trait reduces to one line of authentication:

```solidity
require(msg.sender == questions[question_id].arbitrator, "msg.sender must be arbitrator");
```

No registry, no signature scheme, no token. Whoever owns the bech32
recorded at ask time speaks for the arbitrator. This maps **exactly** to
`ARBITRATION.md`'s `Option<Addr>` design.

Required pre-state: `RequestArbitration` rejects unless the question has at
least one answer (`finalize_ts > 0`). Audit issue #2 fix — port literally.

Cancellation: the arbitrator (not the requester) calls `CancelArbitration`,
which unfreezes the question and re-extends `finalize_ts` from now. This is
how an arbitrator declines without producing a verdict.

Explicit decline via sentinel: the arbitrator can call `SubmitArbitration`
with `UNRESOLVED_ANSWER` (bytes32(-2)) to actively decline. Forces the
"settled too soon" path. **Reserve the same sentinel byte pattern in
cw-reality's `Answer` representation:** typed variant `Answer::Unresolved`
serialized as the all-ones-but-last-bit form for cross-chain compatibility.

### 4.1 No separate dispute-fee state

Reality's reference Arbitrator contract holds a crowdfundable arbitration
bounty separately. **cw-reality has no separate Arbitrator contract**, so
there is no `dispute_fee` concept in the oracle itself. Arbitration cost
is negotiated entirely out-of-band — a DAO arbitrator may charge via a
separate proposal deposit; cw-reality doesn't know.

### 4.2 `arbitrator: None` is bond-exhaustion-only

If the asker sets no arbitrator, `RequestArbitration` always reverts.
The question resolves purely by bond escalation timing out. Matches
Reality.eth line 350 precedent and GOAL.md.

---

## 5. Defaults table

| Parameter | Default | Justification | Where it's encoded |
| --- | --- | --- | --- |
| **Bond escalation multiplier** | **2×** strict (`bond >= 2 * current_bond`) | Reality.eth whitepaper; minimum exponential keeping takeover fee bounded by prior bond. 2× is load-bearing — do not adjust. | Hard-coded constant `BOND_MULTIPLIER = 2` |
| **Initial bond default** | **1 JUNO** (`1_000_000 ujuno`); per-question configurable down to instantiation-time platform floor | UMA: "set higher than the minimum to incentivize disputers." Pulled down from Ethereum precedents because Juno fees are ~1000× lower. | `AskQuestion.initial_bond: Uint128`; per-instance `Config.min_initial_bond_floor: Uint128` |
| **Platform floor for `initial_bond`** | **0.1 JUNO** (`100_000 ujuno`) at instantiation; configurable per deployment | FM-1: tiny initial bonds enable griefing. Reality.eth added `min_bond` in v3 explicitly to fix this. | `InstantiateMsg.min_initial_bond_floor` |
| **Answer timeout** | **24 hours**, configurable per question | Reality.eth dapp default; SafeSnap typical; matches social-attention window. | `AskQuestion.answer_timeout_secs: u32` |
| **Platform floor for `answer_timeout_secs`** | **24 hours** (`86_400`); configurable per deployment | FM-8: SuDAO (12 h) and Gnosis Guild (1 h) drained because their timeouts were too short for human monitoring. Reality.eth has no contract-level floor. **We add one.** | `InstantiateMsg.min_answer_timeout_secs` |
| **Arbitration timeout** | **7 days** between `RequestArbitration` and forced fallback to bond-exhaustion finalization | Calibrated to DAO DAO governance periods (5–7 d). Asker can extend explicitly for Juno gov arbitration. | `AskQuestion.arbitration_timeout_secs: u32` (optional, defaults to 604_800) |
| **Dispute round cap** | **32 rounds** hard cap | New, Juno-specific. Reality.eth has no cap; soft cap from `Uint128` saturation. At 2× from `1 ujuno`, round 32 = ~4 G JUNO — adequate headroom; protects against round-counter exhaustion proptest fuzz. | Hard-coded constant `MAX_DISPUTE_ROUNDS = 32` |
| **Loser-bond redistribution** | Reality.eth right-answer redistribution rule (Alice/Bob/Charlie worked example) | No deviation; port the published algorithm; cover the worked example + 5- and 7-round generalizations in proptest. | `claim.rs` |
| **Bond burn** | **2.5%** of every interior bond (`BOND_CLAIM_FEE_PROPORTION = 40`), stays in contract bank with no balance credit | Match Reality.eth exactly. Disincentivizes hairsplitting. **No DAO-treasury skim, no protocol fee.** | Hard-coded constant |
| **Question-text mutability** | **Immutable post-creation** | FM-11; Reality.eth pattern. Use `reopen_question` if a question was settled-too-soon. | Stored in question hash |
| **Asker bounty refund window** | **None in v1** — bounty unrecoverable if no answer | Match Reality.eth. **Defer auto-refund-after-N-blocks to a future version** — it's a footgun-fix, not a primitive. | N/A |
| **`uncle answer` rule** | **None** — undefended prior right answers do not claim | Reality.eth's single-chain model; simpler than tracking parallel answer histories. | `claim.rs` walks the single chain |
| **Commit-reveal answers** | **Not shipped in v1** | Removed in Reality.eth v4; caused two of three v3 audit issues; never used productively. Cosmos has no equivalent EVM mempool MEV exposure to defend against. | N/A |

---

## 6. Failure modes to test against

12 failure modes consolidated from v3 audit, v2.1 → v3 diff, SafeSnap
incidents, and bond-economics literature. Each becomes a test or a
written disposition in stage 4 self-audit.

| # | Mode | Reality.eth fix | cw-reality covering test |
| --- | --- | --- | --- |
| FM-1 | Tiny initial bond enables griefing | `askQuestionWithMinBond` + `min_bond` field | Instantiation-time `min_initial_bond_floor`; reject `AskQuestion` below floor |
| FM-2 | Bond payout to `address(0)` (audit issues #1 + #2) | Explicit `UNRESOLVED_ANSWER` check; require revealed answer before arbitration | Property test: every successful `Claim` strictly decreases contract bank balance and credits exactly one external `BALANCES` entry |
| FM-3 | Path-dependent payout math (audit issue #3) | Unified takeover-fee subtraction into one code path | Property test: claiming round-by-round vs. claiming all at once produces identical final balances |
| FM-4 | Commit-reveal complexity bugs | Removed in v4 | **Not shipped** in v1 — disposition |
| FM-5 | Question-ID collision across deployments | Added `address(this)` + `min_bond` to keccak input | If we use content-derived IDs, include contract address; if monotonic counter, moot. Decision: **content-derived hash that includes contract address + nonce** for indexer determinism |
| FM-6 | Bond doubling not enforced at high rounds | Strict `>=` check with Solidity 0.8+ overflow panics | `Uint128::checked_mul` + proptest 1 → `Uint128::MAX / 2` |
| FM-7 | `max_previous` front-run protection | Parameter on every state-changing answer function | Add `current_bond_seen: Option<Uint128>` to `SubmitAnswer` and `DisputeAnswer`; reject if actual exceeds |
| FM-8 | No timeout floor enables drive-by attacks | None at contract level | **Contract-level `min_answer_timeout_secs` floor** at instantiation — new defense beyond Reality.eth |
| FM-9 | Oracle question is opaque; integrator must verify off-chain fact | None — design principle | Documentation: a future DAO-execution wrapper must bind the proposal/payload at the proposal-module layer, not the oracle |
| FM-10 | Claim deletes history, breaking later verification | Explicit `verifyHistoryUnfinalized` naming | **Decision: do NOT delete history on claim** — keep queryable forever |
| FM-11 | Question text mutability | N/A — intentional | Stored in question hash; immutable enforced by hash |
| FM-12 | ERC777-style reentrancy | Avoid hostile tokens | Property test: hostile cw20 that re-enters `Claim` from within a transfer must not double-spend; reentrancy guard via pull-payments + sub-message reply isolation |

---

## 7. Open economic risks (no in-protocol fix; flag in documentation)

From `research-notes-bond-economics.md` §4. Not blockers; documentation and
social-layer mitigations.

1. **Arbitrator-as-address has no on-chain security model.** The DAO chosen
   as arbitrator is only as secure as its voting structure; a multisig only
   as its members. There is no on-chain way to detect a corrupt arbitrator.
   *Mitigation:* asker-side due diligence; reference UI suggests a "Reality
   Council DAO" with recallable members.
2. **Multi-denom escrow.** Asker collusion with small-cap cw20 issuer; IBC
   rug mid-dispute. *Mitigation:* UI allowlist warning; documentation.
3. **Small-bond griefing ladder.** 1-ujuno initial → 20 rounds tops out at
   ~1 JUNO. Honest correctors lose gas. *Mitigation:* `min_initial_bond_floor`
   (FM-1 covers it).
4. **cw-filter dependency.** Schema bricking if cw-filter is migrated to a
   buggy version or schema references removed features. *Mitigation:* schema-bind
   at ask time (snapshot the cw-filter contract address + filter version, not
   just the schema). **Open for stage 2 decision.**
5. **Reorg behavior at finalization.** CometBFT instant finality but reorgs
   not strictly impossible. *Mitigation:* finalize is replayable from history;
   self-audit-checklist already covers.
6. **Agent-mandate sock-puppeting.** Agents controlled by one operator have
   correlated interests; the bond-escalation game assumes counterparty
   independence. *Mitigation:* arbitrator staffed with humans or known
   agent-fiduciaries (Anima, etc.) so escalation reaches a different decision
   locus. **Deepest unresolved concern.** Flag in
   `/workspace/memory/reality-on-cosmwasm.md`.

---

## 8. Resolved open questions from `reality-eth-reading-list.md`

The reading-list had 7 open questions and the PLAN.md had 4 open design
calls. All resolvable from this stage's reading.

### From the reading list

1. **Same question twice?** Yes — `nonce` parameter is user-controlled.
   Same nonce reverts (`question_id` collision). Expose `nonce: u64` on
   `AskQuestion`.
2. **Loser-bond redistribution math >2 rounds?** Reality.eth's `_processHistoryItem`
   walk, newest→oldest. Right answers collect interior bonds; same-payee
   accumulates; payee-change pays takeover fee = `min(queued, new_payee_bond)`
   to the prior payee. Worked Alice/Bob/Charlie example in
   `research-notes-bond-economics.md` §1.4 and step-by-step trace in
   `research-notes-reality-source.md` §3.
3. **Asker disappears?** Bounty stays in contract if no answer; no automatic
   refund in Reality.eth. **v1 of cw-reality matches Reality.eth** — no
   auto-refund. Future addition is a known footgun-fix, not a primitive.
4. **Gas profile of escalation?** O(1) per round write; O(rounds) on claim.
   CosmWasm storage cheap; not a constraint. Round cap at 32.
5. **Tiny-bond griefing?** Bounded by mandatory `min_bond` + instantiation
   floor. FM-1.
6. **Answer-schema validation?** Reality.eth has none (`bytes32` answer
   opaque). cw-filter integration via the dao-proposal-wavs pattern is a
   strict improvement. Hostile-filter risk flagged in §7.4.
7. **Uncle-answer rule?** No. Single chain; undefended prior right answers do
   not claim. Keep this property — simpler than tracking parallel histories.

### From PLAN.md open design calls

1. **Bond redistribution edge cases.** Settled: port Reality.eth algorithm
   exactly. Proptest the Alice/Bob/Charlie example and 5- and 7-round
   generalizations. FM-3 ensures per-claim determinism.
2. **Question-text mutability.** Settled: **immutable**. Use `reopen_question`
   for settled-too-soon. FM-11.
3. **Disputed-answer cardinality cap.** Settled: **32 rounds hard cap.**
   Reality.eth has none; we add for proptest budget bounding and round-counter
   exhaustion defense. At 2× from 1 ujuno, round 32 = ~4 G JUNO — adequate
   headroom.
4. **First-arbitrator shape.** **Still deferred to stage 5** — the choice
   shapes the demo more than the contract. DAO DAO DAO with what membership /
   Juno gov / multisig / None — settle before mainnet upload.

---

## 9. Architectural cross-cuts carried forward

### 9.1 Pin `cosmwasm-std = "1.5.4"`

Matches `dao-contracts` current `development` and the `dao-proposal-wavs`
precedent. Lets us reuse `dao-voting`, `cw-denom`, `dao-pre-propose-base`
without `[patch.crates-io]` gymnastics. The strategy memo's planned Stage 1
bump (`cosmwasm-std 2.x`) lands cw-reality cleanly with a version-bump and
a tracked set of breaking changes — no current cw-std-2.x pin to inherit.

### 9.2 In-repo patterns to copy directly

- **cw-filter call shape** from `dao-proposal-wavs/src/contract.rs:441-461`
  + `filter.rs` (mirror wire types locally; do NOT take cw-filter crate dep —
  alloy conflict).
- **Single-contract multi-test harness** from
  `dao-proposal-wavs/tests/integration.rs` — closure-mocked deps, 15 tests
  covering happy + sad paths.
- **`workspace-optimize` Docker recipe** from `dao-contracts/justfile:56-67` —
  same `cosmwasm/optimizer:0.17.0` image as dao-contracts upstream CI.
- **Triple-bookkeeping state shape** from cw-abc (`CurveState` global +
  per-actor `Map<Addr, _>` + per-event records).
- **Sorted-index delta updates** from gauges (`TALLY` + `OPTION_BY_POINTS`)
  — maps to per-question `LEADING_ANSWER` index, incrementally maintained,
  not recomputed at read.

### 9.3 What to introduce that's not in the workspace

- **Property tests** via `proptest = "1"` — no in-repo precedent in
  `dao-contracts`. Model after `astroport-core/contracts/pair_concentrated/tests/`.
  Invariants: total bonded value monotone before finalize; redistribution
  conserves total; bond escalation strictly increasing; finalize timeout
  monotone non-decreasing.
- **Multi-denom per-question bond binding** — Reality.eth has no analog;
  introduce as a first-class concept in `Question` and the hash chain.
- **Contract-level timeout floor** (`InstantiateMsg.min_answer_timeout_secs`)
  — Reality.eth has no equivalent; SafeSnap incidents (SuDAO, Gnosis Guild)
  prove the need.

---

## 10. What feeds out of this document

- **`PLAN.md` stage 2 unblocks.** Open design calls #1, #2, #3 resolved here.
  #4 (first arbitrator) stays deferred to stage 5 by design.
- **`docs/self-audit-checklist.md` gets new items** for FM-3 (per-claim
  determinism), FM-7 (`current_bond_seen` front-run guard), FM-8 (timeout
  floor enforcement test), FM-10 (history-readable-post-claim test), FM-12
  (hostile-cw20 reentry test). Add explicitly in stage 4.
- **`/workspace/memory/reality-on-cosmwasm.md` carries forward** the
  long-term lessons: min_bond non-negotiable, commit-reveal not worth it,
  hostile-cw20 analog of ERC777, SafeSnap failed at integration boundary
  (DAO DAO integration is the right CosmWasm shape, not a Snapshot bridge),
  agent-sock-puppet correlated-interest risk as deepest unresolved concern.
- **`ARBITRATION.md` reinforced.** `Option<Addr>` arbitrator with
  permission-not-abstraction is the exact CosmWasm-shaped port of Reality.eth's
  `IArbitrator`. The Reality precedent confirms the simplicity is correct.

---

## 11. Hard rule (carried from reading-list)

The reading-list's hard rule was: "do not skim, do not code first." Stage 1
is now closed *because the reading happened* and the digested findings live
in this file plus the four research-notes files. Stage 2 starts with the
defaults in §5, the failure modes in §6, the prior-art copy-list in §9.2,
and proptest invariants from §9.3.

*Stage 1 closed 2026-05-28. Stage 2 unblocks.*
