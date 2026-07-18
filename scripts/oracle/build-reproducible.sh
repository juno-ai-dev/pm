#!/usr/bin/env bash
set -euo pipefail

# Build one immutable source commit with the selected production optimizer.
readonly image='cosmwasm/optimizer:0.17.0@sha256:7e0b9229c1a4118d0c9a2af2e7f5d95a91f264c26a2ce5681c779926e74d7f85'
readonly source_commit='454f9777b0bafa71c43b427f7451e626d860269e'

root=$(git rev-parse --show-toplevel)
out=${1:-"$root/artifacts/oracle"}
if [[ $(git -C "$root" rev-parse "$source_commit^{commit}") != "$source_commit" ]]; then
  echo 'pinned source commit is unavailable' >&2
  exit 1
fi
if ! docker version >/dev/null 2>&1; then
  echo 'Docker daemon access is required; no optimizer image was executed.' >&2
  exit 1
fi

worktree=$(mktemp -d)
cleanup() {
  git -C "$root" worktree remove --force "$worktree" >/dev/null 2>&1 || true
}
trap cleanup EXIT
git -C "$root" worktree add --detach "$worktree" "$source_commit" >/dev/null
[[ -z $(git -C "$worktree" status --porcelain) ]] || {
  echo 'source worktree is not clean' >&2
  exit 1
}

# No shared target/registry volumes: independent runs cannot inherit build output.
docker run --rm --platform linux/amd64 --volume "$worktree:/code" "$image"
source_artifact="$worktree/artifacts/cw_reality.wasm"
[[ -f "$source_artifact" ]] || {
  echo "optimizer did not produce $source_artifact" >&2
  exit 1
}
mkdir -p "$out"
install -m 0644 "$source_artifact" "$out/cw_reality.wasm"
digest=$(sha256sum "$out/cw_reality.wasm" | cut -d ' ' -f1)
printf '%s\n' "$digest" | tee "$out/cw_reality.wasm.sha256"
wc -c < "$out/cw_reality.wasm" | tr -d ' ' > "$out/cw_reality.wasm.size"
printf '%s\n' "$source_commit" > "$out/source-commit.txt"
printf '%s\n' "$image" > "$out/optimizer-image.txt"
