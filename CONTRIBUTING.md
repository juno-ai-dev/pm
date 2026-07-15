# Contributing

This repository currently accepts maintenance and research changes. Do not
change deployed prediction-market behavior, protocol parameters, generated
schema, or release artifacts without a separately approved issue.

## Validation

Install the pinned Rust toolchain with rustup, then run:

```sh
make validate
```

That one command runs locked formatting, strict clippy, all 57 contract tests,
schema drift detection, and the current UI policy check. Pull requests must
explain behavior impact and whether optimized Wasm checksums can change. Do not
commit secrets, keyrings, generated build output, or invoke quarantined scripts.

The historical UI is unsupported; see `depricated_ui/UNSUPPORTED.md`.
Dependencies remain lockfile-pinned. Lifecycle/build-script allowlists require
explicit review. Security reports follow `SECURITY.md`.
