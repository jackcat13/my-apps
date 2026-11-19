# Workflow: quality gates

Every check the repo runs, what it catches, and how to make an exception. Decisions behind it:
[ADR 0005](../adr/0005-quality-gates.md).

## Where checks run

| When | Command | What |
|---|---|---|
| Every edit by Claude Code | `.claude/hooks/rustfmt-edited.sh` | `rustfmt` on the edited file |
| End of each Claude Code turn | `.claude/hooks/check-fast-on-stop.sh` | `check-fast`; failures go back to Claude |
| `git commit` | `.githooks/pre-commit` | `check-fast --staged` + `typos` on staged files |
| Before saying "done" | `just check <project>` | The full project gate (below) |
| CI, every PR / push | `.github/workflows/ci.yml` | `just check` per affected project, `just check-repo`, `just check-templates` |
| CI, daily | `.github/workflows/scheduled.yml` | `cargo-deny` on every project (new advisories) |
| CI, weekly | `.github/workflows/scheduled.yml` | miri, fuzzing, mutation testing |

First time on a machine: `scripts/setup.sh` (toolchain, pinned tools from `tools.txt`, git hooks).

## The project gate (`scripts/check.sh`)

Runs from inside the project folder, stops at the first failure.

| Step | Command | Catches |
|---|---|---|
| fmt | `cargo fmt --check` | Unformatted code |
| lints in sync | `scripts/sync-lints.sh --check` | A `[lints]` block edited by hand or stale |
| clippy | `cargo clippy --locked --all-targets --all-features -- -D warnings` | Everything in `lints.toml` |
| clippy (extra targets) | same, per `TARGETS` in `ci.env` | Code that breaks on wasm/embedded targets |
| each feature (crates only) | `cargo hack check --each-feature --no-dev-deps` | A feature that doesn't compile on its own |
| tests + coverage | `cargo llvm-cov nextest --profile ci` | Failing tests; crates under **80 % line coverage** (apps: report only) |
| doctests | `cargo test --doc` | Broken examples (nextest skips doctests) |
| docs | `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | Broken intra-doc links, bad doc markup |
| cargo-deny | `cargo deny check --config deny.toml` | Advisories, yanked crates, licences, banned crates, unknown sources |
| unused dependencies | `cargo machete` | Dependencies nobody uses |
| layering | `scripts/check-layering.sh` | `crates/*` → `apps/*`, or app → app dependencies |
| Cargo.lock unchanged | `git diff --exit-code Cargo.lock` | A lockfile that wasn't committed up to date |
| project ci.sh | `./ci.sh` if present | Whatever the project adds |

Every cargo call uses `--locked`: a stale `Cargo.lock` fails instead of being silently updated.

### Project knobs

A project can only **add** to the gate, never skip a step:

- `ci.env` (sourced shell): `FEATURE_FLAGS` (default `--all-features`), `TARGETS` (extra clippy
  targets), `APT_PACKAGES` (system packages CI installs first).
- `ci.sh` (executable): extra steps, run last.

Document both in the project's `AGENTS.md` ("Quality settings").

## Repo checks (`scripts/check-repo.sh`)

- `lints.toml` copies in sync, `rust-version` == pinned toolchain's major.minor, layering of all projects.
- `typos` (spelling), `taplo fmt --check` (TOML format).
- `ast-grep test` (rule tests) and `ast-grep scan` (custom rules in `lint/ast-grep/rules/`).
- `shellcheck` on every script and hook, `actionlint` and `zizmor` on the workflows.

`scripts/check-templates.sh` generates a project from each template into `target/template-check/`
and runs the full gate on it, so templates can't rot.

## Lints

`lints.toml` is the only place lint levels are set. `just sync-lints` copies it into every
`Cargo.toml` between the `BEGIN/END shared lints` markers. Main rules:

- `pedantic` on; warnings fail CI.
- No panics in shipped code: `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`,
  `unreachable`, `exit`, `mem_forget` are denied.
- No stray output: `dbg_macro`, `print_stdout`, `print_stderr` denied (diagnostics go through `tracing`).
- `cognitive_complexity` (threshold 30), `too_many_arguments`, `wildcard_imports` denied.
- `unsafe_code` denied; `undocumented_unsafe_blocks` denied.
- `allow_attributes` denied: suppressions are `#[expect]`, which fails once it's no longer needed.

`clippy.toml` relaxes panics/prints in tests and bans types and macros that `docs/stack.md` replaces
(`chrono`, `log::*`). A project `clippy.toml` *replaces* the root one: copy it first.

### Opt-in crate-root lints

Some lints are right for some crates only. Add them as crate-root attributes and list them in the
crate's `AGENTS.md`:

- `crates/*`: `#![deny(missing_docs)]` (the template adds it).
- A crate that parses untrusted input (files, network, user data) **must** add
  `#![deny(clippy::indexing_slicing, clippy::arithmetic_side_effects)]`: use `get()`,
  `checked_*`/`saturating_*`, and cap sizes read from the input.

### Custom rules (ast-grep)

For what clippy can't express. Each rule in `lint/ast-grep/rules/` has tests in
`lint/ast-grep/tests/`. Current rules:

- `no-string-error`: no `Result<_, String>`, `Box<dyn Error>`, `Err(format!(…))` outside tests.
- `no-anyhow-in-crates`: shared crates return typed errors.
- `no-blocking-sleep-in-async`: no `std::thread::sleep` in async code.

Adding a rule: write the rule and its test file (valid + invalid cases), run `just check-repo`.

## Making an exception

Always local, always explained, never by weakening shared config:

| Need | How |
|---|---|
| One lint at one place | `#[expect(clippy::lint_name, reason = "why this is fine here")]` |
| An `expect()` on a provable invariant | `#[expect(clippy::expect_used, reason = "…")]` + `.expect("why it can't fail")` |
| CLI output | One output module with `#[expect(clippy::print_stdout, reason = "CLI output")]` |
| `unsafe` | `#[expect(unsafe_code, reason = "…")]` + a `// SAFETY:` comment; miri then runs weekly |
| A security advisory that doesn't apply | `[advisories] ignore` in `deny.toml` with a reason and "revisit when …" |
| A banned crate pulled in transitively | `wrappers`/`skip` on its `[bans] deny` entry, with a reason |
| A new licence | Repo-wide decision: propose it, don't just add it |

Changing `lints.toml`, `clippy.toml`, `deny.toml`, `sgconfig.yml` or the coverage floor is a
repo-wide decision (root `AGENTS.md`, rule 2).

## Tests

- Unit tests next to the code, integration tests in `tests/`. Runner: `cargo-nextest`
  (`.config/nextest.toml`, `ci` profile: no fail-fast, 60 s slow-test timeout).
- Every crash fix comes with a regression test that failed before the fix.
- Code that parses or serializes input gets `proptest` round-trip tests.
- Fuzzing: add a `fuzz/` directory with `cargo fuzz init`; the weekly job runs every target for
  5 minutes and uploads crashes as artifacts. Turn a crash into a regression test.
- Mutation testing (`cargo-mutants`) runs weekly on `crates/*`, report only: read it to find tests
  that don't really check behavior.

## Tools

Pinned in `tools.txt`, installed by `scripts/setup.sh` locally and `taiki-e/install-action` in CI.
To bump a tool, change its version there; Dependabot keeps GitHub Actions current, and the toolchain
is bumped by hand (see `rust-toolchain.toml`).
