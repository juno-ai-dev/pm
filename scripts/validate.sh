#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest tests/oracle/test_verify_deployment.py
./scripts/check-schema.sh
./scripts/check-ui-policy.sh
