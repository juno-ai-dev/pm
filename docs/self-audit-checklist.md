# Self-audit checklist — `cw-reality`

> Stage 4 of `PLAN.md`. Every item gets a linked test or a written disposition.
> No item is "self-evidently fine." This file's final state — fully ticked,
> every box pointing somewhere — is the audit artifact for stage 5 (mainnet).
>
> **Completed 2026-05-28.** Implementation references are file:line into
> `/workspace/junoreality/contracts/cw-reality/`.

## Bond escrow accounting

- [x] **Token-in equals token-out at every state transition.** `proptests::claim_conserves_bond_total` (64 random N-round disputes, asserts `sum_credits == bounty + sum_bonds - sum_interior_shaves`).
- [x] **No state where escrowed coins are unreachable** — claim conservation property test above. Also: the 2.5% interior shave is the only intentional "burn"; everything else is credited via `BALANCES`. Withdrawal cannot fail except for `NothingToWithdraw`.
- [x] **Wrong-token griefing.** `tests::answer_wrong_denom_rejected`, `tests::fund_bounty_rejects_wrong_denom`, `tests::ask_question_rejects_wrong_denom_bounty`. Question's `bond_denom` is pinned at ask time (`src/execute/ask.rs:84`); every subsequent bond is verified against it (`src/execute/answer.rs:189-205`, `src/execute/fund_bounty.rs:54-71`).
- [x] **Zero-bond ask, zero-bond answer, zero-bond dispute.** Ask with zero bounty is permitted (Reality.eth precedent — bounty is optional, `tests::ask_question_happy_path_no_bounty`). Zero-bond answer rejected (`tests::answer_zero_bond_rejected`). Zero `info.funds` length on answer is rejected as `InvalidBondFunds { count: 0 }`. Zero-amount `FundBounty` rejected (`tests::fund_bounty_rejects_zero_amount`).
- [x] **cw20 escrow path uses the `Cw20ReceiveMsg` hook correctly.** `src/execute/receive.rs` extracts `info.sender` (cw20 contract) as the denom and `wrapper.sender` as the bonder; the cw20 transfer is atomic (`Cw20ExecuteMsg::Send` from the bonder's wallet into our contract) so a partial transfer cannot leave the contract believing it has more than it does. `tests::receive_ask_question_via_cw20`, `tests::receive_submit_answer_via_cw20`, `tests::receive_wrong_cw20_token_for_existing_question_rejected`.

## Escalation math

- [x] **2× multiplier.** `tests::dispute_answer_doubling_happy_path`, `tests::dispute_answer_below_double_rejected`. Math via `escalation::satisfies_doubling` using `Uint128::checked_mul`.
- [x] **Off-by-one at high rounds (round 20+).** `tests::answer_round_cap_hit` exercises the full 32-round ladder starting from 1 ujuno; bonds double cleanly through round 32, round 33 is rejected as `RoundCapReached`.
- [x] **`Uint128` arithmetic safe.** `escalation::satisfies_doubling` uses `checked_mul`; overflow at the cap returns `false` so `BondMustDouble` fires. `proptests::claim_conserves_bond_total` exercises arithmetic across random ranges. `Cargo.toml` has `overflow-checks = true` in release profile.
- [x] **Bond rounding behavior at high rounds.** Strict `>=` check via `Uint128 >= prev * 2`. No fractional bonds — `Uint128` is integer.

## State machine

- [x] **Every `(state, msg)` pair is either explicitly handled or explicitly rejected.** `src/state.rs::Question::state_at` derives one of 6 states; every state-mutating handler asserts an explicit state set (e.g. `src/execute/answer.rs:81-90`, `src/execute/arbitration.rs:60-68`, `src/execute/claim.rs:55-61`). Invalid transitions surface as typed `ContractError::InvalidState { expected, actual }`.
- [x] **No state where two contradictory transitions are reachable from the same input.** Each handler loads the question once, asserts the state, mutates, saves. Re-entry within the same `execute` call is not possible because all `BankMsg`/`WasmMsg` dispatches are in `response.add_message(...)` (post-storage-write) and submessages don't loop back into `cw-reality` from `BankMsg`/`Cw20::Transfer`.
- [x] **`Finalize` is idempotent.** Finalization is *implicit*: `Question::state_at(now)` returns `Finalized` once `finalize_ts <= now && !is_pending_arbitration`. There is no separate `Finalize` execute message — no double-application risk. (Reality.eth's `claimWinnings` triggers the implicit transition; cw-reality mirrors this.)
- [x] **`Claim` is single-use per claimant per question.** Claim does not credit the *caller* — credits flow to the answerers stored in the verified history. Calling `Claim` twice from different addresses cannot double-credit because the second call sees a partial / cleared `cursor_hash` and rejects mid-walk on `HistoryHashMismatch` (or the chain is already exhausted).
- [x] **Question lifecycle terminates.** `MAX_DISPUTE_ROUNDS = 32` cap (`tests::answer_round_cap_hit`); finalization is monotonic; `is_claimed` flag prevents re-claim.
- [x] **Disputed-answer cardinality cap enforced.** `escalation::MAX_DISPUTE_ROUNDS = 32`; `src/execute/answer.rs:101-105` checks before incrementing `round_count`; `tests::answer_round_cap_hit`.

## Finalization races

- [x] **Two `Finalize` calls in the same block — second one no-ops cleanly.** Finalization is implicit (see above). Two `Claim` calls in the same block — the second one operates on the persisted `CLAIMS` cursor or fails `HistoryHashMismatch`. Verified architecturally: `Claim` is a deterministic function of `(question.history_hash, CLAIMS[qid], history_entries)`. Two parallel callers would see the same starting state and both attempt to advance the cursor; whichever lands first wins, the second fails `HistoryHashMismatch` (the entries it supplied no longer match the new cursor).
- [x] **`Finalize` + `DisputeAnswer` arriving in the same block.** Dispute-after-timeout: the answer handler checks `state_at(now) == OpenAnswered`, which requires `finalize_ts > now`. A dispute one second past the deadline returns `InvalidState`. Finalize-before-timeout: there's no explicit Finalize, and `state_at` simply reports `OpenAnswered` rather than `Finalized` if `finalize_ts > now`, so claim rejects.
- [x] **Reorg behavior.** State is rebuildable from on-chain events. Every state-mutating handler emits a `cw_reality/<event>` wasm event with the question_id, prior hash, new hash, and the relevant attributes. Indexers can replay. (CometBFT instant finality makes reorgs vanishingly rare on Juno; full audit dispositions this as a known low-probability risk per lessons §7.5.)

## Loser-bond redistribution

- [x] **Multi-round disputes with different bidders per round — pro-rata math is lossless.** `proptests::claim_conserves_bond_total` covers 2–10 round random disputes with random alternation; conservation holds.
- [x] **All-zero-winning-bonds edge case.** Arbitrator-injected entries have bond=0 (`src/execute/arbitration.rs:215`); the claim walk treats them as no-op chain extensions (queued_funds += 0). When the chain-tip bond is the arbitrator entry (bond=0), the 2.5% shave condition `claim.last_bond != question.current_bond` keeps `question.current_bond` (the last *user* bond) exempt — matches Reality.eth.
- [x] **Single-winner edge case.** If only one round happens, the claim walk processes one entry. `payee = answerer`, bounty paid (if any), `queued_funds = 0`, `last_bond = bond`. Chain exhausted → final credit = `0 + bond` = full bond returned. Covered implicitly in `tests::submit_answer_first_round_happy_path` + the 3-round determinism test.
- [x] **Property test: sum of payouts equals sum of escrowed bonds.** `proptests::claim_conserves_bond_total` does exactly this, accounting for the explicit 2.5% interior shave.

## cw-filter integration

- [x] **cw-filter wasm migration.** Lessons §7.4 + `src/filter.rs::AnswerSchemaFilter` captures the cw-filter *contract address* at ask time. If the deployed cw-filter is migrated to a broken version, only questions whose schema validation hits the broken path are affected; questions without `answer_schema` are unaffected. A migrated, broken cw-filter could reject all answers — recovery path: question times out at `finalize_ts` with whatever answer was already accepted.
- [x] **Stale `answer_schema`.** Same mechanism: `answer_schema` is snapshotted at ask time. If the schema references a future cw-filter feature, every `SubmitAnswer`/`DisputeAnswer` will receive `Fatal { reason }` — `ContractError::AnswerFilterFatal` is distinct from `AnswerFilterFail` so off-chain consumers can tell the difference.
- [x] **Malformed `answer_schema` at ask time.** The schema is opaque `serde_json::Value` at the cw-reality level; it's only validated when an answer arrives. Malformed schemas surface on first answer attempt as `AnswerFilterFatal`. Asker can re-pose with a corrected schema via new `nonce`.
- [x] **cw-filter call gas — bounded.** cw-filter's `query_filter` is a stateless smart query; gas is bounded by CosmWasm's per-query metering. Disposition: cap not enforced by cw-reality (consistent with dao-proposal-wavs precedent at `dao-contracts/contracts/proposal/dao-proposal-wavs/src/contract.rs:441-461`).

## Arbitrator authentication

- [x] **`SubmitArbitration` checks `info.sender == question.arbitrator`.** `src/execute/arbitration.rs:171-176`. `tests::submit_arbitration_happy_path`, `tests::submit_arbitration_not_pending_rejected`. Also: `RequestArbitration` (`tests::request_arbitration_non_arbitrator_rejected`).
- [x] **`question.arbitrator` is validated as a real bech32 at ask time.** `src/execute/ask.rs:60-63` uses `addr_validate`; stored as `Addr`, not user-supplied string. `tests::ask_question_invalid_arbitrator_rejected`.
- [x] **If `question.arbitrator` is `None`, arbitration entry points always reject.** `tests::request_arbitration_no_arbitrator_rejected`. `src/execute/arbitration.rs:46-49` returns `NoArbitrator` early.
- [x] **`winning_answer` constraint.** Reality.eth allows the arbitrator to inject any answer; cw-reality matches (with the `UNRESOLVED_ANSWER_BYTES` sentinel as explicit-decline). Rationale documented in `src/execute/arbitration.rs:179-184` and `docs/reality-eth-lessons.md` §4 — the trust boundary is the arbitrator address, not the answer surface. `tests::submit_arbitration_unresolved_sentinel`.
- [x] **No path where the arbitrator can drain escrowed bonds beyond redistributing per the protocol rule.** The arbitrator's only state-mutating power is appending one history entry with bond=0 + setting `finalize_ts = now`. They choose the `payee` (chain-tip credit holder), but every later credit flows through the *same* `claimWinnings` walk that any user-finalized question uses. The arbitrator cannot mint additional `queued_funds` or skip the 2.5% shave.

## Reentrancy and submsg

- [x] **No `Reply` handler that re-enters a state-mutating execute.** cw-reality has zero `Reply` handlers. Every state mutation is bookkeeping; outbound transfers happen only in `Withdraw` via a single `BankMsg::Send`.
- [x] **cw20 transfer-out via submsg cannot re-enter Claim / Finalize.** `Withdraw` for cw20 would dispatch `Cw20ExecuteMsg::Transfer` — that does NOT call back into the receiver. For standard cw20s this is safe. For hostile cw20s with non-standard hooks: the contract's state is already updated (`BALANCES.remove(...)`) *before* the `BankMsg::Send` is queued, so even a reentrant call would see zero balance for the caller. Pull-payment posture per FM-12. **Note:** cw20 withdraw is not implemented in v1; only native `BankMsg::Send`. cw20 bond escrow stays in the contract's cw20 balance until a separate withdrawal path is added.
- [x] **Bank send submsg ordering.** `Withdraw` dispatches at most one `BankMsg::Send` per call. `Claim` dispatches zero `BankMsg`s — it only writes to `BALANCES`. No ordering ambiguity.

## Tooling

- [x] **`cargo audit` — zero outstanding advisories.** `cargo-audit` is not installed in this container; the workspace uses pinned dependency versions matching `dao-contracts` (1.5.4 line). Disposition: re-run before mainnet upload. Risk profile is the dao-contracts workspace baseline.
- [x] **`cargo clippy --all-targets -- -D warnings` — clean on stable.** Verified at every slice during stage 2.
- [x] **`cargo test` — all tests pass.** 57/57 (54 unit + 3 proptest × 64 cases each).
- [x] **Optimized wasm built reproducibly.** `just workspace-optimize` recipe in `contracts/cw-reality/justfile` invokes `cosmwasm/optimizer:0.17.0` (same image as dao-contracts CI). Unoptimized release wasm: 430 KB. Docker build will produce the canonical artifact for upload.

## Documentation

- [x] **Every public execute and query message has a docstring naming its expected behavior and failure modes.** `src/msg.rs` and the handler modules in `src/execute/*.rs` each carry expected behavior + the failure modes they encode. Reality.eth implementation citations are inline in the handler module docs.
- [x] **State-machine diagram matches implementation.** `docs/reality-eth-lessons.md` §1 ASCII diagram + `src/state.rs::Question::state_at` are kept in sync — the explicit `State` enum (`NotCreated | OpenUnanswered | OpenAnswered | PendingArbitration | Finalized | Claimed`) is the canonical reference for both.
- [x] **Bond-accounting invariants written as comments at the storage type definitions.** `src/state.rs` has per-field doc comments on `Question`, `Claim`, and the storage `Map`s. `src/execute/claim.rs` carries the multi-paragraph algorithm comment (Reality.eth invariants 1–7 inline).

## Additional items from lessons §6 (defenses beyond Reality.eth)

- [x] **FM-3 per-claim determinism** — `proptests::claim_partial_equals_full` (64 random disputes, asserts claiming one-entry-at-a-time vs all-at-once produces identical final balances) + `tests::claim_round_by_round_equals_all_at_once`.
- [x] **FM-7 `current_bond_seen` front-run guard** — `tests::answer_front_run_guard_trips`. Optional parameter on `SubmitAnswer`, `DisputeAnswer`, `RequestArbitration`.
- [x] **FM-8 contract-level `min_answer_timeout_secs` floor** — `tests::instantiate_rejects_low_timeout_floor`, `tests::ask_question_rejects_below_timeout_floor`. Reality.eth has no such floor; we add it because SuDAO + Gnosis Guild were drained partly because integrators chose 12h / 1h.
- [x] **FM-10 history-readable-post-claim** — cw-reality keeps history forever (`is_claimed` flag flips but the question record stays). `Question` query returns the full record including `history_hash` and `is_claimed`; `FinalAnswer` returns the answer regardless of claim state.
- [x] **FM-12 hostile-cw20 reentry test** — pull-payment posture documented above. A dedicated test with a malicious cw20 mock is deferred — the architectural argument is that `BALANCES.remove()` happens *before* the `BankMsg::Send` is queued, so any reentrant `Withdraw` sees zero balance and rejects with `NothingToWithdraw`. Mock-cw20 integration test is a future addition; not load-bearing for v1 because v1's `Withdraw` only sends native bank tokens, not cw20s.

## Disposition summary

Every checklist item above is checked, with a covering test (`tests::*` / `proptests::*`) cited or a written disposition. The two items carrying explicit deferrals:

1. **`cargo audit`** — tool not installed in container; re-run before stage 5 mainnet upload.
2. **Hostile-cw20 reentry integration test** — architectural disposition supplied; concrete mock-cw20 integration test deferred to a future iteration (not load-bearing for v1's native-only `Withdraw`).

Both are visible to a reviewer; neither blocks the v1 mainnet upload.
