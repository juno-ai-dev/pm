# uni-7 prediction-market deployment runbook

> **TESTNET ONLY — NOT LAUNCH AUTHORIZATION.** Issue #57 asks for a funded
> deployment, but this checked-in procedure deliberately stops at unsigned,
> unbroadcast transaction generation. A separately authorized operator must
> review, sign, broadcast, and fund every step outside this repository workflow.

This is a fail-closed and restartable procedure for the three reviewed prebuilt
Wasm artifacts. It records only public chain data. Never pass a key name to these
scripts, capture a keyring listing, or put a mnemonic, private key, password, API
token, shell trace (`set -x`), or signed transaction in the work directory.
`FROM` below is a public `juno1...` address. Use a fresh private directory outside
the repository for `WORK` and keep only the final sanitized manifest if a later
evidence PR is authorized.

## Hard prerequisites and blockers

* Chain ID is exactly `uni-7`; native collateral/bond denom is exactly `ujunox`.
* Use two independently operated HTTPS RPC providers. Both must report
  `catching_up=false`; the preflight defaults to at most 20 blocks of drift.
* Consume a twice-reproduced, reviewed `build-release.sh` output. Do not rebuild
  during deployment and do not substitute similarly named files.
* The canonical release gate remains the pinned
  `cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85`.
  A host-native Cargo/`wasm-opt -Oz` build is **non-canonical and not deployable
  until explicitly approved**. It is useful only for local diagnosis and does
  not weaken or replace the pinned optimizer gate.
* **Issue #61 / PR #62 is a deployment blocker.** `deployment.py preflight`
  intentionally requires its reviewed `scripts/release/verify-wasm-exports.py`.
  That gate must prove binary-market exports `instantiate,execute,reply,query`,
  cw-reality exports `instantiate,execute,query,migrate`, and market-factory exports
  `instantiate,execute,reply,query`, and reject checksum collisions. Do not copy
  a local workaround around the blocker.
* The verdict authority for this demo must be an explicitly disclosed,
  test-only `juno1...` address unless a uni-7 DAO core is independently verified.
  It is not the mainnet authority and must not be described as governance-ready.

Required local commands are `python3`, `junod`, and a POSIX shell. Confirm the
`junod` version supports `--generate-only` and `--no-admin` before beginning.

## 1. Immutable inputs and dual-RPC preflight

```sh
umask 077
export WORK="$HOME/uni7-pm-57"                 # outside this checkout
export RELEASE=/absolute/path/to/reviewed-release
export RPC_A=https://first-independent-rpc.example
export RPC_B=https://second-independent-rpc.example
mkdir -p "$WORK/unsigned" "$WORK/receipts" "$WORK/queries"

python3 scripts/uni7/deployment.py preflight \
  --release-manifest "$RELEASE/release-manifest.json" \
  --artifacts "$RELEASE/artifacts" \
  --rpc "$RPC_A" --rpc "$RPC_B" \
  --output "$WORK/deployment-manifest.json"
python3 scripts/uni7/deployment.py validate-manifest \
  --manifest "$WORK/deployment-manifest.json"
```

This checks the pinned optimizer identity, exact filename set, SHA-256 and byte
size, distinct artifact checksums, required exports, chain ID, syncing state,
height/time, and provider independence. Review the generated manifest and the
release's source commit. Stop if any value differs from the approved candidate.

## 2. Generate and inspect three store transactions

Generate one checkpoint at a time. Existing outputs are never overwritten.
Nothing in `prepare-unsigned.sh` can sign or broadcast.

```sh
export FROM=juno1_public_address_only
for wasm in binary_market.wasm cw_reality.wasm market_factory.wasm; do
  scripts/uni7/prepare-unsigned.sh store "$FROM" \
    "$RELEASE/artifacts/$wasm" "$WORK/unsigned/store-$wasm.json"
done
```

Inspect decoded messages, chain ID, sender, fees/gas and artifact paths. An
authorized operator must first dry-run/simulate the exact candidate with a
uni-7 node, then separately sign and broadcast outside this runbook. Do not
broadcast all three blindly. After each authorized broadcast succeeds, query it
from both providers and compare hash, height, code, and events:

```sh
junod query tx TX_HASH --node "$RPC_A" -o json > "$WORK/receipts/NAME-a.json"
junod query tx TX_HASH --node "$RPC_B" -o json > "$WORK/receipts/NAME-b.json"
python3 scripts/uni7/deployment.py record-receipt \
  --target binary_market.wasm --receipt "$WORK/receipts/NAME-a.json" \
  --manifest "$WORK/deployment-manifest.json"
```

Repeat `record-receipt` with the second response: it is idempotent and rejects a
changed code ID/hash/height. Repeat for all artifact targets. Then query each
code ID from both RPCs at a common height:

```sh
junod query wasm code-info CODE_ID --height COMMON_HEIGHT --node "$RPC_A" -o json
junod query wasm code-info CODE_ID --height COMMON_HEIGHT --node "$RPC_B" -o json
```

Normalize a base64 checksum to lowercase hex when required (`base64 -d | xxd
-p -c 256`), and compare it byte-for-byte with the manifest SHA-256. Stop on any
mismatch. Never trust only the store event.

