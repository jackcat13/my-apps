# 0004 — CI checks only affected projects

- Status: Accepted
- Date: 2026-11-18

## Context
With many standalone projects, checking everything on each push is slow. Checking only the changed
folders, though, would miss apps that depend on a changed shared crate.

## Decision
A script computes the affected projects: changed project folders, plus every project depending
(transitively) on a changed crate through a path dependency, or all projects when shared config
changes. A GitHub Actions matrix runs fmt/clippy/test per project, or the project's own `ci.sh`
when it has one.

## Consequences
- Fast CI that still catches breakage spreading from shared crates.
- Dependency detection relies on path dependencies, read from `cargo metadata`.
- Unusual projects add to the standard gate through `ci.env`/`ci.sh`; they can't replace it (0005).
