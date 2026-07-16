# R5 — Product, legal, content, and operations

**Status:** architecture/product posture accepted 2026-07-16; issue #26 legal and operational-readiness evidence remains open
**Not legal advice:** qualified counsel must advise the actual contributors and interface/indexer operators in every jurisdiction they touch

## Product posture

“Experimental/play money” means the protocol is not marketed as a reliable investment, hedge, calibrated forecast, or guaranteed service. It does not mean JUNO lacks value. Native JUNO is transferable, volatile, and used for network governance. Users can lose all collateral paid for losing positions, LPs can lose principal to informed flow, and every position can remain locked by an unanswered or disputed oracle.

Required first-use acknowledgment:

> This experimental protocol uses value-bearing JUNO. It is unaudited until an audit is linked, has no operator promising support or repayment, and may be unlawful or restricted where you are. Market prices are thin-pool quotes, not probabilities or advice. Read the exact on-chain rules, oracle, close time, liquidity, fees, and non-termination risks before signing.

The interface must not call funds “free,” “risk-free,” “insured,” “practice credits,” or “play-only” without the adjacent statement that real JUNO moves.

## No operating entity is not no responsibility

The protocol can lack a company and still have people performing legally and operationally relevant acts. The architecture does not decide their legal classification.

| Participant | Actual activity | Risks/questions requiring their own advice |
| --- | --- | --- |
| Protocol contributors | Publish financial and oracle software, documentation, releases | Developer liability, facilitation, sanctions/export, licensing, consumer claims, source licenses |
| Factory/code uploader | Publishes immutable code on Juno | Whether upload/deployment is an offered service or regulated act; responsibility for known defects |
| Reference frontend host | Selects, ranks, describes markets and constructs transactions | Market/operator/intermediary status, consumer disclosures, prohibited users/topics, geolocation/privacy, takedown duties |
| Indexer/API host | Republishes and organizes market/activity data | Discoverability, content moderation, availability claims, privacy/data obligations |
| RPC/wallet provider | Relays queries/transactions and presents signing data | Their terms, sanctions controls, misleading transaction rendering, service availability |
| Market creator | Writes the proposition, chooses sources/times, funds liquidity, promotes it | Contract legality, manipulation, defamation/privacy, harmful incentives, misleading statements, tax/reporting |
| Trader/LP/challenger | Moves JUNO and takes financial/governance risk | Local eligibility, tax, losses, source/oracle understanding, sanctions |
| Keeper/answerer | Monitors and posts bonded oracle answers | Bond loss, factual due diligence, conflicts, automation error |
| Governance proposer/voter | Funds and adjudicates a dispute | Conflict, payee selection, procedural/legal exposure, public rationale |

Publishing code, hosting a UI, creating a market, or repeatedly operating keepers may be treated differently. No contributor should rely on the absence of incorporation as a legal conclusion.

