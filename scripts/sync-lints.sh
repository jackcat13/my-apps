#!/usr/bin/env bash
# Copy lints.toml into every project's Cargo.toml (and the templates), between the markers.
#   scripts/sync-lints.sh          rewrite the blocks
#   scripts/sync-lints.sh --check  fail if any block differs (used by CI)
# A Cargo.toml without the markers gets the block appended.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

# shellcheck disable=SC2016 # backticks are literal text in the marker, not a command
begin='# BEGIN shared lints: generated from /lints.toml by `just sync-lints`. Do not edit.'
end='# END shared lints'
check=0
[ "${1:-}" = "--check" ] && check=1

block=$(mktemp)
trap 'rm -f "$block"' EXIT
{
  echo "$begin"
  # Strip the comment header and blank lines before the first table.
  sed -n '/^\[/,$p' lints.toml
  echo "$end"
} > "$block"

failed=0
for manifest in apps/*/Cargo.toml crates/*/Cargo.toml templates/*/Cargo.toml; do
  [ -f "$manifest" ] || continue
  new=$(mktemp)
  if grep -qF "$begin" "$manifest"; then
    awk -v begin="$begin" -v end="$end" -v blockfile="$block" '
      $0 == begin { while ((getline line < blockfile) > 0) print line; skip = 1; next }
      skip && $0 == end { skip = 0; next }
      !skip { print }
    ' "$manifest" > "$new"
  else
    { cat "$manifest"; echo; cat "$block"; } > "$new"
  fi
  if ! cmp -s "$manifest" "$new"; then
    if [ "$check" = 1 ]; then
      echo "lints out of sync: $manifest (run: just sync-lints)" >&2
      failed=1
    else
      cp "$new" "$manifest"
      echo "synced $manifest"
    fi
  fi
  rm -f "$new"
done
exit "$failed"
