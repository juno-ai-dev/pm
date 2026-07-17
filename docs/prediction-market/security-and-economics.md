# A2 — Economic and security specification

**Status:** accepted 2026-07-16; numeric canary tier and documented residual risks explicitly accepted
**Security posture:** fully collateralized claims prevent insolvency; they do not prevent a wrong oracle result, harmful market, thin quote, governance failure, or JUNO loss of value

## Financial invariants

Notation: P locked principal; Y/N total outcome supply; rY/rN pool reserves; F LP fee liability; C challenge-bond liability; B actual market bank ujuno.

1. **Complete sets before resolution:** Y = N = P after every successful pre-resolution action. Pool plus all user balances equals each total.
2. **Coverage:** before resolution B >= P + F + C + whole-coin credits. After resolution B >= ceil(T2/2) + F + C + whole-coin credits, where T2 is unpaid position value in half-ujuno numerator units. Forced excess never creates a claim.
3. **Terminal conservation:** resolution initializes P0=P and T2=2P0. Valid/neutral position payments, pool claims, and paired half-dust reduce or reclassify exactly that numerator; aggregate position value equals P0 if every position redeems.
4. **Positive pool:** rY > 0 and rN > 0 whenever Trading; no denominator is zero.
5. **Product direction:** with the specified ceilings, post-trade rY × rN is at least pre-trade product. Rounding never grants caller extra output or reduced input.
6. **Slippage symmetry:** query and execute share math; Buy enforces min_out/deadline and Sell max_in/deadline over current reserves.
7. **Fee conservation:** F changes only by collected buy/sell fee or terminal debit. LP claims plus remaining F never exceed cumulative fees.
8. **Principal separation:** oracle bounty, creation gas, F, C, and forced excess are not P.
9. **Close:** Buy, Sell, and Split reject when block.time >= close_ts regardless of stored/display state or same-block ordering.
10. **Resolution binding:** payout stores once only after matching the immutable oracle/question and all guarantees/fields.
11. **Redemption safety:** balances and T2/F liabilities reduce before outbound message; failure reverts; cumulative accounting prevents repeat/partial double pay. Burning a zero-payout position does not reduce T2.
12. **LP subordination:** LP receives only pool positions, F, assigned neutral dust, and slashed C. It cannot consume collateral backing user balances.
13. **Bounded arithmetic:** all 128-bit stored amounts, including T2, are capped; products/divisions use checked 256-bit intermediates; conversion back checks bounds.
14. **Dust ownership:** each division has the immutable rule in R1. No action splitting improves caller entitlement.
15. **Path independence:** per-address neutral and LP claims use cumulative floors; equivalent partitions have the same final whole payout.
16. **Challenge segregation:** C never enters P, reserves, bounty, F, or spendable surplus and is released/slashed exactly once.
17. **Verdict authorization:** only the immutable `verdict_authority` can make a pending pre-deadline market forward `SubmitArbitration`; v1 pins the Juno Agents DAO core (`juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`). x/gov is deferred compatibility, not the v1 authority.
18. **Immutability:** chain admins for factory/market/oracle are empty; no message mutates economic config.
19. **Cap:** any Split/Buy that would make P exceed the market tier cap rejects before state change.
20. **No sweep:** no address can convert forced funds, abandoned positions, or live redemption liabilities into an admin balance.

Each invariant must be a property test and a multi-contract model assertion, not merely a comment.

## Accepted canary security tier

These values are the accepted first-release implementation envelope. Acceptance does not prove economic security or authorize deployment or scaling.

