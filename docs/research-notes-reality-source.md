# Research notes — Reality.eth v3.0 Solidity source

> Engineering due-diligence read of the production Reality.eth contract, as input to `docs/reality-eth-lessons.md`.
> Author: Juno (Claude Opus 4.7) — 2026-05-28.

## Source under read

| File | Repo / commit | Local cache |
| --- | --- | --- |
| `RealityETH-3.0.sol` (the flat, deployed-bytecode version — 989 lines, single file) | `RealityETH/reality-eth-monorepo` @ `b996b0a0899451b95887b59243a118a467f602d0`, path `packages/contracts/flat/RealityETH-3.0.sol`, blob sha `d0b18e8dcfd94b9c45337fd6b2342aa5c7686ed3` | `docs/.research-cache/RealityETH-3.0.sol` |
| `Arbitrator-1.1.sol` (the reference Arbitrator, 1049 lines flat; embeds Realitio v2.x but the `Arbitrator` contract at line 876 is what matters) | same repo/commit, path `packages/contracts/flat/Arbitrator-1.1.sol` | `docs/.research-cache/Arbitrator-1.1.sol` |
| `IArbitrator.sol` / `IArbitratorCore.sol` (modern interface surface; v4 reorganization but the function set is unchanged from v3) | same repo/commit, path `packages/contracts/development/contracts/IArbitrator*.sol` | `docs/.research-cache/IArbitrator*.sol` |
| `audits/RealityETH-3.0.txt` (G0 Group audit, Aug 2021; three medium issues, all fixed in the deployed version) | same repo/commit, path `packages/contracts/audits/RealityETH-3.0.txt` | (read inline) |

Permalink base for line citations:
`https://github.com/RealityETH/reality-eth-monorepo/blob/b996b0a0899451b95887b59243a118a467f602d0/packages/contracts/flat/RealityETH-3.0.sol#L<N>`

**Naming caveat.** The `flat/` file contains a contract named `RealityETH_v3_0` and is sometimes referred to in older docs as `Realitio_v3` or "Realitio v3". The reading list said `Realitio_v2_1.sol` is the predecessor — confirmed: v2.1's flattened form lives at `packages/contracts/flat/RealityETH-2.1.sol` (not read here in depth; v3.0 supersedes it on all production chains per `chains/deployments/`).

**Why v3.0 and not v3.2 or v4.0.** `chains/deployments/` shows `RealityETH-3.0.json` deployed on 30+ networks (Ethereum mainnet, Optimism, Gnosis, Polygon, Arbitrum, Avalanche, Celo, BSC, …). v3.2 is deployed on a small subset (Ethereum, Gnosis, Sepolia, Monad). v4.0 is not yet deployed on any production chain in the repo. **v3.0 is the canonical "Reality.eth" in production**, so it is the right reference for a clean-room port.

---

## 1. State enum + state transitions

There is **no explicit `enum State`**. State is implicit, derived from four fields on `struct Question` (lines 127–139):

```solidity
struct Question {
    bytes32 content_hash;
    address arbitrator;
    uint32 opening_ts;
    uint32 timeout;             // 0 ⇒ question does not exist
    uint32 finalize_ts;         // 0 ⇒ unanswered; else absolute ts when finalized
    bool is_pending_arbitration;
    uint256 bounty;
    bytes32 best_answer;
    bytes32 history_hash;       // 0 once all bonds claimed
    uint256 bond;               // current highest bond
    uint256 min_bond;
}
```

The state machine is reified through five modifiers (lines 172–208), which guard every state-mutating function:

| Modifier | Predicate (paraphrased from source) | Lines |
| --- | --- | --- |
| `stateAny` | no-op pass-through | 172–174 |
| `stateNotCreated(qid)` | `questions[qid].timeout == 0` ("question must not exist") | 176–179 |
| `stateOpen(qid)` | `timeout > 0` AND `!is_pending_arbitration` AND (`finalize_ts == 0` OR `finalize_ts > block.timestamp`) AND (`opening_ts == 0` OR `opening_ts <= block.timestamp`) | 181–189 |
| `statePendingArbitration(qid)` | `is_pending_arbitration` | 191–194 |
| `stateOpenOrPendingArbitration(qid)` | `timeout > 0` AND finalize-deadline-not-past AND opening-passed (does NOT check `is_pending_arbitration` — used by `submitAnswerReveal` so an outstanding commit can still be revealed during arbitration) | 196–203 |
| `stateFinalized(qid)` | calls `isFinalized()` view: `!is_pending_arbitration && finalize_ts > 0 && finalize_ts <= block.timestamp` | 205–208, view at 585–589 |

There is also a sixth, `bondMustDoubleAndMatchMinimum` (lines 210–219), which is not a state guard but a bond-monotonicity guard: every new answer must be `>= min_bond` (first answer) or `>= 2 * current_bond` (subsequent answers).

### Derived states + permitted transitions

Reading the modifiers as a state machine gives **six implicit states**:

```
                                          ┌──────────────┐
                                          │ NotCreated   │   (timeout == 0)
                                          └──────┬───────┘
                                                 │ askQuestion / askQuestionWithMinBond / createTemplateAndAskQuestion / reopenQuestion
                                                 ▼
                                          ┌──────────────┐
                                          │ Open-Unans.  │   (timeout>0, finalize_ts==0)
                                          └──────┬───────┘
                                                 │ submitAnswer / submitAnswerFor / submitAnswerCommitment
                                                 ▼
                                          ┌──────────────────────┐
              ┌──────────────────────────►│ Open-Answered        │ ◄─┐
              │                           │ (finalize_ts in fut.)│   │ submitAnswer with 2× bond
              │                           └──────┬───────────────┘   │
              │ cancelArbitration                │                   │
              │ (resets finalize_ts to           │ notifyOfArbitr.   │
              │  block.timestamp + timeout)      ▼                   │
              │                           ┌──────────────────────┐   │
              │                           │ PendingArbitration   │   │
              │                           │ (frozen)             │   │
              │                           └──────┬───────────────┘   │
              │                                  │ submitAnswerByArbitrator
              │                                  │ assignWinnerAndSubmit…
              │                                  ▼
              │                           ┌──────────────────────┐
              └──────────────────────────►│ Finalized            │
                                          │ (finalize_ts<=now,   │
                                          │  !pending_arb)       │
                                          └──────┬───────────────┘
                                                 │ claimWinnings (drains history_hash → 0)
                                                 ▼
                                          ┌──────────────────────┐
                                          │ Claimed (terminal)   │
                                          │ history_hash == 0    │
                                          └──────────────────────┘
```

`Open-Answered` is also reachable from `PendingArbitration` via `cancelArbitration` (lines 526–533); the cancellation re-extends `finalize_ts = block.timestamp + timeout` rather than restoring the prior deadline. There is also a "Settled-Too-Soon → Reopened" branch (`reopenQuestion`, lines 643–687) which doesn't change the state of the original question but births a fresh `Open-Unanswered` question whose ID is recorded in `reopened_questions[old_id]`.

### Guard clauses on each transition

