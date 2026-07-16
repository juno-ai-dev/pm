# Frozen cw-reality artifact and deployment runbook

**Issue:** [#3](https://github.com/juno-ai-dev/pm/issues/3)
**Stage:** tooling ready; reproducible artifact, independent audit, deployment, and smoke evidence pending

This directory defines the fail-closed path from immutable source to a frozen
`cw-reality` instance. It does not claim that an audit or deployment happened.
No command below broadcasts a transaction unless the authorized operator adds a
separate, explicit broadcast step outside these scripts.

## Immutable build input

The selected source is repository commit
`454f9777b0bafa71c43b427f7451e626d860269e`, root tree
`e7cad35d114197fbae3cb0ff9e44ac05d0309bfa`, contract tree
`cc53d2ea0aa9fcd04fe11ba910b02db11276a0b1`. The production x86_64 optimizer is
`cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85`.
The digest was resolved and its OCI manifest inspected; the local Docker daemon
is unavailable, so this worktree did not execute it.

Run one clean build with:

```sh
./scripts/oracle/build-reproducible.sh /tmp/oracle-build
```

The `oracle-reproducibility` workflow runs that command on two fresh
`ubuntu-24.04` jobs, uses SHA-pinned checkout/upload/download actions, compares
wasm bytes with `cmp`, and records SHA-256. Do not accept a release artifact
unless the compare job passes. Copy its two distinct builder identities,
checksum, byte size, and resulting file into a manifest conforming to
[`manifest.schema.json`](manifest.schema.json).

## Audit handoff (exact scope)

Give the independent reviewer all of the following as one immutable bundle:

1. source repository, full commit/root tree/contract tree above;
2. `Cargo.lock`, root release profile, and the exact optimizer image digest;
3. both independent build logs and the byte-identical wasm plus SHA-256;
4. `contracts/cw-reality/schema/cw-reality.json` and all contract tests;
5. [`AUDIT-HANDOFF.md`](AUDIT-HANDOFF.md) as the finding/report contract.

The reviewer must return a public or access-controlled immutable report binding
both the source commit and artifact SHA-256. Every severity and disposition is
recorded; unresolved blocking findings prevent upload. Only then may
`audit.status` become `accepted` in the deployment manifest.

## Frozen preparation (no broadcast)

Prerequisites are an explicitly authorized Juno environment/operator, accepted
audit, byte-identical build record, and verified artifact. Preparing an
unsigned instantiate transaction is safe to review:

```sh
./scripts/oracle/prepare-frozen-instantiate.sh \
  juno1OPERATOR CODE_ID /tmp/instantiate-frozen.json
```

The script fixes stored `admin` to `null`, bond floor to `10000000` ujuno, and
answer-timeout floor to `86400` seconds. It passes the CLI `--no-admin` guard,
so the CosmWasm chain admin is empty. It uses `--generate-only` and cannot broadcast.
Before an authorized signer broadcasts, independently inspect the JSON and
recheck chain ID, code checksum, operator authority, fees, and both admin
surfaces. Record store/instantiate transaction hashes and heights in the audit
evidence system; do not put keys or mnemonic material in the repository.

## Height-pinned collection and verification

After an authorized deployment, collect public read-only evidence:

```sh
./scripts/oracle/collect-evidence.sh \
  https://AUTHORIZED_JUNO_RPC CODE_ID juno1CONTRACT evidence.json
python3 scripts/oracle/verify-deployment.py \
  --manifest manifest.json --evidence evidence.json \
  --artifact cw_reality.wasm --repo-root .
```

The verifier checks source repository/commit/tree provenance, immutable
optimizer digest, two distinct byte-identical build records, local artifact
size/checksum, chain ID, code ID, on-chain checksum, contract address, empty
chain admin, empty stored admin, and exact config. Any missing/type-mismatched
field fails. [`evidence.schema.json`](evidence.schema.json) describes collector
output. Negative fixtures cover wrong checksum, chain/stored admin, config, and
build identity.

## Ask / Answer / Finalize / Claim smoke gate

The smoke test is live behavior and cannot be replaced by generated messages.
Under separate broadcast/fund authorization:

1. use a dedicated test address and record starting height/balances;
2. Ask the fixed smoke question with 10,000,000 ujuno bounty/bond floor, 86,400
   seconds, no arbitrator, and nonce 3; record tx, event-derived question ID,
   and full question query;
3. Answer YES (32-byte unsigned big-endian one) with the required 10,000,000
   ujuno bond; record tx and exact history entry;
4. after `finalize_ts`, query `final_answer` and prove state is `finalized`
   (Finalize is time-derived; this contract has no Finalize execute message);
5. construct the newest-first history JSON and Claim; verify state `claimed` and
   pull-payment balance;
6. Withdraw `ujuno`; verify credited balance is drained and bank transfer lands.

`prepare-smoke-test.sh` generates unsigned Ask/Answer/Claim/Withdraw transaction
JSON for review and never broadcasts. It requires the observed question ID and
history JSON. Preserve tx hashes, heights, query responses, event logs, amounts,
and final balances. A generated-only run is not smoke evidence.

## Remaining acceptance gates

- two independent optimizer runs and recorded byte/checksum evidence;
- independent audit report and resolution of blocking findings;
- authorized upload/instantiate with both admin surfaces empty;
- live exact-config verification against the resulting address;
- live Ask/Answer/time-finalize/Claim/Withdraw smoke evidence.
