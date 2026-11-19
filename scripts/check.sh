#!/usr/bin/env bash
# The full quality gate for one project. The same script runs locally (`just check <project>`)
# and in CI. Every step is listed in docs/workflows/quality.md.
#
#   scripts/check.sh apps/<name> | crates/<name>
#
# A project tunes the gate with an optional `ci.env` (sourced, see below) and adds steps with an
# optional executable `ci.sh`. Neither can skip a standard step.
set -euo pipefail
root=$(git rev-parse --show-toplevel)
project=${1:?usage: scripts/check.sh <project folder>}
project=${project%/}
cd "$root/$project" 2>/dev/null || cd "$project"
[ -f Cargo.toml ] || { echo "no Cargo.toml in $project" >&2; exit 2; }

case "$project" in
  crates/* | */crates/*) kind=crate ;;
  *) kind=app ;;
esac

# --- ci.env: the only knobs a project has -------------------------------------------------
# FEATURE_FLAGS  feature selection for every cargo step (default: --all-features)
# TARGETS        extra targets to clippy-check, space separated (e.g. wasm32-unknown-unknown)
# APT_PACKAGES   system packages CI installs before the gate (read by the workflow)
FEATURE_FLAGS="--all-features"
TARGETS=""
# shellcheck disable=SC2034 # read by the workflow
APT_PACKAGES=""
# shellcheck source=/dev/null
if [ -f ci.env ]; then . ./ci.env; fi
# Coverage floor: shared crates must reach it, apps only report. Not overridable.
if [ "$kind" = crate ]; then coverage_min=80; else coverage_min=0; fi

step() {
  local name=$1
  shift
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::group::$name"; else echo "==> $name"; fi
  local status=0
  "$@" || status=$?
  if [ -n "${GITHUB_ACTIONS:-}" ]; then echo "::endgroup::"; fi
  if [ "$status" -ne 0 ]; then
    echo "FAILED: $name ($project)" >&2
    exit "$status"
  fi
}

has_lib() { [ -f src/lib.rs ]; }

clippy_targets() {
  local t
  for t in $TARGETS; do
    rustup target add "$t" >/dev/null
    # shellcheck disable=SC2086
    cargo clippy --locked --target "$t" --lib --bins $FEATURE_FLAGS -- -D warnings
  done
}

coverage() {
  local floor=()
  if [ "$coverage_min" -gt 0 ]; then floor=(--fail-under-lines "$coverage_min"); fi
  # shellcheck disable=SC2086
  NEXTEST_NO_TESTS=pass cargo llvm-cov nextest --locked $FEATURE_FLAGS \
    --config-file "$root/.config/nextest.toml" --profile ci ${floor[@]+"${floor[@]}"}
}

doctests() {
  if has_lib; then
    # shellcheck disable=SC2086
    cargo test --locked --doc $FEATURE_FLAGS
  else
    echo "no library target, skipped"
  fi
}

lockfile_unchanged() {
  if git ls-files --error-unmatch Cargo.lock >/dev/null 2>&1; then
    git diff --exit-code -- Cargo.lock
  fi
}

# shellcheck disable=SC2086
{
  step "fmt" cargo fmt --check
  step "lints in sync" "$root/scripts/sync-lints.sh" --check
  step "clippy" cargo clippy --locked --all-targets $FEATURE_FLAGS -- -D warnings
  step "clippy (extra targets)" clippy_targets
  if [ "$kind" = crate ]; then
    step "each feature builds alone" cargo hack check --locked --each-feature --no-dev-deps
  fi
  step "tests + coverage (floor: ${coverage_min}% lines)" coverage
  step "doctests" doctests
  step "docs" env RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps $FEATURE_FLAGS
  step "cargo-deny" cargo deny --all-features --config "$root/deny.toml" check
  step "unused dependencies" cargo machete
  step "layering" "$root/scripts/check-layering.sh" "$project"
  step "Cargo.lock unchanged" lockfile_unchanged
  if [ -x ci.sh ]; then step "project ci.sh" ./ci.sh; fi
}
echo "OK: $project"