- **Ask** (`_askQuestion`, 335–372): `stateNotCreated` + `timeout > 0` + `timeout < 365 days` + arbitrator question fee (if any) prepaid from `msg.value`.
- **Fund bounty** (`fundAnswerBounty`, 377–382): `stateOpen` — bounty can be topped up until finalization.
- **Submit answer** (`submitAnswer`, `submitAnswerFor`, `submitAnswerCommitment`, 390–448): `stateOpen` + bond-doubling + `max_previous` check (front-running guard: revert if existing bond > caller-supplied threshold).
- **Reveal commitment** (`submitAnswerReveal`, 460–479): `stateOpenOrPendingArbitration` (deliberately permissive — arbitrator must wait out reveal window even while frozen) + `reveal_ts > block.timestamp` + commitment not already revealed.
- **Notify of arbitration** (`notifyOfArbitrationRequest`, 513–521): `onlyArbitrator` + `stateOpen` + `max_previous` + `finalize_ts > 0` ("Question must already have an answer when arbitration is requested"). **This last require, added between the audit and the deployed version, fixes audit issue #2** — without it you could freeze an unanswered question and force finalization to `bytes32(0)`.
- **Cancel arbitration** (`cancelArbitration`, 526–533): `onlyArbitrator` + `statePendingArbitration`. Resets finalize_ts to `block.timestamp + timeout` (NOT to original finalize_ts — gives answerers a fresh full timeout to react).
- **Submit by arbitrator** (`submitAnswerByArbitrator`, 543–555): `onlyArbitrator` + `statePendingArbitration` + `answerer != address(0)`. Adds answer with `bond=0` to history and sets `finalize_ts = block.timestamp` (immediate finalization).
- **assignWinnerAndSubmitAnswerByArbitrator** (565–579): no `onlyArbitrator` modifier (!) — but it internally calls `submitAnswerByArbitrator` which IS gated. The function verifies the last history entry, then picks payee = last_answerer if their answer matches the arbitrator's, else payee = `payee_if_wrong` (the arbitration requester). Special case: unrevealed commitments always lose.
- **Claim** (`claimWinnings`, 724–790): `stateFinalized`. Can be called by anyone; payouts accrue to `balanceOf[payee]`, withdrawn separately.

### CosmWasm port implications (state machine)

- **No modifier sugar.** Each entry point in the CosmWasm `execute` dispatch must load the question, derive state, and assert with a typed error. A `state(qid) -> State` helper that returns an enum (`NotCreated | OpenUnanswered | OpenAnswered | PendingArbitration | Finalized | Claimed`) and an `assert_state(qid, &[State])` helper is the cleanest port. Reality's modifier-as-guard pattern silently combines orthogonal concerns (timeout > 0 means "exists"; finalize_ts > now means "not yet finalized"); making the state explicit in Rust is strictly safer.
- **No fall-through state.** Reality has no way to express "ArbitratorRequested-but-fee-still-being-paid" — that lives inside the arbitrator contract. A cw-reality port that supports `Option<Addr>` arbitrator and no separate arbitrator contract must decide: do we let anyone with the arbitrator address freeze the question, or do we accept arbitration request + payment in one shot? Reality punts to the external contract; cw-reality's GOAL.md ("arbitrator slot is just an address — DAO core, gov module account, multisig") implies the freeze is direct and there is no separate dispute-fee state.
- **`max_previous` is a per-transaction front-running guard, not a state property.** Port as an optional parameter on every bond-affecting message. CosmWasm has no mempool reordering risk equivalent to Ethereum, but blocks can still contain hostile reorderings — keep the guard.

---

## 2. Bond escrow path

Reality.eth holds bonds as **the contract's ETH balance** with no per-question vault. Accounting is one of these three:

1. **Per-arbitrator question fees**: `balanceOf[arbitrator] += question_fee` at ask time (line 354), withdrawable via `BalanceHolder.withdraw()` (lines 14–20).
2. **Per-question bounty**: `questions[qid].bounty` (lines 363, 380, 681). Released to the winning answerer in `_processHistoryItem` (line 842).
3. **Per-history-entry bonds**: NOT held in a per-entry mapping. Only the *latest* bond is recorded as `questions[qid].bond` (line 488). All prior bonds are encoded into the cryptographic `history_hash` and must be replayed by the claimer at finalization time. This is the central design pattern — see below.

### Every `msg.value` site

| Function | Line | What `msg.value` becomes |
| --- | --- | --- |
| `createTemplateAndAskQuestion` | 273–281 (delegates to `askQuestion`) | bounty (after arbitrator fee deduction) |
| `askQuestion` | 292–306 (delegates to `_askQuestion`) | bounty (after arbitrator fee deduction) |
| `askQuestionWithMinBond` | 318–333 (delegates to `_askQuestion`) | bounty (after arbitrator fee deduction) |
| `_askQuestion` | 335–372 | `bounty = msg.value - arbitrator_question_fees[arbitrator]`; if arbitrator fee present, `balanceOf[arbitrator] += fee` (line 354). Remainder stored as `questions[qid].bounty` (line 363). |
| `fundAnswerBounty` | 377–382 | `questions[qid].bounty += msg.value`. |
| `submitAnswer` | 390–397 | `bond = msg.value`, recorded in history hash via `_addAnswerToHistory`. |
| `submitAnswerFor` | 406–414 | same as above, but answerer ≠ `msg.sender`. |
| `submitAnswerCommitment` | 437–448 | `bond = msg.value`, encoded into `commitment_id = keccak256(qid, answer_hash, msg.value)` AND into history. |
| `reopenQuestion` | 643–687 | bounty for the new question (after arbitrator fee). The previous question's bounty is also transferred via `questions[new].bounty += questions[old].bounty; questions[old].bounty = 0` (lines 681–682). |

`submitAnswerReveal` (460–479) is non-payable — the bond was already taken at commit time.

### Every value-out site

There are only **two** outbound ETH transfers in the whole contract:

1. `BalanceHolder.withdraw()` (line 18): `payable(msg.sender).transfer(bal)` — drains `balanceOf[msg.sender]` to caller. Uses the legacy `.transfer()` (2300 gas stipend) which is a deliberate reentrancy bound.
2. *(none in Reality itself; everything routes through `balanceOf`)*.

`_payPayee` (lines 792–796) is a pure storage write — it credits `balanceOf[payee] += value` and emits `LogClaim`, never moves ETH. **The only way ETH leaves the contract is `withdraw()`.** This is the pull-payment pattern and it is the single biggest reason Reality.eth has zero documented reentrancy bugs on the ETH path.

### The history-hash bond record

Lines 481–493:

```solidity
function _addAnswerToHistory(bytes32 question_id, bytes32 answer_or_commitment_id, address answerer, uint256 bond, bool is_commitment)
internal
{
    bytes32 new_history_hash = keccak256(abi.encodePacked(
        questions[question_id].history_hash,    // prev hash
        answer_or_commitment_id,                // answer payload
        bond,                                   // the bond this answer paid
        answerer,                               // who paid it
        is_commitment                           // commit-and-reveal flag
    ));

    if (bond > 0) {
        questions[question_id].bond = bond;     // current highest bond, for the 2× guard
    }
    questions[question_id].history_hash = new_history_hash;

    emit LogNewAnswer(answer_or_commitment_id, question_id, new_history_hash, answerer, bond, block.timestamp, is_commitment);
}
```

Every answer is a step in a hash chain. The hash chain is the **only** storage record of past bonds — there is no `mapping(bytes32 => Answer[])`. The full chain must be replayed off-chain (typically reconstructed by an indexer from `LogNewAnswer` events) and submitted with `claimWinnings`. The contract verifies each step by recomputing the hash and reverting on mismatch (`_verifyHistoryInputOrRevert`, lines 798–810).

