#!/usr/bin/env bash
# Snapshot a release into "releases/<version> - <name>/":
#   docs/              README.md, docs/ and ai-docs/ copied unchanged
#   sources.zip        every tracked and new (not ignored) file of the working tree, releases/ excluded
#   RELEASE_NOTES.md   the release notes, taken from docs/changelog.md
# The snapshot is immutable afterwards (see scripts/release-guard.sh).
set -euo pipefail
cd "$(dirname "$0")/.."
v="${1:?usage: collect-release.sh X.Y.Z \"Release name\"}"
name="${2:?usage: collect-release.sh X.Y.Z \"Release name\"}"
[[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "version must be X.Y.Z, got '$v'" >&2; exit 1; }
[[ "$name" =~ ^[[:alnum:]][[:alnum:][:space:]._+-]{0,59}$ ]] \
  || { echo "release name must be 1-60 letters, digits, spaces or . _ + - (got '$name')" >&2; exit 1; }
for existing in releases/"$v - "*; do
  [[ -e "$existing" ]] && { echo "$existing already exists: release snapshots are never overwritten" >&2; exit 1; }
done
dest="releases/$v - $name"
mkdir -p "$dest/docs"
cp README.md "$dest/docs/"
cp -r docs "$dest/docs/docs"
cp -r ai-docs "$dest/docs/ai-docs"
# Sources: tracked + untracked-but-not-ignored files that exist, without the releases/ snapshots.
git ls-files --cached --others --exclude-standard \
  | grep -v '^releases/' \
  | while IFS= read -r f; do [[ -f "$f" ]] && printf '%s\n' "$f"; done \
  | zip -q -X "$dest/sources.zip" -@
{
  echo "# $(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1) $v — $name"
  echo
  echo "Released on $(date '+%Y-%m-%d'). Changes since the previous release, newest first."
  echo
  # Only the version entries: the changelog title and introduction are left out.
  body=$(awk 'found || /^## /{found=1; print}' docs/changelog.md)
  printf '%s\n' "${body:-No changes.}"
} > "$dest/RELEASE_NOTES.md"
echo "collected into $dest:"
ls -la "$dest"
