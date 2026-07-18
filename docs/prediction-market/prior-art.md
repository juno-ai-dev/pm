# R2 — Prior art, changes, and incidents

**Status:** accepted research and provenance baseline (2026-07-16)
**Access date:** 2026-07-15

The matrix separates concepts worth reusing from implementation code. A concept can be independently implemented; copying code can carry license obligations.

## Comparison matrix

| System | Version/evidence | Observed or claimed behavior | Disposition for Juno v1 | License/provenance |
| --- | --- | --- | --- | --- |
| Gnosis Conditional Tokens | [eeefca66](https://github.com/gnosis/conditional-tokens-contracts/tree/eeefca66eb46c800a9aaab88db2064a99026fde5) | Split a collateral position into a complete partition, merge it, and redeem against a payout vector. | Copy concept: separate backing from exchange. Adapt to two internal balances. | Repository license review required before code work. |
| Gnosis FPMM | [6814c024](https://github.com/gnosis/conditional-tokens-market-makers/tree/6814c0247c745680bb13298d4f0dd7f5b574d0db) | Per-market constant product, ceil division in buy/sell quotes, LP fee pool, funding shares. | Copy formulas as public mechanism; independently implement and test. Lock v1 liquidity instead of copying dynamic funding. | Smart contracts are LGPL-3.0. Legal approval is required before any source-derived implementation. |
| Omen | [FAQ](https://omen.eth.link/faq.pdf), [rules](https://omen.eth.link/rules.pdf), and Gnosis FPMM | An FPMM gives continuous quotes; production UI used a 2% fee; rules define invalidity and known-by dates. | Adapt one-market isolation, rules-first resolution, and plain LP-loss warning. Do not treat 2% as empirically proven for Juno. | Documentation and inherited LGPL implementation provenance must be tracked. |
| Polymarket | [CTF overview](https://docs.polymarket.com/trading/ctf/overview), [positions](https://docs.polymarket.com/concepts/positions-tokens), [resolution](https://docs.polymarket.com/concepts/resolution) | Current docs separate complete-set positions from a CLOB and document unknown as 50/50. | Copy concept: exchange and claims are separate layers; use exact question identity and neutral payout. Reject CLOB for bootstrap v1. | Documentation is an author claim; no Polymarket code is proposed for reuse. |
| Reality.eth | [contract integration](https://realityeth.github.io/docs/html/contracts.html), [whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html), source commit b996b0a0 | 32-byte 0/1 bool convention, invalid max-u256, unresolved max-u256−1, escalating bonds, timeout reset, arbitrator freeze. | Adapt exact bytes and reader-side guarantees. Reject implicit trust in question type or arbitrator availability. | Upstream source is GPL-3.0; local cw-reality claims an Apache-2.0 clean-room port. License counsel must verify the implementation strategy. |
| cw-reality | local source at ee64153 and checked-in schema | Opaque Binary answers; explicit states; native/CW20 input; native-only withdraw; arbitrary arbitrator answer/payee; public timeout cancellation. | Use unchanged, native only, through a market controller. Query all fields omitted from ID and guarantees. | Local package identifies Apache-2.0; reproducible on-chain build match is open. |
| Augur v2/current work | [whitepaper repo](https://github.com/AugurProject/whitepaper) and current project repositories | Dispute economics tie oracle security to value and invalid reporting; the design is substantially more complex than an optimistic single-question oracle. | Copy principle: oracle corruption cost must relate to market value. Reject a bespoke reporting token/fork. | Research concepts only. |
| Manifold | [binary CFMM discussion](https://manifoldmarkets.notion.site/6-CFMM-6e19db13b4c54d69ac7f9dda6f772bd1) and [Multi-CPMM](https://manifoldmarkets.notion.site/Multi-CPMM-62fe5b99013c4d69ac7f9dda6f772bd1) | Creator subsidy makes long-tail continuous markets possible; mechanism and fees evolved. | Adapt creator-funded bootstrap. Reject multi-outcome logic. | Author design notes; no code reuse. |
| Injective | [binary lifecycle](https://docs.injective.network/developers-native/injective/exchange/02_binary_options_markets) | Expiration stops trading; settlement timestamp is separate; admin/oracle settles. | Copy concept: close and expected settlement are separate UI/state terms. Reject mutable admin settlement. | Official project documentation. |
| Zeitgeist | main 39ad8d60 and [release history](https://github.com/zeitgeistpm/zeitgeist/releases) | Releases removed Rikiddo, introduced a hybrid router, changed math, and record runtime/migration fixes. | Copy lesson: version markets and route later; do not migrate live economic state to chase a mechanism. | Repository evidence; review the relevant file license before reuse. |
| LMSR | Hanson's [paper](https://hanson.gmu.edu/mktscore.pdf) | A logarithmic scoring rule always quotes and has sponsor loss controlled by liquidity parameter b. | Keep as benchmark; reject exp/log consensus math and sponsor-subsidy model in v1. | Paper formula, not code. |
| pm-AMM | Moallemi/Robinson [paper](https://www.paradigm.xyz/2024/11/pm-amm) | Static and dynamic curves target uniform modeled loss-vs-rebalancing for binary outcome tokens using normal PDF/CDF math. | Keep as later research; reject larger numerical/time-dependent surface before FPMM data. | Paper formula, not code. |

## What changed in production systems

These changes are more informative than feature lists:

- **Exchange can change without changing backing.** Polymarket's current CLOB still relies on complete-set split, merge, and redeem. Juno should preserve that seam even though internal positions are initially non-transferable.
- **AMMs are not permanent doctrine.** Zeitgeist release notes show a path through mechanism removal and hybrid routing. Juno should instantiate versioned markets and leave existing contracts frozen rather than migrate balances.
- **Question rules are financial state.** Omen's invalid rules and Reality.eth templates make “invalid” and “answered too soon” first-class. Juno cannot rely on a title or frontend description.
- **Oracle readers must set guarantees.** Reality.eth exposes matching queries specifically because consuming any finalized answer is insufficient. cw-reality's smaller guarantee query requires additional full-field checks.
- **Advanced math creates operational scope.** pm-AMM is tailored to outcome tokens, but normal CDF/PDF/inverse approximations and time-varying curves would enlarge consensus and audit risk.

## Primary release/audit lessons

| Evidence | Observed change/failure | Transferable control |
| --- | --- | --- |
| Zeitgeist [v0.4.3](https://github.com/zeitgeistpm/zeitgeist/releases/tag/v0.4.3) | The project's release explicitly calls c2ebd4fb a hotfix for a bug in its exponential function. | Avoid transcendental consensus math in v1; use golden vectors, fuzzing, and version isolation for any later advanced curve. |
| Zeitgeist [v0.3.1](https://github.com/zeitgeistpm/zeitgeist/releases/tag/v0.3.1), [v0.5.2](https://github.com/zeitgeistpm/zeitgeist/releases/tag/v0.5.2), and [v0.5.4](https://github.com/zeitgeistpm/zeitgeist/releases/tag/v0.5.4) | Release notes respectively disabled Rikiddo creation, added hybrid book/AMM routing, and removed Rikiddo. | Mechanisms and routing evolve; do not force live-market migration or couple backing to one exchange. |
| Augur's own [v2 design notes](https://medium.com/@AugurProject/augur-v2-details-2547bbfc3c1f) | Augur made INVALID explicit, citing simpler contracts and removal of a known rounding error; it also changed dispute-profit burning to deter low-cost delay. | Invalid and griefing economics are financial design, and every rounding remainder needs an owner. |
| Zeppelin's [Augur core audit](https://medium.com/zeppelin-blog/augur-core-audit-244160d77c09) | The audit pins reviewed/fixed commits and lists critical findings spanning free complete sets, migration after finalization, invalid staking, stolen balances, and frozen fees. | Audit complete-set conservation, migration authorization, invalid paths, fee ownership, and every fund exit together. |
| QuillAudits' [Gnosis Guild Reality Module attack analysis](https://quillaudits.medium.com/gnosis-guild-dao-proposal-attack-analysis-quillaudits-2e237cbd3f7c) | This third-party audit analysis attributes a roughly 7.5-ETH theft to an optimistic malicious proposal surviving a one-hour challenge setting. It is not a project-authored post-mortem. | A functioning oracle adapter is unsafe with an operationally unmonitorable timeout; enforce a floor and decode the downstream action. |

The Zeitgeist releases and Augur design note are project-primary artifacts; the named audits are auditor-primary artifacts. No project-primary Omen post-mortem for a particular ambiguous market was established in this snapshot, so this memo uses Omen's rules as policy evidence and does not invent an incident claim.

## Failure and discrepancy register

| Failure class | Evidence | Juno control |
| --- | --- | --- |
| Ambiguous or premature market | Omen [invalid-market rules](https://omen.eth.link/rules.pdf) and Reality.eth's [invalid/answered-too-soon conventions](https://realityeth.github.io/docs/html/contracts.html) | Immutable resolution document, opening timestamp, exact neutral mapping, reference-interface review |
| Empty/thin liquidity | Omen and Manifold mechanism docs describe creator-funded AMMs; the worked Juno table shows price impact | Minimum initial liquidity, raw depth and price-impact display, locked LP disclosure |
| Numerical mechanism change | Zeitgeist release history includes removed mechanisms and math/runtime fixes | Integer-only v1 formulas, versioned instances, property tests, no live migration |
| Mutable resolution dependency | Height-pinned Juno evidence shows the production cw-reality instance has an admin | Fresh frozen oracle requirement or explicit additional trust; no address-only pin |
| Documentation/source divergence | Local ARBITRATION.md and README conflict with compiled source | Source/schema matrix, exact query checks, unknown bytes neutral, later docs correction |
| Stalled arbitration | cw-reality source permits public cancellation only after its deadline | Accepted 21-day timeout, explicit monitor, deterministic challenge-bond timeout rule |
| Unanswered forever | cw-reality OpenUnanswered has no time transition | Creation-funded bounty, alerts, disclosed non-termination; no secret override |
| Oracle value exceeds security | Augur's dispute design and optimistic-oracle economics | Market cap bound tied to oracle tier; no arbitrary uncapped permissionless exposure |
| Price mistaken for probability | AMM equations and thin-pool table | Label as marginal quote, show depth/impact/fee, never promise calibration |

No claim here attributes a loss event to a project without a primary post-mortem or source. Where a named incident could not be established from a primary artifact during this snapshot, the memo records the failure class rather than repeating secondary reporting.

## License strategy

The accepted project route is a clean-room independent implementation from the public mathematical specification under Apache-2.0. Implementers must not copy, adapt, or translate Gnosis expression, structure, comments, or tests; citations and notices remain research provenance rather than code derivation.

This project authorization is not qualified legal advice. The local claim that cw-reality is a clean-room Apache-2.0 port does not itself resolve upstream GPL analysis, and issue #26 retains any counsel evidence applicable to actual actors and distribution. Papers and mechanism ideas may be cited, but source licenses still govern copied expression.

## Conclusions

The evidence supports FPMM as the smallest continuous, LP-funded bootstrap mechanism, not as a universally superior market. The architecture should copy complete-set semantics, exact question binding, invalid finality, version isolation, and explicit oracle guarantees. It should adapt LP lifecycle and native integer arithmetic to Juno, and reject v1 leverage, mutable settlement admins, advanced transcendental math, and an empty CLOB.
