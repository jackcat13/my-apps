# Conventions

Rules that apply to every project in the repo. An app may deviate only if its `AGENTS.md` says so
and why.

## Project setup

- Edition **2024**. Toolchain comes from the root `rust-toolchain.toml` (stable).
- Every project keeps the `[lints]` block from its template. Don't weaken a lint globally;
  use a scoped `#[allow(clippy::…)]` with a short reason comment, or `#[expect(…)]`.
- Commit `Cargo.lock` for every project, libraries included.
- `publish = false` unless the crate is actually meant for crates.io.

## Errors

- Libraries (`crates/`, and library modules inside apps): typed errors with `thiserror`.
  One error enum per module boundary that callers need to match on; no stringly-typed errors.
- Binaries (`main.rs`, top-level glue): `anyhow::Result`, adding context with `.context("…")` at each
  I/O or external boundary.
- No `unwrap()`/`expect()` outside tests, except for invariants that truly can't fail. In that case
  use `expect("reason this cannot fail")`.
- No `panic!` for recoverable conditions.

## Logging and diagnostics

- Use `tracing` everywhere (not `log`, not `println!` for diagnostics). Binaries install a
  `tracing-subscriber` with `EnvFilter` (`RUST_LOG`).
- `println!` is only for a CLI's actual output (stdout); diagnostics go to stderr through tracing.

## Code layout

- `main.rs` stays thin: parse config/args, init tracing, call into `lib.rs` or modules, map errors to exit codes.
  Putting the logic in a library target keeps it testable.
- Modules by feature/domain, not by technical layer (`invoice/`, not `models/` + `services/`).
- Use `mod.rs`-free layout (`foo.rs` + `foo/`).
- Keep `pub` minimal; prefer `pub(crate)`.

## Naming

- Crate and folder names: `kebab-case`. They must match (`apps/my-tool` → package `my-tool`).
- Shared crates are named for what they do (`config-loader`), never `common` / `utils` / `shared`.

## Testing

- Unit tests in a `#[cfg(test)] mod tests` next to the code. Integration tests in `tests/`
  against the public API.
- Test behavior, not implementation details. Name a test after the behavior it checks
  (`rejects_expired_token`).
- Tests must not depend on network or global state unless marked `#[ignore]` with a reason.

## Dependencies

- Pick crates from [`stack.md`](stack.md) first. Adding a crate that isn't there for a concern
  `stack.md` already covers needs a reason in the app's `AGENTS.md`.
- Prefer few, well-maintained dependencies. Turn off default features you don't need.
- Shared crates in `crates/` keep dependencies minimal, because every app that uses them inherits them.

## Docs and comments

- Public items of shared crates get doc comments.
- Comments explain *why*, not *what*.
- Each project's `README.md` says what it is and how to run it. Its `AGENTS.md` holds what an agent
  needs to work on it.
