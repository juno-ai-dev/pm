# Reference-interface discovery, review, report, and appeal policy

**Policy ID:** `juno-pm-reference-discovery`
**Policy version:** `1.0.0-draft`
**Status:** implementation specification; public promoted discovery is fail-closed pending the launch gates below
**Scope:** the independently operated reference indexer/API/UI only
**Not legal advice:** qualified counsel must approve the scope for the actual operators and jurisdictions

This policy separates a permissionless protocol from one interface's editorial catalog. It cannot alter a factory, market, oracle, transaction, direct query, or settlement. Other interfaces may publish different policies and outcomes.

## Invariants

1. Every activated market is reachable by exact chain ID and contract address, whether or not it is promoted.
2. A newly observed market starts `Unlisted`, `exact_address_only`, and quarantined with reason code `pending_review`. Search, browse, recommendations, feeds, and ranking exclude it.
3. Discovery status never gates direct contract queries, exact-address transaction construction, wallet signing, oracle participation, resolution, or redemption. If the reference host disables transaction construction for a security incident, it must continue to show verified addresses and direct-query/export instructions where safely possible.
4. Direct chain state controls financial facts. A catalog record is editorial metadata, not a protocol fact or settlement verdict.
5. Every transition is append-only and records actor, UTC time, reason code and explanation, policy version, and non-sensitive evidence references.
6. No reviewer can edit history. Corrections are new transitions.

## Status and visibility model

The only editorial statuses are:

| Status | Promoted discovery | Exact-address access | Meaning |
| --- | --- | --- | --- |
| `Listed` | Allowed | Required | Reviewed under the recorded policy version; listing is not endorsement. |
| `Unlisted` | No | Required | Default pending state or an editorial removal without a specific warning category. |
| `Warning` | Optional only when the transition explicitly permits it | Required | Reachable with a prominent, reason-specific warning; enhanced review applies. |
| `Duplicate` | No | Required | Substantially duplicates a canonical market; point to both addresses without changing either market. |
| `Unsafe` | No | Required | Meets a prohibited-content, security, privacy, legal-scope, or manipulation criterion. |

`visibility` is either `promoted` or `exact_address_only`. `Listed` requires `promoted`; `Unlisted`, `Duplicate`, and `Unsafe` require `exact_address_only`. `Warning` may use either, but promotion requires an affirmative reviewer decision and counsel-approved scope. `quarantined` is true for every new record and remains true for `Unlisted`, `Duplicate`, and `Unsafe`.

Automated checks may create the initial quarantine and attach evidence, but may not produce `Listed`. Only a currently authorized human reviewer may promote a market.

## Review criteria

Review the exact on-chain market identity, immutable resolution bytes, sources, close/open times, creator, liquidity, oracle, and duplicates. Apply the narrowest supported status; do not adjudicate truth or rewrite a proposition.

Mark `Unsafe` or `Unlisted`, according to counsel-approved scope, when a market:

- directly rewards causing death, violence, self-harm, abuse, kidnapping, sabotage, or physical harm;
- solicits or prices illegal transactions, sanctions evasion, trafficking, exploitation, credentials, secrets, or stolen data;
- exposes nonpublic personal data, concerns a minor's sensitive information, or creates a credible stalking, harassment, or doxxing risk;
- states unverified criminal, sexual, medical, or similarly damaging allegations about an identifiable person;
- is cheaply manipulable by a creator/trader, including self-authored harmful acts or private-discretion outcomes;
- impersonates an official source or market;
- has subjective, circular, non-exclusive, materially incomplete, or inaccessible resolution rules;
- violates applicable law or the host's dated counsel-approved policy.

Political, financial, health, election, conflict, disaster, and public-person markets require enhanced human review. Ambiguity is not cured by interface copy: use `Warning` or `Unlisted`. Price, volume, liquidity, creator identity, or governance attention are not editorial validation.

## Transition controls

A transition record follows [`interface-policy.schema.json`](interface-policy/interface-policy.schema.json). It must contain a globally unique event ID, market identity, `from`/`to`, visibility, quarantine flag, actor ID and role, RFC 3339 UTC timestamp, reason code and explanation, policy version, and at least one evidence reference. Evidence references are immutable or versioned pointers; do not embed personal data or secrets.

Allowed roles:

- `automation`: create default quarantine, detect duplicates/malformed metadata, or recommend a status; never list;
- `reviewer`: make an ordinary catalog decision within current written authority;
- `appeal_reviewer`: decide an appeal only when independent under the rule below;
- `incident_responder`: take temporary interface-only emergency action;
- `policy_admin`: publish policy versions and reviewer authorization records; cannot alter chain state.

