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
- `sqlx` is built without any TLS backend feature (`sqlite` + `runtime-tokio` only): SQLite is a
  local file, so there's no connection to secure, and pulling in `tls-rustls` drags in
  `webpki-roots` (CDLA-Permissive-2.0), which fails the licence gate for no benefit here.

## Architecture

`main.rs` sets up tracing and calls `life_dashboard::run()`, which resolves the per-OS data
directory (`etcetera`), starts a background `tokio` runtime, and opens a single `eframe`/`egui`
window (`App`). `App` opens the SQLite database (`db::connect`, migrations in `migrations/`) on
that runtime via a `poll_promise::Promise` and, once ready, hands off to the Settings screen
(`settings::SettingsScreen`): Connections (CRUD against `connections`) and Preferences (display
currency + theme, persisted in the singleton `preferences` row). No connectors or account/position
data yet (see the follow-up GitHub issues for the rest of the plan).

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
