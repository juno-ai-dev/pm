#!/usr/bin/env bash
# Drive the first cw-reality question through Claim + Withdraw on juno-1.
#
# Run AFTER finalize_ts (1780069441 — ~2026-05-29 ~15:44 UTC).
# Idempotent: re-running after success will hit `NothingToClaim` /
# `NothingToWithdraw` (typed errors) without affecting state.
# QUARANTINED: unsupported historical live-network mutation. Never automate.

set -euo pipefail

CONTRACT=juno1g0pveeymzn3a3asu6v2dhkclqhwsndmvjugjx8a4qx554esp5yessuceur
RPC=https://juno-rpc.polkachu.com:443
QID_B64="T0wRUbc0fOTiXr88IM3zbIfLIOpDUTaL2zg0UlFE37c="
# Per onchain-log.md: round 1 history entry
PREV_HASH_B64="AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
ANSWER_B64="AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
BOND="100000"
ANSWERER="juno1mtzvz3vz6ss96ar3e8jd5pn0rh08ql5uvxvzwd"

: "${JUNOD:?set JUNOD to an explicitly reviewed junod binary}"
: "${KR_DIR:?set KR_DIR to an explicitly reviewed keyring directory}"
: "${KEY_NAME:?set KEY_NAME to an explicitly reviewed key name}"
: "${I_UNDERSTAND_THIS_BROADCASTS_TO_JUNO_1:?set exactly YES after reviewing this live-network script}"
if [[ "$I_UNDERSTAND_THIS_BROADCASTS_TO_JUNO_1" != "YES" ]]; then
  echo "refusing live-network execution without I_UNDERSTAND_THIS_BROADCASTS_TO_JUNO_1=YES" >&2
  exit 1
fi

echo "== State before =="
"$JUNOD" query wasm contract-state smart "$CONTRACT" \
  "{\"question\":{\"question_id\":\"$QID_B64\"}}" \
  --node "$RPC" -o json | python3 -c "
import json, sys
d = json.load(sys.stdin)['data']
print('state:', d['state'])
print('round_count:', d['question']['round_count'])
print('current_bond:', d['question']['current_bond'])
print('finalize_ts:', d['question']['finalize_ts'])
print('is_claimed:', d['question']['is_claimed'])
"

echo
echo "== Claim =="
CLAIM_MSG=$(cat <<EOF
{
  "claim": {
    "question_id": "$QID_B64",
    "history_entries": [
      {
        "prev_hash": "$PREV_HASH_B64",
        "answer": "$ANSWER_B64",
        "bond_amount": "$BOND",
        "answerer": "$ANSWERER",
        "is_commitment": false
      }
    ]
  }
}
EOF
)
"$JUNOD" tx wasm execute "$CONTRACT" "$CLAIM_MSG" \
  --from "$KEY_NAME" --keyring-backend test --keyring-dir "$KR_DIR" \
  --chain-id juno-1 --node "$RPC" \
  --gas auto --gas-adjustment 1.4 --gas-prices 0.075ujuno \
  --broadcast-mode sync --yes

echo
echo "== Wait for inclusion =="
sleep 8

echo
echo "== Withdraw =="
"$JUNOD" tx wasm execute "$CONTRACT" '{"withdraw":{"denom":"ujuno"}}' \
  --from "$KEY_NAME" --keyring-backend test --keyring-dir "$KR_DIR" \
  --chain-id juno-1 --node "$RPC" \
  --gas auto --gas-adjustment 1.4 --gas-prices 0.075ujuno \
  --broadcast-mode sync --yes

echo
sleep 8
echo "== State after =="
"$JUNOD" query wasm contract-state smart "$CONTRACT" \
  "{\"question\":{\"question_id\":\"$QID_B64\"}}" \
  --node "$RPC" -o json | python3 -c "
import json, sys
d = json.load(sys.stdin)['data']
print('state:', d['state'])
print('is_claimed:', d['question']['is_claimed'])
"

echo
echo "== Bank balance =="
"$JUNOD" query bank balances "$ANSWERER" --node "$RPC" -o json | python3 -c "
import json, sys
d = json.load(sys.stdin)
for b in d['balances']: print(f\"{b['denom']}: {int(b['amount'])/1e6:.6f}\")
"
