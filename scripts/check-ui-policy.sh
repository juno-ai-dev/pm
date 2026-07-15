#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
test ! -d "$root/ui"
test -f "$root/depricated_ui/UNSUPPORTED.md"
if grep -Eq 'working-directory:[[:space:]]*ui([[:space:]]|$)' "$root/.github/workflows/ui.yml"; then
  echo "UI workflow references nonexistent ui/" >&2
  exit 1
fi
grep -q 'packages: \[\]' "$root/depricated_ui/pnpm-workspace.yaml"
grep -q 'onlyBuiltDependencies:' "$root/depricated_ui/pnpm-workspace.yaml"
