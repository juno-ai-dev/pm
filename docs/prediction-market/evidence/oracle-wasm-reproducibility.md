# cw-reality deployed-wasm reproducibility attempt

**Observed:** 2026-07-15  
**Local source pin:** `ee641534fd7b7b3677bd48d30390422ee3fbe5ed`  
**On-chain code ID:** `5121` on `juno-1`  
**Verdict:** deployed bytes independently retrieved; source-to-byte reproduction not achieved

## Deployed artifact

At Juno height 39,830,878, both `rest.cosmos.directory/juno` and `juno-api.polkachu.com` returned:

- code-info checksum `e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2`;
- decoded `/cosmwasm/wasm/v1/code/5121` length 361,624 bytes;
- SHA-256 of those decoded bytes `e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2`.

Thus the two providers agree with code-info about the deployed wasm. This verifies retrieval and checksum consistency, not its source provenance.

## Intended optimizer environment

The contract's `workspace-optimize` recipe names `cosmwasm/optimizer-arm64:0.17.0` on this ARM64 host. The public OCI image resolved to digest:

```text
cosmwasm/optimizer-arm64:0.17.0
sha256:7881e9cece93bb47a6cf4af620bfb9376431a229389f71845b7ed4d237631341
```

The container entrypoint was not executed. Its config was inspected, and extracted read-only binaries reported:

- Rust `1.86.0`, commit `05f9846f893b09a1be1fc8560e33fc3c815cfecb`;
- Cargo `1.86.0`, commit `adf9b6ad14cfa10ff680d5806741a144f7163698`;
- host `aarch64-unknown-linux-musl`;
- Binaryen `wasm-opt version 116`.

The optimizer source tag `v0.17.0` resolves to commit [b09e2fcf](https://github.com/CosmWasm/optimizer/tree/b09e2fcf849b98ea846d6839f728bd4568dcd1fd). Its Dockerfile pins Rust 1.86.0 and Binaryen 116; its builder runs Cargo with `--release --lib --target wasm32-unknown-unknown --locked`, `RUSTFLAGS=-C link-arg=-s`, then `wasm-opt -Os`.

The [optimizer's own documentation](https://github.com/CosmWasm/optimizer/tree/v0.17.0) warns that ARM and Intel images produce different artifacts and recommends the Intel image for production. Consequently, the repository recipe's host-dependent image selection is not a cross-architecture checksum policy. A future frozen deployment must pin one image by digest—preferably the documented Intel production path—and reproduce it on two independent builders.

## Attempt and result

The Docker CLI was present, but the daemon socket was `root:root` with no usable permission. The exact public ARM64 image was exported and its tool versions checked. An unprivileged PRoot attempt could not execute in this environment (exit 182 with no guest output), so an exact container run was unavailable.

A best-effort diagnostic then used:

- native ARM64 GNU Rust/Cargo 1.86.0 at the same commits;
- the exact `wasm-opt 116` binary extracted from the pinned ARM64 image;
- the locked dependency graph, source unchanged, release profile unchanged, `RUSTFLAGS=-C link-arg=-s`, and the optimizer's `-Os` pass.

It produced:

| Stage | Bytes | SHA-256 |
| --- | ---: | --- |
| Pre-`wasm-opt` | 411,323 | `509a49d22302614b58fcf862a623ec070d31cad2cc9f19a960b48dd9144e36d0` |
| Best-effort optimized | 361,648 | `fa96b82235f23dc84b8ccbf1082ddc122f1d87223014ba5241548f06900bb0f6` |
| Deployed | 361,624 | `e25473e7eb08b5fc23b66926073958458b01a7b9b5642855249bc3d9b7f7f3e2` |

The mismatch is conclusive that this best-effort build is not the deployed byte sequence. It is **not** evidence that the inspected source differs, because the exact musl container environment was not executed and the upstream project documents architecture-sensitive output.

No contract source, checked-in schema, production artifact, or deployment state was changed. Build products remained temporary and outside the repository.

## Gate consequence

The following remain required before the selected oracle checksum can be called independently verified:

1. run the exact chosen optimizer image by immutable digest in an authorized environment;
2. identify the deployment source commit and reproduce its checksum on two independent builders;
3. retain build logs, image digests, source-tree cleanliness, and artifact hashes;
4. independently audit the source and reconcile every finding;
5. for the recommended fresh oracle, instantiate the reproduced/audited code with both chain admin and stored admin absent, then verify those facts on-chain.

Until then, local source observations are compatibility requirements, not proof of every deployed byte's behavior.

## Issue #3 follow-up tooling

The first-stage tooling in [`../oracle-deployment/`](../oracle-deployment/README.md)
selects the documented Intel production optimizer by immutable digest and pins
the corrected repository source at commit
`454f9777b0bafa71c43b427f7451e626d860269e`. A SHA-pinned CI workflow performs
two clean builds and byte comparison when an optimizer-capable runner is
available. The local environment still has no Docker daemon access, so this
addition records no new wasm checksum and does not change the failed historical
reproduction verdict above. Audit, frozen-chain deployment, and live smoke
evidence remain explicitly open.
