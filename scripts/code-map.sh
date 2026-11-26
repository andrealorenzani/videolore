#!/usr/bin/env bash
# The module map, read from the sources: one line per Rust file of the three programs
# (tests left out unless --tests), with its line count and the first line of its `//!` doc.
#   scripts/code-map.sh                 every module
#   scripts/code-map.sh overlap         only the files whose path or doc line contains "overlap"
#   scripts/code-map.sh --tests [word]  the test files too
# Read-only.
set -euo pipefail
cd "$(dirname "$0")/.."
tests=0
if [[ "${1:-}" == "--tests" ]]; then tests=1; shift; fi
want="${1:-}"
find src player/src screen-spy/src -name '*.rs' | sort | while read -r f; do
  if [[ $tests -eq 0 && "$f" =~ (_tests\.rs|/tests\.rs)$ ]]; then continue; fi
  doc=$(grep -m1 '^//!' "$f" | sed 's#^//! \{0,1\}##' || true)
  line=$(printf '%-38s %5s  %s' "$f" "$(wc -l < "$f")" "$doc")
  if [[ -z "$want" ]] || grep -qi -- "$want" <<<"$line"; then
    printf '%s\n' "$line" | cut -c1-200
  fi
done