## 3. Frozen oracle instantiate

Review this exact message (accepted canary floors: 10 JUNO and 24 hours):

```sh
printf '%s\n' '{"admin":null,"min_initial_bond_floor":"10000000","min_answer_timeout_secs":86400}' \
  > "$WORK/oracle.json"
scripts/uni7/prepare-unsigned.sh instantiate "$FROM" ORACLE_CODE_ID \
  pm-cw-reality-uni7-testnet "$WORK/oracle.json" "$WORK/unsigned/oracle.json"
```

The CLI `--no-admin` and stored `admin:null` are separate immutability checks.
Dry-run/simulate before any separately authorized signing. After success, fetch
the receipt from both RPCs and `record-receipt --target oracle`. At the receipt
height and again at a fresh common height, read back:

```sh
junod query wasm contract ORACLE_ADDRESS --height HEIGHT --node "$RPC_A" -o json
junod query wasm contract-state smart ORACLE_ADDRESS '{"config":{}}' \
  --height HEIGHT --node "$RPC_A" -o json
```

Repeat on RPC B. Require code ID/checksum match, chain admin empty, stored admin
null, floor `10000000`, and timeout `86400`.

## 4. Immutable factory instantiate

Create and review `factory.json` from the compiled schema. It must contain only:

```json
{
  "protocol_version": "v1",
  "market_code_id": 0,
  "market_checksum": "REPLACE_64_HEX",
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
  "oracle": "REPLACE_ORACLE_ADDRESS",
  "oracle_code_id": 0,
  "oracle_checksum": "REPLACE_64_HEX",
  "verdict_authority": "REPLACE_DISCLOSED_TEST_ONLY_ADDRESS",
  "collateral_denom": "ujunox",
  "oracle_min_initial_bond_floor": "10000000",
  "oracle_min_answer_timeout_secs": 86400
}
```

Replace both zero code IDs and every `REPLACE_*`; unresolved placeholders are a
hard stop. Cross-check the accepted tier and compiled schema, disclose the
verdict authority in the manifest, then generate:

```sh
scripts/uni7/prepare-unsigned.sh instantiate "$FROM" FACTORY_CODE_ID \
  pm-factory-uni7-testnet "$WORK/factory.json" "$WORK/unsigned/factory.json"
```

Dry-run/simulate, inspect, and only then hand off for separately authorized
signing. Record dual-provider receipts as `factory`. Query `{"config":{}}` and
chain contract info from both providers at one height. Require no chain admin
and exact equality with `factory.json`, including `ujunox`, checksums, code IDs,
tier and disclosed test-only verdict authority.

## 5. Objective demo market

Prepare `create-market.json` against the compiled factory schema. It must cite a
stable primary source and a secondary source, define observation/revision/
disagreement/invalid rules, use `100000000` initial liquidity and `1000000`
oracle bounty, close at least 24 hours in the future, and leave sufficient public
demo time. The required attached funds are `101000000ujunox`; confirm this from
reviewed contract behavior before authorization.

```sh
scripts/uni7/prepare-unsigned.sh execute "$FROM" FACTORY_ADDRESS 101000000 \
  "$WORK/create-market.json" "$WORK/unsigned/create-market.json"
```

Dry-run/simulate first. Following separately authorized execution, record both
receipts as `seeded_market`. Query factory `{"market":{"nonce":NONCE}}`, market
`{"config":{}}`, oracle question identity/state, and all three contract infos
from both providers at a common height. Require exact factory registry address,
creator, tier, question ID/hash, timestamps, funding, immutable verdict
authority, oracle binding and artifact code IDs/checksums.

## 6. Sanitized completion/readback

Add only the public query results needed by the manifest: both endpoint URLs,
height/time, code IDs/checksums, tx hashes, addresses, null admins, exact configs,
question/market identity, source URLs, and explicit test-only authority label.
Do not copy CLI configuration, environment, balances unrelated to this demo,
keyring output, unsigned/signed transaction bodies, or operator machine paths.

```sh
python3 scripts/uni7/deployment.py validate-manifest \
  --manifest "$WORK/deployment-manifest.json" --complete
```

`--complete` requires dual independent observations, all three successful store
receipts, oracle/factory/market addresses and receipts, and null top-level chain
admins. Independent reviewers must re-query RPC B rather than trusting captured
RPC A files. A valid manifest is evidence of a testnet deployment only; it is not
an audit, production readiness, launch approval, or permission to move funds.

## Recovery and stop conditions

Every successful public receipt is an idempotent checkpoint. On restart, run
`validate-manifest`, re-query all recorded IDs/addresses, and continue at the
first absent field. Never regenerate or rebroadcast a step merely because a CLI
timed out: query its tx hash and sender sequence first. Stop on a failed tx,
ambiguous event, chain/provider disagreement, sync status change, changed source
commit, checksum/export mismatch, unexpected admin, config mismatch, incorrect
denom, undisclosed authority, insufficient demo window, or any suspected secret
capture. Quarantine secret-bearing files outside Git and rotate exposed secrets.
