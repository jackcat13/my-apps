---
name: new-app
description: Create a new application in apps/ or a shared crate in crates/ of this monorepo using the cargo-generate templates. Use when the user wants to start a new app, project, tool, service, or extract a shared crate.
---

Follow `docs/workflows/new-app.md` exactly. It's the single source of truth; this skill only
points to it so the workflow stays readable by every agent.

Before generating, confirm with the user: the name, the kind, and the one-line purpose.
If this is the first app of its kind, also follow `docs/stacks/README.md` and agree on the stack
with the user before writing code.

Finish with the verify step (`just check <project>` from the repo root) and the registry row in
the root `AGENTS.md`.
