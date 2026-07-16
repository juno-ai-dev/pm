# R1 — Mechanism and market microstructure

**Status:** accepted architecture specification (2026-07-16)
**Evidence date:** 2026-07-15
**Decision:** binary complete sets plus a constant-product FPMM; one initial LP position locked until resolution

## Recommendation

The v1 market should use two internal outcome ledgers and one FPMM reserve pair. Locking x ujuno creates x YES units and x NO units. Merging x of both burns them and releases x ujuno. The AMM is an inventory manager over those fully backed claims; it is not the source of backing.

Only the market creator supplies liquidity during atomic creation. The contract mints a fixed, non-transferable LP supply to that address. No later add-liquidity, remove-liquidity, or fee withdrawal is allowed before resolution. This is intentionally less flexible than the Gnosis FPMM. It removes fee sniping, asymmetric LP deposits, pre-close runs, and a large share-accumulator surface from the first audit. Permissionless creation still lets any address become an LP by creating a market.

The accepted canary fee is 2% of collateral flow, all owned by the initial LP; there is no protocol fee. Omen documents a 2% purchase fee as production precedent, but that is not evidence that 2% fits Juno flow. The owner accepted this residual risk; measurements remain required before deployment or scaling.

## Units and accounting domains

- All stored and transferred amounts are integer ujuno. One JUNO is 1,000,000 ujuno.
- One outcome unit has a valid terminal claim of one ujuno. A neutral outcome has a rational claim of one-half ujuno.
- Pool reserves and user balances are outcome units, not bank coins.
- Locked principal P is the backing for both complete-set supplies: total YES = total NO = P.
- Fee liability F is bank collateral owed to LPs and is never part of P.
- Challenge-bond liability C is separate from P and F.
- Before resolution, accounted liabilities are P + F + C plus whole-coin credits. At resolution, P is frozen as P0 and terminal position liability becomes T2 = 2P0 half-ujuno numerator units; resolved coverage uses ceil(T2/2) + F + C plus credits. Raw bank balance may be larger because forced transfers are ignored.

## Integer FPMM

Let Y and N be positive YES and NO pool reserves, k = Y × N, fee scale S = 10,000, and fee f in basis points. All products and intermediate divisions use a 256-bit unsigned domain. ceil(a / b) means (a + b - 1) / b with checked arithmetic.

### Buy exact collateral

For a YES buy with gross collateral g:

1. fee = ceil(g × f / S)
2. d = g − fee
3. ending_yes = ceil(Y × N / (N + d))
4. yes_out = Y + d − ending_yes
5. new reserves = (ending_yes, N + d)

The NO formula swaps Y and N. The ceiling on ending reserve rounds output down. The post-trade product cannot be below the pre-trade product. Execution rejects zero d, zero output, an expired deadline, output below min_out, or d greater than 25% of the smaller pre-trade reserve.

The accepted 25% canary bound is a safety/usability limit, not a solvency requirement. It makes extreme one-call quotes visibly unavailable and bounds denominator distance for sells. A trader can submit another bounded trade at the new price.

### Sell for exact collateral

For a YES seller requesting q ujuno net:

1. merge = ceil(q × S / (S − f))
2. fee = merge − q
3. require merge < N
4. yes_in = merge + ceil(Y × N / (N − merge)) − Y
5. new reserves = (Y + yes_in − merge, N − merge)

The seller supplies yes_in, the market removes merge complete sets from the pool, merges them, returns q, and credits fee to F. The ceiling rounds required input up. Execution rejects input above max_in, an expired deadline, merge at or above the opposite reserve, or a merge greater than 25% of the smaller pre-trade reserve.

Queries and executes must call the same pure arithmetic routine over the same reserve snapshot. A quote is advisory; min_out/max_in and deadline are consensus-enforced.

### Marginal display price

Ignoring the next trade's fee and integer step, the YES marginal quote is:

~~~text
p_yes = N / (Y + N)
p_no  = Y / (Y + N)
~~~

This is a reserve-derived quote, not a calibrated forecast. The UI must separately show marginal quote, fee, average execution price, price impact, liquidity, and terminal payout rules.

## Hand-worked conservation example

The example uses the accepted 2% canary fee and begins with 100 JUNO of creator collateral:

~~~text
initial bank balance          100,000,000 ujuno
initial locked principal P    100,000,000
pool YES / NO                 100,000,000 / 100,000,000
fee liability F               0
k                             10,000,000,000,000,000
~~~

A trader buys YES with 10 JUNO:

~~~text
gross                         10,000,000
fee = ceil(gross × 200/10000)    200,000
net split d                    9,800,000
ending YES                    91,074,682
YES delivered                 18,725,318
ending NO                    109,800,000
new k                         10,000,000,083,600,000
P / F                        109,800,000 / 200,000
bank balance                 110,000,000 = P + F
~~~

