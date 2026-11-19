# my-apps — Rust monorepo

Personal monorepo holding all Rust applications. This file is the shared context for every
agent working anywhere in the repo. Each app/crate has its own `AGENTS.md` with context that
applies only there; it is loaded on demand when you work in that folder.

## Layout

| Path | What lives there |
|---|---|
| `apps/<name>/` | One application = one **standalone** Cargo project (own `Cargo.toml`, `Cargo.lock`, `target/`) + its `AGENTS.md`. Optional `ci.env`/`ci.sh` add to the standard gate (never replace it). |
| `crates/<name>/` | Shared library crates, also standalone. Apps depend on them with `path = "../../crates/<name>"`. |
| `docs/` | Shared knowledge. Read the relevant file **before** making a decision it covers. |
| `templates/` | `cargo-generate` templates (`app`, `crate`). |
| `scripts/` | Every check, called by `justfile` locally and by CI. |
| `lint/ast-grep/` | Custom lint rules and their tests. |
| `.github/` | CI: checks only the projects a change affects, plus scheduled jobs. |

Shared config at the root: `lints.toml` (lint levels, synced into each project), `clippy.toml`,
`rustfmt.toml`, `rust-toolchain.toml` (pinned), `deny.toml`, `.config/nextest.toml`, `sgconfig.yml`,
`tools.txt` (pinned tool versions). First time on a machine: `scripts/setup.sh`.

There is **no Cargo workspace** (see `docs/adr/0001-standalone-projects.md`). Run cargo commands
from inside the project folder, and `just` recipes from the repo root.

## Where to look

- `docs/conventions.md` — coding rules for every project (errors, logging, tests, layout). Read before writing code.
- `docs/stack.md` — the default crate for each concern. Use it unless the app's `AGENTS.md` says otherwise.
- `docs/stacks/<kind>.md` — patterns for a kind of app (cli, web, gui, wasm, embedded, game…). Read the matching one when working on an app of that kind.
- `docs/workflows/new-app.md` — how to create a new app or shared crate. Always follow it; never hand-roll a project.
- `docs/workflows/quality.md` — every check, what it catches, and how to make an exception. Read before suppressing anything.
- `docs/workflows/ci.md` — how CI selects and checks projects.
- `docs/adr/` — why the repo is set up this way. Add an ADR for any new repo-wide decision.

## Rules

1. **Never create a `CLAUDE.md`** anywhere in this repo (or above it). Its presence makes Claude Code
   ignore every `AGENTS.md`. Claude-only notes go in `.claude/rules/`. Requires Claude Code ≥ 2.1.277.
2. **File knowledge at the right level.** Something true only for one app → that app's `AGENTS.md`.
   Something reusable across apps → **propose** an addition to `docs/` (or a new `docs/stacks/<kind>.md`)
   to the user; do not silently edit shared docs.
3. **Rule of two for shared code.** Don't create a crate in `crates/` speculatively. When a second app
   needs the same code, extract it into `crates/` and switch both apps to the path dependency.
4. **Keep the registry below current.** Adding, removing, renaming an app/crate, or changing its purpose
   updates its row.
5. **Shared tooling config lives at the root** (see Layout) and is picked up by every project.
   Lint levels are set only in `lints.toml`; run `just sync-lints` after changing it and never edit
   the generated block in a `Cargo.toml`. Override per project only when there's a reason, and
   document the reason in that project's `AGENTS.md`.
6. **A project is done when `just check <project>` passes** (the same gate CI runs). Report only
   checks that actually ran, with their real result.
7. **Never weaken the gates without asking the user.** That covers `lints.toml`, `clippy.toml`,
   `deny.toml`, `sgconfig.yml`/`lint/`, the coverage floor, and the steps in `scripts/`.
8. **Suppressions are local and explained:** `#[expect(lint, reason = "…")]`, never `#[allow]`.
   Fix the code first; suppress only when the lint is wrong for that line.
9. **A crash fix comes with a regression test** that failed before the fix.
10. **New dependencies come from `docs/stack.md`.** Anything else needs a reason in the app's
    `AGENTS.md`. Never add a banned crate or a non-permissive licence.

## Registry

One row per project. Kind: `cli`, `web`, `gui`, `wasm`, `embedded`, `game`, `lib`, …

| Name | Path | Kind | Purpose |
|---|---|---|---|
| life-dashboard | apps/life-dashboard | gui | Personal finance/stats dashboard |
