# A1 — Architecture specification

**Status:** accepted implementation specification (2026-07-16)
**Scope:** one fixed-expiry binary market per contract; native ujuno; internal positions; FPMM; cw-reality

Names below describe conceptual messages and events. They are not generated schema or production code.

## System and trust boundaries

~~~text
creator / trader / LP / challenger / keeper
                    |
                    v
        immutable factory v1 --------------> indexer / API / UI
                    |                           read-only convenience
             instantiate + registry
                    v
         immutable binary market
         - one collateral vault
         - YES/NO user ledgers
         - FPMM inventory
         - fixed LP entitlement
         - fees/dust/challenge liability
         - lifecycle and payout
                    |
          ask/query/answer control
                    v
         frozen cw-reality instance
                    ^
                    | market is configured arbitrator
                    |
 immutable verdict authority --> market GovernanceVerdict only
~~~

Trust is deliberately narrow:

- Consensus, Juno bank/wasm modules, and block time are foundational.
- Market and oracle code/checksums are trusted and must be immutable.
- The pinned verdict authority is trusted only after a bonded challenge to select oracle answer and payee. V1 pins the Juno Agents DAO core. It cannot edit market payout mapping, move collateral, or rotate its own address.
- Creator prose and sources can be bad; neutral resolution limits ambiguity but does not make the market useful.
- Frontends/indexers can lie or fail; they have no financial authority.
- The initial LP bears inventory and liveness risk; it has no senior claim over trader positions.

## Components

### Immutable factory

One factory pins one market code and security tier. It validates objective fields, receives exact creation funds, instantiates a no-admin market, and records the activated result. It has no custody after creation and no edit, pause, migration, settlement, or allowlist path.

### Binary market

One market owns:

- immutable rules, addresses, timeouts, caps, fee, question bytes/hash;
- market-owned oracle question;
- native collateral and complete-set accounting;
- user positions, pool reserves, fixed LP supply;
- lifecycle, challenge, payout, redemption, fee/dust state.

It is the oracle's arbitrator address only so it can request arbitration and relay a governance verdict. No generic proxy execute exists.

### cw-reality

The frozen oracle owns question bounty, answer/counter-answer bonds, timeouts, history claims, and final answer bytes. Users interact with it directly for answering. The market consumes only a finalized, fully matched answer.

### Verdict authority

The immutable `verdict_authority` may call GovernanceVerdict only while the exact market has a live pre-deadline challenge. The v1 deployment profile pins the Juno Agents DAO core `juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac`; a passed proposal executes the message from that core address. Members, proposal modules, voting modules, EOAs, and other contracts are not equivalent callers. The market forwards the exact answer/payee to cw-reality. The authority is not a market admin. DAO code, modules, membership, and voting-rule changes remain external trust risks even though the address is immutable per market. A future market profile may pin the Juno x/gov module account without changing settlement semantics; #4 and #13 are deferred and non-blocking for v1.

### Off-chain services

Indexers reconstruct factory/market/oracle/governance events. Reference UI discovery and warnings are policy, not settlement. Keepers call public paths and use their own funds where required.

## Immutable configuration

At activation the market pins:

- protocol/factory/market version and code identity used by deployment review;
- creator and initial LP address;
- ujuno collateral and oracle bond denom;
- initial liquidity, accepted 2% fee, min trade, max trade/reserve ratio, collateral cap;
- close_ts, opening_ts, maximum duration;
- exact resolution bytes and SHA-256;
- frozen oracle address, expected code ID/checksum/config;
- question ID, nonce, text, Bool type, no filter, initial bond, bounty, 24-hour answer timeout, accepted 21-day arbitration timeout;
- market address as oracle arbitrator;
- immutable verdict-authority address (Juno Agents DAO core for v1);
- challenge-bond rule and outcome;
- exact result bytes/payout mapping;
- no migration/pause/sweep authority.

Address/checksum evidence that a contract cannot query itself is a deployment invariant and registry disclosure. Every queryable question/config field is also rechecked on-chain.

## Lifecycle

~~~text
Initializing
   | AskQuestion submessage + reply exact match
   v
