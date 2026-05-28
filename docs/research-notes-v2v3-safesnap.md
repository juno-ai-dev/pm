# Reality.eth v1→v2→v3→v4 + SafeSnap research notes

> Stage-1 due-diligence reading for `cw-reality`. Concrete lessons we are
> inheriting, with citations. **Read before reading `self-audit-checklist.md`** —
> several boxes there were drawn from incidents catalogued here.

---

## 0. Documentation honesty

Before the content: **most of what we want is not in changelogs**. The
RealityETH monorepo (`RealityETH/reality-eth-monorepo`) has **zero published
GitHub releases** (`gh api repos/RealityETH/reality-eth-monorepo/releases`
returns `[]` as of 2026-05-28), no `CHANGELOG.md`, and PR bodies are mostly
empty. The version story has to be reconstructed from:

- the per-version Solidity files (`RealityETH-3.0.sol`,
  `RealityETH-4.0.sol`, `Realitio_v2_1.sol`, …),
- the audit reports under `packages/contracts/audits/`,
- PR titles and individual commits,
- Edmund Edgar's Medium blog (sparse — 4 posts, last load-bearing one in 2019).

SafeSnap incident history is **substantially fuzzy**:

- The two confirmed post-mortems (SuperUMAn DAO on Polygon, Gnosis Guild on
  Ethereum) are written by third parties (`publish0x` / Medium re-posts of an
  "Everything Blockchain" piece; QuillAudits). Neither carries an Edmund Edgar
  on-record response.
