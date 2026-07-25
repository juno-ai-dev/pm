# uni-7 deployment evidence manifest template

> **Shape reference only — TESTNET ONLY — NOT LAUNCH AUTHORIZATION.** Generate the
> starting JSON with `deployment.py preflight`; do not store credentials or copy
> transaction bodies into it. Every object is closed: `--complete` rejects keys
> not listed here. Every `A` and `B` item comes from the corresponding independent
> preflight provider and normalized HTTPS endpoint.

## Top-level preflight shape

```json
{
  "schema_version": 1,
  "testnet_only": true,
  "authorization": "NOT launch authorization",
  "network": {
    "chain_id": "uni-7",
    "collateral_denom": "ujunox",
    "max_height_drift": 20
  },
  "release": {
    "source_commit": "FULL_APPROVED_40_LOWER_HEX",
    "approved_source_commit": "SAME_FULL_APPROVED_40_LOWER_HEX",
    "release_manifest_sha256": "64_LOWER_HEX",
    "optimizer_image": "cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85"
  },
  "artifacts": {
    "binary_market.wasm": "ARTIFACT_RECORD",
    "cw_reality.wasm": "ARTIFACT_RECORD",
    "market_factory.wasm": "ARTIFACT_RECORD"
  },
  "observations": ["STATUS_A", "STATUS_B"],
  "contracts": {
    "oracle": "CONTRACT_RECORD",
    "factory": "CONTRACT_RECORD",
    "seeded_market": "CONTRACT_RECORD"
  }
}
```

The quoted names above are notation, not validator-accepted values. Replace each
with the exact object below. Preflight does this for artifact, status, and empty
contract records.

### `STATUS_A` / `STATUS_B`

```json
{
  "provider": "reviewed-public-provider-label",
  "endpoint": "https://normalized-distinct-host.example",
  "chain_id": "uni-7",
  "height": 123,
  "time": "2026-07-25T00:00:00Z",
  "catching_up": false
}
```

### `ARTIFACT_RECORD`

```json
{
  "sha256": "64_LOWER_HEX",
  "size_bytes": 123,
  "required_exports": ["EXACT_SORTED_EXPORTS_FROM_PREFLIGHT"],
  "receipt_evidence": ["STORE_RECEIPT_A", "STORE_RECEIPT_B"],
  "chain_evidence": ["CODE_READBACK_A", "CODE_READBACK_B"]
}
```

A store receipt has exactly:

```json
{
  "provider": "provider-a",
  "endpoint": "https://rpc-a.example",
  "txhash": "64_UPPER_HEX",
  "height": 123,
  "code_id": 456
}
```

A code readback has exactly:

```json
{
  "provider": "provider-a",
  "endpoint": "https://rpc-a.example",
  "height": 130,
  "code_id": 456,
  "chain_sha256": "ARTIFACT_64_LOWER_HEX",
  "admin": null
}
```

Both receipt observations must agree exactly except for provenance. All code
readbacks use one common height per provider, and the providers' heights remain
within `max_height_drift`.

## Contract records

Each oracle or seeded-market contract record has exactly:

```json
{
  "receipt_evidence": ["RECEIPT_A", "RECEIPT_B"],
  "readback_evidence": ["READBACK_A", "READBACK_B"]
}
```

The factory record additionally has the public disclosure field
`"authority_kind": "disclosed test-only authority"` (or
`"verified uni-7 DAO core"` only after independent verification).

An oracle/factory receipt has `provider`, `endpoint`, `txhash`, `height`, and
`address`. A seeded-market receipt has those five fields plus `question_id`.
Receipts never contain or imply `admin`; admin comes only from each provider's
independent contract-info query.

Every contract readback starts with exactly these chain facts:

```json
{
  "provider": "provider-a",
  "endpoint": "https://rpc-a.example",
  "height": 130,
  "code_id": 456,
  "chain_sha256": "BOUND_ARTIFACT_64_LOWER_HEX",
  "admin": null
}
```

Then add only the contract-specific fields described below.

### Oracle readback

Add `config` exactly:

