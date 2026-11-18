# my-apps

Monorepo for all my Rust applications. Every app in `apps/` and every shared library in `crates/` is a
standalone Cargo project; the root holds shared docs, tooling config, templates and CI.

## Requirements

- Rust via `rustup` (the toolchain is selected by `rust-toolchain.toml`)
- [`cargo-generate`](https://github.com/cargo-generate/cargo-generate): `brew install cargo-generate` or `cargo install cargo-generate`
- For AI agents: Claude Code ≥ 2.1.277 (reads `AGENTS.md` natively) or any agent that reads `AGENTS.md`

## Quick start

```bash
# new application
cargo generate --path templates/app --destination apps --name my-tool

# new shared crate
cargo generate --path templates/crate --destination crates --name my-lib
```

Then follow the remaining steps in [docs/workflows/new-app.md](docs/workflows/new-app.md) (registry, AGENTS.md).

## Where things are

- [AGENTS.md](AGENTS.md): rules, layout, project registry
- [docs/](docs/): conventions, default stack, workflows, decision records
- [.github/workflows/ci.yml](.github/workflows/ci.yml): CI checks only the projects affected by a change
