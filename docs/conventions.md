# Conventions

Rules that apply to every project in the repo. An app may deviate only if its `AGENTS.md` says so
and why.

## Project setup

- Edition **2024**. Toolchain comes from the root `rust-toolchain.toml` (pinned stable);
  `rust-version` in `Cargo.toml` matches its major.minor.
- Lint levels come only from the root `lints.toml`, synced into each `Cargo.toml` by
  `just sync-lints`. Never edit the generated block. An exception is local:
  `#[expect(clippy::…, reason = "…")]`. A bare `#[allow]` is denied.
- Every check, and how to make an exception: [`workflows/quality.md`](workflows/quality.md).
- Work is done when `just check <project>` passes.
- Commit `Cargo.lock` for every project, libraries included.
- `publish = false` unless the crate is actually meant for crates.io.

## Errors

- Libraries (`crates/`, and library modules inside apps): typed errors with `thiserror`.
  One error enum per module boundary that callers need to match on; no stringly-typed errors.
- Binaries (`main.rs`, top-level glue): `anyhow::Result`, adding context with `.context("…")` at each
  I/O or external boundary.
- No panics in shipped code (enforced: `unwrap_used`, `expect_used`, `panic`, `todo`,
  `unimplemented`, `unreachable` are denied). For an invariant that truly can't fail:
  `#[expect(clippy::expect_used, reason = "…")]` and `.expect("why it cannot fail")`.
- No string errors: no `Result<_, String>`, `Box<dyn Error>` or `Err(format!(…))` (ast-grep rule).
  No `anyhow` in `crates/*`.
- Input-derived indices use `get()`, input-derived arithmetic uses `checked_*`/`saturating_*`, and
  sizes read from input are capped. Crates that parse untrusted input enforce it with
  `#![deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)]`.

## Unsafe

- Denied by default. An exception is `#[expect(unsafe_code, reason = "…")]` on the smallest scope,
  with a `// SAFETY:` comment on every block (enforced), recorded in the project's `AGENTS.md`.
  Miri runs weekly on every project that contains `unsafe`.

## Logging and diagnostics

- Use `tracing` everywhere (not `log`, not `println!` for diagnostics). Binaries install a
  `tracing-subscriber` with `EnvFilter` (`RUST_LOG`).
- `println!`/`eprintln!` are denied. A CLI's actual output goes through one output module with
  `#[expect(clippy::print_stdout, reason = "CLI output")]`; diagnostics go to stderr through tracing.

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
- Every crash fix comes with a regression test that failed before the fix.
- Parsers and serializers get `proptest` round-trip tests; untrusted-input parsers get a `fuzz/` target.
- Shared crates keep **80 % line coverage** (CI gate); apps report coverage only.

## Dependencies

- Pick crates from [`stack.md`](stack.md) first. Adding a crate that isn't there for a concern
  `stack.md` already covers needs a reason in the app's `AGENTS.md`. Replaced crates (`chrono`,
  `openssl`, `native-tls`) are banned in `deny.toml`; `log` macros are banned in `clippy.toml`.
- Only permissive licences (`deny.toml`). Versions stay per project; no shared version table.
- Prefer few, well-maintained dependencies. Turn off default features you don't need.
- Shared crates in `crates/` keep dependencies minimal, because every app that uses them inherits them.

## Docs and comments

- Public items of shared crates get doc comments (`#![deny(missing_docs)]`); docs build with
  warnings as errors.
- Comments explain *why*, not *what*.
- Each project's `README.md` says what it is and how to run it. Its `AGENTS.md` holds what an agent
  needs to work on it.
