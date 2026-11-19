# Shared helpers, sourced by the other scripts.

repo_root() { git rev-parse --show-toplevel; }

# Every project folder (apps/<name>, crates/<name>), one per line.
all_projects() {
  local d
  for d in apps/*/ crates/*/; do
    if [ -f "${d}Cargo.toml" ]; then echo "${d%/}"; fi
  done | sort -u
}

# Repo-relative folders of a project's path dependencies, from `cargo metadata`.
# Arg: project folder relative to the repo root. Must run from the repo root.
path_deps() {
  local root
  root=$(pwd -P)
  cargo metadata --format-version 1 --no-deps --offline --manifest-path "$1/Cargo.toml" 2>/dev/null \
    | jq -r '.packages[].dependencies[] | select(.path != null) | .path' \
    | while read -r abs; do
        abs=$(cd "$abs" 2>/dev/null && pwd -P) || continue
        echo "${abs#"$root"/}"
      done | sort -u
}
