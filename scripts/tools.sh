#!/usr/bin/env bash
# Read tools.txt.
#   scripts/tools.sh list [name...]   comma-separated name@version, for taiki-e/install-action
#   scripts/tools.sh install          install every tool locally (cargo-binstall, brew)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

entries() { grep -vE '^[[:space:]]*(#|$)' tools.txt; }

case "${1:-}" in
  list)
    shift
    entries | while read -r name version _; do
      if [ $# -eq 0 ] || printf '%s\n' "$@" | grep -xF "$name" >/dev/null; then
        printf '%s@%s\n' "$name" "$version"
      fi
    done | paste -sd, -
    ;;
  install)
    command -v cargo-binstall >/dev/null || {
      echo "cargo-binstall is required: brew install cargo-binstall" >&2
      exit 1
    }
    entries | while read -r name version how; do
      case "$how" in
        binstall:*) cargo binstall --no-confirm --locked "${how#binstall:}@$version" ;;
        brew:*)
          brew install --quiet "${how#brew:}"
          echo "note: $name comes from brew, which can't pin; tools.txt expects $version" ;;
        *) echo "tools.txt: unknown install method for $name: $how" >&2; exit 1 ;;
      esac
    done
    ;;
  *) echo "usage: scripts/tools.sh list [name...] | install" >&2; exit 2 ;;
esac
