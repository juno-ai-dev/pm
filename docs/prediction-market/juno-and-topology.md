# R4 — Juno, collateral, and contract topology

**Status:** accepted topology decision (2026-07-16); deployment evidence remains open
**Chain evidence:** juno-1 heights 39,829,829 and 39,830,878; Osmosis height 66,387,548

## Decision

Use one immutable factory and one immutable market code version per factory deployment. Every market is a separate contract with its own bank balance and one market-owned cw-reality question. A later implementation version deploys a new code ID and new immutable factory; it never migrates funded markets or changes an existing factory's permissionless-creation policy.

The v1 dependency set is:

~~~text
immutable factory v1
  |
  +-- instantiate immutable market A --\
  +-- instantiate immutable market B ----> frozen cw-reality instance
  +-- instantiate immutable market C --/       (one question per market)

read-only indexers aggregate all factory versions
Juno x/gov can call only a challenged market's verdict entrypoint
~~~

Neither factory nor market has a CosmWasm migration admin. The recommended oracle also has neither a chain migration admin nor a stored admin. “Immutable” means both layers are empty; a None stored admin does not neutralize a non-empty wasmd admin.

## Native JUNO profile

| Property | Rule |
| --- | --- |
| On-chain denom | Exact string ujuno |
| Base/display conversion | 1 JUNO = 1,000,000 ujuno |
| Storage/arithmetic | Integer ujuno only; never floating point or a display decimal |
| User input | Parse a canonical decimal with at most six fractional digits; reject rather than round excess precision |
| User output | Render integer/1,000,000 with exactly labeled JUNO; also expose raw ujuno |
| Bank funds | Exactly one native coin where required; reject other denoms and unexpected extra funds |
| CW20/IBC | Out of scope, even if a symbol resembles JUNO |

Examples:

| Human | Raw |
| ---: | ---: |
| 0.01 JUNO | 10,000 ujuno |
| 0.1 JUNO | 100,000 ujuno |
| 1 JUNO | 1,000,000 ujuno |
| 100 JUNO | 100,000,000 ujuno |
| 5,000 JUNO governance deposit | 5,000,000,000 ujuno |

Native JUNO is both collateral and governance power. A JUNO price decline changes the external value of claims and may reduce the external cost of governance deposit/capture at the same time. A price increase raises market value at risk without changing integer oracle bonds. Tier limits therefore require periodic deployment-policy review; immutable live markets cannot be repriced or resecured by an admin.

The experimental label does not make JUNO valueless. Solvency, user warnings, and oracle security use its transferable value-bearing behavior.

### Supply, stake, liquidity, and concentration

A same-day supplement at height 39,830,592 observed 139.886 million JUNO native supply and 36.974 million bonded JUNO (about 26.43% of supply). Among 25 bonded validators, the largest held about 13.40% of delegated bonded tokens, the top five 49.43%, and the top ten 69.68%; three validators' delegated stake exceeded 33.4% in aggregate.

This is delegation concentration, not a claim that three operators control every governance result: delegators may override inherited votes, abstention/turnout matters, and operator independence is not established. It is sufficient to reject a model that treats governance voters as uniformly independent.

A [primary single-venue supplement](evidence/2026-07-15-osmosis-juno-liquidity.md) now measures Osmosis at height 66,387,548. Equal-weight pools 497 and 498 held 355,829.816267 and 711,889.169521 JUNO respectively. A calculated 200-JUNO sale into either pool was 0.0562% or 0.0281% of its JUNO reserve, with about 0.356% or 0.328% fee-plus-curve shortfall before taker fees/routing. Twenty-four hourly JUNO/ATOM TWAPs over one day had a 1.886% high/low range and -1.754% first-to-last move.

That is not venue-complete liquidity or long-horizon volatility. External depth can change faster than immutable market terms and is not a solvency input; external pools also do not create prediction-market LP participation. Before accepting a tier, risk review still needs repeat executable-depth measurements, 30/90-day volatility, concentration and historical governance turnout. The snapshot supports keeping a canary small relative to observed external reserves but does not approve the cap, bond, or fee.