| Parameter | Accepted canary value | Basis and limitation |
| --- | ---: | --- |
| Initial liquidity | at least 100 JUNO | R1: a 1-JUNO balanced-pool buy moves end quote about 0.49 points at 2% |
| Locked-principal cap P | 200 JUNO | Contains first-release loss; equals 20× initial oracle bond. The ratio is accepted for the canary; further deployment and scaling still require current economic-security evidence. |
| Per-address outcome exposure | 20 JUNO terminal units per side | Accidental concentration limiter; trivial to bypass with wallets and not a security identity control |
| LP fee | exact 200 bps | Omen precedent and worked accounting; one-day JUNO/ATOM movement is measured, but prediction-event adverse selection/profitability remain unmeasured |
| Protocol fee | 0 | Avoids owner/sweep accounting and legal/economic scope |
| Minimum trade/split increment | 0.01 JUNO | Keeps 2% fee at 200 ujuno and reduces dust/spam; gas still unmeasured |
| Per-call trade | net split/merge <= 25% of smaller reserve | Bounds one-call impact/denominator; users can trade again at new state |
| Minimum oracle initial bond | 10 JUNO | 5% of cap; first honest counter-answer costs at least 20 JUNO |
| Oracle bounty | 1 JUNO | Accepted answerer incentive; no evidence it guarantees service |
| Answer timeout | 86,400 seconds | Existing production floor and Reality.eth's usual recommendation |
| Challenge bond | max(10 JUNO, current oracle bond) | Prevents free governance freeze; can become inaccessible after high escalation |
| Arbitration timeout | 1,814,400 seconds (21 days) | 10-day deposit + 5-day vote + 6-day margin for the pinned Juno Agents DAO process; x/gov compatibility is deferred |
| Creation-to-close lead | at least 24 hours | Gives review/monitoring time; not a semantic-safety guarantee |
| Maximum market duration | 90 days to close | Bounds LP lock/monitoring burden before resolution |
| Opening delay | event-specific; opening_ts >= close_ts and <= close_ts + 30 days | Must reflect source availability; longer cases need a new tier/review |
| Question bytes | <= 16 KiB | Accepted storage/gas bound; measurement missing |
| Discovery metadata | <= 4 KiB | Accepted index/storage bound; measurement missing |
| Market creation rate | No on-chain address rate limit | Sybil-ineffective; 100-JUNO seed plus gas is economic friction |

Raw canary bounds: P <= 200,000,000 ujuno, so any reserve product is <= 40,000,000,000,000,000 (4 × 10^16), far below 256-bit capacity. Fee numerator for a capped 25% call is also far below 128-bit capacity. The implementation still uses checked arithmetic; bounds are not an excuse for unchecked operations.

The factory is fixed to one tier. It does not expose ranges a creator can maximize adversarially. A later factory can use a different accepted tier after a new review.

The [Osmosis supplement](evidence/2026-07-15-osmosis-juno-liquidity.md) is parameter context, not the source of acceptance. Its two observed pools held 1.068 million JUNO, and a 200-JUNO single-pool sale was under 0.057% of either JUNO reserve. Its one-day TWAP sample is too short and collateral-focused to price event-outcome informed flow. The cap, bond ratio, and fee are nevertheless explicitly accepted for implementation with that residual uncertainty; deployment and scaling need current evidence.

## Oracle economic-security relationship

The maximum value redirected by choosing YES rather than NO is bounded by P, but profit depends on acquired positions, AMM slippage, fees, counter-answer bonds, challenge cost, and governance control. Solvency does not constrain answer corruption.

The accepted canary relationship is:

~~~text
P_cap <= 20 × minimum_initial_oracle_bond
challenge_floor >= minimum_initial_oracle_bond
named monitor correction budget >= 2 × minimum_initial_oracle_bond
~~~

For the canary, this means cap 200 JUNO, initial/challenge floor 10 JUNO, and a documented independent monitor able—but never obligated by the protocol—to post at least the first 20-JUNO counter-answer.

This is containment, not a proof of security:

- a correct counter-answer can trigger another doubling;
- governance deposit is normally refundable and is not an attack cost;
- token voting can be captured, bribed, apathetic, conflicted, or slow;
- JUNO external value and governance concentration change;
- the same asset secures trading, oracle bonds, governance deposit, and votes;
- no objective lower bound on governance-corruption cost was established.

Therefore scaling above the canary requires an explicit economic study using current stake/voter concentration, historical participation and execution, committed monitoring capital, market acquisition simulations, and counsel/operations capacity. No uncapped factory is safe by default.