- I could not corroborate the specific claim in the brief that "ENS DAO
  eventually paused/replaced the SafeSnap module in 2023." ENS DAO appears to
  govern via Tally + Snapshot **without** ever having enabled a Reality module
  for executable proposals (see [Section 4.4](#44-ens-dao-the-rumour-could-not-be-corroborated)).
  Treat the brief's ENS claim as unverified.
- The broader migration story **is** well-documented: SafeSnap-Reality is now
  effectively superseded by oSnap (UMA's optimistic oracle), per
  Snapshot's own docs: *"reality.eth is no longer supported with Snapshot's
  most recent UI"* — quoted at
  [docs.snapshot.box/v1-interface/plugins/safesnap-osnap](https://docs.snapshot.box/v1-interface/plugins/safesnap-osnap)
  and the 1inch migration discussion at
  [gov.1inch.network/t/.../793](https://gov.1inch.network/t/opening-discussion-on-migrating-from-reality-eth-to-osnap-for-proposal-execution/793).

Everything below cites primary sources where they exist; `[UNVERIFIED]`
markers tag claims I cannot corroborate.

---

## 1. The Reality.eth version history

### 1.1 Naming and chronology

The contract has been renamed across two organizations and four versions.
Identifying which contract is "v1," "v2," etc. requires reading commits because
no single document numbers them.

| Public version       | On-chain contract name | Repo / commit anchor                                                                                        | Date          |
| -------------------- | ---------------------- | ----------------------------------------------------------------------------------------------------------- | ------------- |
| RealityCheck (pre-v1)| `RealityCheck`         | `realitykeys/realitycheck` — renamed in `realitio/realitio-contracts@1198b7b` (2018-10-10)                   | Oct 2017      |
| Realitio v1          | `Realitio`             | `realitio/realitio-contracts@cd093e4` last touched 2020-09-23, lives at `truffle/contracts/Realitio.sol`     | Oct 2018      |
| Realitio v2          | `RealitioERC20`        | added in `realitio/realitio-contracts` ERC20 audit branch `feature-erc20-audit@ec8db377` (May 2019)         | Jun 2019      |
| Realitio v2.1        | `Realitio_v2_1`        | `realitio/realitio-contracts/truffle/contracts/Realitio_v2_1.sol` — last touched 2021-01-13 (`aac6a3c`)      | Sep 2020      |
| Reality.eth v3.0     | `RealityETH_v3_0`      | `RealityETH/reality-eth-monorepo@064855bf...e4584d7c` (audit-pinned)                                         | Aug 2021      |
| Reality.eth v3.2     | (same surface)         | added `hash` question type, replaced `category` with `description` field (per contracts README)              | 2022          |
| Reality.eth v4.0     | `RealityETH_v4_0`      | `RealityETH/reality-eth-monorepo` v4-candidate PRs #131/#133/#138/#147 (Jan-Feb 2024)                       | Q1 2024       |

Sources:
- `gh api repos/realitio/realitio-contracts/commits?path=truffle/contracts/Realitio.sol`
- `gh api repos/realitio/realitio-contracts/commits?path=truffle/contracts/Realitio_v2_1.sol`
- Audit pin `e4584d7cf6ab2d9a5b129bd970b7d4517811ae6a` from `audits/RealityETH-3.0.txt`
- Monorepo PR list: `gh pr list --repo RealityETH/reality-eth-monorepo --state closed --limit 100`

### 1.2 What changed v1 → v2 (ERC20 + v2.1)

The v1→v2 jump is **not a redesign of the mechanism**. It is "make
`Realitio.sol` work with an ERC20 instead of native ETH." The audit makes this
explicit: *"the principle is that we will have one RealitioERC20 contract per
supported token, and as currently one Arbitrator contract per arbitrator per
RealitioERC20 contract. The token is set during initial setup and once set
cannot be changed"* — quoted at `packages/contracts/audits/RealityETH_ERC20-2.0.txt`
(Kofler, June 2019).

Important security choices made at v2:

1. **One contract per token.** Not a multi-token contract. Token-mismatch attacks
   are physically impossible because no other token can be deposited.
2. **Arbitration still denominated in ETH.** Question fee + bond are in the
   ERC20, but arbitration request payment is ETH on Ethereum mainnet. (Relevant
   for porting: pick **one** denom per question and reject everything else.
   `self-audit-checklist.md` "wrong-token griefing" — covered.)
3. **Hostile-token paranoia.** *"users must implicitly trust that the token the
   contract is interacting with isn't hostile. That said, the code is written
   with the intention that it wouldn't be subject to reentrancy bugs etc"* —
   `RealityETH_ERC20-2.0.txt`. The audit explicitly rejected hardening against
   non-standard ERC20s (no-return tokens etc.); Edmund's stance was "if you need
   that, write a wrapper."
4. **No ERC777 support.** The v3 audit appendix carries this forward: *"RealityETH_ERC20-3.0.sol
   should not be used with ERC20-like token contracts that implement callbacks
   like ERC777 due to potential re-entrancy issues"* — `RealityETH-3.0.txt`.

The v2 → **v2.1** jump (post Sept 2020) is small but interesting; the commits
are individually labeled and dated:

| Commit  | Date       | Change                                                                                                                                                              |
| ------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `35301b6`| 2020-09-10 | *"Apply claim fee as in a0e637747..., but as a new version of the contract"* — introduces the bond-claim fee (2.5%) into the answer payout path                     |
| `e9f90af`| 2020-09-23 | `cancelArbitration` — for cross-chain cases where arbitration cannot be requested atomically                                                                         |
| `6cc0cc0`| 2020-09-23 | `assignWinnerAndSubmitAnswerByArbitrator` — auto-assign the payee inside arbitration; enables trustless arbitrators                                                  |
| `855c130`| 2020-09-23 | `submitAnswerFor` — relayer support; answer on someone else's behalf                                                                                                |
| `aac6a3c`| 2021-01-13 | *"Make sure the address is supplied in submitAnswerFor, this avoids changing an invariant that might conceivably be dangerous in future"* — defensive constraint    |

Source: `gh api repos/realitio/realitio-contracts/commits?path=truffle/contracts/Realitio_v2_1.sol`.

The `cancelArbitration` addition is load-bearing for cross-chain integrations
and **fixed an audited bug** later (see Section 1.4, v3 audit issue #2).

### 1.3 What changed v2.1 → v3 — the substantive jump

Diffing `Realitio_v2_1.sol` (deployed) against `RealityETH-3.0.sol` (audit-pinned
`e4584d7c`) produces 369 lines of changes. Pulling out the *semantic* changes:

#### 1.3.1 `min_bond` (anti-griefing)

```solidity
modifier bondMustDoubleAndMatchMinimum(bytes32 question_id) {
    uint256 current_bond = questions[question_id].bond;
    if (current_bond == 0) {
        require(msg.value >= (questions[question_id].min_bond), "bond must exceed the minimum");
    } else {
        require(msg.value >= (current_bond * 2), "bond must be double at least previous bond");
    }
    _;
}
```

In v2.1 the initial answer could be **any non-zero bond**, including 1 wei.
That meant a griefer could plant a tiny initial answer, then anyone wanting
to overturn it had to start a doubling race from near-zero — but they would do
so against an attacker who could also keep doubling cheaply at the bottom of
the curve. The fix: `askQuestionWithMinBond()` lets the asker set a floor
for the very first answer.

`cw-reality` mapping: this is the `initial_bond` field in our `AskQuestion`
message. Treat it as a **mandatory anti-spam floor**, not optional. Set a
reasonable platform-wide minimum even if the asker passes 0 — `Realitio` did
not enforce this and it cost them three years of low-quality questions.

#### 1.3.2 `UNRESOLVED_ANSWER` / "answered too soon"

```solidity
// Special value representing a question that was answered too soon.
// bytes32(-2). By convention we use bytes32(-1) for "invalid", although the contract does not handle this.
bytes32 constant UNRESOLVED_ANSWER = 0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe;
```

If a question is asked about an event that has not yet occurred (e.g. "Did
team X win the cup?" but the match hasn't been played), the canonical answer
becomes the special sentinel `0xff..fe` ("answered too soon"). A question
that finalizes to this sentinel can then be `reopenQuestion()`'d
(see 1.3.3) — and the **bounty transfers to the reopened question**.

The asymmetry is important: `bytes32(-1)` "invalid" is a *convention* used by
front-ends and integrators (oSnap and Snapshot UI both rely on it), but the
**contract itself does not handle it specially**. Only `bytes32(-2)` "answered
too soon" has on-chain semantics.

`cw-reality` mapping: we should expose a typed `Answer::AnsweredTooSoon`
variant so we don't lose this in JSON round-tripping. Reality.eth's v3 audit
issue #1 (Section 1.4) was a bug where `UNRESOLVED_ANSWER` payout went to
`address(0)` — payable-direction tests are mandatory for this case.

#### 1.3.3 `reopenQuestion` + immutable history

```solidity
function reopenQuestion(uint256 template_id, string memory question, address arbitrator,
                       uint32 timeout, uint32 opening_ts, uint256 nonce,
                       uint256 min_bond, bytes32 reopens_question_id) ...
```

Reality.eth keeps questions immutable post-finalization. To "re-ask" a
question, you create a **new question** that links back to the original via
`reopens_question_id`. The reopen is only legal if the original
`isSettledTooSoon` returns true. Reading the reopened question's result via
`resultForOnceSettled(original_id)` follows the reopen chain.

This is also why PR #138 ("Split the reopening feature into its own version")
removed reopening from the v4 mainline — it's a feature most integrators
don't want, and it complicates state. We probably **do not need it** in v1 of
`cw-reality`, but we should not paint ourselves into a corner: keep
`finalized_with` open enough to admit a future link-back field if needed.

#### 1.3.4 Optional arbitrator

v2.1 required `arbitrator != NULL_ADDRESS` at ask time. v3 lifted that:

```solidity
// v3
if (arbitrator != NULL_ADDRESS && msg.sender != arbitrator) { ... }
```

The intent: allow questions whose final answer is decided purely by the bond
race, with no escalation path. For `cw-reality` we already plan this
(`arbitrator: Option<Addr>`) — confirmed sane by Reality.eth precedent. The
**hard rule** (per Edmund's response in the v2 audit, on coupling): if
`arbitrator` is `Some`, the contract must verify the address is a real
contract / bech32 at ask time, not just trust the bytes.

#### 1.3.5 `question_id` keccak now binds `address(this)` and `min_bond`

```solidity
// v2.1
bytes32 question_id = keccak256(abi.encodePacked(content_hash, arbitrator, timeout, msg.sender, nonce));
// v3
bytes32 question_id = keccak256(abi.encodePacked(content_hash, arbitrator, timeout, uint256(0), address(this), msg.sender, nonce));
```

This is a **cross-deployment-collision defense**. Without
`address(this)`, two Reality.eth deployments on the same chain could share
question IDs, and a question on one could be confused for a question on the
other. The added `uint256(0)` slot is the `min_bond` (defaulted to 0 for the
non-`WithMinBond` entrypoint), making question IDs distinct between
min-bonded and non-min-bonded asks even when all other fields match.

`cw-reality` mapping: include the **contract address** in our `QuestionId`
derivation if we use any form of content-hash derivation, even though
CosmWasm contract addresses are already globally unique. If we use a simple
auto-incrementing counter, this is moot — but consider whether we want
deterministic content-derived IDs (Reality.eth-style) for off-chain indexing.

#### 1.3.6 `BOND_CLAIM_FEE_PROPORTION` = 40 (i.e. 2.5%)

v2.1 introduced the bond-claim fee; v3 carries it forward unchanged (this is
hardcoded as a constant in v3 and v4):

```solidity
uint256 constant BOND_CLAIM_FEE_PROPORTION = 40; // One 40th ie 2.5%
```

When you claim winnings, 2.5% of *each historical loser bond* is withheld
and credited to the question-payer (the asker, in practice). This funds the
bounty pool, slightly disincentivizes hairsplitting arbitration, and ensures
the asker captures *some* value when the question resolves correctly.

`cw-reality` mapping: think about whether we want a claim fee at all. The
self-audit checklist mentions "pro-rata math is lossless" — that's a
choice, not a given. If we add a 2.5% claim fee we **need an explicit recipient**
or it gets stuck in the contract.

### 1.4 v3 audit findings (G0 group, August 2021)

Source: `packages/contracts/audits/RealityETH-3.0.txt`, exported from
`docs/Audit_Reality_v3_202108.pdf` by Edmund Edgar.

Three medium-severity issues, all fixed pre-deployment at the commit
`e4584d7cf6ab2d9a5b129bd970b7d4517811ae6a`:

**Issue 1 — Incorrect bond payout on UNRESOLVED_ANSWER (medium).**
*"When last answer is unrevealed and best answer is UNRESOLVED_ANSWER, the
bond from the unrevealed answer will be paid to address(0) instead of to the
winner which would we be the case if the best answer wasn't an
UNRESOLVED_ANSWER."* — the interaction between commit-reveal pending answers
and the "answered too soon" sentinel routed bonds to the zero address.

**Issue 2 — Arbitration can be initiated with no valid answer, premature finalisation after cancel (medium).**
*"Arbitration can be initiated after an answer commitment is posted, even if
no revealed answers have been posted, after this arbitration is cancelled the
question will be finalised after finalize_ts seconds, even though no answer
has been provided. In results all bonds will be paid out to address(0),
bounty will become unretrievable and best_answer will be set to 0."* —
the `cancelArbitration` path (added in v2.1) interacted dangerously with
commit-only state. Bonds go to `address(0)`, bounty becomes permanently
locked.

**Issue 3 — `answer_takeover_fee` calculation depends on processing order (medium).**
*"In claimWinnings answer_takeover_fee is inconsistently calculated, it is
being subtracted from bounty if the winning answer and second best answer are
processed in one go, but if they are processed separately, the whole bounty
amount is reserved for the winner and can't by used for the fee."*

`cw-reality` mapping:

- Issues 1 and 2 are both **"answer payout went to the null address"** bugs.
  Our self-audit-checklist already has "token-in equals token-out at every
  state transition" — **add a specific test**: every payout path must have a
  validated recipient before transfer is enqueued. No `addr.unchecked()`,
  no `Addr::unchecked("")`, no defaulting.
- Issue 3 is **path-dependent accounting**. Our checklist says "sum of payouts
  equals sum of escrowed bonds" — but Issue 3 shows that an aggregate
  invariant can be true while *per-claim* amounts differ based on how many
  claims arrive in a single tx. Property test must check **per-claim
  determinism** as well: claiming separately vs. claiming together must yield
  identical balances at the end.

### 1.5 v3 → v4 — remove commit-reveal, add freezable

PR #133 (`V4 candidate 4 remove commit reveal`, merged 2024-01-26) explains
the rationale directly:

> *"Commit-reveal was intended to protect users against people stealing their
> answers and front-running them. In practice it was never used in a way where
> that mattered, ie people have never used it to get questions answered while
> competing for a bounty, and instead use it to prove things they already think
> they know. In any case we now have private RPCs and other front-running
> defences. Removing it makes the code simpler and also makes integrations
> easier."*

Source: `gh pr view 133 --repo RealityETH/reality-eth-monorepo`.

This is **extremely important for `cw-reality`**: do not ship commit-reveal in
v1. The mechanism's complexity caused both v3 audit Issues 1 and 2 above. The
author of the contract concluded after ~3 years of mainnet use that the feature
**wasn't worth its complexity**. We inherit that conclusion. Note that
CosmWasm doesn't have an equivalent of "private RPC" defaults, but the
self-audit checklist already addresses front-running concerns at the
state-machine level (no race between submit-answer and dispute-answer in the
same block).

PR #147 (`Feature freezable reality eth 3`, merged 2024-02-27) adds a
"freezable" abstract contract that lets a parent adjudication contract
(Backstop / Subjectivocracy) **pause** claim/withdraw while it investigates a
disputed adjudicator. From the commit:
*"Make an abstract contract that can be frozen and locked down."* The
underlying motivation is Subjectivocracy / Backstop L2 work (see
`RealityETH/subjectivocracy` repo). Not relevant for `cw-reality` v1, but
worth noting as a pattern if/when we want a "pause window" hook.

PR #138 (`Split the reopening feature into its own version`, merged 2024-02-06)
moved `reopenQuestion` out of the mainline contract. Indicates Reality.eth's
author considers reopening a niche feature.

PR #134 (`Small refactor, add function to verify history`, merged 2024-01-26
candidate). From the PR body:
> *"This PR adds the function needed to verify that an earlier bond was
> provided for a particular answer. We want this in Backstop to allow you to
> freeze an adjudicator while we wait to see if they're bad. […] The function
> name also specifies that the question must be unfinalized. This is intended
> to avoid making a footgun where a developer calling this doesn't realize
> that history validation will working if someone claims their rewards,
> because we delete the history when that happens."*

That last sentence is the **load-bearing lesson**: in Reality.eth, **claim
deletes history**. So `verifyHistory` only works pre-finalization. For
`cw-reality`, decide explicitly whether we delete history on claim. If we
keep history forever, we pay storage; if we delete, we must mark
"verifyable" queries with the same "unfinalized only" caveat.

### 1.6 Carried-forward design decisions worth flagging

Five Reality.eth designs that look optional but turned out to be load-bearing:

1. **Bond must double, never just be greater.** A strict 2x doubling rule
   prevents "increment by 1" griefing where an adversary keeps the question
   alive with negligible additional capital. `cw-reality`'s
   `bond ≥ current_bond * 2` rule must be enforced **even at high rounds**
   — checklist item "off-by-one at high rounds" addresses this. Reality.eth's
   v4 still has it:
   ```solidity
   if (current_bond == 0) {
       if (tokens < (questions[question_id].min_bond)) revert BondMustExceedTheMinimum();
   } else {
       if (tokens < (current_bond * 2)) revert BondMustBeDoubleAtLeastPreviousBond();
   }
   ```

2. **`max_previous` (max_bond_seen) parameter at answer time.** Every
   `submitAnswer` accepts a `max_previous` parameter that reverts if the
   actual current bond exceeds it. This protects answerers from front-runs
   that crank the bond just before they're mined. We should mirror this in
   `cw-reality` — `DisputeAnswer { current_bond_seen: Uint128, .. }`.

3. **History stored as a hash chain, not an array.** Reality.eth's contract
   only stores the **most recent history hash**; the claimer supplies the
   full history when calling `claimWinnings`, and the contract verifies by
   re-hashing. This trades on-chain storage for transaction-time data
   submission. For `cw-reality` the gas profile is different — CosmWasm
   storage is cheaper relative to compute — but if we go beyond ~10
   dispute rounds we will want this. **Decision deferred to stage 2.**

4. **Templates stored as block numbers, not strings.** Reality.eth keeps
   `mapping(uint256 => uint256) public templates;` storing the **block number**
   of creation, then emits the template content in an event. Off-chain
   indexers reconstruct from the event. We can do the same with CosmWasm
   attributes; saves a lot of state.

5. **`opening_ts`.** Questions can be created but not answerable until a
   timestamp. Used heavily by SafeSnap: the proposal window is asked open,
   the answer window opens later. **Important for `cw-reality`** if we ever
   want a similar governance flow — design `ask_question` so an `opening_ts`
   future-extension is non-breaking.

---

## 2. SafeSnap / Zodiac Reality Module — design and deployment history

### 2.1 What SafeSnap is

SafeSnap is the **Gnosis Zodiac module** that pairs **Snapshot off-chain voting**
with **Reality.eth as the optimistic oracle for execution**. The module is
called `RealityModule` in code, formerly `DaoModule`. Repo:
`gnosisguild/zodiac-module-reality` (the original `gnosis/dao-module` redirects).

Mechanism: a DAO proposal is encoded as an array of transaction hashes; a
Reality.eth question is asked "did Snapshot proposal X pass and is its payload
Y?"; the question is answered (by anyone) with an initial bond; if the answer
isn't disputed before `timeout`, after a `cooldown` window the Safe executes
the transactions. Question text is:

```solidity
string(abi.encodePacked(proposalId, bytes3(0xe2909f), txsHash))
```

where `txsHash` is the ASCII hex of `keccak256(abi.encodePacked(txHashes))`
and each `txHash` is an EIP-712 hash of `(to, value, keccak256(data),
operation, nonce)`. Source:
[zodiac-module-reality/contracts/RealityModule.sol](https://github.com/gnosisguild/zodiac-module-reality/blob/main/contracts/RealityModule.sol).

### 2.2 GnosisDAO's chosen parameters (GIP-11, March 2021)

The first major DAO to enable SafeSnap was GnosisDAO itself, via
[GIP-11](https://forum.gnosis.io/t/gip-11-enable-safesnap/1250).

Parameters adopted (from the proposal as fetched 2026-05):

- **Minimum bond:** 10 GNO
- **Reality question timeout:** 48 hours
- **Proposal cooldown:** 48 hours
- **Proposal expiration:** 7 days
- **Arbitrator:** `0xffff...ffff` sentinel ("arbitration cannot be called")

The arbitrator choice is interesting: the forum discussion shows GnosisDAO
**explicitly disabled arbitration** rather than handing escalation power to a
multisig. *"granting the multisig arbitrator status would let them determine
who would win the escalation game of reality.eth"* — paraphrase of a forum
participant's objection.

So GnosisDAO's SafeSnap has **no on-chain escalation oracle**: disputes
resolve purely by who's willing to keep doubling the bond. That works if your
governance token is liquid and the DAO Safe itself can post counter-bonds; it
fails badly if no one's watching (see Section 3, SuDAO/Gnosis Guild attacks).

### 2.3 Other early adopters (March 2021 announcement cohort)

From the SafeSnap launch post on Gnosis Medium
([medium.com/gnosis-pm/.../ea67eb95c34f](https://medium.com/gnosis-pm/introducing-safesnap-the-first-in-a-decentralized-governance-tool-suite-for-the-gnosis-safe-ea67eb95c34f)):
*"The initial cohort includes: Yearn, SushiSwap, Synthetix, Balancer,
mStable, PoolTogether, dHedge, BrightID, Stakewise, EPNS, and GnosisDAO."*

Of these, **mStable** is documented as having actively run it
([Medium post Jul 2021](https://medium.com/mstable/mstabledao-safesnap-integration-115eda5a24d4)).
The others: enabled at announcement time but actual usage is undocumented in
the sources I could reach. By 2024, most are migrating to **oSnap**
(UMA-oracle-based) per the Snapshot docs deprecation notice.

### 2.4 Setup-guide warnings

The current `zodiac-module-reality/docs/setup_guide.md` carries two explicit
disclaimers:

- *"DISCLAIMER: Check the deployed Reality.eth contracts before using them."*
- *"DISCLAIMER: DO NOT BLINDLY COPY THE REQUIREMENTS. You should check the
  requirements and make the adjustments for your setup."*

And — critical — operational guidance: *"Because anyone can submit proposals
to your module, it is strongly recommended to put in place monitoring
practices."* Monitor `ProposalQuestionCreated` events.

The README does **not** provide recommended bond/timeout/cooldown values.
This is by design — the module's authors learned that recommending defaults
was dangerous (see SuDAO post-mortem, Section 3.1) — but the absence puts the
burden squarely on integrators.

---

## 3. Confirmed SafeSnap incidents (with caveats)

### 3.1 SuperUMAn DAO (SuDAO) — Polygon, 1 October 2022

**Sources:**

- [publish0x.com Technical Post Mortem](https://www.publish0x.com/everythingblockchain/a-technical-post-mortem-of-superuman-dao-sudao-hack-flaws-in-xozrzmj)
  (returned HTTP 403 to WebFetch; content from search-result excerpts)
- [Medium Coinmonks mirror](https://medium.com/coinmonks/a-technical-post-mortem-of-superuman-dao-sudao-hack-flaws-of-existing-governance-tools-553de4d4736e)

**Loss:** >$56K USD from SuDAO's Safe on Polygon.

**Timeline:**
- Sept 30, 2022: attacker submits a malicious proposal **directly to the
  Reality module** (not via Snapshot — the Reality module accepts any
  proposal, the Snapshot vote is *informational*, the oracle does not verify
  it).
- Attacker reuses the `proposalId` of a recently-passed legitimate proposal
  ("Additional Ambassador Compensation"). This is a UX defense-in-depth
  failure: at-a-glance the malicious proposal **looks like** the legitimate
  one in Snapshot's UI.
- Question timeout was **12 hours**.
- 12 hours later: no one disputes.
- Oct 1, 2022 03:34 UTC: proposal executes, treasury drained.

**Root cause (post-mortem framing):** *"in the absence of an arbitrator, the
design incentivizes the highest bidder (similar to the Dollar Auction game).
If the DAO fails to monitor such requests, malicious resolutions will pass
without the DAOs knowledge."*

**The actual mechanic** is more specific than "no arbitrator." It's:

1. The Reality module's question is `(proposalId, txsHashHash)`. Both are
   user-supplied at proposal time.
2. The Reality.eth contract has **no way to verify the Snapshot vote exists
   or passed**. It treats the question text as opaque.
3. So *anyone* can ask "did proposal X with payload Y pass?" and post a "yes"
   answer with a small bond. The 12-hour timeout means the DAO has 12 hours
   to notice and counter-bond — **on top of** noticing the question exists at
   all.

**Lesson for `cw-reality`:**

- If we ever build a "cw-snapshot" / DAO DAO module on top, the link between
  the off-chain vote and the on-chain proposal **cannot** be done at the
  oracle level. It has to be done either (a) by binding `proposalId` to an
  on-chain snapshot of the vote (CosmWasm DAO DAO already does this — voting
  is on-chain, no Snapshot needed), or (b) by an explicit on-chain
  registration step before any Reality question can be asked.
- The "any unknown account can ask any question" property of Reality.eth is
  intentional; the failure was using it as the only check. **Our checklist
  item "Arbitrator authentication" already covers the on-chain
  question-creator gate; add "proposal/payload binding" as a separate
  consideration if we build a DAO-execution module.**

### 3.2 Gnosis Guild's own deployment — Ethereum, 28 September 2022

**Source:** [QuillAudits analysis](https://quillaudits.medium.com/gnosis-guild-dao-proposal-attack-analysis-quillaudits-2e237cbd3f7c).

**Loss:** ~7.5 ETH from a victim's Safe (Gnosis Guild DAO's safe at
`0x8f9036732b9aa9b82d8f35e54b71faeb2f573e2f`).

**Timeline (per QuillAudits):**
- Sept 28, 2022: attacker pushed a malicious proposal named *"dead"* via
  Snapshot Labs, then submitted the corresponding `(proposalId, txsHash)`
  question to Reality.eth with a small bond.
- Question timeout: **1 hour**.
- 1 hour later: proposal executable, attacker drains 7.5 ETH.

**Same failure mode as SuDAO**, with an even shorter timeout. Gnosis Guild's
own DAO had configured a 1-hour timeout — meaning that if a monitor was
asleep for an hour, the attacker won.

**Lesson for `cw-reality`:**

- **Hard minimum timeout.** Reality.eth itself has no minimum; SafeSnap
  inherits the same. We should set a **contract-level lower bound** on
  `answer_timeout_secs` (e.g. 24 hours) for any question with a non-trivial
  bond. Or: minimum scales with bond size, so high-stakes questions can't be
  trapped by short windows. This is **not in the checklist yet** — add it.
- **The Reality Module's `markProposalWithExpiredAnswerAsInvalid` was added
  post-incident**, per QuillAudits: *"the documentation recommends that
  markProposalWithExpiredAnswerAsInvalid should be called immediately after
  any proposal expires to mark a proposal with an expired answer as
  invalid."* — but this is a band-aid; the actual fix is configuration
  hygiene (longer timeouts + monitors).

### 3.3 The "ENS DAO eventually paused SafeSnap" claim — UNVERIFIED

The brief asked specifically for the 2023 ENS DAO story. **I could not
corroborate it.** What I did find:

- ENS DAO governance runs on **Tally + Snapshot** without a Reality module
  bridge — [docs.ens.domains/dao/governance/process](https://docs.ens.domains/dao/governance/process/).
- ENS executable proposals are submitted via Tally and approved on-chain by
  delegated $ENS holders.
- The 2023 ENS governance discussions on the forum that came up in search
  ([discuss.ens.domains/t/.../19710](https://discuss.ens.domains/t/temp-check-governance-security-compensating-blockful-for-preventing-a-potential-attack-on-the-ens-dao/19710))
  are about a Tally-level governance attack vector (blockful.io discovery),
  **not** about SafeSnap.

It's possible the brief was conflating ENS DAO with another DAO that did
pause SafeSnap, or with the broader SafeSnap-Reality → oSnap migration. The
actual migration story is in Section 4 below.

If the original source of the brief's claim becomes available, this section
should be updated. As of 2026-05, treat the "ENS paused SafeSnap" claim as
**unverified and likely incorrect**.

### 3.4 The broader migration: SafeSnap-Reality → oSnap

This is well-documented and is the *real* answer to "what happened to SafeSnap":

- 2023-12 onwards: UMA launches oSnap (Optimistic Snapshot Execution),
  replacing Reality.eth as the oracle layer with UMA's Optimistic Oracle.
- 2024+: Snapshot deprecates the SafeSnap-Reality module from its main UI.
  Quoted at [Snapshot docs](https://docs.snapshot.box/v1-interface/plugins/safesnap-osnap):
  *"reality.eth is no longer supported with Snapshot's most recent UI."*
  Also: *"If you installed oSnap before December 4, 2023, you are likely using
  the Gnosis SafeSnap version. We recommend migrating to the oSnap Safe App,
  as it is actively maintained by UMA."*
- 1inch's migration discussion ([gov.1inch.network/t/.../793](https://gov.1inch.network/t/opening-discussion-on-migrating-from-reality-eth-to-osnap-for-proposal-execution/793))
  cites the same UI deprecation as the operational reason.

**Why does this matter for `cw-reality`?**

- Reality.eth's *mechanism* didn't fail; the *integrator-facing UX layer*
  did. The optimistic-oracle pattern is alive and well, but the specific
  combination (Reality.eth oracle + Snapshot proposal + Zodiac Safe module)
  was retired in favor of UMA's vertically-integrated version.
- If we want `cw-reality` to be used by DAOs on Juno, we need **either**
  (a) good docs and UI ourselves — Snapshot will not save us — **or**
  (b) deep DAO DAO integration (which is the obvious path here; DAO DAO's
  proposal modules are the CosmWasm analog of the SafeSnap module).
- oSnap's key parameters (per
  [docs.uma.xyz/developers/osnap/osnap-configuration-parameters](https://docs.uma.xyz/developers/osnap/osnap-configuration-parameters))
  recommend **minimum 1,500 USDC or 1 WETH bond** for production DAO use.
  Compare to Reality.eth's "set any min_bond." That's a learned lesson:
  defaults matter, low bonds enable cheap attacks.

---

## 4. Failure-mode catalog (the actionable list)

Numbered failure modes the port must be robust against. Each: one-line
description, citation, fix Reality.eth used, equivalent CosmWasm
consideration.

### FM-1. Tiny initial bond enables griefing

- **Description:** v1/v2/v2.1 allowed any non-zero initial answer bond
  (down to 1 wei). A griefer plants a wrong answer with a microscopic bond;
  anyone wanting to overturn it begins the doubling race from a base too low
  to make the dispute economically meaningful.
- **Citation:** v3 `askQuestionWithMinBond` introduction, diff at
  `Realitio_v2_1.sol` vs `RealityETH-3.0.sol`, lines around the
  `bondMustDoubleAndMatchMinimum` modifier.
- **Fix Reality.eth used:** added a per-question `min_bond` parameter,
  enforced for the first answer.
- **`cw-reality` consideration:** `initial_bond` field on `AskQuestion`
  must be required and **the contract should enforce a platform-wide
  floor** independent of the asker's value. Already in `PLAN.md`'s
  ExecuteMsg sketch — add an instantiation-time `min_initial_bond`
  config.

### FM-2. Bond payout to address(0)

- **Description:** edge cases involving `UNRESOLVED_ANSWER` and
  cancelled-arbitration paths routed bond payouts to the zero address.
- **Citation:** v3 audit `audits/RealityETH-3.0.txt` Issues #1 and #2.
- **Fix Reality.eth used:** explicit check for `UNRESOLVED_ANSWER` and
  unrevealed-answer interaction; require revealed answer before arbitration
  can be initiated.
- **`cw-reality` consideration:** every transfer path must enqueue a
  validated `Addr` (a parsed `info.sender` or a stored question field), never
  a default or zero address. The CosmWasm analog of `address(0)` is a
  contract balance that gets stuck — equally bad. Add a property test:
  every successful `Claim` strictly decreases the contract balance and
  increases exactly one external balance.

### FM-3. Path-dependent payout math

- **Description:** v3 audit Issue #3 — `answer_takeover_fee` calculated
  differently if you claim winning + second-best together vs. separately.
  Aggregate accounting was sound; per-claim accounting wasn't.
- **Citation:** `audits/RealityETH-3.0.txt` Issue #3.
- **Fix Reality.eth used:** unified the fee subtraction into a single code
  path; payout amount is now order-independent.
- **`cw-reality` consideration:** add an explicit property test —
  "claiming round-by-round vs. claiming all rounds at once must produce
  identical final balances." Currently `self-audit-checklist.md` has "sum
  of payouts equals sum of escrowed bonds"; sum-equals-sum can be true while
  per-claim amounts differ.

### FM-4. Commit-reveal complexity bugs

- **Description:** commit-reveal added two-step front-running protection;
  in practice it caused both v3 audit Issues #1 and #2 (interaction with
  `UNRESOLVED_ANSWER`, with cancelled-arbitration). Edmund's own conclusion
  (PR #133): *"In practice it was never used in a way where that mattered."*
- **Citation:** PR #133 body, merged 2024-01-26.
- **Fix Reality.eth used:** removed entirely in v4.
- **`cw-reality` consideration:** **do not ship commit-reveal in v1.**
  CosmWasm doesn't have the same MEV exposure as Ethereum L1 anyway; and
  Juno block times are 6 seconds, so a "submit answer in same block as
  someone else" race is much narrower than EVM. If we add anti-front-running
  later, do it at the sequencer level, not the contract.

### FM-5. Question-ID collision across deployments

- **Description:** v2.1 question IDs did not include the contract address;
  two Reality.eth deployments on the same chain could share IDs.
- **Citation:** diff `Realitio_v2_1.sol@aac6a3c` vs `RealityETH-3.0.sol@e4584d7`,
  the keccak input change adding `address(this)`.
- **Fix Reality.eth used:** added `address(this)` and `min_bond` slot to
  the keccak input.
- **`cw-reality` consideration:** if we use content-derived IDs (which
  helps off-chain indexing), include the contract address. If we use a
  monotonic counter, this is moot. **Make the call in stage 2 and document
  it.**

### FM-6. Bond-doubling not enforced at high rounds (off-by-one risk)

- **Description:** `current_bond * 2` must be enforced exactly. Reality.eth's
  v4 still uses literal `current_bond * 2`. Off-by-one (`>=` vs `>`) or
  overflow at extreme bond sizes can break the doubling invariant.
- **Citation:** `RealityETH-4.0.sol` line 83 (audit-pinned via repo main).
- **Fix Reality.eth used:** strict `>=` check using Solidity 0.8+ overflow
  panics.
- **`cw-reality` consideration:** Use `Uint128::checked_mul` and
  `checked_add`, panic on overflow (cosmwasm-std default). Add proptest
  ranges covering 1 → `Uint128::MAX / 2` for the doubling math. Already in
  `self-audit-checklist.md`.

### FM-7. `max_previous` front-run protection

- **Description:** between the moment a user signs an answer and the
  moment it lands on-chain, an attacker can submit a doubling answer first,
  forcing the user's transaction to revert *or* to silently succeed at a
  higher bond than they expected. v1+ added a `max_previous` parameter to
  `submitAnswer`: revert if the actual current bond exceeds the user's
  expectation.
- **Citation:** function signature `submitAnswer(bytes32, bytes32, uint256
  max_previous)` in `Realitio.sol` since v1.
- **Fix Reality.eth used:** parameter on every state-changing answer
  function.
- **`cw-reality` consideration:** add `current_bond_seen: Uint128` to
  `SubmitAnswer` and `DisputeAnswer`. Already implicit in checklist's
  "finalization races" item but **call it out explicitly** — this is the
  Reality.eth standard pattern.

### FM-8. No timeout floor enables drive-by attacks

- **Description:** SuDAO (12 hours) and Gnosis Guild (1 hour) configured
  timeouts too short for human monitoring. Both were drained.
- **Citation:** SuDAO post-mortem ([Coinmonks](https://medium.com/coinmonks/a-technical-post-mortem-of-superuman-dao-sudao-hack-flaws-of-existing-governance-tools-553de4d4736e));
  QuillAudits Gnosis Guild analysis.
- **Fix Reality.eth used:** none at contract level. Operational guidance to
  use longer timeouts and run monitors.
- **`cw-reality` consideration:** **enforce a contract-level minimum**
  for `answer_timeout_secs`. Suggested: 24 hours. Scale with bond size for
  high-stakes questions. **This is a new checklist item — add it.**

### FM-9. Oracle question is opaque; integrator must verify off-chain fact

- **Description:** Reality.eth has **no way to verify** the
  off-chain fact (Snapshot vote outcome, real-world event). It only verifies
  that someone is willing to bond for an answer. Both SafeSnap incidents
  exploited the fact that the Reality module's question text is
  user-supplied — the link to the actual Snapshot vote is informational, not
  cryptographic.
- **Citation:** SuDAO post-mortem; oSnap migration thread.
- **Fix Reality.eth used:** none — this is a design principle, not a bug.
  Mitigation is operational (monitor + arbitrate).
- **`cw-reality` consideration:** if `cw-reality` ever becomes the oracle
  for a `cw-dao-proposal-reality` proposal module, **the proposal module
  itself** must do the binding, not the oracle. Specifically: the proposal
  module must require an on-chain proposal exists before any Reality
  question can be asked about it, and the question's `content_hash` must
  include the on-chain proposal ID. Add this as a stage-5 (mainnet
  integration) checklist item.

### FM-10. Claim deletes history, breaking later verification

- **Description:** Reality.eth `claimWinnings` deletes the answer history.
  Functions that verify the history (added in PR #134) only work
  pre-finalization. A developer who doesn't realize this builds a footgun.
- **Citation:** PR #134 body, quoted in Section 1.5 above.
- **Fix Reality.eth used:** explicit function naming convention
  (`verifyHistoryUnfinalized`) to make the constraint visible.
- **`cw-reality` consideration:** decide whether `cw-reality` deletes
  history on claim. **Recommendation: keep it.** CosmWasm storage is
  cheaper than EVM storage; the safety win is worth the bytes. If we delete,
  every query that reads history must check finalization first and return a
  distinct error if it's gone.

### FM-11. Question text mutability

- **Description:** Reality.eth questions are immutable post-creation.
  Templates and parameters baked into the question hash. This is a feature,
  not a bug — it means the answer is tied to a specific text — but it forces
  the "reopen with a corrected question" pattern instead of "edit the
  question."
- **Citation:** `Realitio.sol`+ all subsequent versions; content_hash is
  part of `question_id`.
- **Fix Reality.eth used:** N/A — it's intentional.
- **`cw-reality` consideration:** already in `PLAN.md` open design call
  #2 ("question-text mutability"). **Settle: questions are immutable.** This
  is the Reality.eth precedent and it's correct.

### FM-12. ERC777-style reentrancy

- **Description:** v3 audit's standing warning: *"RealityETH_ERC20-3.0.sol
  should not be used with ERC20-like token contracts that implement callbacks
  like ERC777 due to potential re-entrancy issues."*
- **Citation:** `audits/RealityETH-3.0.txt` final note.
- **Fix Reality.eth used:** none — out-of-scope; integrators told to
  avoid hostile token types.
- **`cw-reality` consideration:** CosmWasm's `Cw20ReceiveMsg` hook is
  the equivalent: a hostile cw20 could be the answer's bond denom and call
  back into the contract. Already in `self-audit-checklist.md` under
  "Reentrancy and submsg" but the **specific test must be** "use a cw20 that
  re-enters Claim from within a transfer; verify no double-spend." Add it.

---

## 5. Open questions from the reading list — resolved positions

The reading list `docs/reality-eth-reading-list.md` had 7 open questions.
Resolved positions based on the above:

1. **Same question twice?** Yes — `nonce` parameter on `askQuestion` is
   user-controlled and re-using the same nonce reverts (because the
   `question_id` would collide). Different nonce → different question.
   For `cw-reality`, expose the nonce.

2. **Loser-bond redistribution math with >2 rounds and different bidders?**
   Reality.eth iterates the history hash from the final answer backwards:
   each prior wrong answer's bond is paid to the highest-bonded later
   correct answerer. Math is in `_processHistoryItem` of
   `RealityETH-3.0.sol`. Read alongside v3 audit Issue #3 — the
   accounting is correct in aggregate; pay attention to per-claim
   determinism.

3. **Asker disappears — bond reclamation paths?** Asker's "bond" is the
   bounty (deposited at ask time). Bounty goes to the winning answerer if
   there is one, or stays in the contract if the question never gets an
   answer (no automatic refund). Question fee taken by the arbitrator.
   For `cw-reality`, decide if we want an auto-refund-after-N-blocks path
   for unanswered questions. **Recommendation: yes, refund after a long
   inactivity window** — Reality.eth's lack of this is a footgun.

4. **Gas profile of escalation as bond grows?** Each
   `submitAnswer`/`disputeAnswer` is O(1) — they only update the latest
   history hash. Storage cost is O(rounds) but written incrementally. The
   *claim* is O(rounds) because it walks the history. For CosmWasm with
   cheaper storage and explicit gas metering, this is fine. **Disputed-answer
   cardinality cap (PLAN.md open call #3): pick 32 or 64 rounds** —
   Reality.eth has none in practice but no real question goes past ~15.

5. **Answer-schema validation?** Reality.eth has nothing equivalent —
   answers are `bytes32`, the template says how to interpret them, but the
   contract doesn't validate. cw-filter at `cw-reality` is a *strict
   improvement* — but introduces FM-12-style concerns (hostile filter wasm).
   Already in checklist under "cw-filter integration."

6. **Uncle-answer rule?** No. Reality.eth's history is a single chain; an
   answer that didn't win the doubling race never claims anything,
   regardless of what later rounds do. There's no "if your answer was right
   but you didn't keep raising, you still get a partial payout." This is
   *simple*; we should keep this property.

7. **Arbitrator decline?** Two paths. (a) Arbitrator simply doesn't call
   `submitAnswerByArbitrator` — eventually `cancelArbitration` can be
   called (by the requester, after a timeout) which returns the question to
   the open state. (b) Arbitrator calls `submitAnswerByArbitrator` with
   `UNRESOLVED_ANSWER` to actively decline. Both are supported.

   For `cw-reality`: `InvokeArbitration` should record a deadline;
   `SubmitArbitration` after the deadline reverts; the question
   auto-cancels back to the "open" state on a separate `CancelArbitration`
   call. Match Reality.eth's pattern; the explicit-decline-via-sentinel
   needs a typed answer variant (`Answer::Unresolved`).

---

## 6. Sources index

Primary contract sources (commit-pinned):

- `realitio/realitio-contracts/truffle/contracts/Realitio.sol` —
  [github.com/realitio/realitio-contracts/blob/master/truffle/contracts/Realitio.sol](https://github.com/realitio/realitio-contracts/blob/master/truffle/contracts/Realitio.sol)
- `realitio/realitio-contracts/truffle/contracts/Realitio_v2_1.sol` —
  [github.com/realitio/realitio-contracts/blob/master/truffle/contracts/Realitio_v2_1.sol](https://github.com/realitio/realitio-contracts/blob/master/truffle/contracts/Realitio_v2_1.sol)
- `RealityETH/reality-eth-monorepo` `RealityETH-3.0.sol` at
  `e4584d7cf6ab2d9a5b129bd970b7d4517811ae6a` (audit-pinned)
- `RealityETH/reality-eth-monorepo` `RealityETH-4.0.sol` at `main`

Audits:

- `packages/contracts/audits/RealityETH-2.0.rst` (Kofler, v2 / RealityCheck era)
- `packages/contracts/audits/RealityETH_ERC20-2.0.txt` (Kofler, June 2019)
- `packages/contracts/audits/RealityETH-3.0.txt` (G0 group, August 2021)
- `packages/contracts/audits/RealityETH_ERC20-3.0.txt` (see RealityETH-3.0.txt)

PRs that mattered (Reality.eth monorepo):

- [#131 — V4 formatting and linting](https://github.com/RealityETH/reality-eth-monorepo/pull/131)
- [#133 — V4 candidate 4 remove commit reveal](https://github.com/RealityETH/reality-eth-monorepo/pull/133) — quoted in Section 1.5
- [#134 — verify history function](https://github.com/RealityETH/reality-eth-monorepo/pull/134) — quoted in Section 1.5
- [#138 — Split reopening into its own version](https://github.com/RealityETH/reality-eth-monorepo/pull/138)
- [#140 — Move duplicated code into a common contract](https://github.com/RealityETH/reality-eth-monorepo/pull/140)
- [#147 — Freezable Reality.eth v3](https://github.com/RealityETH/reality-eth-monorepo/pull/147)

SafeSnap / Zodiac:

- [gnosisguild/zodiac-module-reality](https://github.com/gnosisguild/zodiac-module-reality)
- [GIP-11: Enable SafeSnap (Gnosis forum)](https://forum.gnosis.io/t/gip-11-enable-safesnap/1250)
- [Setup guide](https://github.com/gnosisguild/zodiac-module-reality/blob/main/docs/setup_guide.md)
- [Snapshot SafeSnap docs](https://docs.snapshot.box/v1-interface/plugins/safesnap-reality)
- [SafeSnap launch post (Gnosis Medium)](https://medium.com/gnosis-pm/introducing-safesnap-the-first-in-a-decentralized-governance-tool-suite-for-the-gnosis-safe-ea67eb95c34f)

Incidents:

- [QuillAudits — Gnosis Guild attack analysis](https://quillaudits.medium.com/gnosis-guild-dao-proposal-attack-analysis-quillaudits-2e237cbd3f7c)
- [Coinmonks / Everything Blockchain — SuDAO post-mortem](https://medium.com/coinmonks/a-technical-post-mortem-of-superuman-dao-sudao-hack-flaws-of-existing-governance-tools-553de4d4736e)
- [publish0x mirror — SuDAO post-mortem](https://www.publish0x.com/everythingblockchain/a-technical-post-mortem-of-superuman-dao-sudao-hack-flaws-in-xozrzmj) (HTTP 403 to WebFetch; cached in search results)

Migration to oSnap:

- [Snapshot oSnap docs](https://docs.snapshot.box/v1-interface/plugins/safesnap-osnap)
- [1inch — migration discussion](https://gov.1inch.network/t/opening-discussion-on-migrating-from-reality-eth-to-osnap-for-proposal-execution/793)
- [oSnap configuration parameters (UMA)](https://docs.uma.xyz/developers/osnap/osnap-configuration-parameters)

Reality.eth own docs:

- [Whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)
- [Using Reality.eth from a contract](https://reality.eth.limo/app/docs/html/contracts.html)
- [v3 audit (HTML export)](https://reality.eth.link/app/docs/html/audit_v3.html)

Edmund Edgar's blog (Medium @edmundedgar):

- [Snopes meets Mechanical Turk (Oct 2017)](https://medium.com/@edmundedgar/snopes-meets-mechanical-turk-announcing-reality-check-a-crowd-sourced-smart-contract-oracle-551d03468177) — original announcement
- [Reality Check Bug Bounty (May 2018)](https://medium.com/@edmundedgar/reality-check-bug-bounty-46054a94820e)
- [Realitio mainnet trial (Aug 2018)](https://medium.com/@edmundedgar/realitio-the-crowd-sourced-smart-contract-oracle-now-in-a-real-money-trial-on-mainnet-f46bf016759d)
- [The Parasite and the Whale (Apr 2019)](https://medium.com/@edmundedgar/the-parasite-and-the-whale-7cb3c87e9902) — Augur/UMA criticism

---

## 7. What this file feeds

- **`docs/self-audit-checklist.md`** — add explicit items for FM-3 (per-claim
  determinism), FM-8 (timeout floor), FM-9 (oracle/integrator binding), FM-10
  (history-on-claim policy), FM-12 (hostile cw20 reentry test).
- **`PLAN.md` stage 2 (contract build)** — open design calls #2 and #3 are
  now resolvable: questions are immutable, dispute cardinality cap at 32 or
  64 rounds.
- **`memory/reality-on-cosmwasm.md`** — long-term lessons. The big ones to
  carry forward: (a) min_bond is non-negotiable, (b) commit-reveal is not
  worth it, (c) ERC777-style reentrancy is the analog of hostile cw20, (d)
  SafeSnap failed at the **integration boundary**, not the oracle — DAO DAO
  integration is the right CosmWasm shape, not a generic "Snapshot bridge."
