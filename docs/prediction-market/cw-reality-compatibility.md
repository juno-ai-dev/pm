# R3 — cw-reality compatibility

**Status:** accepted integration specification (2026-07-16), authority profile amended 2026-07-17 by issue #45; audit/build and deployment evidence remain open
**Canonical source:** local commit ee641534fd7b7b3677bd48d30390422ee3fbe5ed
**Schema:** checked-in cw-reality 0.1.0-alpha.1 combined schema, SHA-256 a50ecbb0…20af
**Deployment snapshot:** Juno height 39,829,829

## Compatibility decision

Use cw-reality unchanged. Each market is its question's configured arbitrator-controller. Public users answer and counter-answer directly on cw-reality. A public market challenge escrows a separate bond and makes the market atomically call RequestArbitration. Only the market's immutable `verdict_authority` may invoke GovernanceVerdict, which forwards SubmitArbitration so cw-reality observes the configured market sender. V1 pins the Juno Agents DAO core `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`; future x/gov support remains address-compatible but deferred under #4/#13.

No existing cw-reality production address is acceptable as an immutable dependency. The accepted canary dependency is a new instance of independently audited code, instantiated with no chain migration admin and InstantiateMsg.admin = None, a 10,000,000-ujuno initial-bond floor, and an 86,400-second answer-timeout floor. Factory tiers also pin its address, code ID, checksum, and config. These values are accepted for implementation; audit, reproducible-build, and deployment evidence remain open.

## Field-by-field gate

| Property | Source behavior | Market requirement |
| --- | --- | --- |
| Oracle address | CosmWasm query target is caller-selected | Immutable tier value; reject any other address |
| Oracle checksum | Not returned by a smart query | Factory tier records code ID/checksum established by chain evidence; deployment tooling rechecks |
| Question asker | Stored, included in ID | Must equal the market address |
| Text | Stored; SHA-256 content hash enters ID | Must byte-equal immutable resolution document |
| Answer type | Stored but omitted from ID | Must equal Bool |
| Bond denom | Stored and length-prefixed into ID | Must equal ujuno |
| Initial/min bond | Stored and enters ID | Must equal accepted tier floor or higher |
| Answer timeout | Stored and enters ID | Exactly 86,400 seconds for v1 |
| Arbitrator | Stored and enters ID | Must equal market address |
| Arbitration timeout | Stored but omitted from ID | Exactly accepted value: 1,814,400 seconds (21 days) |
| Answer schema | Stored but omitted from ID | None in v1; a filter is not a financial safety boundary |
| Opening timestamp | Stored and enters ID | Must equal immutable market opening_ts and be at or after close_ts |
| Nonce | Stored and enters ID | Factory-assigned immutable nonce |
| Bounty | Stored but omitted from ID | At least accepted tier bounty, funded separately at creation |
| State | Time-derived | Activation requires OpenUnanswered |
| Current answer/bond | Mutable | Snapshot on challenge; finalized values rechecked at resolution |

At resolution the market first queries FinalAnswerIfMatches with tier minimum final bond, 86,400-second timeout, market address as required arbitrator, and ujuno denom. It then queries Question and rechecks every immutable field above plus the expected question ID. The payout is stored only if both responses agree on answer and final bond and the question state is Finalized or Claimed.

## Atomic question creation and ID binding

AskQuestion emits the question ID but returns no response data. Event parsing is not a safe sole binding. The market instead reproduces the exact id.rs algorithm:

~~~text
sha256(
  canonicalize(oracle_contract_address)
  || canonicalize(market_address)
  || nonce as u64 big-endian
  || sha256(exact UTF-8 question text)
  || arbitrator-present tag 0x01
  || canonicalize(market_address)
  || answer_timeout as u32 big-endian
  || initial_bond as u128 big-endian
  || byte_length("ujuno") as u32 big-endian
  || UTF-8 "ujuno"
  || opening_ts as u64 big-endian
)
~~~

Creation sequence:

1. Factory instantiates the market with creator funds and typed immutable tier/question fields.
2. Market injects pinned fields, constructs the exact JCS question text, stores status Initializing, and sends AskQuestion to the pinned oracle with the market as CosmWasm sender and the bounty as native ujuno.
3. Oracle creates the question or the transaction reverts.
4. Market reply computes the ID using the pinned source algorithm, queries Question at that ID, and compares every field.
5. Only an exact match changes status to Trading and emits market_activated. Any mismatch errors the reply and atomically rolls back the market, question, and funds.

This avoids depending on Instantiate2 for question ownership. Instantiate2 remains useful for predictable indexing but is not load-bearing.

