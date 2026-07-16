#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 RPC_ENDPOINT CODE_ID CONTRACT_ADDRESS OUTPUT.json" >&2
  exit 2
}
[[ $# -eq 4 ]] || usage
rpc=$1
code_id=$2
contract=$3
output=$4
[[ $code_id =~ ^[1-9][0-9]*$ ]] || usage
[[ $contract == juno1* ]] || usage

for command in junod jq; do
  command -v "$command" >/dev/null || { echo "$command is required" >&2; exit 1; }
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
junod status --node "$rpc" > "$tmp/status.json"
chain_id=$(jq -er '.node_info.network // .NodeInfo.network' "$tmp/status.json")
height=$(jq -er '(.sync_info.latest_block_height // .SyncInfo.latest_block_height) | tonumber' "$tmp/status.json")
junod query wasm code-info "$code_id" --height "$height" --node "$rpc" --output json > "$tmp/code.json"
junod query wasm contract "$contract" --height "$height" --node "$rpc" --output json > "$tmp/contract.json"
junod query wasm contract-state smart "$contract" '{"config":{}}' \
  --height "$height" --node "$rpc" --output json > "$tmp/config.json"
jq -n \
  --arg endpoint "$rpc" --arg chain_id "$chain_id" --arg address "$contract" \
  --argjson height "$height" --slurpfile code "$tmp/code.json" \
  --slurpfile contract "$tmp/contract.json" --slurpfile config "$tmp/config.json" '
  {
    schema_version: 1,
    observed: {height: $height, endpoint: $endpoint},
    chain_id: $chain_id,
    code_info: {
      code_id: ($code[0].code_id | tonumber),
      checksum: ($code[0].checksum | ascii_downcase)
    },
    contract_info: {
      address: $address,
      code_id: ($contract[0].contract_info.code_id | tonumber),
      admin: ($contract[0].contract_info.admin // null)
    },
    config: {
      admin: ($config[0].data.admin // null),
      min_initial_bond_floor: $config[0].data.min_initial_bond_floor,
      min_answer_timeout_secs: $config[0].data.min_answer_timeout_secs
    }
  }' > "$output"
jq -e . "$output" >/dev/null
printf 'wrote height-pinned evidence to %s\n' "$output"
