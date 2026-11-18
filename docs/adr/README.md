# Architecture Decision Records

One file per repo-wide decision: `NNNN-short-title.md`, numbered in order, never renumbered.
A superseded ADR stays, with its status set to `Superseded by NNNN`.

## Template

```markdown
# NNNN — Title

- Status: Accepted | Superseded by NNNN
- Date: YYYY-MM-DD

## Context
What forced a decision.

## Decision
What we chose.

## Consequences
What gets easier, what gets harder, what we must keep doing.
```

## Index

- [0001 — Standalone projects, no Cargo workspace](0001-standalone-projects.md)
- [0002 — AGENTS.md as the only agent context file](0002-agents-md-only.md)
- [0003 — cargo-generate for scaffolding](0003-cargo-generate.md)
- [0004 — CI checks only affected projects](0004-ci-affected-projects.md)
