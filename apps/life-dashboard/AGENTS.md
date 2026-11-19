# life-dashboard

Context for agents working in `apps/life-dashboard/`. The shared rules in the root `AGENTS.md`
and `docs/` apply too; this file only adds what's specific to this app.

## Purpose

Personal finance/stats dashboard: a desktop app to track net worth and investments across
accounts. For the user only, run locally. Out of scope for now: mobile (Android), web (WASM).

## Kind and stack

- Kind: gui
- Stack guide: `docs/stacks/gui.md`
- Deviations from `docs/stack.md` and why: `eframe` is built without its default `accesskit`
  feature. `accesskit` pulls in `zbus`/`zvariant` for Linux AT-SPI, which `cargo deny` flags as
  unmaintained (RUSTSEC-2026-0370/0371/0372). This drops screen-reader/accessibility-tree export
  on Linux; re-enable once `accesskit` moves off the unmaintained chain.

## Architecture

Empty shell: `main.rs` sets up tracing and calls `life_dashboard::run()`, which opens a single
`eframe`/`egui` window (`App`) with a blank central panel. No data model or connectors yet (see
the follow-up GitHub issues for the rest of the plan).

## Commands

```bash
cargo run                        # from apps/life-dashboard/
just check apps/life-dashboard  # from the repo root: the full gate, what CI runs
```

## Quality settings

- `ci.env`: none (defaults: `--all-features`, no extra targets, no system packages)
- `ci.sh`: none
- Opt-in crate-root lints (`indexing_slicing`, `arithmetic_side_effects`): none. Required if this
  app parses untrusted input (docs/workflows/quality.md).

## Notes and pitfalls

<!-- Things learned while working on this app. If one turns out to be general, propose moving it to docs/. -->