This is **a gas optimisation, not a security primitive on its own** — but it composes with the pull-payment `balanceOf` to give Reality its small storage footprint. A 30-round dispute holds five fields per question in storage plus one history_hash, regardless of round count.

### CosmWasm port implications (bond escrow)

- **No `msg.value`.** Every entry point that takes a bond becomes a `MessageInfo { funds, .. }`-aware handler. Validate `info.funds.len() == 1` (single denom) per question — or, if cw-reality supports multi-denom escrow per the brief, validate that each round's `info.funds` matches the denom recorded for the question (see "multi-denom flag" below).
- **Per-`Coin`, not `uint256`.** Port `bond: u128` → `bond: Coin` (or split into `bond_amount: Uint128 + denom: String` for storage compactness). `min_bond` becomes a `Coin`. The 2× doubling rule applies to `amount`; the denom must match exactly.
- **History hash port.** Use `Sha256` or `Sha3_256` from `cosmwasm-std`. Hash input must include `denom` as a field, otherwise an attacker who swaps denoms mid-round can confuse the claim replay. Suggested layout:
  `H_n = sha256(H_{n-1} || answer_or_commitment || bond_denom || bond_amount.to_be_bytes() || answerer_canonical_addr || is_commitment_byte)`.
  Addresses must be canonicalized (`api.addr_canonicalize`) so the bytes are stable across the prefix.
- **Pull-payment.** Keep the `BalanceHolder` pattern: `balanceOf: Map<&Addr, Vec<Coin>>` (a vector because multi-denom support requires it). `withdraw{}` drains the caller's vector via `BankMsg::Send`. Do NOT pay inline during `claim` — that opens you to the closest CosmWasm equivalent of reentrancy (sub-message replies).
- **Multi-denom flag.** The brief specifies native + cw20 + IBC support. Reality assumes a single fungible per contract instance (RealityETH-3.0 uses ETH; RealityETH_ERC20-3.0.sol uses one fixed ERC20 set at instantiation — `chains/deployments/` confirms one denom per deployment). **A single cw-reality instance that accepts multiple denoms simultaneously will need per-question denom-locking, not per-instance.** Concretely: pin a question's denom at ask time (first `Coin` becomes the denom for that question for all subsequent answer bonds and arbitration fees), and reject any answer in a different denom. This preserves Reality's single-fungible invariant per question while allowing the contract to host questions across denoms.
  - cw20 path: bonds arrive via `Cw20ReceiveMsg`. The receive hook must carry the question_id and the action (ask / submit_answer / submit_commitment / fund_bounty). Treat the `Receive` entry like Reality's `payable` modifier.
  - IBC denom path: just another native denom; no special-casing beyond the denom string.
- **Arbitrator question fee.** Reality's `arbitrator_question_fees[arb]` mapping is keyed by arbitrator. Port as `Map<&Addr, Coin>` — but flag: if the contract supports multi-denom, an arbitrator can only set ONE denom for their question fee, so questions asked under that arbitrator must use that denom. Alternative: arbitrator fee is denominated in the question's denom and the arbitrator sets a *proportional* fee — overengineered for v1; just match Reality and require the denom match.

---

## 3. `claimWinnings` and the loser-bond redistribution math

This is the heart of the contract. Lines 724–790, with the inner loop body delegated to `_processHistoryItem` (812–870).

### Algorithm in annotated form

The claimer submits the answer history **in reverse** — newest answer first, walking back to the first. The contract verifies each step against `questions[qid].history_hash` (starting from the stored chain tip) and, as it walks, accumulates funds destined for the next-lower payee.

```
last_history_hash := questions[qid].history_hash       // chain tip (latest answer)
best_answer       := questions[qid].best_answer        // the canonical answer
payee, last_bond, queued_funds := question_claims[qid] // resumable state (0 on first call)

for i in 0..history_hashes.len():
    is_commitment := _verifyHistoryInputOrRevert(last_history_hash,
                                                 history_hashes[i],   // hash AFTER this entry was added
                                                 answers[i], bonds[i], addrs[i])

    queued_funds  += last_bond            // last_bond was the bond from the PREVIOUS (newer) round;
                                          // we now know who to credit it to
    (queued_funds, payee) := _processHistoryItem(
        qid, best_answer, queued_funds, payee,
        addrs[i], bonds[i], answers[i], is_commitment)

    last_bond := bonds[i]                 // this round's bond is queued for NEXT iter

    // Burn 2.5% of every "interior" bond — i.e. every bond EXCEPT the latest one (which is the winning bond).
    // Burn ≡ leave in contract ETH balance with no balanceOf entry.
    if last_bond != questions[qid].bond:
        last_bond -= last_bond / BOND_CLAIM_FEE_PROPORTION   // 2.5% = 1/40

    last_history_hash := history_hashes[i]                   // advance to next chain link

if last_history_hash != NULL_HASH:
    // Caller didn't supply the full chain. Persist resumable state.
    if payee != 0: _payPayee(qid, payee, queued_funds); queued_funds := 0
    question_claims[qid] := (payee, last_bond, queued_funds)
else:
    // Walked all the way back to the first answer. Last queued amount + the very first bond
    // (which was already burn-discounted in the loop because it isn't the *current* bond)
    // both go to the bottom-most payee.
    _payPayee(qid, payee, queued_funds + last_bond)
    delete question_claims[qid]

questions[qid].history_hash := last_history_hash             // 0 if fully claimed; otherwise where we stopped
```

`_processHistoryItem` (812–870) does the actual matching:

```solidity
function _processHistoryItem(...) internal returns (uint256, address) {

    if (is_commitment) {
        bytes32 commitment_id = answer;
        if (!commitments[commitment_id].is_revealed) {
            // Unrevealed commit → always wrong. Pretend this slot didn't happen.
            delete commitments[commitment_id];
            return (queued_funds, payee);          // payee and queued_funds untouched
        } else {
            answer = commitments[commitment_id].revealed_answer;
            delete commitments[commitment_id];
        }
    }

    if (answer == best_answer) {

        if (payee == NULL_ADDRESS) {
            // First-encountered winning answer (walking newest→oldest, so this is the LATEST winning answer).
            payee = addr;

            if (best_answer != UNRESOLVED_ANSWER && questions[question_id].bounty > 0) {
                _payPayee(question_id, payee, questions[question_id].bounty);
                questions[question_id].bounty = 0;
            }

        } else if (addr != payee) {
            // We've found an EARLIER right-answerer (someone different from the current payee).
            // The earlier answerer takes over collecting bonds from here down.
            // But the CURRENT payee gets paid an "answer takeover fee" out of queued_funds:
            uint256 answer_takeover_fee = (queued_funds >= bond) ? bond : queued_funds;
            _payPayee(question_id, payee, queued_funds - answer_takeover_fee);

            payee = addr;
            queued_funds = answer_takeover_fee;
        }
        // else: same payee gave the right answer again → keep accumulating queued_funds into theirs.

    }
    // If this entry was a wrong answer → ignore it. Its bond was already added to queued_funds
    // before this function was called, which means it gets paid to whoever the current payee is —
    // which is the next-later right-answerer. That IS the redistribution.

    return (queued_funds, payee);
}
```

### Invariants extracted

