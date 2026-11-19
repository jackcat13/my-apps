# Workflow: CI

Defined in `.github/workflows/ci.yml` (every PR and push to `main`) and
`.github/workflows/scheduled.yml` (daily and weekly). The workflows only call scripts from
`scripts/`, the same ones `just` runs locally, so a green `just check` means a green CI.
What each check does: [quality.md](quality.md). Releases and deployment are out of scope until an
app needs them (documented in that app's `AGENTS.md`).

## Jobs

- **detect**: `scripts/changed-projects.sh` lists the affected projects.
- **repo**: `just check-repo` and `just check-templates`. Always runs.
- **check** (matrix, one job per affected project): installs `APT_PACKAGES` from the project's
  `ci.env`, then `just check <project>`.
- **CI result** (`ci-ok`): sums up the others. Make it the single required status check in branch
  protection (a GitHub setting, done by hand).

## Which projects run

`scripts/changed-projects.sh` builds the list:

1. The files changed since the base (PR base, or the previous `main` commit on push).
2. Each changed file under `apps/<name>/` or `crates/<name>/` selects that project.
3. Every project that depends (transitively) on a selected project through a path dependency
   (read from `cargo metadata`) is selected too.
4. A change to shared config or tooling (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`,
   `lints.toml`, `deny.toml`, `sgconfig.yml`, `tools.txt`, `justfile`, `.cargo/`, `.config/`,
   `.github/`, `scripts/`, `lint/`) selects **every** project.
5. Changes only to `docs/`, `templates/`, or root markdown select no project (templates are covered
   by the repo job).

Locally:

```bash
just affected                                   # vs origin/main
ALL=1 FORMAT=lines scripts/changed-projects.sh  # every project
LOCAL=worktree FORMAT=lines scripts/changed-projects.sh
```

## Hardening

- Actions are pinned by commit SHA with the version in a comment; Dependabot updates them weekly.
- `permissions: {}` at the top, `contents: read` per job; `persist-credentials: false` on checkout.
- `actionlint` and `zizmor` lint the workflows; `shellcheck` lints the scripts.
- Tool versions come from `tools.txt`; the toolchain from `rust-toolchain.toml`.

## Project-specific needs

A project never replaces the gate. It adds to it with `ci.env` (features, extra targets, system
packages) and `ci.sh` (extra steps). Example for a wasm app:

```bash
# apps/my-web-ui/ci.env
TARGETS="wasm32-unknown-unknown"
```
