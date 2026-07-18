# A3 — User journeys and acceptance cases

**Status:** accepted specification (2026-07-16)
**Accepted canary tier:** 100-JUNO initial pool, 200-JUNO cap, 2% fee, 10-JUNO oracle/challenge floor, 1-JUNO bounty

The amounts are accepted for implementation, not launch authorization.

Current on-chain acceptance-to-test traceability is maintained in
[`lifecycle-assurance.md`](lifecycle-assurance.md); factory, UI, indexer, ops, and
live-governance follow-ups are identified there explicitly.

## Creator and initial LP

1. Creator prepares typed resolution fields, sources, close/open times, and a tier-compliant market; the UI previews the exact bytes the market will construct.
2. Reference UI shows that the creator is the only v1 LP, liquidity is non-transferable and locked through resolution, and no admin can repair the question.
3. Creator sends Factory.CreateMarket with exactly 101 JUNO: 100 initial pool principal plus 1 oracle bounty.
4. Factory validates objective fields and instantiates an adminless market.
5. Market keeps 100 JUNO, asks cw-reality as the market address, and sends 1 JUNO bounty.
6. Nested reply computes and queries the question ID and every field. Any mismatch rolls everything back.
7. Factory records the active market. Creator owns fixed LP supply representing the initial pool, not a claim on trader positions.

Acceptance:

- A second denom, extra unlabeled coin, weak tier field, or ambiguous address mismatch rejects with no partial market/question.
- The queried oracle asker and arbitrator both equal market address.
- Factory and market chain admins are empty.
- Creator cannot remove liquidity, migrate, pause, edit text, or settle.

## Trader: buy, sell, and close

From the R1 worked example:

1. Balanced pool begins at 100 YES / 100 NO, quote 50%.
2. Trader queries a 10-JUNO YES buy and receives estimate 18.725318 YES, 0.2 JUNO fee, end quote about 54.661%.
3. Trader signs exact market address, 10 JUNO, selected YES, min_out, and a short deadline.
4. Execute recomputes current reserves. If output is below min_out, deadline passed, cap exceeded, or close reached, it fails atomically.
5. On success, trader owns internal YES; P is 109.8 JUNO and F is 0.2.
6. Trader later requests exactly 5 JUNO for at most 9.540206 YES. Execute returns 5, leaves 9.185112 YES, and F becomes 0.302041.
7. At block.time >= close_ts, Buy, Sell, and Split reject regardless of UI/indexer lag.

The trader cannot transfer the internal position. Before resolution, equal YES+NO can be merged for collateral even after close. After resolution, positions redeem under the stored payout.

## Normal YES

~~~text
close/open reached
answerer posts exact 32-byte YES + at least 10 JUNO oracle bond
oracle finalize_ts = answer block time + 86,400 seconds
no counter-answer or challenge
after finalize_ts, keeper/resolver calls market Resolve
market checks FinalAnswerIfMatches + full Question
market stores (YES=1, NO=0)
holders redeem; LP settles pool and fees
oracle answerer separately claims/withdraws oracle bounty/bonds
~~~

Acceptance:

- Resolve one second before oracle finality fails without state change.
- Resolve at/after finality succeeds once.
- Every YES unit pays one ujuno and every NO unit zero.
- Payout total over pool/users is P; F goes only to LP.
- Repeated Resolve and repeated redemption cannot pay twice.

## Normal NO

The flow is identical with exact 32-byte zero. Market stores (0,1). A UI must not infer NO from absence of YES or a short zero byte.

Acceptance:

- 32 zero bytes pay NO; one zero byte is noncanonical and neutral.
- YES balances remain queryable but redeem zero and burn only when holder requests.
- LP outcome value is its NO reserve plus F.

## Neutral INVALID/UNRESOLVED/unrecognized

1. Final oracle bytes are INVALID (all ff), UNRESOLVED (all ff except fe), or any other non-0/1 Binary.
2. Resolve passes oracle identity/finality checks and stores (1/2,1/2), plus the exact raw answer.
3. An address redeeming cumulative 9 YES units gets 4 ujuno and leaves one half-dust when it exhausts its position.
4. A later address with another odd numerator completes a pair; one ujuno accrues to the immutable LP.
5. Splitting the same address's redemption into calls gives the same whole payout.

