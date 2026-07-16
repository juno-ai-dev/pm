#!/usr/bin/env bash
set -euo pipefail

# Prepare (never broadcast) the Ask/Answer/Claim/Withdraw smoke transactions.
usage() {
  echo "usage: $0 FROM CONTRACT QUESTION_ID_BASE64 HISTORY.json OUTPUT_DIR" >&2
  exit 2
}
[[ $# -eq 5 ]] || usage
from=$1
contract=$2
qid=$3
history=$4
out=$5
[[ $from == juno1* && $contract == juno1* ]] || usage
jq -e 'type == "array"' "$history" >/dev/null
mkdir -p "$out"

ask='{"ask_question":{"text":"issue-3 frozen deployment smoke test","answer_type":"bool","bond_denom":"ujuno","initial_bond":"10000000","answer_timeout_secs":86400,"arbitrator":null,"arbitration_timeout_secs":null,"answer_schema":null,"opening_ts":null,"nonce":3}}'
answer=$(jq -cn --arg q "$qid" '{submit_answer:{question_id:$q,answer:"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAE=",current_bond_seen:null}}')
claim=$(jq -cn --arg q "$qid" --slurpfile h "$history" '{claim:{question_id:$q,history_entries:$h[0]}}')
withdraw='{"withdraw":{"denom":"ujuno"}}'

# Ask necessarily precedes discovery of QUESTION_ID. For a real rehearsal, first
# generate/broadcast Ask under separate authorization, then rerun with its ID.
junod tx wasm execute "$contract" "$ask" --from "$from" --amount 10000000ujuno \
  --chain-id juno-1 --gas 2000000 --generate-only --output json > "$out/ask.json"
junod tx wasm execute "$contract" "$answer" --from "$from" --amount 10000000ujuno \
  --chain-id juno-1 --gas 2000000 --generate-only --output json > "$out/answer.json"
junod tx wasm execute "$contract" "$claim" --from "$from" \
  --chain-id juno-1 --gas 2000000 --generate-only --output json > "$out/claim.json"
junod tx wasm execute "$contract" "$withdraw" --from "$from" \
  --chain-id juno-1 --gas 2000000 --generate-only --output json > "$out/withdraw.json"
for tx in "$out"/*.json; do jq -e . "$tx" >/dev/null; done
printf 'unsigned, unbroadcast smoke transaction set: %s\n' "$out"