Trading ---------------- block.time >= close_ts ----------------+
   |                                                        |
   | buy/sell/split                                         v
   |                                              AwaitingResolution
   |                                                        |
   +-- merge ------------------------------------------------+
                                                            |
                              oracle finalized --> Resolve --+--> Resolved
                                                            |
                      bonded Challenge before answer finality
                                                            v
                                                   PendingArbitration
                                                    |              |
                                authority verdict before deadline   |
                                                    |              |
                                                    +--> Resolved   |
                                                                   |
                                      deadline/cancel + slash ------+
                                             back to AwaitingResolution
                                             challenge_used remains true
~~~

The externally reported state is derived before every execute:

- Initializing until oracle reply verification succeeds.
- Trading only while activated and block.time < close_ts.
- AwaitingResolution when activated, unresolved, not challenged, and block.time >= close_ts.
- PendingArbitration while the market challenge is live.
- Resolved after one-time payout storage.

No close transaction is required. A delayed block jumps directly across the boundary. Transaction order within one block cannot permit a trade when block.time equals close_ts.

## Action surface

| Conceptual execute | Caller/funds | State/time | Effect |
| --- | --- | --- | --- |
| Factory.CreateMarket | Anyone; exact initial liquidity + oracle bounty | Factory always permissionless; fields within immutable tier | Instantiate market, atomically activate question, append registry |
| Split | Anyone; one exact ujuno coin | Trading and before close | Increase P and both user positions by amount, subject to cap |
| Merge | Position owner; no attached funds | Any unresolved state | Burn equal user YES/NO, reduce P, send equal ujuno |
| Buy | Anyone; exact gross ujuno | Trading and before close | Charge fee, split net, update pool, credit selected position |
| Sell | Position owner; no attached funds | Trading and before close | Debit max outcome input, update pool, merge, send requested net |
| Challenge | Anyone; exact required ujuno | AwaitingResolution; oracle OpenAnswered before finalize; never used | Snapshot answer/bond, escrow C, request oracle arbitration atomically |
| GovernanceVerdict | Exact pinned authority; no funds | Pending, correct question, block.time < deadline | Forward answer/payee, resolve in reply, refund/slash C |
| FinalizeStalledChallenge | Anyone; no funds | Pending at/after deadline or oracle already publicly cancelled | Cancel/sync oracle, slash C to LP, return awaiting |
| Resolve | Anyone; no funds | Awaiting; oracle finalized | Match guarantees/full fields, store payout exactly once |
| RedeemPositions | Owner; no funds | Resolved | Burn requested user positions, credit/send deterministic payout |
| RedeemLp | Initial LP; no funds | Resolved | Cumulatively burn fixed LP units, settle proportional pool/fees |
| ClaimLpAccrual | Initial LP; no funds | Any time claimable after resolution | Drain later neutral dust/challenge slash credit |

Split rejects if new P exceeds cap. Direct split/merge never changes FPMM reserves or price because user positions are outside pool inventory. Split is disabled after close to avoid needless outcome-known balance creation. Merge stays available before resolution because a complete set is always exactly one collateral claim and reducing liabilities is safe.

No TransferPosition exists. No AddLiquidity/RemoveLiquidity exists. No generic oracle relay exists.

### Slippage/deadlines

Buy supplies min_out and deadline. Sell supplies exact return, max_in, and deadline. The market recalculates from current state and rejects if block.time > deadline. A deadline does not override close_ts.

The query and execute formulas are identical, but a quote is not reserved. Event ordering and front-running are user risks bounded by the signed parameters.

## Oracle creation and activation

During instantiate, market state is nonfinancially usable only as Initializing:

1. validate exact creator funds, typed question fields, and internal arithmetic;
2. inject pinned fields and construct the exact JCS resolution bytes;
3. retain pool principal and send declared bounty with those bytes in AskQuestion;
4. compute expected question ID from canonical addresses and pinned source algorithm;
5. in reply, query the exact Question and compare every field;
6. establish Y = N = P = initial liquidity, fixed LP supply, F = C = 0;
7. set activated and emit immutable identity.

