#!/usr/bin/env bash
# Install actionlint in CI (Linux x86_64) at the version pinned in tools.txt.
# taiki-e/install-action has no manifest for actionlint, and its cargo-binstall
# fallback can't install it (actionlint is a Go binary, not a crate).
# Downloads the release tarball and checks it against the release's checksums file.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

version=$(awk '$1 == "actionlint" { print $2 }' tools.txt)
[ -n "$version" ] || { echo "actionlint missing from tools.txt" >&2; exit 1; }

base="https://github.com/rhysd/actionlint/releases/download/v$version"
tarball="actionlint_${version}_linux_amd64.tar.gz"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

curl -fsSL --retry 3 -o "$tmp/$tarball" "$base/$tarball"
curl -fsSL --retry 3 -o "$tmp/checksums.txt" "$base/actionlint_${version}_checksums.txt"
(cd "$tmp" && grep " $tarball\$" checksums.txt | sha256sum -c -)

bin_dir="$HOME/.local/bin"
mkdir -p "$bin_dir"
tar -xzf "$tmp/$tarball" -C "$bin_dir" actionlint
if [ -n "${GITHUB_PATH:-}" ]; then echo "$bin_dir" >> "$GITHUB_PATH"; fi
"$bin_dir/actionlint" -version
