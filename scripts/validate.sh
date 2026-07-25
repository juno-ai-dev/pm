#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
manifest="$root/contracts/Cargo.toml"

cargo fmt --manifest-path "$manifest" --all -- --check
cargo clippy --manifest-path "$manifest" --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path "$manifest" --workspace --locked
"$root/scripts/release/check-wasm-sanity.sh"
python3 -m unittest tests/quality/test_wasm_exports.py
python3 -m unittest tests/oracle/test_verify_deployment.py
python3 -m unittest tests/governance/test_prepare_rehearsal.py
python3 -m unittest tests/policy/test_interface_policy.py
python3 -m unittest tests/quality/test_quality_report.py
python3 scripts/quality/verify-report.py quality/gas-storage-report.json
./scripts/check-schema.sh
./scripts/check-ui-policy.sh
