# R3 — Canonical question and resolution specification

**Status:** accepted protocol document (2026-07-16)
**Encoding version:** juno-pm-question/1

## Binding rule

The market binds one exact UTF-8 resolution document. The creator supplies typed semantic fields; the market injects its own address and the factory-pinned economic/address fields, then constructs the document with the fixed schema below and JCS. Callers cannot supply arbitrary pre-serialized bytes. Its resulting bytes are:

1. stored in the market;
2. passed unchanged as cw-reality Question.text;
3. SHA-256 hashed by both market and oracle;
4. indirectly included in the source-defined question ID;
5. exposed by query and events through its hash.

Human-readable metadata at a URI is a mirror. If it differs from the on-chain bytes, the on-chain document controls. Frontends must display the hash and offer the exact bytes; a title or API record is not authoritative.

The interoperable representation is JSON Canonicalization Scheme (JCS) as specified by [RFC 8785](https://www.rfc-editor.org/rfc/rfc8785). The implementation phase must use a fixed typed document, a deterministic JCS encoder, and cross-language golden bytes; it must not rely on map iteration order. Because the market constructs rather than parses the document, there is no “structured fields versus supplied JSON” mismatch. A failure to encode within size/gas bounds aborts creation.

## Required document

~~~json
{
  "answer_encoding": {
    "invalid_hex": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
    "no_hex": "0000000000000000000000000000000000000000000000000000000000000000",
    "unresolved_hex": "fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe",
    "unknown_policy": "neutral",
    "yes_hex": "0000000000000000000000000000000000000000000000000000000000000001"
  },
  "answer_timeout_secs": 86400,
  "arbitration_timeout_secs": 1814400,
  "challenge_bond_rule": "max(tier_floor,current_oracle_bond)",
  "close_ts": 0,
  "collateral_denom": "ujuno",
  "definitions": [],
  "verdict_authority": "juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac",
  "invalid_conditions": [],
  "language": "en",
  "market_controller": "<market address>",
  "observation": {
    "cutoff_ts": 0,
    "end_ts": 0,
    "inclusivity": "inclusive",
    "revision_policy": "",
    "start_ts": 0,
    "timezone": "UTC"
  },
  "opening_ts": 0,
  "oracle": "<frozen cw-reality address>",
  "oracle_bond_denom": "ujuno",
  "oracle_initial_bond": "0",
  "oracle_question_type": "bool",
  "payouts": {
    "invalid": ["1/2", "1/2"],
    "no": ["0", "1"],
    "unrecognized": ["1/2", "1/2"],
    "unresolved": ["1/2", "1/2"],
    "yes": ["1", "0"]
  },
  "primary_sources": [],
  "proposition": "",
  "question_version": "juno-pm-question/1",
  "secondary_sources": [],
  "source_disagreement_policy": "",
  "title": ""
}
~~~

Zeroes and placeholders are invalid in an activated market. Amounts are decimal strings when they can exceed safe JSON-number precision. Timestamps are absolute Unix seconds and the rendered interface must show ISO-8601 UTC.

## Semantic requirements

The proposition must be one objectively decidable yes/no statement. The document must define:

- the exact subject and named entities;
- observation start, end, cutoff, timezone, and boundary inclusivity;
- what counts as occurrence or non-occurrence;
- measurement unit, precision, rounding, and authoritative revision vintage;
- cancellations, postponements, ties, recounts, corrections, source outage, and conflicting-source precedence;
- conditions producing INVALID rather than YES or NO;
- sources precise enough that a resolver can retrieve the same record;
- opening_ts at or after the observation can be answered;
- close_ts no later than opening_ts and normally before the outcome is cheaply knowable.

Relative dates, “official” without naming the authority, subjective adjectives, unverifiable private facts, non-exclusive YES/NO definitions, and propositions a trader can cheaply make true are reference-interface rejection reasons. Contracts remain permissionless and cannot prove semantic quality.

## Source precedence

Primary sources are an ordered list, not a bag. Each entry includes publisher, exact dataset/document, URL or identifier, retrieval method, publication/revision timing, and fallback condition. The source_disagreement_policy must say whether the first available source controls, a later official correction controls by a fixed deadline, or disagreement produces INVALID. It may not be changed after market activation.

Secondary sources may demonstrate availability but never override an available primary source unless the immutable policy says exactly when.

## Time policy

~~~text
creation_ts < close_ts <= observation end/cutoff <= opening_ts
~~~

Some propositions need trading to close before the event begins; others can trade until an objective cutoff. opening_ts is the earliest honest answer time, not a predicted settlement time. A source-publication delay belongs between cutoff and opening.

At block.time >= close_ts, every trade and price-changing action rejects. At block.time >= opening_ts, cw-reality may accept an answer. If times are equal, no same-block trade can execute after answers become admissible.

## Invalid and unrecognized

INVALID is appropriate when the immutable rules cannot produce a unique YES/NO—for example, all named sources permanently fail under the stated fallback, definitions conflict, the event is cancelled under an invalid policy, or the proposition contains a discovered logical impossibility. A normal NO must not be relabeled invalid merely because it is inconvenient.

UNRESOLVED means the verdict authority explicitly declines. It is terminal and neutral in v1; cw-reality has no reopen path.

Any other finalized bytes, including UTF-8 strings, short 0/1 bytes, a 32-byte integer other than 0 or 1, or an arbitrator-authored value, are terminal neutral. The interface may label them “unrecognized oracle result,” not pretend the oracle said INVALID.

## Example

The following is illustrative, not an approved real market:

~~~json
{
  "answer_encoding":{"invalid_hex":"ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff","no_hex":"0000000000000000000000000000000000000000000000000000000000000000","unresolved_hex":"fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe","unknown_policy":"neutral","yes_hex":"0000000000000000000000000000000000000000000000000000000000000001"},
  "answer_timeout_secs":86400,
  "arbitration_timeout_secs":1814400,
  "challenge_bond_rule":"max(tier_floor,current_oracle_bond)",
  "close_ts":1798671600,
  "collateral_denom":"ujuno",
  "definitions":["Published means visible in the named JSON feed with a non-null final field."],
  "verdict_authority":"juno18k65at7fkf8elhece0fnhsvuxggqg6cved6trp5fyk3lftfn93xsmpeaac",
  "invalid_conditions":["The named feed is permanently retired and its stated archival fallback is unavailable by the revision deadline."],
  "language":"en",
  "market_controller":"<market address>",
  "observation":{"cutoff_ts":1798758000,"end_ts":1798758000,"inclusivity":"inclusive","revision_policy":"Corrections published by 1798844400 control; later revisions do not.","start_ts":1798671600,"timezone":"UTC"},
  "opening_ts":1798844400,
  "oracle":"<frozen cw-reality address>",
  "oracle_bond_denom":"ujuno",
  "oracle_initial_bond":"1000000",
  "oracle_question_type":"bool",
  "payouts":{"invalid":["1/2","1/2"],"no":["0","1"],"unrecognized":["1/2","1/2"],"unresolved":["1/2","1/2"],"yes":["1","0"]},
  "primary_sources":[{"fallback_condition":"HTTP 404 or cryptographic verification failure for 72 hours","identifier":"dataset/example/final","publisher":"Example Authority","retrieval":"HTTPS JSON","url":"https://example.invalid/final"}],
  "proposition":"Will the Example Authority's final field for dataset/example/final equal true for the stated observation period?",
  "question_version":"juno-pm-question/1",
  "secondary_sources":[],
  "source_disagreement_policy":"The first available primary source controls; if its stated fallback condition is met and no fallback is listed, resolve INVALID.",
  "title":"Example Authority final field for the stated period?"
}
~~~

## Objective creation checks

The factory/market can enforce:

- byte size at or below the accepted maximum (16 KiB);
- valid typed strings and exact market-constructed JCS bytes;
- version exactly juno-pm-question/1;
- timestamps and ordering;
- ujuno collateral/bond denoms;
- pinned oracle, market controller, and immutable verdict authority (the Juno Agents DAO core in v1; x/gov compatibility is deferred);
- tier bond/timeouts/caps;
- exact answer and payout tables;
- nonempty title, proposition, source list, invalid conditions, and policies.

It cannot prove truth, objectivity, legality, safety, source independence, or adequate definitions. Independent interfaces own their listing decisions.

## Reviewer checklist

- Can two reasonable resolvers reach different answers from the text?
- Are all times absolute UTC and all boundaries inclusive/exclusive?
- Can the outcome be known before close?
- Can a trader cheaply cause the outcome?
- Does every cancellation/postponement/source-outage path end in YES, NO, or INVALID?
- Are source revisions bounded?
- Do displayed bytes/hash match the oracle Question query?
- Do all addresses and economic parameters match the market query?
- Are harmful, illegal, private, or defamatory incentives excluded from the reference interface?
