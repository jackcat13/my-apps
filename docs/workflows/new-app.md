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
# application
cargo generate --path templates/app --destination apps --name <name> \
  --define description="<one-line purpose>"

# shared library crate
cargo generate --path templates/crate --destination crates --name <name> \
  --define description="<one-line purpose>"
```

If a kind-specific template exists (`templates/<kind>/`), use it instead of `templates/app`.

The templates already contain the standard `[lints]` block, `AGENTS.md`, and `README.md`. They don't
create a nested git repo.

## 3. Fill in the context

- In `<path>/AGENTS.md`, fill the *Purpose*, *Stack* and *Notes* sections. Delete placeholder
  lines you have no content for.
- Add the dependencies the app needs, preferring `docs/stack.md` (`cargo add <crate>` from inside
  the project folder).

## 4. Register

Add a row to the **Registry** table in the root `AGENTS.md`:
`| <name> | apps/<name> | <kind> | <one-line purpose> |` (remove the `_none yet_` row if present).

## 5. Verify

From inside the project folder:

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

If the project needs non-default CI (extra targets, system libraries, nightly), add an executable
`ci.sh` in the project folder (see `docs/workflows/ci.md`).

## Extracting a shared crate (rule of two)

1. Generate the crate in `crates/` (step 2).
2. Move the code, give it a typed error (`thiserror`) and doc comments.
3. In each app: `<crate> = { path = "../../crates/<crate>" }`, remove the duplicated code.
4. Register the crate. Mention it in each consuming app's `AGENTS.md`.
5. CI rechecks every app that depends on the crate automatically.