The same trader then sells 9,540,206 YES for exactly 5 JUNO:

~~~text
requested return               5,000,000
complete sets merged           5,102,041
sell fee                         102,041
YES supplied                   9,540,206
pool YES / NO                 95,512,847 / 104,697,959
trader YES remaining           9,185,112
new k                         10,000,000,139,179,273
P / F                        104,697,959 / 302,041
bank balance                 105,000,000 = P + F
~~~

Supply reconciliation is exact:

~~~text
YES = pool 95,512,847 + user 9,185,112 = 104,697,959
NO  = pool 104,697,959 + user 0        = 104,697,959
~~~

Terminal examples:

| Result | LP pool positions | Trader positions | LP fees | Total bank paid |
| --- | ---: | ---: | ---: | ---: |
| YES | 95.512847 JUNO | 9.185112 JUNO | 0.302041 JUNO | 105.000000 JUNO |
| NO | 104.697959 JUNO | 0 | 0.302041 JUNO | 105.000000 JUNO |
| Neutral | 100.105403 JUNO | 4.592556 JUNO | 0.302041 JUNO | 105.000000 JUNO |

The LP is short the outcome traders bought: after this round trip the LP finishes below 100 JUNO if YES wins and above it if NO wins. Fees do not guarantee profitability.

## Liquidity and price impact

The following table applies one isolated YES buy to an initially balanced pool with a 2% fee. “Pool” is the creator collateral and each starting outcome reserve. Average is gross JUNO paid per YES unit; end quote is the new marginal YES display price.

| Pool | Gross trade | YES received | Average | End quote |
| ---: | ---: | ---: | ---: | ---: |
| 10 JUNO | 0.1 JUNO | 0.195048 | 0.512694 | 50.488% |
| 10 JUNO | 1 JUNO | 1.872531 | 0.534037 | 54.661% |
| 100 JUNO | 0.1 JUNO | 0.195904 | 0.510454 | 50.049% |
| 100 JUNO | 1 JUNO | 1.950489 | 0.512692 | 50.488% |
| 100 JUNO | 10 JUNO | 18.725318 | 0.534036 | 54.661% |
| 1,000 JUNO | 0.1 JUNO | 0.195990 | 0.510230 | 50.005% |
| 1,000 JUNO | 1 JUNO | 1.959040 | 0.510454 | 50.049% |
| 1,000 JUNO | 10 JUNO | 19.504892 | 0.512692 | 50.488% |

Inference: a 10-JUNO pool makes a 1-JUNO order move the marginal quote by about 4.66 percentage points before any competing information. A 100-JUNO pool keeps that representative order below 0.5 points. Therefore 100 JUNO is the accepted canary minimum initial liquidity if the product calls 1 JUNO a normal order. If observed Juno users trade materially smaller or larger sizes, a future tier should change rather than preserve a cosmetically convenient value.

Accepted canary activation bounds:

- initial liquidity: at least 100 JUNO and even in ujuno only for cleaner neutral examples;
- minimum gross buy / requested sell return: 10,000 ujuno (0.01 JUNO);
- per-call net split or merge: at most 25% of the smaller reserve;
- reserves must remain at least one outcome unit;
- creator supplies oracle bounty and initial pool principal as separately labeled funds.

These bounds are accepted for implementation with their residual risks. Future gas and usage measurements remain deployment and scaling evidence; they do not reopen the accepted implementation values.

## LP lifecycle and fees

At activation, the contract mints fixed LP supply equal to initial locked principal to the creator. LP balances are internal and non-transferable. No LP supply changes later.

Every fee is credited immediately to F. The LP may not claim F before resolution. At resolution the contract computes:

~~~text
LP position value =
  pool_yes × payout_yes + pool_no × payout_no

LP terminal entitlement =
  LP position value + F + assigned neutral dust + slashed challenge bonds
~~~

Let Q2 be the pool's terminal position value in half-ujuno numerator units and S the fixed LP supply. Partial LP redemption uses cumulative formulas, not independent floor-per-call:

~~~text
allocated_position_numerator_after =
  floor(Q2 × cumulative_LP_burned / S)
position_whole_after = floor(allocated_position_numerator_after / 2)
fee_whole_after = floor(F_at_resolution × cumulative_LP_burned / S)

this_position_payment = position_whole_after − prior_position_whole
this_fee_payment = fee_whole_after − prior_fee_whole
~~~

The final LP burn allocates every remaining numerator/fee unit; an odd final position numerator enters the shared half-dust counter. This makes splitting one LP claim path-independent. With one initial LP, the result is the full pool entitlement, subject only to the same neutral half-dust pairing rule as users.

### Why liquidity is locked