## Challenge economics

One challenge is allowed per market. Required C is queried from the oracle and exact at execution:

~~~text
C = max(tier challenge floor, current oracle bond)
~~~

| Path | Objective on-chain observation | Challenge bond |
| --- | --- | --- |
| “Correct” challenge | Pre-deadline governance verdict bytes differ from snapshot | Full refund to challenger |
| “Incorrect” challenge | Pre-deadline verdict exactly equals snapshot bytes | Full credit to immutable LP |
| Neutral/noncanonical verdict | Differs from snapshot even if market payout is neutral | Full refund |
| Proposal rejected | No executing verdict before deadline | Full credit to LP at timeout |
| Deposit never reached / proposal stale | No executing verdict before deadline | Full credit to LP at timeout |
| Proposal passed but execution failed | Market/oracle unchanged; no executing verdict | Retry may succeed pre-deadline; otherwise full credit to LP |
| Verdict transaction after deadline | Market rejects | Full credit to LP through timeout path |
| No proposal/governance unavailable | No executing verdict | Full credit to LP |
| Direct public oracle cancel at deadline | Market sync observes no pending arbitration | Full credit to LP exactly once |

“Correct” and “incorrect” here describe whether governance changed the snapshot, not objective truth. Governance chooses the oracle-bond payee separately. Attached challenge funds are never forwarded to governance or cw-reality.

Residual tradeoff: a legitimate challenger can lose C because deposit sponsorship or governance failed. Refunding that path would make settlement freezes nearly free. The UI must show this before signature.

## Threat model

| Adversary/failure | Attack | Control | Residual |
| --- | --- | --- | --- |
| Malicious creator | Ambiguous/harmful rules, spoofed source/address, weak tier | Factory pins addresses/bytes/tier; reference review/warnings | Semantics/content cannot be proven on-chain |
| Trader/MEV | Sandwich, stale quote, close-boundary ordering, rounding split | min/max, deadline, per-call cap, exact close check, caller-adverse/cumulative rounding | Public mempool and price impact remain |
| LP | Fee sniping, withdrawal run, backing seizure | One initial LP; no entry/exit; fixed terminal formula | LP capital locked and can lose heavily |
| Oracle answerer | Early/opaque/wrong answer, bond grief | opening_ts, exact payout bytes, 24h reset, challenge, cap | Unknown bytes neutral; correct result still depends on monitoring |
| Challenger | Cheap freezes, finality race | exact C, one challenge, current-bond guard, full timeout slash | Legitimate challenge accessibility decreases with bond |
| Governance | Wrong answer/payee, unavailable, late | only challenged path, cap, unknown neutral, strict deadline | Canonical wrong answer and bond-payee theft remain possible |
| Migration admin | Replace market/oracle code | Empty chain admins and frozen instances | Chain governance/consensus upgrade remains foundational |
| Frontend/indexer | Alter rules/denom/quote/message, hide market | direct queries, decoded signing, hash display, no authority | Users can choose malicious interfaces |
| Chain/RPC | Halt, time drift, fork, gas spam, provider disagreement | block-time semantics, multiple height-pinned providers, conservative UI | Settlement/trading liveness depends on chain |
| Forced sender | Inflate raw bank balance to fake solvency/claim | internal liabilities; excess never claimable | Funds permanently stuck |
| Spam creator | Duplicate/illegal/impersonated markets | seed capital/gas, quarantine/discovery policy | Direct permissionless spam remains |
| Thin liquidity | Move quote cheaply and advertise “probability” | minimum seed, impact/depth display, cap | Quotes remain manipulable and uncalibrated |
| Lost key/abandonment | LP/user never claims | no expiry/seizure | Collateral can remain locked forever |
| Arithmetic/gas | Overflow, zero reserve, worst-case redemption | checked Uint256, caps, bounded messages, property/gas tests | Implementation defects remain audit risk |

## Failure-mode analysis

### Wrong canonical oracle answer

