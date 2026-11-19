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
- `reqwest` is built with `rustls-tls-native-roots` (not the default `rustls-tls`/`rustls-tls-webpki-roots`):
  the webpki-roots-based rustls features pull in `webpki-roots` (or, via `rustls-platform-verifier`,
  `webpki-root-certs`), both `CDLA-Permissive-2.0`, which fails the licence gate. The native-roots
  variant uses the OS certificate store (`rustls-native-certs`) instead — fine here since every
  connector talks to either `localhost` (IBKR's gateway) or a real institution's public API.
- `wiremock` (dev-dependency only): not in `docs/stack.md`, added so connector tests exercise the
  real HTTP client against a mock server instead of hand-rolling one — see "Tests" below.

## Architecture

`main.rs` sets up tracing and calls `life_dashboard::run()`, which resolves the per-OS data
directory (`etcetera`), starts a background `tokio` runtime, and opens a single `eframe`/`egui`
window (`App`). `App` opens the SQLite database (`db::connect`, migrations in `migrations/`) on
that runtime via a `poll_promise::Promise` and, once ready, hands off to a `Shell` with two
top-level screens:

- **Dashboard** (`dashboard::DashboardScreen`): read-only, loaded straight from the local database
  (never the network) — each account's latest snapshot, current positions and a few recent
  transactions. Works fully offline; shows whatever was persisted by the last successful refresh.
- **Settings** (`settings::SettingsScreen`): Connections (CRUD against `connections`, plus a
  per-connection "Refresh" button) and Preferences (display currency + theme, persisted in the
  singleton `preferences` row).

Refreshing a connection (`settings::SettingsScreen::spawn_refresh`) runs `sync::refresh_connection`
on the background runtime: it picks the `connectors::Connector` matching the connection's
`connector_type`, fetches, and — only once the fetch has fully succeeded — persists the result as
accounts (`db::accounts`, matched/created by the connector's external id), positions
(`db::positions`, replaced wholesale each refresh — they're a point-in-time snapshot, not history),
a new snapshot (`db::snapshots`, append-only total value) and any new transactions
(`db::transactions`, append-only, de-duplicated by timestamp/kind/amount/asset). A failed fetch
returns before touching any of that, so previously persisted data is untouched; the UI shows the
error as a dismissible banner next to the connection, not a crash.

`connectors::interactive_brokers::InteractiveBrokersConnector` is the first (and so far only)
connector: it talks to the user's locally running IBKR Client Portal Gateway over plain HTTPS
(`GET /portfolio/accounts`, `/portfolio/{id}/positions/0`, `/iserver/account/trades`). The gateway
handles authentication itself via the user's browser session, so the connection's `credentials`
field just holds the gateway's base URL (e.g. `https://localhost:5000/v1/api`), not a secret. The
aggregator-tier and CSV connectors planned next implement the same `connectors::Connector` trait.

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

- A test helper that does `tempfile::tempdir()` and returns only the `SqlitePool` drops the
  `TempDir` guard at the end of the helper, deleting the database file (and its directory) while
  the pool is still open — writes then fail with "attempt to write a readonly database" even
  though reads keep working (the open fd survives unlinking on Linux; SQLite's journal/WAL
  creation needs the directory, which is gone). Return the `TempDir` alongside the pool and bind
  it (`let (_dir, pool, ..) = helper().await;`) so it outlives every use.
