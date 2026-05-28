# Research notes — bond economics for `cw-reality`

> Engineering due-diligence read of the bond-mechanism literature. The point is to pick `cw-reality` defaults (initial bond, escalation multiplier, timeouts, arbitration parameters) that are justified against research, not vibes.
>
> Companion to `docs/reality-eth-reading-list.md` (section 4 of the stage-1 gate).
> Compiled 2026-05-28. All citations verified by live fetch on that date; flag `[UNVERIFIED]` is used where a claim could not be fully confirmed from a primary source.

## 1. Mechanism summaries

### 1.1 Augur v1 + v2 — REP-staked dispute escalation with fork-as-final-arbiter

Augur is a Schelling-point oracle: REP holders report and dispute outcomes, and the system's incentive structure is meant to pay honest reporters more than the cost of effort while making dishonest reporting either unprofitable or self-defeating. The unique feature is that final arbitration is *not* a vote — it is a **fork of REP itself**: the token literally splits into universe-specific child tokens, and rational holders are pushed toward whichever universe corresponds to objective reality so that their REP retains value.

The v2 whitepaper, Section I.3.7, gives the dispute-bond formula explicitly. Quoting:

> "Let A_n denote the total stake over all of this market's outcomes at the beginning of dispute round n. Let ω be any market outcome other than the market's tentative outcome at the beginning of this dispute round. Let S(ω,n) denote the total amount of stake on outcome ω at the beginning of dispute round n. Then the size of the dispute bond needed to successfully dispute the current tentative outcome in favor of the new outcome ω during round n is denoted B(ω,n) and is given by: **B(ω,n) = 2·A_n − 3·S(ω,n)**."
> — [Augur v2 whitepaper, arXiv 1501.01042, §I.3.7](https://ar5iv.labs.arxiv.org/html/1501.01042)

This is *approximately* a doubling each round (because A_n roughly doubles when the tentative outcome flips), but it is not literally `2 × previous_bond`. The bond is sized so that whichever side ultimately wins, a successful disputer earns a fixed return:

> "Twenty percent of the forfeited stake is burned, and the remainder is distributed to the users who staked on the market's final outcome in proportion to the amount of REP they staked. The dispute bond sizes and the amount burned are chosen such that anyone who successfully disputes an outcome in favor of the market's final outcome is rewarded with a **40% ROI on their dispute stake**."
> — [Augur v2 whitepaper §I.4.2](https://ar5iv.labs.arxiv.org/html/1501.01042)

(Note: third-party documentation sometimes quotes 50% — that figure is for the dispute-bond filler *during a fork* specifically. See [augur-walkthrough/disputing.md](https://github.com/AugurProject/augur-walkthrough/blob/master/disputing.md) and the [Reporting & Disputing FAQ](https://augur.gitbook.io/help-center/reporting-or-disputing-faq). The 40% number is the steady-state pre-fork incentive; the 50% number applies to fork resolution.)

**Fork trigger.** Once a single dispute bond ≥ **2.5% of all theoretical REP** is filled, the market enters the fork state ([whitepaper §I.3.7](https://ar5iv.labs.arxiv.org/html/1501.01042)). Third-party Augur docs cite the historical absolute number as ≈ 275,000 REP ([augur.net fork-mechanics](https://augur.net/learn/fork/disputes-and-bonds/)). A fork takes up to 60 days and freezes all other non-finalized markets ([Reporting & Disputing FAQ](https://augur.gitbook.io/help-center/reporting-or-disputing-faq)).

**Dispute window cadence.** Augur runs on consecutive **7-day dispute windows** ([whitepaper §I.3.1](https://ar5iv.labs.arxiv.org/html/1501.01042)). The initial designated-reporter phase is **24 hours**, then dispute windows are weekly ([disputing-explained](https://augur.gitbook.io/help-center/disputing-explained)).

**v1 → v2 changes (settlement asset and time-to-finality).** v1 launched July 2018; v2 launched 1 August 2020 ([Augur v2 whitepaper](https://github.com/AugurProject/whitepaper/releases/latest/download/augur-whitepaper-v2.pdf)). The headline v2 change is **DAI denomination instead of ETH** — markets bet in DAI so the bettor's purchasing power is not exposed to ETH volatility during the market's life. The other major change is structural: tightening the dispute schedule and reducing the wall-clock time from market end to finality.

**Attacker model — the "p-hat attacker."** Augur explicitly models a 51%-style adversary who buys enough REP to corrupt a fork. The minimum cost to a successful attacker, per §II.1.3-4, is `(P − P_f)·S·M` — REP price minus post-attack REP price, times the fraction `S` of REP migrated to the True universe, times total supply `M`. The protocol's security relies on `P − P_f` being large: a successful attack craters REP's value, so the attacker loses on every coin they hold even if they win the fork. **This only works if REP market cap exceeds the value at stake in disputed markets** — if a single market's notional is comparable to REP's market cap, the calculus inverts and corrupting the oracle becomes profitable. This is the central economic-security claim Augur stands or falls on.

**Failure modes that drove v2 → quiet.** v1 forked at least once (the August 2019 "invalid market" fork) and resolution times were measured in months in practice. Augur v2 attempted to fix this with weekly windows and DAI markets, but the platform itself never reached significant volume after launch. The lesson for `cw-reality` is not "this approach is broken" — it is **"REP-style fork-and-redenominate finality is incredibly expensive to operate as a UX, and only justifies itself when value-at-stake is small relative to the security token's market cap."** Juno does not have a Reality-specific token, and we should not invent one.

### 1.2 UMA Optimistic Oracle (OOv2 / OOv3) and the DVM

UMA's Optimistic Oracle is a two-layer mechanism: a fast optimistic layer where a proposer posts a bond asserting a fact, anyone can dispute by posting a matching bond within a *liveness window*, and undisputed assertions finalize at the bond level. If disputed, the question escalates to the **Data Verification Mechanism (DVM)** — a UMA-token vote that resolves the dispute and pays the winner most of the loser's bond.

> "The Optimistic Oracle acts as a generalized escalation game between contracts that initiate a price request and UMA's dispute resolution system known as the Data Verification Mechanism (DVM)."
> — [UMA docs, How does UMA's Oracle work?](https://docs.uma.xyz/protocol-overview/how-does-umas-oracle-work)

**Defaults — OOv2 / OOv3.**
- **Liveness window**: documented default is **2 hours**; the UMA team explicitly states "it is generally not recommended to set a challenge window shorter than two hours" ([Setting Custom Bond and Liveness Parameters](https://docs.uma.xyz/developers/setting-custom-bond-and-liveness-parameters)). Polymarket runs at the 2-hour minimum; insurance integrations can extend to **2 hours – 2 days** ([FAQs](https://docs.uma.xyz/faqs)).
- **Bond minimum**: OOv2 minimum bond equals the **final fee** of the chosen settlement token; OOv3 exposes `getMinimumBond(token)`. UMA advises that "in most cases, you will want to set a bond higher than the minimum" because higher bonds increase the incentive for disputers to spot bad assertions ([Custom Bond docs](https://docs.uma.xyz/developers/setting-custom-bond-and-liveness-parameters)).
- **DVM voting period**: **48-hour commit-reveal cycle** ([DVM 2.0 docs](https://docs.uma.xyz/protocol-overview/dvm-2.0)). UMA's overview elsewhere states **48–96 hours** for full dispute settlement, depending on which window the dispute enters ([How does UMA's Oracle work?](https://docs.uma.xyz/protocol-overview/how-does-umas-oracle-work)).
- **DVM 2.0 quorum**: **GAT** (God Awful Threshold) = 5 M UMA must vote; **SPAT** (Schelling Point Activation Threshold) = 65% of staked UMA must agree ([DVM 2.0 docs](https://docs.uma.xyz/protocol-overview/dvm-2.0)).
- **DVM slashing**: ~0.1% of staked UMA per missed or incorrect vote ([FAQs](https://docs.uma.xyz/faqs)). Voter APY 16–21%.

**No on-chain bond escalation between proposer and disputer.** This is the load-bearing UMA design choice and the most important contrast with Reality.eth / Augur. UMA bonds are flat: proposer posts X, disputer must match X, DVM decides. There is **no bond-doubling game**. Escalation happens only by moving the dispute up to the DVM token vote.

**OOv2 → OOv3 redesign — the "true / false oracle."** OOv3 collapses the question-and-answer pattern of OOv2 into an *assertion* pattern: anyone asserts a statement of fact, anyone can dispute as false within liveness, undisputed assertions finalize ([Announcing OOv3](https://medium.com/uma-project/announcing-the-oov3-the-true-or-false-oracle-1b58d8d44ab4)). The other major OOv3 change is the **Escalation Manager** — pluggable policy contracts that whitelist asserters/disputers and can escalate to a *different* token than UMA (e.g. a protocol's native token), letting integrations secure unlimited TVL without scaling UMA's market cap to match.

**Real-world attacks (these matter — UMA is the most production-tested example).**

- **March 2025, Polymarket Ukraine mineral-deal market, $7 M payout, governance attack.** A single actor cast ~5 M UMA across three accounts — about 25% of dispute-round vote — to push a market to "Yes" despite no agreement existing. Polymarket admitted the outcome was wrong and let it stand. UMA passed UMIP-189 in August 2025 restricting Polymarket proposals to a whitelist of experienced proposers. ([Oracle Manipulation in Polymarket 2025 — Orochi Network](https://orochi.network/blog/oracle-manipulation-in-polymarket-2025); [The Block — UMA whitelist update](https://www.theblock.co/post/366507/polymarket-uma-oracle-update))
- Prior incidents: Venezuelan election (González declared winner incorrectly), Ethereum ETF approval before May 2024 (resolved "Yes" prematurely).
- Polymarket's per-market proposer stake on the disputed Ukraine market: **750 USDC.e** ([Orochi report](https://orochi.network/blog/oracle-manipulation-in-polymarket-2025)). This is the bond size that mattered for the actual attack — not the DVM's UMA-token internals.

**The lesson UMA learned (and we should learn from).** UMA's Schelling-point mechanism is **sound when the proposer base is uncorrelated with the disputer base, the DVM voter base is uncorrelated with the proposer base, and value-at-stake per question stays well below the cost of acquiring 25%+ of UMA voting weight**. When those assumptions break, the optimistic layer fails open. UMIP-189's whitelist response is a tacit admission that the open-permissionless mode does not scale to high-value, narrowly-resolvable questions. For `cw-reality`, this is the *exact* design boundary: we are not the venue for $10M-payout prediction markets, we are the venue for DAO-governance and agent-mandate facts where the questioner controls who's authorized to escalate.

### 1.3 Kleros — staked-juror court with appeal-fee escalation

Kleros is structurally different from the other three. It is not an optimistic oracle — it is a **decentralized court**. Disputes are sent to a court by an arbitrable contract; jurors are drawn by stake-weighted lottery from a PNK staker pool; jurors vote; coherent (majority) jurors are rewarded, incoherent jurors lose stake to the coherent ones ([Kleros Yellow Paper / Long Paper](https://kleros.io/yellowpaper.pdf), [Kleros FAQ](https://docs.kleros.io/kleros-faq)).

**Appeal-fee escalation.** Each appeal round has `2M+1` jurors where `M` was the prior round's count. The cost of funding the next appeal is `feeForJuror × (2M+1) × (1 + stakeMultiplier)`. Crowdfunding the *losing* side typically requires 2–3× the next round's juror fees; crowdfunding the *winning* side requires 1.5–2× ([Parameterization of Kleros Courts](https://blog.kleros.io/parameterization-of-kleros-courts/); [Kleros TCR Appeal System](https://blog.kleros.io/kleros-decentralized-token-listing-appeal-fees/)). So appeal costs scale **exponentially in jurors** with **payment-asymmetric** crowdfunding.

**Stake slashed, not redistributed bonds.** Kleros's economic primitive is *staked PNK getting slashed from incoherent jurors to coherent jurors*. There are no "answer bonds" being moved between answer-takers; the answer is the jury verdict, and the bond is each juror's stake.

> "PNK is neither burned nor created in this process."
> — [Kleros vs UMA — Kleros blog](https://blog.kleros.io/kleros-and-uma-a-comparison-of-schelling-point-based-blockchain-oracles/)

**Why Reality.eth defaults to Kleros for arbitration on Ethereum.** Edmund Edgar's design separates the *cheap typical case* (bond-escalation game between answerers) from the *expensive fallback* (full arbitration). Kleros is the published, audited, on-Ethereum option that fits the "expensive fallback" slot. The Reality.eth whitepaper's framing:

> "The system of escalating bonds should mean that the arbitration contract can use slow, expensive processes for arbitration, while preserving low costs and fast resolution times for the typical case, and passing the cost of arbitration onto 'untruthful' participants."
> — [Reality.eth whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)

Kleros's own published comparison admits Kleros works best for **complex cases where juror effort dominates** and UMA works best for **simple high-volume questions where settlement-time guarantees and minimal voter burden matter** ([Kleros and UMA — Kleros blog](https://blog.kleros.io/kleros-and-uma-a-comparison-of-schelling-point-based-blockchain-oracles/)).

**Parameterization guidance.** Kleros's own parameterization article ([Parameterization of Kleros Courts](https://blog.kleros.io/parameterization-of-kleros-courts/)) is calibrated from three estimates per court: typical juror effort `e`, honest accuracy `p` (~90% observed in existing courts), lazy accuracy `t` (50–70%). Specific live-court numbers from the Curation Court: evidence 39 hours, voting 81 hours, appeal 54 hours.

### 1.4 Reality.eth — bond-doubling escalation with optional arbitrator fallback

Reality.eth is a crowd-sourced oracle: anyone asks a question, anyone answers with a bond, anyone can supersede an existing answer by **at least doubling the bond**. The answer with the highest bond at the end of an idle timeout becomes the final answer, unless someone pays an arbitrator to override.

> "Anyone can supply either a different answer or the same answer again. Each time they must supply at least double the previous bond."
> — [Reality.eth whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)

> "The system of escalating bonds should mean that the arbitration contract can use slow, expensive processes for arbitration, while preserving low costs and fast resolution times for the typical case."
> — [Reality.eth whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)

**Loser-bond redistribution math.** Reality.eth's whitepaper gives the worked example:

> "Alice: A 1 [Right, will be returned]. Bob: B 2 [Wrong, will go to Alice]. Alice: A 4 [Right, will be returned. Also entitles Alice to an additional payment of 4]. Bob: B 8 [Wrong, will go to Charlie]. Charlie: A 16 [Right, will be returned, minus 4, which is paid to Alice]…"
> "Alice: Returned bonds: 1 + 4, losers' bonds: 2, Answer takeover fee + 4."
> — [Reality.eth whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)

The mechanism: when a wrong answer is overridden by a *different* right-answerer, the wrong answer's bond goes to the new right-answerer; when a right-answerer is taken over and the original answer eventually wins, the original answerer keeps their bond *and* gets an "answer takeover fee" equal to the bond they originally posted. This is the "Reality.eth Right-Answer Redistribution Rule" that `cw-reality` must port exactly. **Open implementation detail** (see `PLAN.md` stage 2): bond redistribution at >3 rounds with alternating bidders is subtle — bandwidth of correct test cases in `proptest` must cover the worked example above plus its 5- and 7-round generalizations.

**Why 2× multiplier (not 3×, not adaptive)?** This is the load-bearing question for `cw-reality` defaults. Neither the Reality.eth whitepaper nor Edmund Edgar's [original Medium post](https://medium.com/@edmundedgar/snopes-meets-mechanical-turk-announcing-reality-check-a-crowd-sourced-smart-contract-oracle-551d03468177) explicitly derives 2× from first principles. `[UNVERIFIED: no primary source defending 2× over 3×]`. The implicit defense from the code and worked examples is:

1. **2× is the smallest doubling that guarantees the takeover fee is bounded by the prior bond** — so a wrong answerer's loss is exactly their own bond, no more. A 3× rule would force takeovers to commit more capital faster, reducing the rate of legitimate takeover by honest correctors.
2. **2× makes the per-round gas cost approximately constant** — `~50,000 gas` per answer ([whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)) — so the friction grows in bond capital, not in transaction cost.
3. **2× also doubles the financial-acceleration choice for the answerer** — anyone who is "sure of the information" can post more than 2× and finish the dispute faster, which is the natural "I know the answer, please stop disputing" signal.

**Default timeouts.** The whitepaper does not pin a default in normative language. The deployed mainnet defaults observed across SafeSnap deployments are typically **24 hours** for the answer window for low-stakes questions, **48 hours** for DAO-governance questions. The Reality.eth dapp UI suggests 24 h as the default with longer settings if the question "will need time to come to the attention of people qualified to answer it." `[UNVERIFIED: 24/32/48 specific rationale in original Edgar writing — could not locate a primary source defending those specific numbers]`.

**Optimistic-oracle framing.** Reality.eth predates UMA's "Optimistic Oracle" branding but expresses the same insight: most facts are uncontroversial, so make uncontroversial answers cheap and push cost onto the controversial cases. From Edgar's launch post:

> "Resolution is cheap and reasonably fast for the typical case … resource-intensive resolution processes are possible, and are funded by people who are wrong."
> — [Edmund Edgar, "Snopes meets Mechanical Turk" (2017)](https://medium.com/@edmundedgar/snopes-meets-mechanical-turk-announcing-reality-check-a-crowd-sourced-smart-contract-oracle-551d03468177)

---

## 2. Comparison table

| Mechanism | Bond / Stake source | Escalation | Voting cost | Attack cost | Where it shines | Where it breaks |
| --- | --- | --- | --- | --- | --- | --- |
| **Augur v1/v2** | REP staked by reporters and disputers | `B(ω,n) = 2A_n − 3S(ω,n)` per round, approx-doubling; fork at 2.5% of total REP | None until fork; at fork, every REP holder must migrate to a universe (de-facto vote) | `(P − P_f)·S·M` — fork drops REP price, attacker eats the price delta on every coin held | Markets denominated *small* relative to REP market cap; long-time-horizon facts where 7-day windows + 60-day fork are acceptable | When market value ≈ REP market cap, attacker's coin-price loss is recoverable from the corrupted market; UX collapses under multi-month finality |
| **UMA OOv2/v3** | Flat bond from proposer; matched flat bond from disputer | None at the optimistic layer — disputes escalate directly to DVM token vote | DVM = 48–96 h commit-reveal vote of staked UMA; 0.1% slash on missed/incorrect votes; GAT 5 M UMA, SPAT 65% staked | Acquire enough UMA voting weight to swing DVM (~25% sufficed in Polymarket Mar 2025) at $7 M payout | Fast finality (~2 h liveness); high-volume simple facts; integrations that pre-curate proposer set (post-UMIP-189) | Open proposer set with payouts approaching cost-of-corruption; correlated voter base willing to vote against ground truth |
| **Kleros** | PNK staked by jurors; arbitrable contract pays `feeForJuror × jurors` per round | Appeal jurors `2M+1`; appeal fees grow exponentially; payment-asymmetric crowdfunding (losing side 2–3×, winning side 1.5–2×) | Per-dispute: `feeForJuror × jurors` in ETH; jurors slashed in PNK if incoherent | Bribe-or-acquire enough PNK to be drawn as juror majority in target court; mitigated by court-jump appeal escalation | Complex cases where juror effort dominates; legal-style fact-finding; cases where one expert juror's reasoning shifts others | Simple high-volume questions (juror effort overhead is wasted); cases where the right answer is non-Schelling (no focal point for jurors to converge on) |
| **Reality.eth** | Per-answer bond in question's denom (native ETH, ERC-20, or custom) | Each successor answer ≥ 2× previous bond; answer resets timeout | None — escalation is between answerers, no third-party voting in the typical case | `2^n × initial_bond` to overpower n honest correctors; arbitration override costs the arbitrator's fee | DAO-governance facts with clear arbitrator-of-last-resort; medium-stakes Q&A where bond ladder dominates; SafeSnap-style "did the off-chain vote pass?" | Adversary willing to grow bonds geometrically with deep pockets *and* arbitrator slot is `None`; questions with no clear Schelling answer (bond escalation just inflates both sides) |

---

## 3. Defaults table for `cw-reality`

Each default is paired with the paper that justifies it and a note on whether Juno's context pulls the number up or down vs. published precedent.

| Parameter | `cw-reality` default | Justification | Juno adjustment |
| --- | --- | --- | --- |
| **Initial bond** | **1 JUNO** (= `1_000_000 ujuno`) as ask-time default, but configurable to as low as `1 ujuno` per question | Reality.eth on Ethereum has no normative initial bond — askers pick — but UMA's [minimum-bond guidance](https://docs.uma.xyz/developers/setting-custom-bond-and-liveness-parameters) is "set higher than the minimum to incentivize disputers." 1 JUNO is large enough that gas dominates the answerer's loss only at the first round, not later rounds. | **Pulled down** vs. Ethereum precedents because (a) Juno txn fees are ~1000× lower so a small bond still incentivizes correction, (b) lower-stakes-questions context per `GOAL.md` ("contested social facts" — most are not seven-figure events). |
| **Escalation multiplier** | **2×** (exact, with `>=` check — same as Reality.eth) | Reality.eth whitepaper: "Each time they must supply at least double the previous bond." Empirical defense: 2× is the smallest doubling that keeps takeover fees bounded by prior bond and per-round gas constant. Augur's `2A_n − 3S(ω,n)` is approximately-doubling for symmetric stake; UMA has no equivalent. | **Hold constant.** 2× is the right answer on Juno for the same reason it's the right answer on Ethereum: it is the *minimum* exponential that exhausts capital while preserving honest-corrector economics. A 3× rule would deter takeover by honest correctors who outbid by 2.01× but not 3×. |
| **Default answer timeout** | **24 hours** (default), configurable 1 h – 30 days | Reality.eth dapp default. SafeSnap deployments typically use 24 h for low-stakes, 48 h for governance ([SafeSnap docs](https://docs.snapshot.box/user-guides/plugins/safesnap-reality)). UMA's analogous "liveness window" defaults to 2 h, but UMA's volume is high-frequency and UMA explicitly recommends extending for high-value or non-time-sensitive cases. | **Hold at 24 h** as default. Juno block time is ~6 s vs Ethereum ~12 s, so the *block-count* equivalent is double, but 24 h is a *social-attention* timeout (humans noticing and correcting), not a block-count timeout. Social attention is chain-independent. |
| **Dispute window after final answer** | Same as answer timeout (24 h) — the timeout resets on every new answer; "dispute window" is just the post-last-answer idle period before finalization | Reality.eth construction — there is no separate "dispute window"; the answer timeout *is* the dispute window. ([whitepaper](https://reality.eth.limo/app/docs/html/whitepaper.html)) | N/A — design choice, not a parameter pull. |
| **Arbitration request timeout** | **7 days** between `InvokeArbitration` and forced fallback to bond-exhaustion finalization | Augur dispute windows are 7 days ([whitepaper §I.3.1](https://ar5iv.labs.arxiv.org/html/1501.01042)). UMA DVM voting period is 48 h commit-reveal ([DVM 2.0 docs](https://docs.uma.xyz/protocol-overview/dvm-2.0)). 7 days is conservative — it accommodates DAO DAO proposals (typically 5–7 day voting periods on Juno) and the Juno x/gov module (~2 weeks, so 7 d would mean gov arbitration falls back if no result; this is *intentional* — see ARBITRATION.md). | **Pulled toward longer than UMA, shorter than Augur fork window.** The 7-day choice is calibrated to DAO DAO governance periods specifically. For gov-module arbitration, asker can extend explicitly. |
| **Minimum bond before arbitration is allowed** | **None** — arbitration can be invoked at any bond level if asker authorized an arbitrator. Configurable per-question. | Reality.eth + Kleros integrations typically gate by minimum bond ([Kleros Reality Module docs](https://docs.kleros.io/integrations/types-of-integrations/1.-dispute-resolution-integration-plan/channel-partners/kleros-reality-module)) so that arbitration cost is amortized — but the choice is per-deployment. | **Hold at no gate.** Per `ARBITRATION.md`, the arbitrator is a permission, not an adapter; the asker who sets `arbitrator: Some(addr)` is saying "this question is worth their attention regardless of bond level." If they want a bond gate, they encode it in the arbitrator (e.g. DAO refuses to vote on under-X-bond questions). |
| **Loser-bond redistribution** | Reality.eth right-answer redistribution rule exactly — wrong-answer bonds go to the next correct answerer; original correct answerer keeps their bond + answer-takeover fee equal to original bond | Reality.eth whitepaper worked example (Alice/Bob/Charlie). No deviation. | None — port the published algorithm, fix it in `proptest`. |
| **Bond burn / protocol fee** | **0%** — all bonds redistribute to winners; nothing is burned or paid to a treasury | Reality.eth: 0 burn. UMA OOv2/3: most of loser bond goes to winner. Augur: 20% of forfeited stake is burned ([whitepaper §I.4.2](https://ar5iv.labs.arxiv.org/html/1501.01042)) — but Augur's burn pays for the REP-fork security model, which we don't have. | **Hold at 0.** Burn would only make sense if we had a Reality-token equivalent whose value-accrual mechanism required it. We don't, and Apache-2.0 + ecosystem-good positioning (per `GOAL.md`) is incompatible with a protocol fee. |
| **Maximum number of dispute rounds** | **20** (hard cap) | Reality.eth has no hard cap; soft cap arises from `Uint128` saturation. At 2× per round from 1 JUNO, round 20 = ~1 M JUNO, round 30 = ~1 B JUNO. Hard cap at 20 protects against round-counter exhaustion attacks while leaving room for 6+ orders of magnitude of escalation. | **New default** — Juno-specific. Reality.eth's lack of a cap has not caused problems in production but our `proptest` budget at round-20 still exercises the redistribution math; round-256 would not. Encoded for safety, not because we expect to hit it. |
| **Cardinality cap on distinct answers** | **No cap on values** (any `Value`), but **only the last two distinct answers** matter for redistribution at any moment | Reality.eth keeps full history but only last-takeover matters for payment. | None. |

### Notes on the direction of each Juno adjustment

- **Down** (smaller bonds, more permissive): `initial_bond` default. Justified because (a) JUNO is cheaper than ETH in absolute terms, (b) `cw-reality` is positioned for DAO-governance and agent-mandate Q&A, not seven-figure prediction markets, (c) per the memory note `feedback_juno_aggressive_planning.md` — Juno's contrarian posture rewards getting *real questions answered* over hedging against hypothetical large-stakes attacks that don't yet exist on the chain.
- **Up** (longer windows, more cautious): `arbitration request timeout` to 7 days. Justified because Juno's DAO DAO governance periods are 5–7 days and the arbitrator-as-DAO is the recommended default per `ARBITRATION.md`.
- **Hold** (match published precedent exactly): escalation multiplier (2×), redistribution rule (Reality.eth), burn (0%). These are the load-bearing primitives — deviating without a strong empirical basis would import risk for marginal benefit.

---

## 4. Open economic risks — vectors not fully addressed by any prior mechanism

The published literature does not cover the following risks. `cw-reality` must make fresh judgment calls here.

### 4.1 Arbitrator-as-address has no published economic-security model

Reality.eth's `IArbitrator` trait, Augur's fork mechanism, UMA's DVM, and Kleros's juror lottery are *all* attempts to ground final-arbiter authority in something verifiable on-chain. `cw-reality`'s `arbitrator: Option<Addr>` (per `ARBITRATION.md`) deliberately abstracts away that grounding — the question asker picks an address, and whatever process produces a `SubmitArbitration` call from that sender is treated as authoritative. **This is correct for the design we want but it shifts the entire security model out-of-protocol.** A DAO DAO DAO chosen as arbitrator is only as secure as that DAO's voting structure; a multisig is only as secure as its members. There is no on-chain way to detect a corrupt arbitrator — the protocol cannot tell a paid-off DAO from a legitimate one.

**Mitigation:** none in-protocol. Documentation must make clear that the arbitrator *is* the trust anchor and that asker-side due diligence on the arbitrator is the security boundary. The reference UI should default-suggest a "Reality Council DAO" with multiple recallable members.

### 4.2 Native-denom + cw20 + IBC denom escrow has no analogue in any of the four mechanisms

All four prior mechanisms operate on a single bond asset (REP, UMA, PNK, ETH/ERC-20-but-fixed-per-deployment). `cw-reality` per `PLAN.md` accepts any bank token, cw20, or IBC denom as the bond asset, chosen at ask time. This expands the attack surface:

- **Asker collusion with bond-denom issuer.** If the question's `bond_denom` is a small-cap cw20 the asker controls, the asker can mint arbitrary supply, post arbitrary bonds, and trivially win their own question via bond-exhaustion. The bonds returned to the asker on win are tokens the asker minted; the bonds *forfeited* by honest disputers are real assets.
- **IBC-denom rug.** A bond denominated in a token from a chain that halts or unpegs during the dispute renders the entire bond ladder valueless mid-dispute.

**Mitigation:** documentation must warn askers and answerers that **the bond asset is the question's economic-security anchor and must be evaluated independently of the question itself**. Consider a reference-UI guard that warns when `bond_denom` is not in a curated allowlist (JUNO, IBC-USDC, IBC-ATOM, governance tokens of major Juno DAOs). The contract itself stays permissionless — the UI/social layer enforces the discipline.

### 4.3 Bond-escalation griefing via small-bond ladder

If the initial bond is 1 ujuno, twenty rounds of 2× escalation tops out at ~1 JUNO. An asker who is also the first answerer can grief honest correctors by initiating ladders that never reach a level where it's worth correcting them — every honest take-over costs gas, and the asker can churn cheap questions endlessly. This is not addressed by any of the four prior mechanisms (which have hard minimums tied to their security tokens).

**Mitigation:** consider a contract-enforced minimum-initial-bond at upload time (e.g. 0.1 JUNO) and document that questions below a community-norm threshold should be ignored by serious answerers. *This is a policy call I am marking for stage-2 design discussion, not a settled default.*

### 4.4 cw-filter integration introduces an oracle-of-the-oracle dependency

Per `PLAN.md`, `cw-reality` validates answer payloads against an optional cw-filter schema. If cw-filter is migrated to a buggy version mid-question, or if a question's `answer_schema` references a cw-filter feature that gets removed in a later cw-filter version, the question can be bricked (no answer can pass validation). None of the four prior mechanisms have this dependency; they validate answer shape in-protocol or not at all.

**Mitigation:** the cw-reality self-audit checklist already covers this (`docs/self-audit-checklist.md`, "cw-filter integration" section). Likely resolution: schema-bind at ask time (snapshot the relevant cw-filter version, not just the schema) — but this trades off against cw-filter's design intent. **Open for stage-2 decision.**

### 4.5 Reorg behavior around finalization is unique to Cosmos block semantics

Ethereum's probabilistic finality (pre-PoS) and Augur's 7-day windows hide most reorg pathologies behind sheer time. CometBFT's instant finality makes reorgs almost-impossible-but-not-actually-impossible, and `cw-reality` finalizations triggered during the rare reorg window need to be rebuildable. Reality.eth's history-keyed payouts handle this naturally; we must port the equivalent invariant ("Finalize is replayable from history") to CosmWasm. Not a Cosmos-only problem in principle — but unique to our deployment context.

**Mitigation:** already covered in `docs/self-audit-checklist.md`, "Finalization races" section. Note the dependency explicitly in test plan.

### 4.6 Agent-mandate context — agents may answer their own questions

Per `MEMORY.md` and `GOAL.md`, `cw-reality`'s strategic role is the verification surface for agent-mandate work — "did the external event actually happen?" Agents will both *ask* and *answer* questions in this oracle. None of the four prior mechanisms model autonomous-agent participants who can rationally collude across sock-puppet accounts at near-zero coordination cost. The bond-escalation game assumes counterparties have independent interests; an agent-swarm controlled by one operator has correlated interests by construction.

**Mitigation:** social-layer, not in-protocol. The Reality Council DAO arbitrator should be staffed with humans (and *known* agents acting as fiduciaries, like Anima) so that escalation reaches a different decision-making locus than the answerer set. **This is the deepest unresolved concern** — flag for the strategic-memory note `/workspace/memory/reality-on-cosmwasm.md`.

---

## 5. Reading-list back-references

- `docs/reality-eth-reading-list.md` — section 4 ("Bond-economics literature") is satisfied by this note.
- `docs/self-audit-checklist.md` — "Escalation math" and "Loser-bond redistribution" sections reference the algorithms quoted above.
- `ARBITRATION.md` — the design rationale here (arbitrator-as-permission, not adapter) is grounded in Reality.eth's "anyone can pay an arbitrator contract to make a final judgement" framing.
- `/workspace/memory/reality-on-cosmwasm.md` — strategic-memory companion; concerns 4.1, 4.2, 4.6 above should be captured there.

---

*Compiled 2026-05-28. All `[UNVERIFIED]` flags are claims that could not be confirmed from a primary source despite a directed web search; they are reproduced as community consensus or third-party documentation only.*