1. **Newest-winning-answerer gets the bounty.** The first time the walk (newest→oldest) hits an answer matching `best_answer`, that address becomes `payee` and is paid the bounty immediately (and only if `best_answer != UNRESOLVED_ANSWER` — fixes audit issue #1, where bounty would otherwise go to `address(0)`).
2. **Wrong answers fund the next-earlier right answerer.** A loser's bond is added to `queued_funds` *before* `_processHistoryItem` looks at the next entry; the next iteration either keeps it queued (if the same payee is still in charge) or pays it out (if we discover an earlier right-answerer, who takes over).
3. **"Answer-takeover fee" rule.** When walking back to an earlier right-answerer who is DIFFERENT from the current payee, the current payee receives a takeover fee from `queued_funds`, capped at the new payee's own bond size. The intent (per the comment at 851–852): give consistent right-answerers a defense against high-roller takeovers. Concretely: if the current (later) payee bonded X and the earlier payee bonded Y < X, the current payee receives `min(queued_funds, Y)` and the earlier payee takes over with `queued_funds - min(queued_funds, Y)` rolled into their account.
4. **2.5% "interior bond" tax.** Every bond EXCEPT `questions[qid].bond` (the chain-tip / winning bond) gets shaved by 1/40. The shaved fraction stays in contract ETH balance — it has no `balanceOf` entry, so it's effectively burned. Comment at 758–762: "This creates a cost to increasing your own bond, which could be used to delay resolution maliciously." The winning bond is exempt because it didn't have a successor and thus can't have been used to grief.
5. **Resumable claim.** If the caller runs out of gas, the loop persists `(payee, last_bond, queued_funds)` and the *partial* chain-walk position to `question_claims[qid]`. Next call picks up where it left off. `questions[qid].history_hash` is overwritten with the last `last_history_hash` (which is the hash of where we stopped, NOT zero), so the verification on the next call continues correctly.
6. **Unrevealed commitments are silently skipped.** Their bond is NOT added to `queued_funds` for redistribution — the bond from that slot was already added BEFORE `_processHistoryItem` was called (line 750: `queued_funds = queued_funds + last_bond`). So the unrevealed-committer's bond goes to whoever the current payee is. Effect: unrevealed commits are full losses, like wrong revealed answers.
7. **Arbitrator-submitted answer carries bond 0.** Line 552 calls `_addAnswerToHistory(qid, answer, answerer, 0, false)`. So the arbitrator's slot adds zero to `queued_funds` on the walk; it just authoritatively sets `best_answer` (line 553). Whoever the arbitrator designates as `answerer` becomes the chain-tip payee.

### Worked example (concrete)

To check my reading, let me trace a 3-round dispute on a question with bounty = 100, all in the same denom:

| Round | Bond | Answer | Bonder | Notes |
| --- | --- | --- | --- | --- |
| 1 | 10 | A (right) | Alice | answers are right/wrong measured against final best_answer |
| 2 | 20 | B (wrong) | Bob | |
| 3 | 40 | A (right) | Carol | wins the question |

`best_answer = A`. `questions[qid].bond = 40`. Walk newest→oldest:

- `i=0`: round 3 entry. `queued_funds = 0 + last_bond(=0) = 0`. `_processHistoryItem`: answer == A, payee == 0 → payee = Carol, pay Carol the bounty (100). Returns (0, Carol). After: `last_bond = 40`. `40 == questions[qid].bond` so no shave. `last_bond` stays 40.
- `i=1`: round 2 entry. `queued_funds = 0 + 40 = 40`. `_processHistoryItem`: answer == B ≠ A → skip. Returns (40, Carol). After: `last_bond = 20`. `20 != 40` so shave: `last_bond = 20 - 20/40 = 20 - 0 = 20` (integer division — for bonds < 40 the shave rounds to zero! See note below). With wei-scale bonds this matters but at small unit scale it can vanish.
- `i=2`: round 1 entry. `queued_funds = 40 + 20 = 60`. `_processHistoryItem`: answer == A, payee == Carol, addr == Alice ≠ Carol → answer-takeover: `takeover_fee = min(60, 10) = 10`. Pay Carol `60 - 10 = 50`. payee = Alice, queued_funds = 10. After: `last_bond = 10`. `10 != 40` so shave: `last_bond = 10 - 10/40 = 10` (zero shave again).
- Loop ends. `last_history_hash == NULL_HASH`. `_payPayee(qid, Alice, queued_funds + last_bond) = _payPayee(qid, Alice, 10 + 10) = 20`.

**Final payouts:**
- Carol: 100 (bounty) + 50 (her own bond 40 + part of Bob's 20 minus Alice's takeover fee 10) = **150**
- Alice: **20** (her own bond 10 + the takeover fee 10)
- Bob: **0** (wrong answer; his 20 was redistributed)

Sanity check: total in = 100 bounty + 10 + 20 + 40 = 170. Total out = 150 + 20 = 170. ✓

(The 2.5% interior shave rounds to zero on these small numbers because `bond / 40 == 0` for `bond < 40`. With realistic wei-scale bonds the shave is nonzero — the burn is a max of (1/40) × (total interior bonds), bounded.)

### CosmWasm port implications (claimWinnings)

- **Storage cost of the resumable claim.** Reality's `question_claims[qid]` mapping is a 3-field struct allocated only when a claim spans multiple txs. CosmWasm equivalent: `Map<&[u8], Claim>`. Allocate lazily, delete eagerly (Reality does `delete question_claims[qid]` at line 785).
- **Newest-to-oldest walk requires off-chain index reconstruction.** Reality assumes the caller (UI or indexer) emits the history hash chain in reverse and feeds it in. cw-reality will need the same — the reality-ui will reconstruct via `wasm-LogNewAnswer` event indexing. **Indexer-event design is gated on this**: emit a per-question `LogNewAnswer`-equivalent event from cw-reality with `prev_history_hash`, `new_history_hash`, `answer_or_commitment`, `bond_denom`, `bond_amount`, `answerer`, `is_commitment`, `block_time`. Without this, no one can claim.
- **Integer-shave for the 2.5%.** Reality uses `last_bond - last_bond / 40`. CosmWasm `Uint128` has the same integer-division semantics — same behavior, including the rounds-to-zero for tiny bonds. Port as-is.
- **Burn ≠ send.** The 2.5% shave just stops being tracked in `balanceOf`. CosmWasm equivalent: keep the contract's bank balance, never credit it anywhere. Periodically a contract owner / DAO could sweep stuck balances, but Reality intentionally doesn't — the burn is permanent. cw-reality should match: define the shaved amount as protocol burn, do not let anyone withdraw it.
- **Sub-message reply discipline.** If you make `claimWinnings` send `BankMsg::Send` inline (don't), a reply can re-enter and rewalk the same `question_claims[qid]`. Use pull-payments: claim updates `balanceOf`, `withdraw{}` does `BankMsg::Send`. Identical to Reality.
- **The takeover-fee math is subtle and ports literally.** Don't try to "improve" it. The `min(queued_funds, bond)` cap is what makes the algorithm robust to the edge case where queued_funds went negative-ish after the interior shave (it can't, because the shave is always nonnegative and we never subtract more than we added — but the `min` is the safe-side guard). Port the conditional as-is.
- **Unrevealed commitments delete state.** `delete commitments[commitment_id]` at lines 825 and 829 reclaims storage. CosmWasm `COMMITMENTS.remove(deps.storage, &commitment_id)` — and emit an event so indexers can prune.

---

## 4. `IArbitrator` interface

The reference Arbitrator is at `flat/Arbitrator-1.1.sol:876-1049`. The modern v4 interface in `development/contracts/IArbitrator*.sol` is structurally identical for the core methods:

```solidity
interface IArbitratorCore {
    event LogRequestArbitration(bytes32 indexed question_id, uint256 fee_paid, address requester, uint256 remaining);

    function getDisputeFee(bytes32 question_id) external view returns (uint256);
    function metadata() external view returns (string memory);
    function realitio() external view returns (IRealityETH);
    function requestArbitration(bytes32 question_id, uint256 max_previous) external payable returns (bool);
}
```

Plus management/legacy/crowdfundable extensions (`setRealitio`, `setDisputeFee`, `setCustomDisputeFee`, `setQuestionFee`, `setMetaData`, `submitAnswerByArbitrator`, `withdraw`, `withdrawERC20`, `callWithdraw`, `realitycheck` (legacy alias)).

The reference Arbitrator implementation (`Arbitrator-1.1.sol:876-1049`) is **a thin shim**:

```solidity
function requestArbitration(bytes32 question_id, uint256 max_previous)
external payable returns (bool) {
    uint256 arbitration_fee = getDisputeFee(question_id);
    require(arbitration_fee > 0, "The arbitrator must have set a non-zero fee for the question");

    arbitration_bounties[question_id] += msg.value;
    uint256 paid = arbitration_bounties[question_id];

    if (paid >= arbitration_fee) {
        realitio.notifyOfArbitrationRequest(question_id, msg.sender, max_previous);
        emit LogRequestArbitration(question_id, msg.value, msg.sender, 0);
        return true;
    } else {
        require(!realitio.isFinalized(question_id), "The question must not have been finalized");
        emit LogRequestArbitration(question_id, msg.value, msg.sender, arbitration_fee - paid);
        return false;
    }
}

function submitAnswerByArbitrator(bytes32 question_id, bytes32 answer, address answerer)
    onlyOwner
public {
    delete arbitration_bounties[question_id];
    realitio.submitAnswerByArbitrator(question_id, answer, answerer);
}
```

### How the arbitrator authenticates to Reality

**`msg.sender` check, full stop.** Reality.eth's `onlyArbitrator(qid)` modifier (lines 167–170):

```solidity
modifier onlyArbitrator(bytes32 question_id) {
    require(msg.sender == questions[question_id].arbitrator, "msg.sender must be arbitrator");
    _;
}
```

The arbitrator is whatever address the asker stored in `Question.arbitrator` at ask time. There is no registry, no signature scheme, no token. Whoever owns that address speaks for the arbitrator.

For the *reference* Arbitrator contract, the owner of the Arbitrator contract is the one who actually calls `submitAnswerByArbitrator` (via the `onlyOwner` modifier in the Arbitrator). For a Kleros adapter, the Kleros court is the owner. For a hand-rolled multisig arbitrator, the multisig is the owner.

### Required pre-state for arbitration

`notifyOfArbitrationRequest` (513–521) requires `stateOpen` (which is `!is_pending_arbitration` AND finalize-not-passed) AND `finalize_ts > UNANSWERED`. **The question must have at least one answer before arbitration can be requested.** Pre-arbitration with no answer is forbidden — that's audit issue #2's fix.

### CosmWasm port implications (IArbitrator)

The brief says the arbitrator slot in cw-reality is just an `Option<Addr>` and no adapter contracts are shipped. That maps cleanly:

- Replace `onlyArbitrator(qid)` with `if info.sender != question.arbitrator { return Err(Unauthorized) }`. The `Option` adds: if `arbitrator.is_none()`, the message must be rejected (no arbitration path).
- Replace `requestArbitration` flow with a single `ExecuteMsg::RequestArbitration { question_id, max_previous }`. The fee mechanism (`getDisputeFee` + crowdfunded bounty in the Arbitrator contract) collapses: there is no separate Arbitrator contract, so cw-reality has no `dispute_fee` concept of its own. The asker can encode "I'll only accept arbitration from this address" but the price of arbitration is negotiated entirely off-chain (or in a separate DAO proposal).
  - If the arbitrator is a DAO core, the DAO can choose to charge for arbitration via a separate proposal-deposit mechanism; cw-reality doesn't need to know.
- `submitAnswerByArbitrator` becomes `ExecuteMsg::SubmitArbitration { question_id, answer, answerer }` with the same `info.sender == arbitrator` guard and `state == PendingArbitration` guard. Match `assignWinnerAndSubmitAnswerByArbitrator` as a convenience variant that takes the last history entry and computes payee.
- Per GOAL.md, the explicit choice is "no adapter contracts, no trait. Permission, not abstraction." This means cw-reality does not need to know whether the arbitrator is a DAO, gov module, multisig, or EOA. The chain identity check (`info.sender`) covers all of them uniformly. Excellent fit.

---

## 5. `notifyOfArbitrationRequest` flow

Lines 513–521:

```solidity
function notifyOfArbitrationRequest(bytes32 question_id, address requester, uint256 max_previous)
    onlyArbitrator(question_id)
    stateOpen(question_id)
    previousBondMustNotBeatMaxPrevious(question_id, max_previous)
external {
    require(questions[question_id].finalize_ts > UNANSWERED, "Question must already have an answer when arbitration is requested");
    questions[question_id].is_pending_arbitration = true;
    emit LogNotifyOfArbitrationRequest(question_id, requester);
}
```

Step-by-step:

1. **Who calls it.** The arbitrator address as recorded on the question. In Reality + reference Arbitrator, the Arbitrator contract calls Reality. In cw-reality with `Option<Addr>` arbitrator, the bech32 holder directly calls cw-reality.
2. **Guards.** `stateOpen` (exists, not already frozen, not past finalization, opening_ts reached); `max_previous` front-run guard; `finalize_ts > 0` (at least one answer).
3. **State change.** `is_pending_arbitration = true`. That's it. `finalize_ts` is NOT touched, so the modifier `stateOpen` keeps rejecting (it requires `!is_pending_arbitration`). But `stateOpenOrPendingArbitration` (used by `submitAnswerReveal`) still passes, so outstanding commits can be revealed during the freeze.
4. **Freeze mechanism.** Single bool. No timer, no deadline on the arbitrator. Reality has no way to forcibly unfreeze — the arbitrator must call either `submitAnswerByArbitrator` (which sets `finalize_ts = block.timestamp` and immediately finalizes) or `cancelArbitration` (which flips the bool back and resets the timeout window).
5. **Payment flows.** ZERO. `notifyOfArbitrationRequest` itself is non-payable. The fee was paid to the Arbitrator contract earlier; Reality never sees it. The `requester` parameter is informational only — used in the emitted event so off-chain UIs can credit the requester, and reused later in `assignWinnerAndSubmitAnswerByArbitrator(payee_if_wrong=requester)` to make the requester the payee if their dispute was correct.

### CosmWasm port implications (notifyOfArbitrationRequest)

- This message does no payment. The cw-reality `ExecuteMsg::RequestArbitration` (called by the arbitrator address, on behalf of a requester) carries only `(question_id, requester, max_previous)`.
- Since cw-reality has no separate fee-collecting Arbitrator contract, the conceptual "arbitration fee" is paid out-of-band (e.g. as part of the DAO proposal that triggers the arbitrator's call to cw-reality). cw-reality is denom-neutral here.
- Match the freeze invariant: revealing a commitment must work while frozen. Use a `stateOpenOrPendingArbitration` equivalent that omits the `!pending` check.
- The asymmetry between `notifyOfArbitrationRequest` (must have an answer) and `cancelArbitration` (no answer constraint) needs care: after a cancel, the question goes back to Open-Answered with a fresh timeout. The "at least one answer" gate on notify means the answer slot is guaranteed non-empty by the time we freeze — so we can never cancel back into an answerless-but-finalize_ts-set zombie state. **Port the require literally; do NOT skip it.** That's the audit-#2 fix.

---

## 6. Finalization timing rules

The question is finalized when `isFinalized(qid)` returns true (lines 585–589):

```solidity
function isFinalized(bytes32 question_id)
view public returns (bool) {
    uint32 finalize_ts = questions[question_id].finalize_ts;
    return ( !questions[question_id].is_pending_arbitration
             && (finalize_ts > UNANSWERED)
             && (finalize_ts <= uint32(block.timestamp)) );
}
```

Three conditions, all required:
1. NOT in arbitration freeze.
2. At least one answer has been given (`finalize_ts > 0`).
3. The grace period has elapsed (`finalize_ts <= now`).

`finalize_ts` is set by `_updateCurrentAnswer` (495–499) to `block.timestamp + timeout` every time a new "best" answer is submitted (via `submitAnswer` / `submitAnswerFor` / `submitAnswerReveal` if the revealed bond matches the current bond). Each new highest-bond answer resets the clock.

`_updateCurrentAnswerByArbitrator` (502–506) sets `finalize_ts = block.timestamp` exactly — instant finalization on arbitrator submission.

`cancelArbitration` (530–531) sets `finalize_ts = block.timestamp + timeout` — re-arms a full timeout from now.

### The `min_timeout` parameter

`min_timeout` does NOT appear as a question field. It only shows up as a **caller-side validation** in `getFinalAnswerIfMatches` (697–708):

```solidity
function getFinalAnswerIfMatches(
    bytes32 question_id,
    bytes32 content_hash, address arbitrator, uint32 min_timeout, uint256 min_bond
)
    stateFinalized(question_id)
external view returns (bytes32) {
    require(content_hash == questions[question_id].content_hash, "content hash must match");
    require(arbitrator == questions[question_id].arbitrator, "arbitrator must match");
    require(min_timeout <= questions[question_id].timeout, "timeout must be long enough");
    require(min_bond <= questions[question_id].bond, "bond must be high enough");
    return questions[question_id].best_answer;
}
```

This is the "trust-discriminating reader" pattern. A downstream contract (e.g. SafeSnap) consumes the answer and requires the question used **at least** a given timeout and **at least** a given winning bond — if not, revert. This puts the policy on the *reader*, not the writer. There's no contract-level `min_timeout` floor; `_askQuestion` only requires `timeout > 0 && timeout < 365 days` (lines 340–341).

### `delay_finalization`

**Not a field in v3.0.** I grepped the full file for `delay`, `delay_finalization`, `pending_finalization` — none present. The reading list mentioned it as a concept but in v3.0 the only "delay" mechanism is `timeout` and the implicit grace window between last answer and `finalize_ts`. The `is_pending_arbitration` flag is the only finalization-blocker.

(The reading-list author may have been thinking of v2.x or of SafeSnap's surrounding contracts. I do not see it in v3.0.)

### Exact rule that lets `getFinalAnswer` return without revert

`getFinalAnswer` (594–598) and `resultFor` (603–607) both have `stateFinalized(qid)` modifier ⇒ `isFinalized(qid) == true` ⇒ the three-condition predicate above. Subtle:
- Question still in arbitration ⇒ revert.
- Question has no answer yet (`finalize_ts == 0`) ⇒ revert.
- Question answered but not yet past the grace window ⇒ revert.

`resultForOnceSettled` (618–630) wraps `resultFor` with the "settled too soon" escape: if `best_answer == UNRESOLVED_ANSWER` (the bytes32(-2) sentinel), it checks `reopened_questions[qid]` for a replacement and returns its result. One layer of indirection only (line 625 comment: "We only try one layer down rather than recursing to keep the gas costs predictable").

### CosmWasm port implications (finalization timing)

- Port `isFinalized` as a pure view fn. CosmWasm `block_info().time.seconds()` substitutes for `block.timestamp`. Use `Timestamp::seconds()` consistently — don't mix nanos. `uint32` truncation in Solidity is not a concern in Rust if we use `u64` for timestamps; just be mindful that wasmd's BFT timestamps can drift up to a few seconds from wall clock.
- The `min_timeout` reader pattern is gold. **Implement `QueryMsg::FinalAnswerIfMatches { question_id, content_hash, arbitrator, min_timeout, min_bond, min_denom }`** — including a `min_denom: Option<String>` so a downstream contract reading the answer can require a specific denom for the winning bond. This is critical for multi-denom support: a SafeSnap-equivalent that wants USDC-denominated dispute bonds shouldn't accidentally consume the answer of a question that was disputed in shitcoin.
- No `delay_finalization` to port. If we want one later, it'd be a `delay_seconds` field on the Question whose effect is `effective_finalize_ts = max(finalize_ts, last_answer_ts + delay)`. Don't add it preemptively; Reality doesn't have it.
- The "settled too soon" sentinel (`bytes32(-2) = 0xfff...fffe`) and `reopenQuestion` (lines 643–687) port literally. CosmWasm uses a `Binary` or fixed-32-byte `[u8; 32]` for answers; reserve the same sentinel value. Keep the one-layer-deep recursion limit; do NOT recurse arbitrary depth — it's a denial-of-service vector.

---

## 7. Commit-reveal pattern (`uncleared_withdrawal` / commitments)

**v3.0 still has commit-reveal.** It's not optional but it IS opt-in per-answer (`submitAnswerCommitment` is one of three answer paths, alongside `submitAnswer` and `submitAnswerFor`).

### Mechanics (lines 142–146 + 419–479)

```solidity
struct Commitment {
    uint32 reveal_ts;
    bool is_revealed;
    bytes32 revealed_answer;
}

mapping(bytes32 => Commitment) public commitments;
```

Indexed by `commitment_id = keccak256(question_id, answer_hash, bond)` where `answer_hash = keccak256(answer, nonce)`.

Flow:

1. **Commit** (`submitAnswerCommitment`, 437–448). Same modifiers as `submitAnswer` (`stateOpen`, bond-doubling, max_previous). Sends bond. Stores `Commitment.reveal_ts = block.timestamp + timeout/8` (where `COMMITMENT_TIMEOUT_RATIO = 8`, line 38). Adds an entry to history with the commitment_id as the "answer" payload and `is_commitment = true`.
2. **Reveal** (`submitAnswerReveal`, 460–479). Non-payable. `stateOpenOrPendingArbitration` — explicitly allowed during arbitration freeze (so the arbitrator can wait it out). Recomputes commitment_id from supplied `(answer, nonce, bond)`, checks the stored Commitment exists with the right `reveal_ts > now`, marks revealed, stores answer.
3. If the revealed bond equals the current `questions[qid].bond` (i.e. no one outbid them in the meantime), the reveal also updates `best_answer` and bumps `finalize_ts = now + timeout`.
4. If never revealed before `reveal_ts`, the bond is forfeit (treated as wrong in `claimWinnings._processHistoryItem`, lines 821–826).
5. **`msg.sender` is not authenticated on reveal.** The reveal can come from anyone — useful for "delegate the reveal to a service" UX, but means an attacker who learns your `(answer, nonce)` pair can front-run you and reveal first. They get nothing (`balanceOf[caller]` is not touched in reveal — the answer is credited to whoever was stored at commit time in the history hash), but the answer becomes public.

### Timing constraints

- `reveal_ts = block.timestamp + timeout / 8` at commit time. With a `timeout` of 86400 (1 day), commits have ~3 hours to reveal.
- `reveal_ts` is independent of `finalize_ts`. A commit made shortly before finalization has a shorter effective reveal window (capped by finalize_ts).
- Integer division: for `timeout < 8`, reveal_ts == block.timestamp ⇒ effectively impossible to reveal. The contract doesn't guard against this, but `timeout > 0` is the only floor. **Port a minimum sane timeout (e.g. 8 seconds) in cw-reality to avoid this footgun.**

### Why commit-reveal exists

Front-running protection. A naive answerer broadcasting a high-bond answer for a still-open question can be sniped: an attacker watching the mempool sees the answer + the implied "this is the right answer", front-runs with a slightly higher bond on the same answer, and wins the bounty + the next-round bond redistribution. Commit-reveal lets the answerer broadcast a hash, then reveal once they're confident the bond ordering is locked.

(Cosmos has no public mempool front-running risk in the same way Ethereum does, but Tendermint reorderings within a block and proposer-MEV are not zero. Keep commit-reveal in the port — it's a small surface and the alternative is to require the user to wait out an extra timeout.)

### CosmWasm port implications (commit-reveal)

- Port the `Commitment` struct directly. `Map<&[u8], Commitment>` keyed by commitment_id.
- Hash inputs include `bond.denom` and `bond.amount.u128().to_be_bytes()` — the commitment_id must be denom-aware or an attacker who commits in denom X can reveal claiming denom Y.
- `submitAnswerReveal` keeps the `stateOpenOrPendingArbitration` permissiveness — let revealers reveal during arbitration freeze. Match Reality exactly.
- Minimum-timeout floor at instantiation: refuse `timeout < 8 * MIN_REVEAL_SECONDS`. Suggest `MIN_REVEAL_SECONDS = 60` ⇒ floor `timeout >= 480`. Document.
- Reveal authentication: match Reality — anyone can reveal, the credit goes to the address stored at commit time (which is `_answerer` or `msg.sender` per submitAnswerCommitment line 444). This requires the history hash chain to encode the original answerer, not the revealer. Port this exactly.

---

## 8. Anything surprising

### Reentrancy posture
- **No `ReentrancyGuard`.** Reality.eth predates the convenience of OZ's modifier. Its defense is **pull-payments**: the only outbound ETH is `BalanceHolder.withdraw()` (line 18), which uses `.transfer()` (2300 gas, no re-enter possible). All bond/bounty math is bookkeeping into `balanceOf`. No external call happens during state-changing functions. This is what makes the contract safe.
- **G0 audit notes (line 47 of audit txt)**: *"RealityETH_ERC20-3.0.sol should not be used with ERC20-like token contracts that implement callbacks like ERC777 due to potential re-entrancy issues."* — The ETH variant is safe; the ERC20 variant is safe only with non-callback tokens.
- **CosmWasm port**: cw20 tokens have a `Send`/`Receive` pattern. The Receive callback is the entry point, which doesn't create reentrancy by itself, but if cw-reality forwards `BankMsg::Send` of cw20s inline (via `Cw20ExecuteMsg::Transfer`), the submessage reply CAN re-enter cw-reality. Use pull-payments: credit `balanceOf[Coin]`, withdraw via separate message. Identical to Reality's approach.

### History-hash commit pattern
- Already covered in §2; the key surprise is how MUCH this saves on storage. A 30-round dispute has zero per-round storage; the whole history exists only in events + the running hash.
- **Indexer dependency.** This is a hard architectural choice: **claiming requires off-chain event indexing.** If your indexer is down, no one can claim. cw-reality inherits this. The indexer-proxy in this workspace can serve the events but we need to confirm `LogNewAnswer`-equivalent events are indexed.

### Per-call event design
- `LogNewAnswer` (line 83–91) contains the full state needed to reconstruct one history step: `answer, question_id, history_hash, user, bond, ts, is_commitment`. Indexers grab these.
- `LogClaim` (116–120) carries `question_id, user, amount` — but `amount` may be partial (split claim). Indexers need to sum.
- **Port these one-to-one.** Use `Event::new("wasm-LogNewAnswer")` (or a cleaner namespaced name like `wasm-cw_reality/new_answer`) with the same attributes. Choose canonical address encoding for the indexer's sake.

### `cancelArbitration` re-extends, doesn't restore
- Subtle UX: if a question was 5 seconds from finalizing when arbitration was requested, and then cancelled an hour later, the question now has a fresh full `timeout` to run. Comment on line 531: `finalize_ts = uint32(block.timestamp) + questions[question_id].timeout`. Doesn't restore the original finalize_ts. **Port literally — restoring the original would let an attacker freeze-and-immediately-cancel to insta-finalize at the prior bond.**

### `assignWinnerAndSubmitAnswerByArbitrator` skips its own `onlyArbitrator`
- Lines 565–579. The wrapper function does NOT have `onlyArbitrator`. It calls `submitAnswerByArbitrator` (line 578) which IS gated. **But the wrapper itself reads `questions[question_id].history_hash` and `questions[question_id].bond` and runs the verify-and-pick logic before that gate.** A non-arbitrator caller would do the work, then bounce off the `onlyArbitrator` check inside `submitAnswerByArbitrator`. Wasteful gas griefing but not a security issue; the actual state change is gated.
- In CosmWasm port: do the `info.sender == arbitrator` check at the top of the wrapper handler — there's no reason to defer it and waste compute.

### `assignWinnerAndSubmitAnswerByArbitrator` does NOT check `statePendingArbitration` directly
- Lines 565–579 again. The wrapper verifies the LAST history entry (calls `_verifyHistoryInputOrRevert` with `questions[question_id].bond` as the bond — line 567). If someone calls it before notify-of-arbitration was sent, `_verifyHistoryInputOrRevert` will pass (the chain tip is still the most recent answer entry), but the inner `submitAnswerByArbitrator` will revert on `statePendingArbitration`. So effectively it IS gated, just indirectly. Port: assert state at the top of the wrapper too, explicit beats implicit.

### Bond doubling and `min_bond` for first answer
- Lines 210–219 in `bondMustDoubleAndMatchMinimum`: first answerer's bond must satisfy `>= min_bond`; subsequent answerers must satisfy `>= 2 * current_bond`. The `min_bond` is per-question (set at ask time via `askQuestionWithMinBond`, otherwise 0).
- **Edge case if `min_bond = 0` and `msg.value > 0`**: first bond can be any positive value, the second bond must be 2× that. So a 1-wei first bond locks the doubling at small numbers. With wei-scale, the chain can grow ~80 rounds before hitting `uint256` overflow. CosmWasm `Uint128` permits ~120 doublings from 1 — enough for any realistic dispute. Document the limit; refuse asks with `min_bond == 0 && timeout < some_floor` or accept the footgun.

### Storage zeroing
- Reality uses `delete commitments[id]` in `_processHistoryItem` (lines 825, 829). This zeros storage and returns gas refund. CosmWasm equivalent: `COMMITMENTS.remove(deps.storage, &id)`. Less crucial in CosmWasm (no refund), but still good hygiene to keep state small.

### `withdraw()` does not revert on zero balance
- `BalanceHolder.withdraw()` happily processes a zero balance — it sets `balanceOf[sender] = 0` (already 0) and calls `.transfer(0)` which succeeds. UX-fine. Port matches: just emit the event with `value=0` if someone calls withdraw with no credit. Actually, do NOT emit on zero — wastes gas — but Reality does. Match Reality unless we have a reason not to.

### Reopener of a reopener is forbidden
- Line 660–661: `require(!reopener_questions[reopens_question_id], "Question is already reopening a previous question");` — you can only reopen a question that was the *original*, not a reopen-of-a-reopen. Prevents chain growth in the resolve-replacement path. Port.

### `submitAnswerFor` exists for delegation
- Lines 406–414. Same semantics as `submitAnswer` but credits a different `answerer`. Used by services that pay the bond on a user's behalf. The credit is recorded in the history hash, so when the dispute is claimed, the user (not the service) gets the bond back. Port if there's a use case; otherwise skip in v1.

### `_askQuestion` allows zero bounty
- Lines 343, 362–365: a question can be asked with `msg.value == 0` (after the optional arbitrator fee). The bounty just stays at 0. The contract makes no money off bounty-less questions. Port: same — `info.funds` can be empty for ask if the arbitrator question fee is 0.

### `arbitrator == address(0)` is permitted
- Line 350: `if (arbitrator != NULL_ADDRESS && msg.sender != arbitrator)`. The asker can specify NO arbitrator. In that case `onlyArbitrator(qid)` becomes `msg.sender == address(0)` — un-satisfiable. **There is no way to request arbitration on a no-arbitrator question.** Bond exhaustion is the only resolution path. This matches GOAL.md's `Option<Addr>` arbitrator with `None` meaning "bond-exhaustion-only" perfectly.

### `setQuestionFee` is unprotected
- Lines 241–246. Anyone can register themselves as an "arbitrator" by calling `setQuestionFee(0)` then having questions reference them. There is no whitelist. The check on whether to send the fee is `msg.sender != arbitrator` (line 350) — askers can ask through any address. So `setQuestionFee` is effectively self-service: you declare your fee, askers pay it when asking. Reality does not validate that the named arbitrator is "real" — it's purely an honor system tracked off-chain. Port as-is, or harden via instantiation-time arbitrator whitelist (overengineered; skip).

### Gas optimization that constrains design
- The history-hash chain forces the **newest-first walk in claimWinnings**. This is non-negotiable — the chain only verifies forward (from tip backwards). Any port that tries to walk oldest-first cannot use the hash chain for authentication; it would need full per-round storage. Keep the newest-first walk.
- Constants packed into the bond doubling: `BOND_CLAIM_FEE_PROPORTION = 40` (= 2.5%). If we want to make this configurable per cw-reality instance, expose it at instantiation. Reality hardcodes; we can hardcode too.

### One missing-from-source surprise: NO question_fee for re-asking
- `reopenQuestion` (lines 643–687) calls `askQuestionWithMinBond` (line 676) which charges the arbitrator question_fee. So if you reopen a question, you pay the question fee again. The bounty is transferred from the old question (line 681). Be aware in the port.

---

## Appendix A — Quick lookup table

| Concept | File:lines |
| --- | --- |
| `struct Question` | RealityETH-3.0.sol:127-139 |
| `struct Commitment` | RealityETH-3.0.sol:142-146 |
| `struct Claim` (resumable claim state) | RealityETH-3.0.sol:150-154 |
| Constants (UNANSWERED, BOND_CLAIM_FEE_PROPORTION, UNRESOLVED_ANSWER, COMMITMENT_TIMEOUT_RATIO) | RealityETH-3.0.sol:32-45 |
| All modifiers | RealityETH-3.0.sol:167-226 |
| `_askQuestion` (where the arbitrator fee is taken) | RealityETH-3.0.sol:335-372 |
| `submitAnswer` | RealityETH-3.0.sol:390-397 |
| `submitAnswerCommitment` | RealityETH-3.0.sol:437-448 |
| `submitAnswerReveal` | RealityETH-3.0.sol:460-479 |
| `_addAnswerToHistory` (history hash chain) | RealityETH-3.0.sol:481-493 |
| `_updateCurrentAnswer` / `_updateCurrentAnswerByArbitrator` | RealityETH-3.0.sol:495-506 |
| `notifyOfArbitrationRequest` | RealityETH-3.0.sol:513-521 |
| `cancelArbitration` | RealityETH-3.0.sol:526-533 |
| `submitAnswerByArbitrator` | RealityETH-3.0.sol:543-555 |
| `assignWinnerAndSubmitAnswerByArbitrator` | RealityETH-3.0.sol:565-579 |
| `isFinalized` | RealityETH-3.0.sol:585-589 |
| `getFinalAnswerIfMatches` (the reader-side trust knob) | RealityETH-3.0.sol:697-708 |
| `claimWinnings` (the heart) | RealityETH-3.0.sol:724-790 |
| `_processHistoryItem` (the math) | RealityETH-3.0.sol:812-870 |
| `_verifyHistoryInputOrRevert` | RealityETH-3.0.sol:798-810 |
| `_payPayee` (the pull-payment credit) | RealityETH-3.0.sol:792-796 |
| `BalanceHolder.withdraw` (the only outbound ETH) | RealityETH-3.0.sol:14-20 |
| `reopenQuestion` (settled-too-soon escape) | RealityETH-3.0.sol:643-687 |
| `resultForOnceSettled` (one-deep reopen lookup) | RealityETH-3.0.sol:618-630 |

## Appendix B — Diff items vs `reality-eth-reading-list.md` open questions

| Open question | Answer |
| --- | --- |
| Does Reality.eth allow asking the same question twice? | Yes. `question_id = keccak256(content_hash, arbitrator, timeout, min_bond, address(this), msg.sender, nonce)` (line 325). The `nonce` lets the same `msg.sender` re-ask with the same params. Different askers get different IDs automatically. |
| What is the loser-bond math with >2 escalation rounds? | See §3. Newest-first walk; right answers collect interior bonds; same-payee accumulates; payee-change pays takeover fee = min(queued, new_payee_bond) to the old payee. |
| How does Reality handle a question whose asker disappears? | Asker has no special role after ask. The bounty is paid to the winning answerer regardless. No bond reclamation path for the asker; the bounty is unrecoverable to the asker. |
| Gas profile of escalation as bond grows? | O(1) per round — single hash, two events. Claim is O(rounds) but resumable. |
| Griefing via tiny-bond escalation? | Bounded by `min_bond` at ask time. Without min_bond, 1-wei start can drag the chain through ~80 rounds before uint256 overflow. CW port has ~120 with `Uint128`. Set a sane `min_bond` floor at ask time. |
| Answer-schema validation? | None. `bytes32 answer` opaque to the contract. Interpretation is wholly off-chain. cw-filter integration must be additive (a wrapper or a post-finalization check), not baked into cw-reality. |
| "Uncle answer" rule — can an undefended prior answer still claim? | Yes — that's exactly what the `_processHistoryItem` walk does. If you gave the right answer in round 3 and then was outbid by a wrong answer in round 4, and the wrong answer becomes the chain tip but the arbitrator (or a later round) restores the truth, your round-3 bond is recovered + you get the round-4 loser's bond as redistribution. |
| Arbitrator decline? | Implicit: `cancelArbitration` (lines 526–533) unfreezes the question and resets timeout. There is no "I refuse" final answer; the arbitrator just declines to call `submitAnswerByArbitrator` and instead calls cancel. Alternative — they can submit `UNRESOLVED_ANSWER` (the bytes32(-2) sentinel) which forces the "settled too soon" path. |