```json
{
  "admin": null,
  "min_initial_bond_floor": "10000000",
  "min_answer_timeout_secs": 86400
}
```

The inner `admin` is the oracle's stored config. The outer `admin` is independent
chain contract-info evidence; both are mandatory and null.

### Factory readback

Add `config` with the exact PR #66 response shape:

```json
{
  "contract_profile": "uni7",
  "protocol_version": "v1",
  "market_code_id": 456,
  "market_checksum": "BINARY_MARKET_64_LOWER_HEX",
  "tier_id": 1,
  "tier": {
    "min_initial_liquidity": "100000000",
    "max_initial_liquidity": "200000000",
    "min_oracle_bounty": "1000000",
    "max_oracle_bounty": "1000000",
    "oracle_initial_bond": "10000000",
    "answer_timeout_secs": 86400,
    "arbitration_timeout_secs": 1814400,
    "fee_bps": 200,
    "min_trade": "10000",
    "max_trade_bps": 2500,
    "max_position_per_side": "20000000",
    "collateral_cap": "200000000",
    "challenge_bond": "10000000"
  },
  "oracle": "juno1_REPLACE",
  "oracle_code_id": 457,
  "oracle_checksum": "CW_REALITY_64_LOWER_HEX",
  "verdict_authority": "juno1_REPLACE",
  "collateral_denom": "ujunox",
  "oracle_min_initial_bond_floor": "10000000",
  "oracle_min_answer_timeout_secs": 86400
}
```

The factory record's `authority_kind` carries the disclosure classification;
it is not part of the on-chain config response. Every address, code ID, checksum,
oracle floor, tier field, profile, version, and denom is cross-linked by the
validator.

### Seeded-market readback

Add all four exact objects: `config`, `identity`, `question`, and `registry`.

`config`:

```json
{
  "contract_profile": "uni7",
  "protocol_version": "v1",
  "factory": "juno1_REPLACE",
  "creator": "juno1_REPLACE",
  "initial_lp": "SAME_CREATOR_ADDRESS",
  "oracle": "juno1_REPLACE",
  "verdict_authority": "juno1_REPLACE",
  "tier": 1,
  "collateral_denom": "ujunox",
  "close_ts": 2000000000,
  "opening_ts": 2000086400,
  "initial_liquidity": "100000000",
  "oracle_bounty": "1000000",
  "oracle_initial_bond": "10000000",
  "answer_timeout_secs": 86400,
  "arbitration_timeout_secs": 1814400,
  "fee_bps": 200,
  "min_trade": "10000",
  "max_trade_bps": 2500,
  "max_position_per_side": "20000000",
  "collateral_cap": "200000000",
  "challenge_bond": "10000000"
}
```

`identity`:

```json
{
  "contract_profile": "uni7",
  "collateral_denom": "ujunox",
  "protocol_version": "v1",
  "factory": "juno1_REPLACE",
  "market": "juno1_REPLACE",
  "nonce": 7,
  "question_id": "64_LOWER_HEX"
}
```

`question` (normalized from the public question query):

```json
{
  "question_id": "64_LOWER_HEX",
  "question_hash": "64_LOWER_HEX"
}
```

`registry` (normalized from the factory registry query and public creation
funding evidence):

```json
{
  "contract_profile": "uni7",
  "collateral_denom": "ujunox",
  "nonce": 7,
  "market": "juno1_REPLACE",
  "creator": "juno1_REPLACE",
  "tier_id": 1,
  "question_id": "64_LOWER_HEX",
  "question_hash": "64_LOWER_HEX",
  "close_ts": 2000000000,
  "opening_ts": 2000086400,
  "initial_liquidity": "100000000",
  "oracle_bounty": "1000000",
  "initial_funding": "101000000",
  "created_height": 123,
  "created_time": 1999999000
}
```

Both providers must return matching normalized contract facts. The validator
cross-links creator/initial LP, factory, oracle, verdict authority, tier/tier ID,
nonce, market address, question ID/hash, timestamps, liquidity, bounty, total
initial funding, all three code IDs, and all three on-chain checksums.