Acceptance:

- Unknown bytes never stall or parse loosely.
- INVALID and UNRESOLVED remain distinguishable in queries/events even though payout matches.
- All redeemed whole payouts plus paired LP half-dust equal P when all positions redeem.
- Resolution starts T2 at 2P; every whole payout reduces it by two and every paired half-dust moves two into one LP whole-coin credit.
- Address splitting cannot increase trader payout.

## Counter-answer

~~~text
first answer NO with 10 JUNO at t0
counter-answer YES with >=20 JUNO at t1 < t0+24h
oracle best answer becomes YES
finalize_ts becomes t1+24h
~~~

The market remains AwaitingResolution. A stale UI showing t0 finality must not enable resolution because direct oracle query controls.

Acceptance:

- Counter below double or wrong denom fails at oracle.
- Resolve during the reset window fails.
- FinalAnswerIfMatches reports the current final bond, which must meet tier minimum.
- Multiple counter-answers can extend delay; UI shows current, not original, countdown.

## Bonded challenge and DAO resolution

Assume current oracle answer is NO with 20 JUNO bond:

1. Challenger reads exact answer, bond, finalize_ts, required C = max(10,20) = 20 JUNO, and the verdict-authority warning.
2. Before oracle finality, challenger sends exactly 20 JUNO to market Challenge with current_bond_seen = 20.
3. Market snapshots values, accounts C separately, and atomically calls RequestArbitration. Oracle becomes PendingArbitration.
4. A proposer prepares a Juno Agents DAO proposal whose wasm execute message names the market/question/answer/payee explicitly. No attached funds are permitted.
5. DAO members adjudicate under the DAO's current rules. The DAO core address, not a member, proposal module, or voting module, must execute the passed proposal.
6. Before the arbitration deadline, passed proposal execution calls market GovernanceVerdict.
7. Market authenticates the immutable Juno Agents DAO core and forwards SubmitArbitration. Oracle finalizes. Market reply stores payout.
8. If verdict bytes differ from snapshot NO, challenger gets 20 JUNO back. If identical, the 20 JUNO accrues to LP.

Acceptance:

- Challenger, creator, LP, EOA, factory, DAO member/module, or spoofed authority cannot call GovernanceVerdict.
- Wrong question/market, attached funds, invalid payee, or no pending challenge rejects.
- Arbitrator answer may be new; noncanonical bytes produce neutral.
- Governance-selected payee is forwarded and disclosed.
- Oracle failure rolls the whole verdict transaction back, including challenge settlement.

## Stalled/rejected/failed DAO proposal

1. Market remains PendingArbitration through the deadline because no verdict executed. It does not matter on-chain whether no proposal existed, voters rejected it, or execution failed.
2. At block.time >= deadline, any address may call FinalizeStalledChallenge. Alternatively anyone may have directly called cw-reality CancelArbitration.
3. Market calls or observes cancellation, reduces C once, credits full C to LP, and returns to AwaitingResolution.
4. cw-reality finalize_ts is now plus 24 hours.
5. Challenge cannot be used again. Answer/counter-answer and eventual normal Resolve remain possible.

Acceptance:

- GovernanceVerdict at the deadline rejects; timeout wins.
- A direct oracle cancellation cannot strand or double-slash C.
- Retry of a passed-but-failed proposal can work only before deadline.
- Challenger is warned before signing that DAO process failure loses C.

## Unanswered question

~~~text
opening_ts reached
oracle remains OpenUnanswered
market remains AwaitingResolution
alerts at +1h, +12h, +24h, then daily
answerers may post exact bytes with required bond
until one does, Resolve fails and positions/LP stay locked
~~~

The bounty is an incentive, not a guarantee. There is no creator, keeper, governance, or admin neutral override.

Acceptance:

- Passage of close/open/answer timeout alone never fabricates finality.
- UI states “unanswered—no maximum settlement time,” not “resolving soon.”
- Merge of complete sets remains available; unpaired positions remain locked.
- A later valid first answer starts the normal 24-hour flow.

## Answerer