Prediction-market rules are changing and jurisdiction-specific. For example, the U.S. CFTC published a 2026 [prediction-markets proposed rule](https://www.cftc.gov/LawRegulation/FederalRegister/proposedrules/2026-05105.html); a proposal is not final law and is cited only to show that current advice must be time- and actor-specific.

## Qualified-counsel gate

Before any public interface, deployment, or organized operations, counsel must provide dated written advice identifying:

- each contributor, uploader, host, indexer, keeper sponsor, and governance-proposal sponsor;
- target/foreseeable jurisdictions and whether access restrictions or licensing apply;
- treatment of event contracts, gambling, derivatives, money transmission, promotion, consumer protection, tax/reporting, sanctions, AML/KYC, minors, and data/privacy;
- whether topic restrictions are legally required and who implements/responds;
- whether “experimental,” open-source, no-entity, and no-custody facts change any result;
- source-license obligations for cw-reality, Reality.eth research provenance, and any FPMM-derived implementation;
- document retention, incident reporting, subpoenas/orders, and a contact process;
- the exact warnings and terms appropriate for each independently operated interface.

The advice must be refreshed when law, contributors, hosting, geographies, collateral, fees, or governance roles change. This memo identifies questions and controls; it does not answer them.

## Permissionless protocol versus discoverability

The immutable factory accepts every address that satisfies objective parameters. It has no creator allowlist, geography check, content moderator, or delisting execute. Anyone can query and interact directly.

Each independent interface controls only its own catalog. The reference policy is:

1. Newly created markets are available by exact address/hash but are not automatically promoted.
2. Automated checks quarantine duplicates, malformed metadata, impersonation indicators, missing sources, dangerous terms, and economically trivial liquidity.
3. Human review may mark Listed, Unlisted, Warning, Duplicate, or Unsafe for that interface.
4. Status, reason, reviewer policy version, time, and evidence are published. It never changes settlement.
5. Reports reference chain ID, market address, question hash, category, and evidence. Reporters do not submit seed phrases or private identity data.
6. A different reviewer handles appeals where practicable. An appeal can change discovery status, never rules or payout.
7. Independent interfaces may reach different decisions. None is “the protocol's” authoritative truth.

Search and ranking must disclose paid promotion and creator/LP relationships. Volume, marginal price, and liquidity must not be presented as editorial validation.

## Reference-interface prohibited content

The reference interface will not list or promote markets that:

- directly reward causing death, violence, self-harm, abuse, kidnapping, sabotage, or other physical harm;
- solicit or price illegal transactions, sanctions evasion, trafficking, exploitation, or stolen data;
- concern a minor's sexual, medical, location, or other sensitive private information;
- expose nonpublic personal data, dox a person, or create a credible stalking/harassment risk;
- state unverified criminal, sexual, medical, or similarly damaging allegations about an identifiable person;
- are cheaply manipulable by a trader or creator, including self-authored “will I do X” harmful acts;
- impersonate an official market/source or use confusingly similar metadata;
- use subjective, non-exclusive, circular, or materially incomplete resolution rules;
- have sources inaccessible to ordinary resolvers or depend on private creator discretion;
- violate applicable law or the host's counsel-approved policy.

Political, financial, health, conflict, disaster, election, and public-person topics require enhanced review even when not categorically prohibited. Interfaces should prefer a narrow warning/unlisting decision to pretending they can prevent direct on-chain use.

## Transaction and display contract

Before signature, the reference UI shows from on-chain queries:

- market/factory addresses and code versions;
- exact collateral denom and raw/display amount;
- action, funds, min_out/max_in, deadline, and resulting position estimate;
- marginal quote, average execution price, fee, impact, and pool depth;
- close_ts, opening_ts, optimistic earliest finality, and arbitration maximum;
- exact resolution bytes/hash and primary sources;
- oracle address/checksum/admin status, question ID, current answer/bond/state;
- governance verdict authority, challenge bond, and timeout slash;
- liquidity lock and LP payoff examples;
- warning when indexer data differs from direct RPC query.

Wallet signing JSON is decoded and compared to this summary. A frontend never asks for a seed phrase, private key, or arbitrary blind signature. Links cannot prefill a different market without an address/hash warning.

Financial facts remain independently queryable. If direct RPC and indexer disagree, trading is disabled in the reference UI until the discrepancy is explained; direct contract state controls.

## Protocol assumptions it cannot enforce

- user identity, age, residency, sophistication, sanctions status, or tax compliance;
- creator truthfulness, source availability, or semantic clarity;
- whether a user read warnings or used a third-party frontend;
- JUNO fiat value, exchange liquidity, chain uptime, validator/governance independence;
- RPC honesty or wallet rendering;
- somebody answering an oracle question or funding a governance deposit;
- lawfulness in every jurisdiction;
- removal of on-chain text;
- censorship by validators, RPCs, indexers, app stores, hosts, or wallets;
- profitability, forecast calibration, or timely exit.

These are disclosed constraints, not TODOs an admin key can solve.

## Operations model

There is no protocol operator with privileged state access. Before a reference launch, named people or service providers must accept each off-chain monitoring role and publish coverage/backup contacts.

| Monitor | Trigger | Urgency | Permitted response |
| --- | --- | --- | --- |
| Factory creation | Every new market | Routine | Verify objective fields; queue discoverability review |
| Collateral invariant | bank < accounted liabilities, supply mismatch, nonpositive reserve | Critical | Stop interface routing, compare independent RPCs, publish incident; no fund mutation |
| Close boundary | 24h/1h before and first block at/after close | High | Check UI stops quotes/trades; alert if execute succeeds |
| Oracle opening | opening reached | High | Verify question fields and bounty; prompt independent answerers |
| Unanswered | +1h, +12h, +24h, then daily | High | Notify keepers/users; never fabricate a market answer |
| Answer/counter-answer | Event and bond/finalize change | High | Re-query direct state, display exact bytes and countdown |
| Challenge | Market enters PendingArbitration | Critical | Verify bond segregation, publish proposal instructions/deadline |
| Governance proposal | submitted/deposit/vote/result/execute | Critical | Track deposit, tally, inner message, gas/result; warn on failure |
| Arbitration deadline | 72h/24h/1h before and boundary | Critical | Warn; after boundary call or invite permissionless stalled sync |
| Final answer | Oracle Finalized/Claimed | High | Recheck guarantees and invite permissionless Resolve |
| Resolution | payout stored once | Critical | Reconcile P/F/C, bytes, positions, and bank |
| Redemption failure | Any failed claim or bank send | High | Reproduce query/simulation; publish safe workaround if reviewed |
| Indexer drift | missed/reordered event or query mismatch | High | Reindex from finalized height; direct-state warning |
| Chain/RPC | halt, lag, divergent heights | High | Disable stale quotes, show provider heights; do not invent wall-clock closure |
| Reserve anomaly | unexplained reserve/product movement | Critical | Stop routing, preserve traces, reconcile every execute |

No alert handler may pause a contract, choose a verdict, migrate code, or transfer user funds.

## Runbooks

### Unanswered

1. Confirm chain progress, opening_ts, exact question, and bounty on two providers.
2. Notify independent answerers with canonical byte examples and current required bond.
3. A keeper choosing to answer signs its own direct cw-reality transaction and bears bond risk.
4. Record transaction hash and new finalize_ts.
5. If still unanswered, continue daily disclosure. There is no emergency neutral button.

### Challenge/governance

1. Confirm market challenge state equals cw-reality PendingArbitration and snapshot the deadline.
2. Publish a machine-readable, independently decoded proposal template.
3. Identify a voluntary deposit sponsor; never imply the 5,000 JUNO is guaranteed.
4. Verify the inner sender is the pinned gov module, market/question IDs match, funds are empty, and answer/payee are explicit.
5. Track deposit and vote without predicting passage.
6. On execution, verify both market and oracle events plus challenge refund/slash.
7. On rejection/failure/no execution, warn that the full challenge bond will be slashed at timeout.
8. At timeout, invite the permissionless market sync/cancel path and verify the 24-hour answer clock restart.

### Solvency discrepancy

1. Query raw bank balance and every internal liability from two endpoints at the same height.
2. Distinguish benign forced excess from bank below liabilities.
3. Preserve block, transaction, event, and indexer data.
4. Stop reference routing and publish scope/known facts.
5. Do not recommend a migration, social payout, or exploit transaction without separate review and authority.
6. Keep valid redemption access visible if on-chain execution remains safe.

### Malicious frontend/indexer

1. Compare signed message bytes, direct state, and indexed display.
2. Revoke only compromised web/API credentials and distributions; on-chain keys do not exist.
3. Publish verified contract addresses/hashes through independent channels.
4. Rebuild index from a finalized checkpoint and document affected displays/actions.

## Incident roles

Required functions before launch, even if performed by volunteers:

- incident coordinator: timeline, severity, decision log;
- chain investigator: height-pinned queries and transaction traces;
- accounting investigator: liability/reserve reconciliation;
- oracle/governance liaison: question and proposal monitoring, without verdict authority;
- interface/indexer maintainer: warnings and reindex;
- communications owner: factual public updates and corrections;
- legal/privacy contact: counsel escalation and lawful process;
- independent reviewer: approves remediation claims.

Names, time zones, contact methods, backup coverage, and authority limits are an operations acceptance gate. “The community” is not an assigned responder.

## Launch acceptance

The product/operations architecture is accepted, but issue #26 remains the readiness gate until counsel advice is attached, content reviewers and incident roles are named, monitoring is exercised against test/canary events, every warning is usability-tested, and an independent reviewer confirms that unlisting cannot influence settlement. No qualified legal advice is claimed here.
