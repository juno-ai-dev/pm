#!/usr/bin/env bash
# Regenerate schema and compare it with the checked-in tree, restoring the tree
# even when drift is found. This makes the same check safe locally and in CI.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
reality_schema="$root/contracts/cw-reality/schema/cw-reality.json"
types_schema="$root/contracts/pm-types/schema/pm-types.json"
market_schema="$root/contracts/binary-market/schema/binary-market.json"
factory_schema="$root/contracts/market-factory/schema/market-factory.json"
snapshot=$(mktemp -d)
cleanup() {
  cp "$snapshot/cw-reality.json" "$reality_schema"
  cp "$snapshot/pm-types.json" "$types_schema"
  cp "$snapshot/binary-market.json" "$market_schema"
  cp "$snapshot/market-factory.json" "$factory_schema"
  rm -rf "$snapshot"
}
trap cleanup EXIT
cp "$reality_schema" "$snapshot/cw-reality.json"
cp "$types_schema" "$snapshot/pm-types.json"
cp "$market_schema" "$snapshot/binary-market.json"
cp "$factory_schema" "$snapshot/market-factory.json"
(
  cd "$root/contracts/cw-reality"
  cargo run --locked --example schema >/dev/null
)
(
  cd "$root/contracts"
  cargo run --locked -p pm-types --example pm_types_schema >/dev/null
)
(
  cd "$root/contracts/binary-market"
  cargo run --locked --example binary_market_schema >/dev/null
)
(
  cd "$root/contracts/market-factory"
  cargo run --locked --example market_factory_schema >/dev/null
)
# cosmwasm-schema also emits ignored per-message files; the maintained artifact
# in this repository is the combined schema only.
cmp "$snapshot/cw-reality.json" "$reality_schema"
cmp "$snapshot/pm-types.json" "$types_schema"
cmp "$snapshot/binary-market.json" "$market_schema"
cmp "$snapshot/market-factory.json" "$factory_schema"
