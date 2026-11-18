#!/usr/bin/env bash
# Print the projects (apps/<name>, crates/<name>) CI must check, as a JSON array.
# Writes `projects=<json>` to $GITHUB_OUTPUT when set.
#
# Env:
#   BASE_SHA  commit to diff against (merge-base is used). Empty or all-zero => all projects.
#   ALL=1     force all projects.
# See docs/workflows/ci.md.
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

all_projects() {
  for d in apps/*/ crates/*/; do
    if [ -f "${d}Cargo.toml" ]; then echo "${d%/}"; fi
  done | sort -u
}

emit() {
  local json
  json=$(sed '/^$/d' | awk 'BEGIN{printf "["} {printf "%s\"%s\"", (NR>1?",":""), $0} END{printf "]"}')
  echo "$json"
  if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "projects=$json" >> "$GITHUB_OUTPUT"; fi
}

base="${BASE_SHA:-}"
if [ "${ALL:-}" = "1" ] || [ -z "$base" ] || [ -z "${base//0/}" ] || ! git cat-file -e "$base^{commit}" 2>/dev/null; then
  all_projects | emit
  exit 0
fi

changed_files=$(git diff --name-only "$(git merge-base "$base" HEAD)" HEAD)

# Shared config => everything.
if printf '%s\n' "$changed_files" | grep -E '^(rust-toolchain\.toml|rustfmt\.toml|clippy\.toml|\.cargo/|\.github/)' >/dev/null; then
  all_projects | emit
  exit 0
fi

# Directly changed projects (that still exist).
selected=$(echo "$changed_files" | sed -nE 's#^((apps|crates)/[^/]+)/.*#\1#p' | sort -u | while read -r p; do
  if [ -n "$p" ] && [ -f "$p/Cargo.toml" ]; then echo "$p"; fi
done)

root=$(pwd -P)

# Repo-relative folders of a project's path dependencies, resolved from its folder.
path_deps() {
  sed -nE 's/.*path[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$1/Cargo.toml" | while read -r rel; do
    if abs=$(cd "$1/$rel" 2>/dev/null && pwd -P); then echo "${abs#"$root"/}"; fi
  done
}

# Add dependents of selected projects, transitively, until nothing new is added.
while :; do
  added=""
  for p in $(all_projects); do
    # Captured into variables: piping into `grep -q` would SIGPIPE the producer under pipefail.
    if printf '%s\n' "$selected" | grep -xF "$p" >/dev/null; then continue; fi
    deps=$(path_deps "$p")
    if [ -n "$deps" ] && [ -n "$selected" ] && printf '%s\n' "$deps" | grep -xF -f <(printf '%s\n' "$selected" | sed '/^$/d') >/dev/null; then
      added="$added$p"$'\n'
    fi
  done
  [ -z "$added" ] && break
  selected=$(printf '%s\n%s' "$selected" "$added" | sed '/^$/d' | sort -u)
done

echo "$selected" | emit
