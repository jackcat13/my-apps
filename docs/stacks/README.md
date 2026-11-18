# Stack guides

One file per kind of app (`cli.md`, `web.md`, `gui.md`, `wasm.md`, `embedded.md`, `game.md`, …).

These guides are **written when the first app of that kind is created**, from decisions actually
made, not in advance. When you start the first app of a new kind:

1. Decide the stack with the user and record it in `docs/stack.md`.
2. Create `docs/stacks/<kind>.md` using the outline below.
3. If the kind needs special CI (targets, system libraries), note the `ci.sh` pattern in the guide.
4. If it's worth it, add a dedicated template `templates/<kind>/` and mention it in
   `docs/workflows/new-app.md`.

## Outline for a guide

```markdown
# <Kind> apps

## Stack
Crates used and the role of each.

## Project layout
Folder/module structure specific to this kind.

## Patterns
How we do the recurring things (config, state, errors at the boundary, …).

## Testing
What and how to test for this kind.

## CI and tooling
Extra targets, toolchain overrides, `ci.sh` template.

## Pitfalls
Lessons learned. Keep adding to this section.
```
