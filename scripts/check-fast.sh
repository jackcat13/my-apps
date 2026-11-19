#!/usr/bin/env bash
# Fast checks (fmt + clippy) on the projects touched by uncommitted changes.
# Used by the git pre-commit hook (--staged) and the Claude Code Stop hook.
#   scripts/check-fast.sh [--staged]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode=worktree
[ "${1:-}" = "--staged" ] && mode=staged
projects=$(LOCAL=$mode FORMAT=lines scripts/changed-projects.sh)
[ -n "$projects" ] || { echo "check-fast: no Rust project changed"; exit 0; }

failed=0
for p in $projects; do
  echo "==> $p"
  (cd "$p" && cargo fmt --check) || { echo "fmt: run \`cargo fmt\` in $p" >&2; failed=1; }
  (cd "$p" && cargo clippy --quiet --all-targets --all-features -- -D warnings) || failed=1
done
scripts/sync-lints.sh --check || failed=1
exit "$failed"
