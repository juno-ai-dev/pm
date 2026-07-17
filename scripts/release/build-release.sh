#!/usr/bin/env bash
set -euo pipefail

readonly optimizer_image='cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85'
root=$(git rev-parse --show-toplevel)
out=${1:?usage: build-release.sh OUTPUT_DIR}

[[ -z $(git -C "$root" status --porcelain --untracked-files=no) ]] || {
  echo 'tracked source must be clean before a release build' >&2
  exit 1
}
docker version >/dev/null 2>&1 || {
  echo 'Docker daemon access is required; no optimizer was executed' >&2
  exit 1
}

stage=$(mktemp -d)
cleanup() { rm -rf "$stage"; }
trap cleanup EXIT
# The optimizer writes /code/artifacts. Use an isolated copy so two runs share
# neither target state nor output while preserving the exact checked-out bytes.
git -C "$root" archive --format=tar HEAD | tar -xf - -C "$stage"
# The optimizer only treats workspace members below a `contracts/` path as
# contracts when invoked at a workspace root. This repository's contract
# workspace is itself mounted at /code, so invoke each deployable package
# explicitly while retaining the shared workspace for path dependencies.
for contract in binary-market cw-reality market-factory; do
  docker run --rm --platform linux/amd64 \
    --volume "$stage/contracts:/code" \
    "$optimizer_image" "$contract"
done

mkdir -p "$out/artifacts"
for artifact in binary_market.wasm cw_reality.wasm market_factory.wasm; do
  test -f "$stage/contracts/artifacts/$artifact" || {
    echo "optimizer did not produce $artifact" >&2
    exit 1
  }
  install -m 0644 "$stage/contracts/artifacts/$artifact" "$out/artifacts/$artifact"
done
printf '%s\n' "$optimizer_image" > "$out/optimizer-image.txt"
python3 "$root/scripts/release/generate-metadata.py" "$out"
