#!/usr/bin/env bash
# Generate a new project from a template. See docs/workflows/new-app.md for the full workflow.
#   scripts/new-project.sh app|crate <name> "<one-line purpose>"
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

kind=${1:?usage: new-project.sh app|crate <name> "<purpose>"}
name=${2:?missing name}
description=${3:?missing one-line purpose}
case "$kind" in
  app) dest=apps ;;
  crate) dest=crates ;;
  *) echo "kind must be app or crate" >&2; exit 2 ;;
esac
template="templates/$kind"
[ "$kind" = app ] && [ -n "${TEMPLATE:-}" ] && template="templates/$TEMPLATE"

[ -e "$dest/$name" ] && { echo "$dest/$name already exists" >&2; exit 1; }
cargo generate --path "$template" --destination "$dest" --name "$name" \
  --define description="$description" --silent
scripts/sync-lints.sh
(cd "$dest/$name" && cargo generate-lockfile)
echo "created $dest/$name. Next: fill its AGENTS.md, add the registry row, run: just check $dest/$name"