The implementation phase must maintain a cross-language golden-vector test for the ID algorithm, including bech32 canonical lengths. Any oracle code change that changes id.rs requires a new market code version and tier.

## Exact result mapping

The market never parses text or a loose boolean:

| Meaning | Length | Hex bytes | Payout (YES, NO) |
| --- | ---: | --- | --- |
| NO | 32 | 0000000000000000000000000000000000000000000000000000000000000000 | (0, 1) |
| YES | 32 | 0000000000000000000000000000000000000000000000000000000000000001 | (1, 0) |
| INVALID | 32 | ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff | (1/2, 1/2) |
| UNRESOLVED | 32 | fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe | (1/2, 1/2) |
| Any other finalized Binary, including wrong length | any | exact opaque bytes | (1/2, 1/2) |

INVALID follows Reality.eth's max-u256 convention. UNRESOLVED is the source constant. cw-reality itself does not establish the bool convention, so this table is a market protocol decision recorded in ADR-014.

## Normal and counter-answered settlement

~~~text
close/open boundary
      |
      +--> no answer ------------------------------> remains OpenUnanswered
      |
      +--> answer A, bond b --> 24-hour clock
                                  |
                                  +--> later answer B, bond >= 2b
                                  |       --> new 24-hour clock
                                  |
                                  +--> no answer/challenge until deadline
                                          --> oracle Finalized
                                          --> anyone calls market Resolve
                                          --> guarantees + full fields checked
                                          --> one-time payout stored
~~~

Trading fails at block.time >= close_ts independently of whether any close call occurs. An answer cannot land before opening_ts. If opening_ts equals close_ts, same-block ordering is safe: trading is already rejected and answering is allowed.

## Public challenge and governance verdict

Accepted canary challenge bond:

~~~text
required = max(10,000,000 ujuno, oracle.current_bond)
~~~

The accepted floor and formula are implementation requirements. The challenge accepts exactly one native ujuno coin and no extras.

~~~text
oracle OpenAnswered before finalize_ts
      |
      +--> Challenge(required bond, current_bond_seen)
             market queries question and snapshots:
               answer, current bond, finalize_ts, challenger, deadline
             market stores challenge liability
             market calls RequestArbitration with current_bond_seen
             entire transaction is atomic
                    |
                    +--> DAO proposal executes before arbitration deadline
                    |      sender = pinned Juno Agents DAO core
                    |      GovernanceVerdict(answer, payee)
                    |      market forwards SubmitArbitration
                    |      cw-reality finalizes
                    |      verdict != snapshot: refund challenge bond
                    |      verdict == snapshot: credit challenge bond to LP
                    |
                    +--> deadline reached without accepted verdict
                           anyone calls FinalizeStalledChallenge
                           market calls or observes CancelArbitration
                           full challenge bond credited to LP
                           oracle answer clock restarts for 24 hours
~~~

Only one pending challenge exists. Challenge rejects if the oracle is already Finalized, PendingArbitration, unanswered, or at/after finalize_ts. The oracle query and RequestArbitration execute in one transaction, while current_bond_seen protects the forwarded call.

The challenge bond never enters P, the AMM, oracle bounty, or trading fees. A different governance verdict includes canonical neutral or any other byte string and earns refund. An identical verdict loses the bond. This objective comparison does not claim governance was correct; it prices the act of forcing governance to reconsider the current answer.

At block.time >= arbitration_deadline, GovernanceVerdict rejects even if nobody has yet called CancelArbitration. This removes same-time ambiguity. Anyone may call cw-reality CancelArbitration directly after its deadline, so FinalizeStalledChallenge must also handle an already-cancelled oracle by querying it, slashing once, and returning the market to AwaitingResolution.

A rejected, failed, stale, never-deposited, or absent proposal is indistinguishable to the market unless it executes a verdict. All therefore take the timeout branch and lose the challenge bond. That harsh rule is the anti-freeze safe default and must be prominent before a challenger signs.

## V1 DAO DAO feasibility

The initial authority is the Juno Agents DAO core `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`. A passed proposal must execute GovernanceVerdict from that exact core. Members, proposal modules, voting modules, EOAs, and other contracts fail authentication. Issue #45 requires contract-level sender/failure coverage and a reviewable non-broadcast proposal packet. No live proposal, vote, execution, funding, or gas evidence is claimed or authorized by this memo. DAO core code, modules, membership, and voting-rule changes are external trust risks even though each market's authority address cannot rotate.

## Deferred x/gov feasibility evidence

Observed at height 39,829,829:

