#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
manifest="$root/contracts/Cargo.toml"
wasm_artifacts=()

# Each package needs an isolated target directory. Building the workspace's
# deployables together feature-unifies `library` through market-factory and
# strips child contract entry points.
for spec in \
  'binary-market binary_market instantiate execute reply query' \
  'cw-reality cw_reality instantiate execute query migrate' \
  'market-factory market_factory instantiate execute reply query'
do
  set -- $spec
  package=$1
  artifact=$2
  shift 2
  target_dir="$root/target/wasm-sanity/$artifact"
  CARGO_TARGET_DIR="$target_dir" cargo build --manifest-path "$manifest" \
    --release --target wasm32-unknown-unknown --lib --locked -p "$package"
  wasm_path="$target_dir/wasm32-unknown-unknown/release/$artifact.wasm"
  python3 "$root/scripts/release/verify-wasm-exports.py" exports \
    "$wasm_path" "$@"
  wasm_artifacts+=("$wasm_path")
done

python3 "$root/scripts/release/verify-wasm-exports.py" distinct "${wasm_artifacts[@]}"
