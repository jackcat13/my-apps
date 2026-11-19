#!/usr/bin/env bash
# Enforce the dependency layers: crates/* never depend on apps/*, and apps never depend on
# other apps. Shared code goes in crates/ (rule of two, root AGENTS.md).
#   scripts/check-layering.sh [project...]   (default: every project)
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
# shellcheck source=scripts/lib.sh
. scripts/lib.sh

# Project names never contain spaces (kebab-case), so word splitting is safe here.
# shellcheck disable=SC2046
[ $# -eq 0 ] && set -- $(all_projects)

failed=0
for p in "$@"; do
  [ -n "$p" ] || continue
  while read -r dep; do
    [ -n "$dep" ] || continue
    case "$p:$dep" in
      crates/*:apps/*) echo "layering: $p depends on app $dep (crates never depend on apps)" >&2; failed=1 ;;
      apps/*:apps/*) echo "layering: $p depends on app $dep (extract shared code into crates/)" >&2; failed=1 ;;
      *:apps/*|*:crates/*) ;;
      *) echo "layering: $p depends on $dep, outside apps/ and crates/" >&2; failed=1 ;;
    esac
  done < <(path_deps "$p")
done
exit "$failed"
