#!/usr/bin/env bash
set -euo pipefail

# This command only prepares unsigned transaction JSON. It never broadcasts.
usage() {
  echo "usage: $0 FROM_ADDRESS CODE_ID OUTPUT.json" >&2
  exit 2
}
[[ $# -eq 3 ]] || usage
from=$1
code_id=$2
output=$3
[[ $from == juno1* && $code_id =~ ^[1-9][0-9]*$ ]] || usage

msg='{"admin":null,"min_initial_bond_floor":"10000000","min_answer_timeout_secs":86400}'
junod tx wasm instantiate "$code_id" "$msg" \
  --from "$from" --label 'cw-reality-frozen-v1' --no-admin \
  --chain-id juno-1 --gas 2000000 --generate-only --output json > "$output"

# Fail closed if a future junod version unexpectedly inserts a chain admin.
if jq -e '.. | objects | select(has("admin")) | .admin | select(. != null and . != "")' "$output" >/dev/null; then
  echo 'refusing generated transaction containing a non-empty admin' >&2
  rm -f "$output"
  exit 1
fi
jq -e . "$output" >/dev/null
printf 'unsigned, unbroadcast frozen instantiate transaction: %s\n' "$output"