The factory receives its instantiate reply only after the nested market execution succeeds. It records the market only then. Any nested error reverts factory, market, question, and bank movement.

## Storage/accounting model

Conceptual singleton records:

- Config: immutable fields above;
- Lifecycle: activated, payout, resolution answer/time, challenge_used;
- Accounting: P, F, C, pool Y/N, total YES/NO, LP supply/burned/paid, neutral half-dust, LP later accrual;
- Terminal accounting after resolution: T2 (unpaid position liability measured in half-ujuno numerator units) and resolution-time pool/user claim snapshots;
- Challenge: challenger, answer bytes, oracle bond, start, deadline, refundable/slash status.

Conceptual maps:

- positions[address] = YES, NO;
- neutral_redemption[address] = cumulative numerator, whole paid, finalized-half flag;
- lp_accrual = immutable initial-LP whole-coin credit for later dust/slash.

Required equalities before resolution:

~~~text
total_yes = total_no = P
total_yes = pool_yes + sum(user_yes not burned)
total_no  = pool_no  + sum(user_no not burned)
bank_ujuno >= P + F + C + other explicit whole-coin credits
pool_yes > 0 and pool_no > 0 while Trading
~~~

At resolution, the market freezes P0 = P and initializes T2 = 2 × P0. Thereafter arbitrary winning, losing, and neutral burns can make total YES and NO supplies differ, so the pre-resolution equality is no longer asserted. Resolved coverage is:

~~~text
bank_ujuno >= ceil(T2 / 2) + F + C + LP_whole_coin_accrual
~~~

T2 decreases by two for each whole ujuno of position value paid. A finalized half remainder stays in T2 until paired with another half; pairing reduces T2 by two and increases LP whole-coin accrual by one. Moving value between those ledgers does not change total liabilities.

Maps cannot be summed on-chain cheaply; totals update in the same transaction and property/multi-contract tests compare them against a model.

### Funds sequence

Buy:

~~~text
bank += gross
P += net
F += fee
gross = net + fee
~~~

Sell:

~~~text
P -= complete_sets_merged
F += complete_sets_merged − net_return
bank -= net_return
~~~

Merge:

~~~text
P -= amount
bank -= amount
~~~

Challenge:

~~~text
bank += bond
C += bond
later: C -= bond; either bank refund or LP accrual += bond
~~~

Resolution sets P0 and T2 but does not change bank balance or aggregate liability. Redemption reduces T2 and/or F before queueing a BankMsg; transaction failure rolls back both. Losing-position burns reduce token supply but not T2 because they carry zero terminal numerator.

## Resolution

Resolve is checks-effects-only until all oracle queries pass:

1. reject attached funds and wrong lifecycle;
2. query FinalAnswerIfMatches with tier constraints;
3. query full Question and verify immutable identity/config;
4. require both answers/bonds agree and state Finalized or Claimed;
5. exact-map bytes to YES, NO, or neutral;
6. store P0 = P and T2 = 2P0, answer, payout numerator/denominator, height/time once;
7. emit resolution identity and terminal-accounting snapshot.

Later Resolve rejects AlreadyResolved without changing data. It cannot change payout if oracle code later changes.

GovernanceVerdict uses a reply: state is marked reply-in-progress, oracle SubmitArbitration is sent, reply re-runs the same resolution checks, stores payout, and settles C. Any failure atomically restores PendingArbitration and its bond.

## Redemption and last claimant behavior

User chooses YES and NO amounts up to current balance. Balances and the selected outcome supplies are reduced before payout is sent.

- Valid result: exact winning amount, no rounding; T2 falls by twice the payment. Burning losing units leaves T2 unchanged.
- Neutral: cumulative per-address floor of (YES burned + NO burned)/2. Partial calls equal one aggregate call; T2 falls by twice each whole payment.
- When an address exhausts positions with an odd numerator, its half remainder is finalized to global half-dust. Every pair credits one ujuno to the immutable LP.
- Abandoned positions never expire or sweep.

