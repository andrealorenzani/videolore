#!/usr/bin/env bash
# Where requirements live in the code: the source files that name each REQ-NNN outside the
# tests (the implementation), and every test that covers it (`// covers:`), as
# file:line test_name.
#   scripts/req-code.sh REQ-325 [REQ-331 ...]
# Read-only. The requirement text itself: scripts/req.sh.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ $# -ge 1 ]] || { echo "usage: $0 REQ-NNN [REQ-NNN ...]" >&2; exit 2; }
for req in "$@"; do
  [[ "$req" =~ ^REQ-[0-9]{3}$ ]] || { echo "not a requirement id: $req" >&2; exit 2; }
  n=${req#REQ-}
  echo "== $req"
  echo "-- named in the code (not tests):"
  grep -rlE "REQ-$n\b" src player/src 2>/dev/null | grep -vE '(_tests|/tests)\.rs$' | sort | sed 's/^/  /' || true
  echo "-- tests (// covers:):"
  # a covers line names REQ-325 or 325 in a list ("covers: REQ-325, 327" or "REQ-320..322" is not expanded)
  grep -rnE "// covers:.*(REQ-$n\b|[ ,]$n\b)" src tests player 2>/dev/null | while IFS=: read -r file line _; do
    name=$(awk -v s="$line" 'NR>s && /fn [a-z0-9_]+/ {match($0, /fn [a-z0-9_]+/); print substr($0, RSTART+3, RLENGTH-3); exit}' "$file")
    echo "  $file:$line $name"
  done
done
