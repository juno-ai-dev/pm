# Juno exit-liquidity and short-window volatility evidence

**Observed:** 2026-07-15  
**Venue:** Osmosis `osmosis-1`  
**Height:** 66,387,548  
**Block time:** 2026-07-15T16:45:07.107892122Z  
**Classification:** direct chain observation unless a paragraph says calculation, registry metadata, or inference

This memo measures one external venue. It is not a claim about all JUNO liquidity, and it does not imply that external AMM liquidity will seed a prediction market.

## Asset and software pins

The Cosmos Chain Registry `osmosis/assetlist.json` at commit [286ef1a6](https://github.com/cosmos/chain-registry/commit/286ef1a6ca0164a13a590fd86ade7acd612f6c52) maps native Juno `ujuno`, transferred over Osmosis `channel-42`, to:

```text
ibc/46B44899322F3CD854D2D46DEEF881958467CDD4B3B10086DA49296BBED94BED
```

That mapping is registry metadata, not chain execution. The pool balances below are direct Osmosis state. A height-pinned node-info query reported Osmosis `31.0.2`, app commit `a56c05b0e83341b9a3c0e6e3508520f15e9f2e49`, and Cosmos SDK `v0.50.14`. Osmosis describes these GAMM pools as Balancer-style pools in its [v31.0.2 source repository](https://github.com/osmosis-labs/osmosis/tree/v31.0.2).

The [exact two-provider HTTP archive](raw/osmosis-66387548/README.md) contains the block, pool, and 24-hour TWAP responses and their integrity hashes.

## Observed dominant legacy pools

A live all-pools discovery query immediately around the snapshot found many JUNO pools, including dust pools and concentrated pools. The two largest visible equal-weight legacy pools by JUNO reserve were selected, then their balances were queried at the exact archived height; no claim is made that they are the only routeable liquidity.

| Pool | Pair | Swap fee | JUNO reserve | Counter reserve | No-fee reserve-ratio spot |
| --- | --- | ---: | ---: | ---: | ---: |
| 497 | JUNO/OSMO | 0.3% | 355,829.816267 JUNO | 226,447.388767 OSMO | 0.636392394383 OSMO/JUNO |
| 498 | JUNO/ATOM | 0.3% | 711,889.169521 JUNO | 9,795.259081 ATOM | 0.013759528169 ATOM/JUNO |

Together those pools held 1,067,718.985788 JUNO. This is inventory, not guaranteed executable depth: balances can move, routes can fail, and arbitrage/counter-asset risk matters.

## Worked 200-JUNO exit comparison

This is a deterministic calculation over the observed equal weights and balances, not an executed quote. It includes the pool's 0.3% swap fee but excludes any separate protocol taker fee, gas, or router behavior. For a gross 200-JUNO sale into a single pool:

```text
net_in = 200 × (1 - 0.003) = 199.4 JUNO
counter_out = counter_reserve × net_in / (juno_reserve + net_in)
```

| Pool | 200 JUNO as % of JUNO reserve | Calculated counter output | Curve impact before fee | Fee + curve shortfall from pre-trade spot |
| --- | ---: | ---: | ---: | ---: |
| 497 | 0.056207% | 126.825572894 OSMO | 0.056007% | 0.355839% |
| 498 | 0.028094% | 2.742881636 ATOM | 0.028002% | 0.327918% |

Inference: the then-candidate, now-accepted 200-JUNO market collateral cap is small relative to these two observed external reserves. That helps bound an ordinary holder's exit-slippage concern at this snapshot. It does **not** prove that 200 JUNO is a safe prediction-market cap: oracle corruption value, event-driven inventory risk, governance delay, user concentration, and native-Juno venue availability remain separate constraints.

## One-day JUNO/ATOM movement

Osmosis pool 498's on-chain arithmetic TWAP from 2026-07-14T16:28:23Z to 2026-07-15T16:28:23Z was 0.013860588048898842 ATOM/JUNO. The height-66,387,548 no-fee reserve-ratio spot was 0.013759528169, 0.729117% below that trailing TWAP.

Twenty-four consecutive one-hour arithmetic-TWAP queries, each requested against height 66,387,548, produced the following transformed series:

| Hour start UTC | ATOM/JUNO |
| --- | ---: |
| 2026-07-14 16:28 | 0.014002605351938333 |
| 2026-07-14 17:28 | 0.013892646052411111 |
| 2026-07-14 18:28 | 0.013885685597095833 |
| 2026-07-14 19:28 | 0.013904178452478333 |
| 2026-07-14 20:28 | 0.013858437008086111 |
| 2026-07-14 21:28 | 0.013784540702107777 |
| 2026-07-14 22:28 | 0.013786608580135000 |
| 2026-07-14 23:28 | 0.013950280351414722 |
| 2026-07-15 00:28 | 0.013959609351380000 |
| 2026-07-15 01:28 | 0.013967054000000000 |
| 2026-07-15 02:28 | 0.013967054000000000 |
| 2026-07-15 03:28 | 0.013964561636575000 |
| 2026-07-15 04:28 | 0.013928965981589166 |
| 2026-07-15 05:28 | 0.013938353949319444 |
| 2026-07-15 06:28 | 0.013947659046296111 |
| 2026-07-15 07:28 | 0.013895917551755277 |
| 2026-07-15 08:28 | 0.013766517130112222 |
| 2026-07-15 09:28 | 0.013743402367386666 |
| 2026-07-15 10:28 | 0.013750442898837777 |
| 2026-07-15 11:28 | 0.013750636930266666 |
| 2026-07-15 12:28 | 0.013750640000000000 |
| 2026-07-15 13:28 | 0.013750640000000000 |
| 2026-07-15 14:28 | 0.013750640000000000 |
| 2026-07-15 15:28 | 0.013757036234386666 |

Calculated over those observations:

- minimum 0.013743402367386666 and maximum 0.014002605351938333, a 1.886018% high/low range;
- first-to-last change -1.753739%;
- sample standard deviation of the 23 hourly log returns 0.003943567, or about 0.394357%;
- largest absolute hourly log return 0.011801877, or about 1.180188%.

This is only one day, one pair, and a TWAP-smoothed series. It is deliberately not annualized and cannot establish long-horizon JUNO volatility. More importantly, collateral volatility is not the prediction AMM's only LP risk: a market can jump toward a 0/1 terminal payout when information arrives even if JUNO itself is stable.

## Parameter consequence

This snapshot supplies a primary, reproducible single-venue measurement that was previously absent. It supports the accepted small-canary decision relative to external reserves, but it does not close deployment or scaling evidence gates:

- the accepted 200-JUNO cap retains explicitly accepted risk against the 10-JUNO oracle/challenge economics and correlated positions;
- the now-accepted 2% LP fee is not validated by a one-day collateral series because adverse selection is driven mainly by the event outcome and time-to-resolution;
- venue-complete liquidity, longer-window volatility, and repeat measurements at stressed dates remain deployment-planning work.
