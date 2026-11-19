# Default stack

The default crate for each concern. No versions are pinned here: use the latest release when adding
a dependency (`cargo add <crate>`). Deviating from this list is allowed when an app's `AGENTS.md`
records why.

Change this file deliberately (see the "File knowledge at the right level" rule in the root
`AGENTS.md`) and add an ADR in `docs/adr/` if the change is significant.

Choices that replace a well-known alternative are enforced: `deny.toml` bans the replaced crates
and `clippy.toml` bans their types and macros (see `workflows/quality.md`).

## Cross-cutting

| Concern | Choice | Notes |
|---|---|---|
| Errors (libraries) | `thiserror` | Typed error enums |
| Errors (binaries) | `anyhow` | With `.context()` |
| Logging / tracing | `tracing` + `tracing-subscriber` | `EnvFilter` from `RUST_LOG` |
| Serialization | `serde` (+ `serde_json`, `toml`) | |
| Async runtime | `tokio` | Only when async is actually needed |
| HTTP client | `reqwest` | `rustls` TLS, no OpenSSL |
| Date/time | `jiff` | |
| Config files | `serde` + `toml` | Add `figment` only if layered config is needed |
| Unit testing extras | `pretty_assertions`, `rstest` | Add as needed |
| Snapshot testing | `insta` | |
| Property testing | `proptest` | |

## By kind of app

Details and patterns live in `docs/stacks/<kind>.md` once the first app of that kind exists.

| Kind | Starting choices |
|---|---|
| CLI | `clap` (derive) |
| Web backend | `axum` on `tokio`, `tower-http` |
| Database | `sqlx` |
| GUI (desktop) | `eframe`/`egui`, `tokio` + `poll-promise` for async off the UI thread, `directories` for config/data dirs |
| WASM / frontend | to decide with the first wasm app |
| Embedded | to decide with the first embedded app |
| Game | to decide with the first game |

A row that says "to decide" gets filled in (and a `docs/stacks/<kind>.md` written) when the first
app of that kind is created.
