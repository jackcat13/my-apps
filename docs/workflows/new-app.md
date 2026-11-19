# Workflow: create a new app or shared crate

Always follow these steps. Never create a project with `cargo new` or by hand.

## 1. Pick a name and kind

- `kebab-case`, describes what it does. Check the root `AGENTS.md` registry for collisions.
- Kind: `cli`, `web`, `gui`, `wasm`, `embedded`, `game`, `lib`, …
- If it's the **first app of this kind**, follow `docs/stacks/README.md` too.
- Shared crate? Only when a second app needs the same code (rule of two).

## 2. Generate

From the repo root:

```bash
just new app <name> "<one-line purpose>"     # application in apps/
just new crate <name> "<one-line purpose>"   # shared library crate in crates/
```

This runs `cargo generate` on `templates/app` or `templates/crate`, syncs `lints.toml` into the new
`Cargo.toml`, and creates `Cargo.lock`. For a kind-specific template (`templates/<kind>/`), set
`TEMPLATE=<kind>` in front of the app command.

The templates already follow every rule: thin `main.rs` + `lib.rs` with `tracing` and `anyhow`
(apps), a `thiserror` error type and `#![deny(missing_docs)]` (crates), `AGENTS.md`, `README.md`.

## 3. Fill in the context

- In `<path>/AGENTS.md`, fill the *Purpose*, *Stack*, *Quality settings* and *Notes* sections.
  Delete placeholder lines you have no content for.
- Add dependencies from inside the project folder with `cargo add <crate>`, preferring
  `docs/stack.md`.
- Parses untrusted input? Add the opt-in lints and a fuzz target (`docs/workflows/quality.md`).
- Needs extra targets, features or system packages in CI? Add a `ci.env`.

## 4. Register

Add a row to the **Registry** table in the root `AGENTS.md`:
`| <name> | apps/<name> | <kind> | <one-line purpose> |` (remove the `_none yet_` row if present).

## 5. Verify

```bash
just check apps/<name>    # or crates/<name>
```

## Extracting a shared crate (rule of two)

1. Generate the crate (step 2).
2. Move the code, give it a typed error (`thiserror`) and doc comments. Reach 80 % coverage.
3. In each app: `<crate> = { path = "../../crates/<crate>" }`, remove the duplicated code.
4. Register the crate. Mention it in each consuming app's `AGENTS.md`.
5. CI rechecks every app that depends on the crate automatically.
