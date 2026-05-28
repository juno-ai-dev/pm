# `cw-reality` vs. Reality.eth — substantial differences

`cw-reality` is a CosmWasm port of [Reality.eth](https://reality.eth.link/),
canonically `RealityETH-3.0.sol`
([`RealityETH/reality-eth-monorepo@b996b0a0`](https://github.com/RealityETH/reality-eth-monorepo/blob/b996b0a0899451b95887b59243a118a467f602d0/packages/contracts/flat/RealityETH-3.0.sol)).

Most of the mechanism is preserved literally — bond doubling, the
newest-to-oldest history walk on claim, the right-answer redistribution rule,
the 2.5% interior burn, `UNRESOLVED_ANSWER` semantics, the question-content
binding via hash, the arbitrator-as-address permission model. The Alice / Bob /
Carol worked example from the Reality.eth whitepaper produces identical
payouts under cw-reality.

This document lists the parts that **do not** match line-for-line, and why.

---

## 1. Removed

### 1.1 Commit-reveal answers

Reality.eth v3 supports a `submitAnswerCommitment` / `submitAnswerReveal` flow
to hide answers during the dispute window (defending against mempool
front-running). Reality.eth v4 removed it; Edmund Edgar's stated conclusion
was *"in practice it was never used in a way where that mattered"*. Two of
the three v3 audit findings were in this code path.

**cw-reality v1 ships without it.** CosmWasm chains don't have an
Ethereum-style public mempool with the same MEV exposure profile, so the cost
of carrying it forward outweighed the benefit.

### 1.2 History deletion on claim

`RealityETH-3.0.sol:_payFinalizeBond` deletes the history-hash chain at claim
time for a gas refund. Reality.eth's own PR #134 flagged this as a footgun —
`verifyHistory` only works pre-finalization.

**cw-reality keeps history forever.** CosmWasm storage is cheaper than EVM
storage and the queryability win is worth the bytes — `History { question_id }`
remains answerable after `Claimed`.

---

## 2. Added

### 2.1 Multi-denom support, with denom binding per question

Reality.eth assumes **one fungible per contract instance** — the ETH contract,
the per-ERC20 contract. cw-reality supports native, IBC, and cw20 bonds in
**one** deployment by pinning the denom at ask time:

- `Question.bond_denom` is set from the asker's first `info.funds` (or the
  cw20 `Receive` carrier).
- Every subsequent answer in a different denom is rejected with a typed error.
- The denom bytes are included in the history-hash input (see §3.3 below) so a
  hostile reorganiser cannot swap denoms mid-round.
- cw20 bonds enter via `Cw20ReceiveMsg` with a `ReceiveAction` enum carrying
  the `(question_id, action)` tuple — this is the CosmWasm-shaped equivalent
  of Solidity's `payable` modifier.
- IBC denoms (`ibc/...`) need no special handling; they're bank-native to
  CosmWasm.

The trade-off is documented in `docs/reality-eth-lessons.md §2.4`: the bond
denom **is** the economic-security anchor. A small-cap cw20 issuer who
controls the asker role can mint arbitrary supply to win their own question.
This is documented as a social-layer concern; cw-reality stays permissionless
and lets reader-side tools (UI allowlists, the `FinalAnswerIfMatches`
required-denom field) enforce policy.

### 2.2 Contract-level `min_answer_timeout_secs` floor

Reality.eth has no contract-level minimum on the answer timeout. SafeSnap
incidents at SuDAO (12h timeout) and Gnosis Guild (1h timeout) were drained
because their per-question timeouts were too short to be monitored.

cw-reality adds an instantiation-time floor:
`InstantiateMsg.min_answer_timeout_secs`. Asks below the floor are rejected
with a typed error before any bond escrow. The protocol floor is **1 hour
(3600s)** — sub-hour configurations are rejected at instantiation.

### 2.3 Dispute round cap

Reality.eth has no explicit round cap; the soft cap is `uint256` saturation on
bond doubling. cw-reality imposes `MAX_DISPUTE_ROUNDS = 32` — at 2× from 1
`ujuno` the round-32 bond is ~4 billion JUNO, which is adequate headroom for
any real dispute. The cap bounds proptest budgets and defends against
round-counter exhaustion.

### 2.4 Public `CancelArbitration` after `arbitration_deadline`

Reality.eth's `cancelArbitration` is arbitrator-only. cw-reality preserves
that path but adds: **anyone** can call `CancelArbitration` after
`Question.arbitration_deadline` has passed without a verdict, returning the
question to `OpenAnswered` with a fresh timeout. This prevents an unresponsive
arbitrator from freezing a question indefinitely.

### 2.5 Schema validation of answer payloads via cw-filter

Reality.eth answers are opaque `bytes32`. cw-reality optionally binds a
**cw-filter** schema at ask time (`AskQuestion.answer_filter`) so non-matching
answer payloads are rejected at submission. The cw-filter wire types are
**mirrored locally in `src/filter.rs`** rather than taking a crate dep — the
cw-filter crate transitively pulls in `alloy-rpc-types-eth`, which conflicts
with this workspace. The wire format is the contract.

### 2.6 Property tests

Reality.eth has no fuzzing harness in-tree. cw-reality bundles `proptest`
invariants in `src/proptests.rs`, covering at minimum:

- `claim_conserves_bond_total` — sum of credits equals bounty + sum of bonds
  minus sum of interior shaves.
- `claim_partial_equals_full` — FM-3 (per-claim determinism, the audit-issue-3
  fix).
- `shave_is_bond_minus_fortieth` — `shave(b) == b - b/40`.

---

## 3. Changed (encoding, not mechanism)

### 3.1 Explicit state machine

Reality.eth has no `enum State`; state is derived from four `Question` fields
(`timeout`, `finalize_ts`, `is_pending_arbitration`, `history_hash`) gated
through five Solidity modifiers. Combining orthogonal concerns silently is
fine in Solidity; it's worse in Rust.

cw-reality has an explicit
`State { NotCreated, OpenUnanswered, OpenAnswered, PendingArbitration, Finalized, Claimed }`
and an `assert_state(qid, &[State])` helper. Every state transition is checked
against an allowed set, not against derived booleans. Behaviour matches
Reality.eth exactly — the difference is in the type, not the rules.

### 3.2 Pull-payment shape

Reality.eth uses `mapping(address => uint256) balanceOf` because it only
handles one fungible. cw-reality uses
`BALANCES: Map<(&Addr, &str), Uint128>` keyed by `(payee, denom)`. The
withdrawal path is also denom-keyed: `Withdraw { denom }` drains one denom at
a time via `BankMsg::Send`. No inline `BankMsg` ever runs during a state
mutation; sub-message replies are CosmWasm's reentrancy surface and we avoid
them on the credit path entirely.

### 3.3 History-hash input

Reality.eth:

```solidity
new_history_hash = keccak256(prev_hash, answer, bond, answerer, is_commitment)
```

cw-reality:

```
H_n = sha256(
    H_{n-1}
    || answer_or_commitment_bytes
    || bond_denom_bytes
    || bond_amount.to_be_bytes()
    || answerer_canonical_addr
    || is_commitment_byte
)
```

Three deltas:
- **`sha256` not `keccak256`** — CosmWasm's `api.secp256k1_verify` family
  doesn't include a stable keccak primitive; sha-256 is what the chain
  already does.
- **`bond_denom_bytes` is mixed in** — defends the multi-denom invariant
  (see §2.1).
- **Canonical address bytes** via `api.addr_canonicalize`, not the EVM 20-byte
  address layout. Same intent: deterministic bytes for the hash chain.

### 3.4 Question-ID derivation

Reality.eth v3 fixed FM-5 (cross-deployment collision) by mixing
`address(this)` and `min_bond` into the question-ID keccak. cw-reality
codifies the same defense from day one: the `question_id` is a SHA-256 of
`(contract_address, asker, content, arbitrator, denom, min_bond, timeout,
nonce, opening_ts)`. Same property, different hash.

### 3.5 Sentinel constant

The `UNRESOLVED_ANSWER` sentinel — Reality.eth's `bytes32(-2)`,
`0xff...fe` — is preserved bit-exact as `UNRESOLVED_ANSWER_BYTES` in
`state.rs`. The byte pattern is identical so cross-chain reasoners and any
future bridge can compare without translation. Exposed as the typed variant
`Answer::Unresolved` to consumers.

### 3.6 Defaults

| Parameter | Reality.eth (dapp UI) | cw-reality (v1 defaults) | Why |
| --- | --- | --- | --- |
| Bond multiplier | 2× | 2× | Load-bearing; do not adjust. |
| Initial bond default | ~0.01 ETH | **1 JUNO** (`1_000_000 ujuno`) | Juno fees are ~1000× lower; UMA's "set above the minimum" principle. |
| Initial bond floor | per-question only | `min_initial_bond_floor` at instantiation | FM-1 defense at the contract layer, not just the asker's discretion. |
| Answer timeout | 24h (default), no contract floor | 24h default, **1h contract floor** | New defense — see §2.2. |
| Arbitration timeout | n/a (arbitrator-side concern) | **7 days** before `CancelArbitration` opens to the public | Calibrated to DAO DAO governance windows (5–7d). |
| Dispute round cap | none (uint256 saturation) | **32** | See §2.3. |
| Interior burn | 2.5% (`BOND_CLAIM_FEE_PROPORTION = 40`) | 2.5% (`BOND_CLAIM_FEE_DENOM = 40`) | Identical. No DAO-treasury skim, no protocol fee. Stays in contract bank with no credit. |
| Question text mutability | immutable | immutable | Identical. Use `reopen_question` for settled-too-soon. |
| Bounty refund window | none | none | Identical — see Reality.eth design rationale. |

---

## 4. Deliberately not changed

Worth calling out, because the temptation to "improve" them in a port is
high and would have been wrong:

- **The 2× bond multiplier.** The whitepaper proves takeover fees are bounded
  by the prior bond exactly at this multiplier. Any other value either makes
  takeover fees unbounded or breaks the redistribution conservation property.
- **The 2.5% interior burn.** Disincentivises hairsplitting; routes nothing
  to a protocol treasury — matching Reality.eth keeps cw-reality a neutral
  primitive.
- **The newest-to-oldest claim walk.** It's the only walk that yields O(1)
  storage per round.
- **`arbitrator = None` collapsing to bond-exhaustion-only.** No fallback
  arbitrator, no committee — if the asker chose `None`, the question
  finalises purely by timeout, exactly as Reality.eth line 350 specifies.
- **Asker-disappears bounty behaviour.** Bounty stays locked in the contract.
  Reality.eth has no auto-refund; cw-reality v1 matches. Future auto-refund
  is a known footgun-fix, not a primitive.

---

## 5. Credit

This contract would not exist without Reality.eth. The mechanism design — and
specifically the worked Alice/Bob/Carol redistribution example that anchors
the proptest invariants — is Edmund Edgar's, the Reality.eth team's, and
the community of bond-escalation-oracle researchers who refined it
(Augur REP rounds, UMA optimistic oracle, Kleros' juror-bond shape).

Reality.eth ships under GPL-3.0; cw-reality ships under Apache-2.0 because
this is a clean-room CosmWasm reimplementation, not a derived work. The
Reality.eth source informed the design but is not copied. Where the lessons
doc cites specific line numbers in `RealityETH-3.0.sol`, those are pointers
for auditors verifying behavioural equivalence — not licensed inclusions.
