#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
manifest="$root/contracts/Cargo.toml"

cargo fmt --manifest-path "$manifest" --all -- --check
cargo clippy --manifest-path "$manifest" --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path "$manifest" --workspace --locked
cargo build --manifest-path "$manifest" --release --target wasm32-unknown-unknown \
  --lib --locked -p binary-market -p cw-reality -p market-factory
python3 -m unittest tests/oracle/test_verify_deployment.py
python3 -m unittest tests/policy/test_interface_policy.py
./scripts/check-schema.sh
./scripts/check-ui-policy.sh
