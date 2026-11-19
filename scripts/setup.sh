#!/usr/bin/env bash
# One-time local setup: toolchain, pinned tools, git hooks. Safe to re-run.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

rustup toolchain install   # reads rust-toolchain.toml
scripts/tools.sh install
git config core.hooksPath .githooks
echo "setup done: tools installed, pre-commit hook enabled (.githooks/)"
