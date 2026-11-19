#!/usr/bin/env bash
# Repo-wide checks that don't belong to one project. Runs in CI on every change.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

step() {
  local name=$1
  shift
  echo "==> $name"
  "$@" || { echo "FAILED: $name" >&2; exit 1; }
}

# rust-version in every manifest equals the pinned toolchain's major.minor.
versions_match() {
  local channel want failed=0 m
  channel=$(sed -nE 's/^channel *= *"([^"]+)".*/\1/p' rust-toolchain.toml)
  want=$(echo "$channel" | cut -d. -f1,2)
  for m in apps/*/Cargo.toml crates/*/Cargo.toml templates/*/Cargo.toml; do
    [ -f "$m" ] || continue
    if ! grep -qE "^rust-version *= *\"$want\"" "$m"; then
      echo "$m: rust-version must be \"$want\" (rust-toolchain.toml pins $channel)" >&2
      failed=1
    fi
  done
  return "$failed"
}

shell_scripts() { git ls-files 'scripts/*.sh' '.githooks/*' '.claude/hooks/*.sh'; }

step "lints in sync" scripts/sync-lints.sh --check
step "rust-version matches toolchain" versions_match
step "layering" scripts/check-layering.sh
step "typos" typos
step "TOML format (taplo)" taplo fmt --check
step "ast-grep rule tests" ast-grep test --skip-snapshot-tests
step "ast-grep scan" ast-grep scan --error
# shellcheck disable=SC2046
step "shellcheck" shellcheck -x $(shell_scripts)
step "actionlint" actionlint
step "zizmor" zizmor --quiet .github/workflows
echo "OK: repo checks"