Exact mapping makes behavior deterministic but not true. If a wrong 0 or 1 finalizes unchallenged, the market pays it. No admin can repair it. Security is monitoring, bond escalation, challenge, cap, and clear rules before finality.

### Unknown answer

Any value outside exact 32-byte 0/1 settles neutral. This prevents an arbitrary arbitrator byte from bricking redemption, at the cost of giving governance a neutral escape.

### Unanswered

No final answer means no payout. Bounty/keeper alerts reduce likelihood but cannot prove liveness. There is no emergency override. Maximum delay is unbounded and disclosed.

### Repeated counter-answers

Every valid higher bond resets 24 hours, up to cw-reality's round cap and practical capital. The market does not impose a separate forced result. Users see latest bond/finalize time.

### Stalled governance

Freeze is bounded to 21 days per the accepted question configuration. At deadline, public cancellation restarts the latest answer's 24-hour clock and C is slashed. Since one challenge is allowed, governance cannot be invoked repeatedly for that market.

### Chain halt/time jump

No wall clock changes state. When blocks resume, the first block's time applies. Trading rejects if it has crossed close; deadline branches use the same block time. UI estimates are not authority.

## Audit scope

An independent audit must include:

- factory nested instantiate/reply and fund rollback;
- local question-ID derivation and every omitted-field comparison;
- FPMM formula equivalence, caller-adverse rounding, bounds, and slippage;
- P/Y/N/F/C and actual bank reconciliation for every execute/error;
- neutral dust and partial/last LP redemption;
- close/open/deadline boundaries and same-block ordering;
- challenge front-run guard, direct oracle cancellation, governance reply atomicity;
- exact sender/payee/answer/question validation;
- no migration/admin/pause/sweep surfaces;
- event/query completeness and indexer assumptions;
- cw-reality source, native claim/withdraw, arbitrary answer/payee, timeout, and deployed reproducible build;
- source-license provenance.

The audit report, commit, optimized wasm hashes, severity dispositions, and residual risks must be linked. “Audited dependencies” do not remove integration review.

## Implementation-phase verification plan

### Unit

- every formula boundary, fee/dust direction, exact examples, time predicate, byte mapping, fund validation, and error.

### Property/model

- random action sequences compared with arbitrary-precision reference;
- all 20 invariants after every success and state unchanged after every error;
- aggregate versus partitioned operations and neutral address remainders;
- reserve/product/cap bounds and no caller rounding gain.

### Multi-contract

- factory → market → oracle nested success/rollback;
- direct answer/counter, guarantee mismatch, challenge/verdict/cancel/claim/withdraw;
- bank send failure and reply failure atomicity;
- actual Juno bech32 canonicalization golden vectors.

### Adversarial

- sandwiches/stale quotes, same-block close, answer/finality/challenge races;
- spoofed oracle/question/denom/gov, unexpected funds, malicious payee;
- forced transfer, abandoned claims, direct oracle cancel, counter-answer at limits;
- hostile/lagging indexer and two-RPC disagreement.

### Migration

- assert factory/market/oracle chain admins are empty;
- assert migrate transactions fail;
- assert new factory versions cannot mutate/query-as-authority old markets;
- deployment tooling fails closed on any checksum/admin mismatch.

### Gas/storage

- maximum question/metadata, worst reserve arithmetic, largest events;
- instantiate with nested Ask/query reply, buy/sell, challenge, governance reply;
- position/LP redemption patterns and oracle claim history;
- measure on the current Juno software generation with margin below transaction/block limits.

### On-chain rehearsal

- frozen oracle lifecycle;
- a capped no-value canary under separate authorization;
- standard governance proposal with exact MsgExecuteContract sender, deposit, vote, answer/payee, success and passed-but-failed behavior;
- timeout/cancel near boundaries;
- final accounting reconciliation from raw state and bank queries.

## Acceptance boundary

The canary tier and residual governance risk were explicitly accepted for implementation on 2026-07-16. Passing arithmetic tests still proves solvency, not oracle or legal safety, and does not authorize deployment. A higher cap cannot be justified by copying the 20× ratio; it needs fresh evidence and a new decision.
