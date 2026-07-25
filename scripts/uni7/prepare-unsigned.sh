#!/usr/bin/env bash
set -euo pipefail

# Generate one unsigned uni-7 transaction. This script has no sign/broadcast path.
usage() {
  cat >&2 <<'EOF'
usage:
  prepare-unsigned.sh store FROM artifact.wasm OUTPUT.json
  prepare-unsigned.sh instantiate FROM CODE_ID LABEL msg.json OUTPUT.json
  prepare-unsigned.sh execute FROM CONTRACT funds-ujunox msg.json OUTPUT.json

FROM must be a public juno address, not a key name. Existing outputs are never
replaced; that makes each reviewed step safe to resume.
EOF
  exit 2
}
[[ $# -ge 1 ]] || usage
command -v junod >/dev/null || { echo 'junod is required' >&2; exit 1; }
action=$1
shift
case "$action" in
  store)
    [[ $# -eq 3 ]] || usage
    from=$1 artifact=$2 output=$3
    args=(tx wasm store "$artifact" --from "$from")
    ;;
  instantiate)
    [[ $# -eq 5 ]] || usage
    from=$1 code_id=$2 label=$3 message=$4 output=$5
    [[ $code_id =~ ^[1-9][0-9]*$ ]] || usage
    args=(tx wasm instantiate "$code_id" "$(python3 -m json.tool --compact "$message")"
          --from "$from" --label "$label" --no-admin)
    ;;
  execute)
    [[ $# -eq 5 ]] || usage
    from=$1 contract=$2 funds=$3 message=$4 output=$5
    [[ $funds =~ ^[1-9][0-9]*$ ]] || usage
    args=(tx wasm execute "$contract" "$(python3 -m json.tool --compact "$message")"
          --from "$from" --amount "${funds}ujunox")
    ;;
  *) usage ;;
esac
[[ $from =~ ^juno1[023456789acdefghjklmnpqrstuvwxyz]{20,90}$ ]] || usage
[[ ! -e $output ]] || { echo "refusing to replace restart checkpoint: $output" >&2; exit 1; }
mkdir -p "$(dirname "$output")"
temporary="${output}.tmp"
trap 'rm -f "$temporary"' EXIT
junod "${args[@]}" --chain-id uni-7 --gas 2000000 --generate-only --output json > "$temporary"
python3 - "$temporary" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
value = json.loads(path.read_text())
# Generated transactions can contain signer metadata but never signatures.
signatures = value.get("signatures", value.get("tx", {}).get("signatures", []))
if signatures not in (None, []):
    raise SystemExit("refusing generated transaction containing signatures")
text = path.read_text().lower()
for forbidden in ("mnemonic", "private_key", "seed_phrase", "keyring", "password"):
    if forbidden in text:
        raise SystemExit(f"refusing secret-bearing output containing {forbidden}")
PY
mv "$temporary" "$output"
trap - EXIT
printf 'unsigned and unbroadcast uni-7 transaction: %s\n' "$output"
