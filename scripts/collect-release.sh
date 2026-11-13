#!/usr/bin/env bash
# Snapshot every document into release/<version>/ (README.md, docs/, ai-docs/).
# The snapshot is immutable afterwards (see scripts/release-guard.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: collect-release.sh X.Y.Z}"
dest="release/$v"
[[ -e "$dest" ]] && { echo "$dest already exists: release snapshots are never overwritten" >&2; exit 1; }
mkdir -p "$dest"
cp README.md "$dest/"
cp -r docs "$dest/docs"
cp -r ai-docs "$dest/ai-docs"
echo "collected into $dest:"; find "$dest" -type f | sort
