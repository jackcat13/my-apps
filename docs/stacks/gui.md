# GUI (desktop) apps

## Stack

- `eframe` + `egui` — immediate-mode GUI, pure Rust, no platform webview. Cross-platform on
  Linux, macOS, Windows out of the box.
- `tokio` — background async runtime for I/O (connectors, network calls, file parsing) that
  must not block the UI thread. Only add once a feature actually needs it.
- `poll-promise` — bridges a `tokio` task into the egui `update` loop: spawn the future, store
  the `Promise` in app state, and poll it each frame instead of blocking.
- `directories` — resolves per-OS config/data directories (`ProjectDirs::from(...)`) instead of
  hand-rolling `$HOME`/`%APPDATA%` logic.

Android and WebAssembly targets are explicitly out of scope: WASM would need a backend (CORS, no
local subprocess, no native SQLite) and Android needs its own NDK/signing toolchain.

## Project layout

- `src/main.rs` — thin: sets up `tracing`, calls `life_dashboard::run()`.
- `src/lib.rs` — the `eframe::App` implementation and `run()`, which builds `NativeOptions` and
  calls `eframe::run_native`.
- As the app grows, split UI panels and state into their own modules (e.g. `src/ui/`,
  `src/state.rs`); keep `lib.rs` to wiring `run()` together.

## Patterns

- **Background work**: never block `update()`. Spawn long-running work (file I/O, network
  requests, DB queries) on a `tokio` runtime owned by the app, wrap the result in a
  `poll_promise::Promise`, and poll it in `update()`; call `ctx.request_repaint()` after
  spawning so egui keeps redrawing until the promise resolves.
- **Config/data dirs**: resolve paths once at startup via
  `directories::ProjectDirs::from("<qualifier>", "<org>", "life-dashboard")`, store the resolved
  `ProjectDirs` in app state, and pass concrete paths down rather than re-resolving per call.
- **State**: keep `App` as plain data (`Default`-constructible where possible); avoid hidden
  global state so the UI stays easy to test by constructing `App` directly.

## Testing

- Pure logic (data transforms, formatting, parsing) goes in plain functions with unit tests,
  kept separate from the `eframe::App` type so it doesn't need a display to test.
- The `eframe::App` itself is not unit-tested directly (no headless display in CI); verify it by
  running the app locally (`cargo run`).

## CI and tooling

- No extra `ci.env` needed for a pure-Rust `eframe` build on Linux/macOS/Windows native targets.
- If a future egui feature needs system libraries (e.g. `libgtk` for native file dialogs on
  Linux), add them via the app's `ci.env`/`ci.sh` rather than changing the shared CI defaults.

## Pitfalls

- Adding `tokio`/`poll-promise`/`directories` to `Cargo.toml` before any code uses them fails
  `cargo machete` (unused dependencies). Add each dependency only when the feature that needs it
  lands.