- governance module account: juno10d07y265gmmuvt4z0w9aw880jnsr700jvss730;
- standard minimum deposit: 5,000 JUNO;
- maximum deposit period: 10 days;
- voting period: 5 days;
- accepted arbitration window: 21 days, leaving six days after the maximum standard timeline.

The [Cosmos SDK v0.50 x/gov specification](https://docs.cosmos.network/sdk/v0.50/build/modules/gov/README) says accepted proposal messages are executed by the governance module account. The intended inner message is conceptually:

~~~json
{
  "@type": "/cosmwasm.wasm.v1.MsgExecuteContract",
  "sender": "juno10d07y265gmmuvt4z0w9aw880jnsr700jvss730",
  "contract": "<immutable market address>",
  "msg": "<base64 JSON GovernanceVerdict with exact question_id, answer, payee>",
  "funds": []
}
~~~

This historical evidence preserves future x/gov compatibility; it is not the v1 authority profile and does not block DAO-based implementation. This is not a rehearsed proposal file. Juno CLI/protobuf encoding, signer acceptance, gas, and wasm submessage execution remain open until separately authorized work under #4/#13:

1. proposal creation with this exact inner sender and no funds;
2. deposit sponsorship and maximum deposit-period timing;
3. passed execution observed by market and oracle;
4. wrong market, question ID, answer encoding, and invalid payee rejection;
5. proposal rejection, expiry, passed-but-failed execution, and retry behavior;
6. execution near and at the arbitration deadline;
7. gas used by market forwarding and oracle history update;
8. event/indexer reconstruction and challenge-bond refund/slash.

The market validates the caller, exact question ID, pending challenge, pre-deadline time, empty attached funds, and payee address. The pinned authority remains trusted to choose both answer and payee because cw-reality accepts any `Binary`, validates the payee with `deps.api.addr_validate`, and provides no history-membership proof for SubmitArbitration. An unrecognized answer limits market-collateral payout harm by producing neutral, but that market-side mapping does not change the oracle history entry: a malicious payee can still redirect oracle bounty and bond winnings. This trust must not be hidden.

## Stalled and unanswered behavior

Stalled arbitration is bounded: at 21 days, public cancellation is possible, the challenge bond is slashed, and the answer gets another 24-hour window. Repeated arbitration is not possible for the same market after one challenge: v1 sets challenge_used permanently, so a griefing sequence cannot repeatedly invoke governance. Counter-answers may still reset the optimistic clock.

An unanswered question has no protocol terminal state. V1 does not add a privileged neutral override because success requires settlement from a finalized oracle answer. Controls are:

- creation-funded oracle bounty; accepted minimum 1 JUNO;
- monitoring at opening, +1 hour, +12 hours, +24 hours, and daily thereafter;
- a keeper runbook that submits a canonical answer with the required oracle bond;
- UI disclosure that no answer means collateral remains locked;
- a future re-question/emergency mechanism only through a new market version, never a hidden live-market admin.

This is a clearly disclosed non-termination condition, not neutral resolution.

## Source and documentation consistency

| Topic | Public contract documentation | Source/schema | Architecture |
| --- | --- | --- | --- |
| Submitted-answer restriction | ARBITRATION.md states there is no history-membership proof | Arbitrator may author any Binary | Unknown maps neutral for market payout only |
| Payee | ARBITRATION.md includes the required field and payout consequence | Required String, validated by the handler | Governance chooses it; market validates before forwarding |
| Adapters | README states no adapter contract ships in v1 | None exist | Consistent: market is controller; address permission only |
| Bool wire format | AnswerType suggests type | No encoding enforcement/tests | Exact 32-byte table above |
| Question return | surrounding Reality.eth precedent returns ID | cw-reality emits only | Compute locally and full-query in reply |
| Filter immutability | comments imply captured address is enough | address code may migrate | v1 answer_schema None |
| CW20 support | receive paths exist | Withdraw emits native BankMsg | v1 native only |
| Reopen | upstream Reality.eth docs describe reopen | no cw-reality reopen execute | no re-question in v1 |

## Implementation test matrix

- golden question-ID vectors and every omitted-field mismatch;
- atomic rollback when AskQuestion or reply verification fails;
- exact canonical and arbitrary noncanonical result bytes;
- challenge/finalize ordering one second before, at, and after each deadline;
- answer front-run guard and same-block counter-answer;
- direct oracle cancellation followed by market synchronization;
- all challenge verdict/timeout bond branches;
- governance sender spoofing and attached-fund rejection;
- oracle/factory/market checksum/admin mismatch in deployment tooling;
- unanswered monitoring and idempotent keeper attempts.
