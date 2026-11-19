# 0001 — Standalone projects, no Cargo workspace

- Status: Accepted
- Date: 2026-11-18

## Context
The repo will hold any kind of Rust app (CLI, web, GUI, wasm, embedded, games). These have different
targets, toolchains and dependency trees, and they should never break each other.

## Decision
Every app in `apps/` and every shared crate in `crates/` is a standalone Cargo project with its own
`Cargo.lock` and `target/`. No root `Cargo.toml`. Shared crates are consumed through path dependencies.

## Consequences
- Apps are fully isolated: dependency upgrades, toolchains and targets never collide.
- Lint levels can't be inherited, so each project carries a `[lints]` block copied from the templates.
- Tooling config is still shared through parent-directory lookup: `rust-toolchain.toml`,
  `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml`.
- Dependency versions aren't unified. `docs/stack.md` aligns crate *choices* only.
- No shared `target/`, so more disk use and compile time per project.
- Cargo commands must run inside a project folder.

## Addendum (2026-11-18): shared lints without a workspace

A Cargo workspace was reconsidered to stop copying the `[lints]` block. It was rejected: a
workspace means one `Cargo.lock` and unified dependency versions, and the only stable way to
inherit lints (`[workspace.lints]`) requires one. Lints in `.cargo/config.toml` `rustflags` are
fragile (depend on the directory, force rebuilds).

Instead, the canonical table lives in the root `lints.toml`, `just sync-lints` writes it into every
project between markers, and CI fails when a copy drifts. The copy exists, but no one maintains it
by hand. Only general rules are shared; versions and build profiles stay per project. See
[0005](0005-quality-gates.md).
