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
  cd "$root"
  cargo run --locked -p cw-reality --example schema >/dev/null
)
diff -ru "$snapshot/schema" "$schema"