LP pool settlement values its remaining pool positions under the same payout and adds F. Its paid position component reduces T2; its paid fee component reduces F. C is never included until objectively slashed. Cumulative LP burn allocates proportional whole coins; a final half-position remainder enters the shared half-dust counter. Later paired neutral-dust/slash value accrues to a separate claimable ledger for the immutable initial LP even if LP units were already burned.

There is no “last user gets the vault” rule. Forced excess and abandoned liabilities remain. The final valid claimant receives only computed entitlement.

## Query surface

All financial facts must be queryable without an indexer:

- Factory.Config, Factory.Market, Factory.ListMarkets;
- Market.Config/Identity, State, Accounting, Pool, QuoteBuy, QuoteSell;
- Position(address), LpPosition, Challenge, Resolution;
- Solvency view returning actual bank balance, each tracked liability, accounted total, and forced excess when nonnegative;
- immutable question bytes/hash and oracle verification fields.

Quotes include reserve height/time, gross/net, fee, input/output, average price, marginal before/after, impact, and caller constraints required. They do not return “probability” as a protocol fact.

Pagination has fixed maximums. Query failures are explicit; missing records do not masquerade as zero unless zero is the documented balance default.

## Event contract

Every event includes protocol_version, factory, market, and action-specific identity. Important events:

- market_created / market_activated;
- split / merge;
- trade with side, outcome, gross/net, fee, input/output, reserves before/after, P/F;
- challenge_requested with question, snapshot answer/bond, challenger, C, deadline;
- governance_verdict_forwarded and challenge_refunded/challenge_slashed;
- arbitration_stalled;
- market_resolved with exact answer base64/hex and payout;
- positions_redeemed, lp_redeemed, lp_accrual_claimed.

Amounts are raw decimal ujuno strings. Timestamps are Unix seconds. Answer events include both lowercase hex and standard base64 under fixed attribute names. Events are convenience; queries control.

Indexers key by chain ID + contract address + transaction hash + event index, process only finalized-enough blocks under their policy, and support rollback/replay. They reconcile periodic direct snapshots to event-derived totals.

## Permissions

| Capability | Authority |
| --- | --- |
| Create market | Any address satisfying objective factory rules |
| Trade/split/merge/redeem/resolve/sync | Any eligible holder/caller under immutable rules |
| Answer/counter-answer | Any cw-reality participant with bond |
| Request arbitration | Market only, after any user funds the one valid Challenge |
| Select answer/payee | Exact pinned verdict authority only, only through pending market |
| Change question, payout, fee, close, cap | Nobody |
| Migrate/pause/sweep factory, market, oracle | Nobody |
| Filter a particular website/API | That independent operator only |

## Failure behavior

| Failure | Result |
| --- | --- |
| Wrong/multiple funds | Reject before state change |
| Arithmetic overflow/zero denominator | Checked error; no saturation in financial math |
| Oracle Ask/reply mismatch | Entire creation rolls back |
| Oracle query unavailable | Execute fails; state/funds unchanged |
| Trade at close boundary | Reject even if UI quote was earlier |
| Slippage/deadline | Reject |
| Bank send/submessage failure | Whole transaction rolls back |
| Noncanonical final answer | Resolve neutral, not error/stall |
| Unanswered oracle | Remain AwaitingResolution indefinitely |
| Counter-answer | Oracle clock resets; market remains awaiting |
| Governance absent/failed/rejected | Timeout; challenge bond slashed; oracle clock restarts |
| Governance call at/after deadline | Reject; timeout path controls |
| Direct public oracle cancellation | Market sync observes it and applies one timeout slash |
| Chain halt | No wall-clock transitions; resume on block time |
| Indexer outage | Direct queries/actions remain; reference UI warns/disables stale quotes |
| Forced bank funds | Solvency excess only; no claimant/sweep |

## Upgrade and recovery

There is no live upgrade path. New behavior means new immutable factory/market/oracle addresses and explicit routing. No authority may recover a typo, ambiguous question, lost LP key, or abandoned position by rewriting state. This raises the pre-creation review burden and makes canary caps essential.

A vulnerability response can warn, unlist, stop reference routing, and publish a new version. It cannot pause or seize existing claims. Governance remains unable to author a payout directly; even its verdict must finalize through the pinned oracle and exact mapping.
