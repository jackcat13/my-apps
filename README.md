# my-apps

Monorepo for all my Rust applications. Every app in `apps/` and every shared library in `crates/` is a
standalone Cargo project; the root holds shared docs, tooling config, templates and CI.

## Requirements

- Rust via `rustup` (the toolchain is pinned by `rust-toolchain.toml`)
- [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall) and Homebrew. Then run
  `scripts/setup.sh` once: it installs the toolchain, every pinned tool from `tools.txt` (`just`,
  `cargo-generate`, `cargo-deny`, `cargo-nextest`, …) and the git pre-commit hook.
- For AI agents: Claude Code ≥ 2.1.277 (reads `AGENTS.md` natively) or any agent that reads `AGENTS.md`

## Quick start

```bash
just new app my-tool "What it does"     # new application
just new crate my-lib "What it does"    # new shared crate
just check apps/my-tool                  # the full quality gate, same as CI
```

Then follow the remaining steps in [docs/workflows/new-app.md](docs/workflows/new-app.md) (registry, AGENTS.md).

## Where things are

- [AGENTS.md](AGENTS.md): rules, layout, project registry
- [docs/](docs/): conventions, default stack, workflows, decision records
- [docs/workflows/quality.md](docs/workflows/quality.md): every quality gate and how to make an exception
- [.github/workflows/ci.yml](.github/workflows/ci.yml): CI checks only the projects affected by a change
