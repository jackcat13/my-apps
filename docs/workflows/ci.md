# Workflow: CI

Defined in `.github/workflows/ci.yml`. Runs on pull requests and on pushes to `main`. Checks only;
releases and deployment are out of scope until an app needs them (documented in that app's
`AGENTS.md`).

## Which projects run

`.github/scripts/changed-projects.sh` builds the list:

1. The files changed since the base (PR base, or the previous `main` commit on push).
2. Each changed file under `apps/<name>/` or `crates/<name>/` selects that project.
3. Every project that depends (transitively) on a selected project through a
   `path = "…"` dependency (resolved to its real folder) is selected too.
4. A change to shared config (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `.cargo/`,
   `.github/`) selects **every** project.
5. Changes only to `docs/`, `templates/`, or root markdown select nothing.

Run it locally to see what CI would check:

```bash
BASE_SHA=$(git merge-base origin/main HEAD) .github/scripts/changed-projects.sh
ALL=1 .github/scripts/changed-projects.sh   # every project
```

## What runs per project

From inside the project folder, after installing the toolchain from `rust-toolchain.toml` and
restoring the cache:

- If the project has an executable `ci.sh`, run **only** `./ci.sh`. It replaces the defaults, so
  it's responsible for fmt/clippy/test as well as whatever extra it needs (targets, system packages
  through `sudo apt-get`, …).
- Otherwise:
  ```bash
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo test
  ```

The `ci-ok` job sums up the matrix. Make it the single required status check in branch protection.

## Typical `ci.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
rustup target add wasm32-unknown-unknown
cargo fmt --check
cargo clippy --all-targets --target wasm32-unknown-unknown -- -D warnings
cargo test
```
