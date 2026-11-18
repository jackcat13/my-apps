# 0002 — AGENTS.md as the only agent context file

- Status: Accepted
- Date: 2026-11-18

## Context
Context for AI agents should be readable by any tool, not tied to one. Claude Code reads `AGENTS.md`
natively since v2.1.277, but only when no `CLAUDE.md` exists in the directory or any ancestor.

## Decision
Use `AGENTS.md` files only: the root one for shared context, one per project for local context.
No `CLAUDE.md` anywhere. Claude-only additions go in `.claude/rules/` or `.claude/skills/`, which
don't disable `AGENTS.md`.

## Consequences
- One source of truth, readable by Codex, Cursor, Claude Code and others.
- Requires Claude Code ≥ 2.1.277.
- A stray `CLAUDE.md` silently disables all context, which is why the root `AGENTS.md` forbids it.
