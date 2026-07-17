# Contract release and empirical quality gates

This runbook produces review artifacts only. It never uploads code, deploys a
contract, signs a transaction, or moves funds.

## Reproducible optimizer build

The release workflow executes `cosmwasm/optimizer:0.17.0` by immutable digest in
two clean jobs without shared caches. Each job runs:

```sh
./scripts/release/build-release.sh out
```

`compare-builds.py` requires identical Wasm bytes, manifest, SBOM, and optimizer
identity. `release-manifest.json` binds the source commit, `Cargo.lock` SHA-256,
optimizer repository and digest, canonical schema-tree SHA-256, and each Wasm
SHA-256/size. `sbom.cdx.json` is a deterministic CycloneDX inventory derived
from the locked Cargo graph. Verify downloaded outputs with:

```sh
python3 scripts/release/compare-builds.py build-one build-two
python3 scripts/quality/verify-report.py quality/gas-storage-report.json \
  build-one/release-manifest.json
```

A raw Cargo Wasm build is only a sanity check and is not deployable provenance.

## Size, gas, and storage policy

Machine-readable limits are in `quality/thresholds.json`. Wasm files over 800
KiB fail. Empirical transaction/query gas must retain 20% headroom below the
review ceilings. Transaction storage delta and query response limits are 128
KiB. A breach fails unless the report records a named approver and reason; this
is review evidence, not launch authorization.

`quality/gas-storage-report.json` lists both realistic and maximum-bound cases:
create/activate (including maximum question/metadata), buy/sell reserve bounds,
challenge/verdict, resolve, fragmented position and LP redemption, and maximum
factory pagination. Measurements must be made against the current candidate
commit and a recorded Juno chain height/software binary using simulation or an
authorized no-value rehearsal. Record actual `gas_used`, state byte deltas, and
query response bytes; do not copy estimates into empirical fields.

The checked-in template deliberately says `measurement_required`, keeps every
measurement null, and requires explicit canary review. CI validates that this
open gate cannot be silently omitted or represented as measured. Replacing it
with `status: measured` requires chain, binary, commit, complete positive gas
values, byte measurements, and threshold review. A measured report should be a
separate evidence PR after the current Juno software generation is selected;
no transaction is authorized by this runbook.
