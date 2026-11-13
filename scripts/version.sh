#!/usr/bin/env bash
# Single place to read, compute and change the project version everywhere.
#   scripts/version.sh get            current version (Cargo.toml)
#   scripts/version.sh next-patch     current with last number +1   (0.1.2 -> 0.1.3)
#   scripts/version.sh next-major     next major release            (0.1.2 -> 1.0.0)
#   scripts/version.sh set X.Y.Z      write X.Y.Z everywhere, then check
#   scripts/version.sh check          fail if any location disagrees
# Locations: Cargo.toml [package] version, Cargo.lock (own package), README.md version badge.
set -euo pipefail
cd "$(dirname "$0")/.."

name=$(sed -n 's/^name = "\(.*\)"/\1/p' Cargo.toml | head -1)
get() { sed -n '/^\[package\]/,/^\[/ s/^version = "\(.*\)"/\1/p' Cargo.toml | head -1; }
lock_version() { awk -v n="$name" '$0=="name = \""n"\"" {getline; gsub(/version = |"/,""); print; exit}' Cargo.lock; }
readme_version() { sed -n 's/.*badge\/version-\([0-9][0-9.]*\)-.*/\1/p' README.md | head -1; }

check() {
  local c l r; c=$(get); l=$(lock_version); r=$(readme_version)
  if [[ "$c" == "$l" && "$c" == "$r" ]]; then echo "version $c consistent"; return 0; fi
  echo "version mismatch: Cargo.toml=$c Cargo.lock=$l README badge=$r" >&2; return 1
}

case "${1:-}" in
  get) get ;;
  next-patch) IFS=. read -r a b c <<<"$(get)"; echo "$a.$b.$((c + 1))" ;;
  next-major) IFS=. read -r a _ _ <<<"$(get)"; echo "$((a + 1)).0.0" ;;
  set)
    v="${2:?usage: set X.Y.Z}"
    [[ "$v" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "invalid version: $v" >&2; exit 1; }
    sed -i "/^\[package\]/,/^\[/ s/^version = \".*\"/version = \"$v\"/" Cargo.toml
    cargo check -q --offline 2>/dev/null || cargo check -q
    sed -i "s/badge\/version-[0-9][0-9.]*-/badge\/version-$v-/" README.md
    check ;;
  check) check ;;
  *) sed -n '2,8p' "$0"; exit 1 ;;
esac