The answerer interacts directly with the pinned cw-reality address and exact question ID:

1. Verify question text/hash, opening, current state/bond, denom, arbitrator, and timeouts.
2. Encode exactly 32-byte result; preview Binary/base64 and raw hex.
3. Attach required native ujuno and current_bond_seen.
4. After finality, supply verified history to Claim as required by cw-reality, then Withdraw ujuno.

Answering can lose the entire bond. Market position ownership does not authorize or subsidize an answer.

## Resolver and keeper

Resolvers/keepers have no special key. They:

- call Resolve after direct oracle finality;
- monitor unanswered/counter/challenge/deadline states;
- may call stalled synchronization;
- may answer with their own bond and risk;
- cannot choose a payout, get a protocol reward, or bypass checks.

All calls are idempotent or cleanly reject. A keeper service is replaceable; protocol safety does not rely on its identity, while liveness relies on somebody acting.

## Initial LP terminal journeys

Using the completed buy/sell state:

| Outcome | Pool claim | Fees | LP total | Initial 100-JUNO change |
| --- | ---: | ---: | ---: | ---: |
| YES | 95.512847 | 0.302041 | 95.814888 | −4.185112 |
| NO | 104.697959 | 0.302041 | 105.000000 | +5.000000 |
| Neutral | 100.105403 | 0.302041 | 100.407444 before later dust | +0.407444 before later dust |

Trader payouts complete the 105-JUNO bank reconciliation shown in R1. LP cannot claim trader backing and cannot claim before resolution.

Acceptance:

- LP partial burns equal one aggregate burn.
- Fee/slash/dust accrual cannot be claimed by another address.
- Later paired neutral dust remains claimable after base LP units are burned.
- Lost LP key does not permit creator/governance recovery.

## Partial redemption and forced funds

A holder can redeem any balance subset. Valid winning claims pay exact units. Neutral claims use cumulative address accounting. State is updated before transfer, and failed send reverts.

If a third party force-sends 7 ujuno:

- actual bank rises by 7;
- P, F, C, positions, LP entitlement, and quotes do not change;
- Solvency query reports 7 forced excess;
- no sweep or “last redeemer” obtains it.

Acceptance:

- Repeated small redemptions never exceed aggregate entitlement.
- Forced excess cannot mask B < liabilities in the reported reconciliation.

## Chain halt or delayed blocks

1. Indexer sees no new blocks and UI marks data stale using both height and last block time.
2. No wall-clock close/finality transition is claimed.
3. Reference UI disables quotes/signing when freshness policy fails.
4. When Juno resumes, contract uses the resumed block.time. If it is at/after close, trades reject immediately. If oracle deadline passed, the appropriate finality/timeout call can execute.

Acceptance:

- Local browser time never authorizes an action.
- A large block-time jump cannot reopen trading.
- Two RPCs at different heights show an explicit disagreement, not averaged state.

## Failed or corrupt indexer

1. Indexer omits a counter-answer and reports the old countdown.
2. User or UI direct-query sees different oracle state.
3. Reference UI disables transaction construction, displays both heights, and reindexes.
4. Direct contract action remains possible with verified state.

Acceptance:

- No execute accepts indexer-supplied balances/reserves/finality as authority.
- Reprocessing events reconstructs the same totals as market Accounting at a finalized height.
- An interface can unlist a market but cannot change direct queries or settlement.

## Governance failure after a canonical wrong answer

If a challenged canonical wrong answer is affirmed or governance is unavailable and the latest answer later finalizes, the market pays that canonical result. Neutral fallback does not protect against wrong 0/1. This is a deliberate residual risk bounded by the market cap and disclosed before trading.

## Cross-journey completion test

A reviewer must be able to trace:

~~~text
creator 101 JUNO
  -> 100 market P + 1 oracle bounty
trader gross buys
  -> net P + fee F
trader sells/merges
  -> P reduction + user bank return + F
final oracle bytes
  -> immutable payout
user + pool redemption
  -> P
LP fee/dust/slash
  -> F + assigned non-principal liabilities
~~~

At every arrow, the responsible contract, sender permission, amount unit, rounding, failure rollback, event, and query evidence are specified in A1/R1.