Transitions to `Listed` require a human `reviewer` or independent `appeal_reviewer`. Transitions from `Unsafe` or `Duplicate`, and every appeal disposition, require a reviewer other than the actor responsible for the challenged decision. When staffing cannot provide that separation, the appeal remains pending and the existing status remains unchanged.

## Reports and privacy

A report contains only: generated report ID; market chain/address; category; concise description; public or access-controlled evidence references; consent to follow-up; and timestamps/status. Anonymous reporting is supported. Reporter name, address, email, IP address, wallet ownership, government ID, demographic data, and free-form attachments are not requested by this schema.

The intake must reject and not persist:

- passwords, seed phrases, private keys, API/auth tokens, wallet backup material, or credentials;
- unnecessary personal, medical, financial-account, precise-location, or identity data;
- illegal media or raw harmful/private material when a minimal reference is sufficient.

The fixture validator rejects common secret patterns and non-schema fields; production intake must add transport, access-control, malware, abuse, and deletion controls. If prohibited data arrives outside the accepted schema, restrict access, preserve only what law/counsel requires, record a sanitized incident reference, and delete the payload under the approved process.

**Draft retention default, not approved policy:** unaccepted payloads are not retained; accepted report content and decision evidence are retained for 90 days after final disposition, while the minimal public decision log may persist for accountability. Appeals and legal holds pause deletion only under documented authority. Public logs use opaque actor IDs and sanitized evidence references. Counsel/privacy approval must replace or explicitly accept this draft before intake or promoted discovery is enabled.

## Appeals

An appeal identifies the challenged transition and supplies a concise basis plus non-sensitive evidence references. It does not pause or alter the market. The original decision remains effective until a new transition is recorded.

1. Validate and acknowledge the appeal without collecting unnecessary identity data.
2. Assign an authorized `appeal_reviewer` who is not the challenged actor. If independence is unavailable, leave it pending and disclose the staffing gate.
3. Recheck the original policy version and current policy. Record whether the disposition applies the old rule, the new rule, or both.
4. `upheld` records no status change; `modified` or `reversed` must reference a new transition event; `dismissed` states the procedural reason.
5. Publish a sanitized rationale, evidence references, actor ID, time, and policy version. Never imply that the decision changes resolution or establishes truth.

## Emergency interface actions

An authorized incident responder may immediately stop promotion, add a warning, remove cached media, disable reference-interface routing/signing, revoke compromised web/API credentials, or force direct-state reconciliation. The responder records an emergency transition and evidence promptly. The action must be reviewed by a different authorized reviewer within the counsel-approved deadline; until that deadline is approved, emergency actions remain fail-closed and unpromoted.

Responders cannot pause/migrate a contract, block exact-address chain access, change or submit an oracle answer, relay a governance verdict, seize/sweep funds, rewrite positions, or promise recovery. Verified contract addresses, direct query instructions, and safe redemption access remain visible when feasible.

## Operator authority and launch gates

Before promoted public discovery or report intake is enabled, the operator must publish a versioned authorization record containing:

- named reviewers, appeal reviewers, incident responders, policy administrator, backups, scope, effective/expiry dates, and contact route;
- a dated counsel approval identifying operator, jurisdictions, prohibited/enhanced-review scope, retention/deletion policy, privacy notice, lawful-process route, and emergency review deadline;
- tested access controls, audit-log storage, deletion workflow, report abuse controls, and an exercise proving unlisting cannot influence direct queries, exact-address transaction construction, or settlement.

These names and approvals do not exist in this repository and are not inferred. Until all are recorded, `promoted_discovery_enabled` and `report_intake_enabled` are false. This document advances issue #22 but does not satisfy its human/counsel acceptance gate or authorize launch.

## Machine-readable fixtures

- [`interface-policy.schema.json`](interface-policy/interface-policy.schema.json) defines a policy fixture with transitions, reports, and appeals.
- [`valid-policy.json`](interface-policy/fixtures/valid-policy.json) demonstrates default quarantine, independent listing/review, a privacy-safe report, and an independent appeal.
- [`invalid-secret-report.json`](interface-policy/fixtures/invalid-secret-report.json) must fail because report text contains credential material.
- `python3 -m unittest tests/policy/test_interface_policy.py` validates schema shape, transition invariants, reviewer separation, fail-closed gates, and secret rejection.