- A removable pool can be made unusable immediately before information arrives.
- Withdrawal formulas must decide which outcome inventory and accrued fees leave, which is consensus-critical and highly directional.
- Late LP entry can capture fees earned before entry without an accumulator.
- A minimum-liquidity burn reduces but does not remove those issues.

The cost is real: the creator cannot recover capital until oracle finality, including disputes and stalls. The UI must show that maximum delay and must never describe the position as withdrawable liquidity.

## Neutral payout and integer dust

YES and NO each have payout numerator 1 with denominator 2. For each address, partial redemptions use cumulative burned position numerator:

~~~text
new_whole_credit = floor((cumulative_yes_burned + cumulative_no_burned) / 2)
payment = new_whole_credit − prior_whole_credit
~~~

Thus splitting an address's redemption into calls cannot improve it. When that address has burned all positions and its cumulative numerator is odd, one half-ujuno remainder is finalized into a global half-dust counter. Each pair of finalized half-dust units becomes one whole ujuno credited to the immutable LP dust account. A user cannot gain by address splitting; doing so can only donate more rounding value to the LP. The same rule applies to pool-position settlement.

If all positions redeem, the number of half-dust units is even because total neutral numerator is 2P0. Aggregate position payouts plus LP-assigned dust equal P0 exactly. Each whole position payment reduces T2 by two; a paired dust unit moves two numerator units from T2 into one whole LP credit. If users abandon positions, the associated terminal numerator and collateral remain locked; there is no expiry sweep.

Valid YES/NO redemption has denominator one and no payout dust. Bank-message failure reverts the whole transaction after state changes. Position balances are reduced before the transfer is queued, and CosmWasm transaction atomicity restores them if the message fails.

## Other dust and forced funds

| Source | Rule |
| --- | --- |
| Buy fee | ceil against buyer; all fee goes to F |
| Buy invariant division | ceil ending reserve; remainder remains in pool |
| Sell gross-up | ceil against seller; difference goes to F |
| Sell invariant division | ceil required input; remainder remains in pool |
| LP proportional claim | cumulative floor; final LP burn receives remainder |
| Neutral payout | per-address cumulative floor; paired half-dust goes to LP |
| Challenge slash | whole ujuno only; credited to LP terminal liability |
| Forced bank transfer | never creates a balance, fee, principal, or refundable surplus; permanently unclaimable in v1 |

There is no admin sweep. Raw bank balance is an upper bound, while solvency uses internal liabilities.

## Mechanism comparison

| Mechanism | Bootstrap | Consensus math | Provider risk | v1 disposition |
| --- | --- | --- | --- | --- |
| FPMM | Requires creator collateral; always quotes while reserves exist | Checked integer multiply/divide | LP bears informed flow and 0/1 convergence | Accept |
| CLOB | Empty without makers; best capital efficiency when populated | Fill, signature, cancellation, partial-order accounting | Professional maker inventory | Defer; compatible with later tokenized positions |
| LMSR | Sponsor deliberately subsidizes; always quotes | exp/log fixed point; b calibration | Bounded sponsor loss, not LP capital | Reject for v1 |
| pm-AMM | LP-funded and designed for outcome-token dynamics | normal PDF/CDF/inverse and time dependence | More uniform modeled LVR | Research later |
| Parimutuel | Simple pooled funding | Pro-rata division | No continuous LP | Reject: no secondary exit |
| Hybrid | Can route between book and AMM | Union of both systems plus routing | Split | Defer until both venues exist |

Hanson's [LMSR paper](https://hanson.gmu.edu/mktscore.pdf) is the primary mechanism source. Moallemi and Robinson's [pm-AMM paper](https://www.paradigm.xyz/2024/11/pm-amm) is the primary source for its normal-distribution invariant and LVR motivation. The pinned [Gnosis FPMM](https://github.com/gnosis/conditional-tokens-market-makers/blob/6814c0247c745680bb13298d4f0dd7f5b574d0db/contracts/FixedProductMarketMaker.sol) is the formula and rounding precedent. These support the comparison; the recommendation is an inference from Juno's expected thin launch and the no-code/audit constraints.

## Implementation-phase tests derived here

- model every formula against an arbitrary-precision reference over bounded random reserves;
- prove product direction, nonzero reserves, slippage, deadline, and close boundary;
- execute the full buy/sell example above byte-for-byte;
- fuzz repeated small calls versus one aggregate call to prove no caller rounding gain;
- cover fee extremes and multiplication bounds;
- cover every neutral remainder ordering and abandoned-position case;
- reconcile bank balance to P + F + C after every action, with random forced transfers;
- verify LP terminal value for YES, NO, neutral, partial burns, and challenge slashes.

## Revisit triggers

Reconsider locked FPMM liquidity after audited transferable positions exist, at least six months of volume/size data are available, or market creators consistently fail to supply the accepted minimum. Reconsider 2% if measured volume, LP loss, or routing shows it is outside the accepted adverse-selection/usage target.
