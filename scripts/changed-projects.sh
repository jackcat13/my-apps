#!/usr/bin/env bash
# Print the projects (apps/<name>, crates/<name>) affected by a change.
#
# Env:
#   BASE_SHA  diff against this commit (merge-base). Empty, all-zero or unknown => all projects.
#   ALL=1     force all projects.
#   LOCAL=worktree|staged  use uncommitted (or only staged) changes instead of BASE_SHA.
#   FORMAT=json|lines      output format (default json). json also goes to $GITHUB_OUTPUT.
# See docs/workflows/ci.md.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
# shellcheck source=scripts/lib.sh
. scripts/lib.sh

emit() {
  if [ "${FORMAT:-json}" = lines ]; then sed '/^$/d'; return; fi
  local json
  json=$(sed '/^$/d' | jq -R . | jq -sc .)
  echo "$json"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "projects=$json" >> "$GITHUB_OUTPUT"; fi
}

case "${LOCAL:-}" in
  staged) changed_files=$(git diff --name-only --cached) ;;
  worktree) changed_files=$(git diff --name-only HEAD; git ls-files --others --exclude-standard) ;;
  "")
    base="${BASE_SHA:-}"
    if [ "${ALL:-}" = "1" ] || [ -z "$base" ] || [ -z "${base//0/}" ] || ! git cat-file -e "$base^{commit}" 2>/dev/null; then
      all_projects | emit
      exit 0
    fi
    changed_files=$(git diff --name-only "$(git merge-base "$base" HEAD)" HEAD)
    ;;
  *) echo "LOCAL must be worktree or staged" >&2; exit 2 ;;
esac

# Shared config or tooling => everything.
shared='^(rust-toolchain\.toml|rustfmt\.toml|clippy\.toml|lints\.toml|deny\.toml|sgconfig\.yml|tools\.txt|justfile|\.cargo/|\.config/|\.github/|scripts/|lint/)'
if printf '%s\n' "$changed_files" | grep -E "$shared" >/dev/null; then
  all_projects | emit
  exit 0
fi

# Directly changed projects (that still exist).
selected=$(printf '%s\n' "$changed_files" | sed -nE 's#^((apps|crates)/[^/]+)/.*#\1#p' | sort -u | while read -r p; do
  if [ -n "$p" ] && [ -f "$p/Cargo.toml" ]; then echo "$p"; fi
done)

# Add dependents of selected projects (path dependencies, from cargo metadata), transitively.
while [ -n "$selected" ]; do
  added=""
  for p in $(all_projects); do
    # Captured into variables: piping into `grep -q` would SIGPIPE the producer under pipefail.
    if printf '%s\n' "$selected" | grep -xF "$p" >/dev/null; then continue; fi
    deps=$(path_deps "$p")
    if [ -n "$deps" ] && printf '%s\n' "$deps" | grep -xF -f <(printf '%s\n' "$selected") >/dev/null; then
      added="$added$p"$'\n'
    fi
  done
  [ -z "$added" ] && break
  selected=$(printf '%s\n%s' "$selected" "$added" | sed '/^$/d' | sort -u)
done

printf '%s\n' "$selected" | emit
