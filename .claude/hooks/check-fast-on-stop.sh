#!/usr/bin/env bash
# Stop hook: run fmt + clippy on changed projects when Claude ends a turn. On failure, exit 2 so
# Claude gets the errors and fixes them before handing off. Gives up after 3 tries per session so
# it can never loop forever.
set -uo pipefail
cd "${CLAUDE_PROJECT_DIR:-.}" || exit 0
input=$(cat)
command -v cargo >/dev/null || exit 0

session=$(printf '%s' "$input" | jq -r '.session_id // "unknown"')
counter="${TMPDIR:-/tmp}/my-apps-stop-hook-$session"
tries=$(cat "$counter" 2>/dev/null || echo 0)

if output=$(scripts/check-fast.sh 2>&1); then
  rm -f "$counter"
  exit 0
fi
if [ "$tries" -ge 3 ]; then
  rm -f "$counter"
  echo "check-fast still fails after 3 attempts; tell the user what is left." >&2
  exit 0
fi
echo $((tries + 1)) > "$counter"
{
  echo "scripts/check-fast.sh failed. Fix these before finishing (never by weakening lints):"
  printf '%s\n' "$output" | tail -n 80
} >&2
exit 2