## Height-pinned chain profile

Observed values and raw evidence are in [the snapshot](evidence/2026-07-15-juno.md):

| Item | Height 39,829,829 |
| --- | --- |
| Chain/application | juno-1, junod v29.1.0, commit 9e38daa0 |
| Cosmos SDK / wasmd / wasmvm | v0.50.13 / v0.54.0 / v2.2.4 |
| Maximum block | 22,020,096 bytes; 100,000,000 gas |
| Code upload | Everybody |
| Default instantiate | Everybody |
| Standard governance | 5,000 JUNO; up to 10-day deposit; 5-day vote |
| Gov module address | juno10d07y265gmmuvt4z0w9aw880jnsr700jvss730 |
| Production oracle | code 5121, checksum e25473…f3e2, non-empty migration admin |
| Same-day stake supplement | 139.886m supply; 36.974m bonded; top-five delegated share 49.43% |
| Independent refresh | Two providers and exact envelopes at 39,830,878; state bodies agree |
| Governance precedent | Passed proposals 357 and 363 used x/gov-originated `MsgExecuteContract` |
| Single-venue liquidity | Osmosis pools 497/498 held 1.068m JUNO total; one-day TWAP sample only |

Block limits are not safe transaction limits. The phase has no market wasm artifact, so it cannot honestly report instantiate, storage, trade, or resolution gas. The accepted 16-KiB question and 4-KiB discoverability-metadata limits remain contingent on implementation-phase gas measurement before deployment. That measurement must cover worst-case events and reply queries, not just a happy-path simulation.

wasmd v0.54 exposes Instantiate2 at the platform version level. The architecture does not depend on it: the instantiated market knows its own address and asks its question from instantiate/reply. If future indexing uses predictable addresses, Juno must separately rehearse creator, salt, checksum, and canonical-address derivation.

## One contract per market versus multi-market

| Dimension | One contract per market | One multi-market contract |
| --- | --- | --- |
| Collateral audit | Bank balance directly scopes one market | Internal ledger must partition one shared bank balance |
| Failure blast radius | Arithmetic/state bug affects one funded instance, though common code can affect all | One bad execute/migration can touch every market |
| Immutability/versioning | Old instances stay frozen; new factory routes new code | Version branches or whole-contract migration |
| Address/indexing | More addresses and instantiate events | One address, market-ID keys |
| Base storage/gas | Repeats contract metadata/config per market | Amortizes code-instance/config overhead |
| Cross-market netting | None, intentionally | Tempting but violates v1 isolation |
| Forced funds | Attributable to one vault | Ambiguous across internal vaults |
| Incident response | Interfaces can hide one instance; funds remain isolated | Pausing or migration risks unrelated markets |

Conceptual cost:

~~~text
one-per-market total =
  N × (instantiate overhead + immutable config + market state)

multi-market total =
  one instantiate + N × (market-key prefixes + market state)
~~~

The multi-market design will be cheaper in repeated base state and address indexing. Exact difference remains missing and is now authorized implementation-measurement work. The safety benefit of directly auditable isolated balances is load-bearing and outweighs an unmeasured cost optimization for v1. A future change requires measured gas/storage, an incident-blast-radius review, and an ADR; it cannot migrate old balances.

## Factory

The factory is a noncustodial creation router and registry for exactly one market code ID and one security tier. It:

- accepts any caller;
- validates objective bounds and exact native funds;
- instantiates the pinned market code with no admin;
- records market address, creator, immutable content hash, close/open times, code version, and tier;
- exposes pagination and emits a canonical creation event;
- never receives trade collateral after the creation transaction;
- has no execute path to edit, pause, migrate, delist, or settle a market.

Funds sent to factory creation are forwarded atomically to the market instantiate. The market retains initial pool principal and sends the separately declared oracle bounty with AskQuestion. Any failure rolls the transaction back.

