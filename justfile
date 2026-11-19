# Entry point for every check. CI calls the same recipes. Recipes run from the repo root.
# First time: scripts/setup.sh (installs just itself too).

# List recipes.
default:
    @just --list

# Install the toolchain, the pinned tools (tools.txt) and the git hooks.
setup:
    scripts/setup.sh

# Full gate for one project (what CI runs): just check apps/my-tool
check project:
    scripts/check.sh {{project}}

# Full gate for every project, then the repo-wide checks.
check-all:
    for p in $(FORMAT=lines ALL=1 scripts/changed-projects.sh); do scripts/check.sh "$p"; done
    scripts/check-repo.sh

# fmt + clippy on projects with uncommitted changes. Add --staged for staged changes only.
check-fast *flags:
    scripts/check-fast.sh {{flags}}

# Repo-wide checks: lint sync, versions, layering, typos, TOML, ast-grep, shell and workflow lint.
check-repo:
    scripts/check-repo.sh

# Generate a project from each template and run the full gate on it.
check-templates:
    scripts/check-templates.sh

# Copy lints.toml into every project's Cargo.toml.
sync-lints:
    scripts/sync-lints.sh

# Projects affected since a base commit (default: origin/main).
affected base="origin/main":
    BASE_SHA=$(git merge-base {{base}} HEAD) FORMAT=lines scripts/changed-projects.sh

# New project from a template: just new app my-tool "What it does"
new kind name description:
    scripts/new-project.sh {{kind}} {{name}} "{{description}}"
