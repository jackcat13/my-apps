#!/usr/bin/env bash
# PostToolUse hook: format the .rs file Claude just edited. Never blocks.
set -uo pipefail
file=$(jq -r '.tool_input.file_path // empty')
case "$file" in
  *.rs) [ -f "$file" ] && rustfmt --edition 2024 --quiet "$file" 2>/dev/null ;;
esac
exit 0
