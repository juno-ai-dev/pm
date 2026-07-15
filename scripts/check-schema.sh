#!/usr/bin/env bash
# Regenerate schema and compare it with the checked-in tree, restoring the tree
# even when drift is found. This makes the same check safe locally and in CI.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
schema="$root/contracts/cw-reality/schema"
combined="$schema/cw-reality.json"
snapshot=$(mktemp -d)
cleanup() {
  cp "$snapshot/cw-reality.json" "$combined"
  rm -rf "$snapshot"
}
trap cleanup EXIT
cp "$combined" "$snapshot/cw-reality.json"
(
  cd "$root/contracts/cw-reality"
  cargo run --locked --example schema >/dev/null
)
# cosmwasm-schema also emits ignored per-message files; the maintained artifact
# in this repository is the combined schema only.
cmp "$snapshot/cw-reality.json" "$combined"
