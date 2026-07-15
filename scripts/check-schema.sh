#!/usr/bin/env bash
# Regenerate schema and compare it with the checked-in tree, restoring the tree
# even when drift is found. This makes the same check safe locally and in CI.
set -euo pipefail

root=$(git rev-parse --show-toplevel)
schema="$root/contracts/cw-reality/schema"
snapshot=$(mktemp -d)
cleanup() {
  rm -rf "$schema"
  cp -a "$snapshot/schema" "$schema"
  rm -rf "$snapshot"
}
trap cleanup EXIT
cp -a "$schema" "$snapshot/schema"
(
  cd "$root/contracts/cw-reality"
  cargo run --locked --example schema >/dev/null
)
# cosmwasm-schema also emits ignored per-message files; the maintained artifact
# in this repository is the combined schema only.
rm -rf "$schema/raw"
diff -ru "$snapshot/schema" "$schema"
