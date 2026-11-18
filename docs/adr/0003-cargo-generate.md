# 0003 — cargo-generate for scaffolding

- Status: Accepted
- Date: 2026-11-18

## Context
Every project must start with the same lint block, `AGENTS.md` and `README.md` skeleton, because
there's no workspace to inherit from.

## Decision
Scaffold with `cargo-generate` from local templates in `templates/` (`app`, `crate`, and later
kind-specific ones). The steps that need judgment (registry row, filling the app's `AGENTS.md`)
stay in `docs/workflows/new-app.md`, done by the agent or by hand, not by template hooks.

## Consequences
- Requires `cargo-generate` installed locally.
- Templates must be kept in sync with `docs/conventions.md` (e.g. the `[lints]` block).
