# 0005 — Strict, machine-enforced quality gates

- Status: Accepted
- Date: 2026-11-18

## Context
The repo is the foundation for every future app, and much of the code will be written by AI agents.
Written conventions alone drift; checks that fail the build don't. The setup is modeled on the
storytold/ArtCraft Rust repos (no-panic lints on every crate, cargo-deny, a single check entry
point, custom ast-grep lints, nextest, fuzzing), adapted to standalone projects.

## Decision
- One canonical `lints.toml` (strict: pedantic, no-panic set, `allow_attributes` denied so every
  suppression is a reasoned `#[expect]`), synced into each project (addendum to 0001).
- Narrower lints are crate-root attributes (`missing_docs` for crates, indexing/arithmetic for
  untrusted-input parsers), not variant lint files.
- Stack choices are enforced: `deny.toml` bans replaced crates, `clippy.toml` bans their types and
  macros. Versions stay per project.
- One gate script per project (`scripts/check.sh`), run by `just check` and by CI. Projects can add
  to it (`ci.env`, `ci.sh`) but not skip any step.
- Gate: fmt, clippy `-D warnings`, `cargo hack --each-feature` (crates), nextest under llvm-cov
  (80 % lines for crates, report for apps), doctests, docs with `-D warnings`, cargo-deny,
  cargo-machete, layering, `--locked` + unchanged `Cargo.lock`.
- Repo checks: lint sync, `rust-version` == toolchain, typos, taplo, ast-grep (rules with tests),
  shellcheck, actionlint, zizmor, templates generated and gated.
- Scheduled: cargo-deny daily; miri, fuzzing, cargo-mutants (report) weekly.
- Pinned everything: toolchain (`rust-toolchain.toml`), tools (`tools.txt`), actions (commit SHA).
- Feedback as early as possible: rustfmt on every Claude Code edit, fast checks at the end of each
  agent turn and on `git commit`.

## Consequences
- New code meets the bar from the first commit; agents get mechanical, immediate feedback.
- More friction: some lints need `#[expect]` with a reason; the full gate is slower than `cargo test`.
- More tools to install (`scripts/setup.sh`) and to bump (`tools.txt`, toolchain by hand,
  actions by Dependabot).
- Changing shared config is a repo-wide decision and reruns every project in CI.