There is no universal canonical registry after later factory versions. Indexers enumerate an explicit list of immutable factory addresses. That is an off-chain discovery dependency, not a financial authority.

Objective factory bounds include:

- exact ujuno collateral and oracle-bond denom;
- frozen oracle address/checksum/config represented by the factory tier;
- exact governance verdict address;
- market code ID/checksum represented by the factory;
- close/open ordering, minimum lead time, and maximum duration;
- initial liquidity, fee, question/metadata size, oracle bond/bounty, challenge bond, and collateral cap ranges;
- AnswerType Bool, answer_schema None, 24-hour answer timeout, and accepted 21-day arbitration timeout.

It cannot enforce prose quality, legality, geographic eligibility, or source truth.

## Market address and storage isolation

One market stores:

- immutable config and resolution bytes/hash;
- lifecycle and one-time payout;
- total YES/NO supply and locked principal;
- user YES/NO balances;
- positive FPMM reserves and fixed LP balance/supply;
- fee and neutral-dust accumulators;
- challenge snapshot/liability;
- oracle question ID and verification fields.

The bank balance is reconciled only to that state. An invariant violation on market A does not let A read or send market B's bank funds.

## Admin and migration matrix

| Component | Admin/migration | Can do | Cannot do |
| --- | --- | --- | --- |
| Factory v1 | None | Permissionless instantiate and append-only record under immutable rules | Change code/tier, block creator, alter live market |
| Market v1 | None | Execute immutable financial/state rules | Migrate, pause, change question/payout/fee/cap |
| Recommended cw-reality | Chain admin None; stored admin None | Execute immutable oracle rules | Migrate code/config |
| Existing production cw-reality | juno1mtz…xvzwd at both layers | Migration authority is an additional resolution trust | Cannot be treated as code-pinned |
| Juno x/gov | Pinned address per market | Submit one pre-deadline verdict/payee through challenged market | Trade, withdraw, pause, edit rules, call unchallenged verdict |
| Creator | Ordinary user plus initial LP | Create, trade, challenge, later claim LP value | Settle/admin/migrate |
| Indexer/frontend | None on-chain | Read, rank, filter, warn | Create balances or alter settlement |

There is no emergency pause. At and after close, time checks stop trading. If a vulnerability is found, reference interfaces warn and stop routing new users; unaffected on-chain redemption paths remain available. A pause key capable of blocking valid redemption is rejected.

## Market versioning

Version identity is the tuple:

~~~text
(chain_id, factory_address, factory_code_checksum,
 market_address, market_code_checksum,
 oracle_address, oracle_code_checksum, question_id)
~~~

Events and queries expose protocol version. A new code ID means a new factory address and a new indexer allowlist entry. Routers may display multiple versions but must not imply fungibility or move positions between them.

No migration of live accounting is supported. If a future critical bug requires recovery, the response is disclosure and a separately reviewed opt-in migration market where users redeem/transfer through existing authorized paths; governance cannot confiscate or rewrite old balances.

## Deployment dependency checklist

This is a future planning gate, not authorization:

1. independently audit cw-reality and reproduce the selected wasm checksum;
2. instantiate a frozen oracle with no chain/stored admin, then re-query both;
3. rehearse normal, challenged, stalled, arbitrary-byte, and native withdrawal oracle flows;
4. audit market/factory code and reproduce both wasm checksums;
5. benchmark worst-case factory instantiate/reply, trade, challenge, resolution, and batched redemption against current Juno gas;
6. query current chain ID/software, consensus, wasm access, governance params/module account, and every code/contract admin at one finalized height;
7. execute the authorized governance verdict rehearsal;
8. obtain parameter risk acceptance, legal advice, content/runbook review, and named monitoring coverage;
9. deploy factory with admin None and verify its code/checksum/config;
10. create a capped canary market only under a separately authorized phase; reconcile every liability and event before broader discovery.

Any mismatch blocks deployment planning. A familiar address, passing local tests, or a successful query is not a waiver.
