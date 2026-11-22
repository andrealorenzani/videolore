#!/usr/bin/env bash
# Print one requirement: its status, its file and line, its full paragraph and the tests that cover it.
# Usage: scripts/req.sh REQ-177 [REQ-075 ...]
# Read-only. Requirements are the lines "- REQ-NNN (status): text" in docs/business*.md.
set -uo pipefail
cd "$(dirname "$0")/.."
[ $# -ge 1 ] || { echo "usage: $0 REQ-NNN..." >&2; exit 2; }
status=0
for id in "$@"; do
  [[ $id =~ ^REQ-[0-9]+$ ]] || { echo "bad id: $id" >&2; status=2; continue; }
  hit=$(grep -nE "^[[:space:]]*[-*][[:space:]]*\**${id}\b" docs/business.md docs/business-*.md 2>/dev/null | head -n 1)
  if [ -z "$hit" ]; then echo "$id: not found in docs/business*.md" >&2; status=1; continue; fi
  file=${hit%%:*}; rest=${hit#*:}; line=${rest%%:*}; text=${rest#*:}
  st=$(printf '%s' "$text" | grep -oE "^[[:space:]]*[-*][[:space:]]*\**${id} \([^)]*\)" | sed -E 's/.*\((.*)\)/\1/')
  echo "$id — ${st:-no status} — $file:$line"
  printf '%s\n' "$text" | sed -E 's/^[[:space:]]*[-*][[:space:]]*//'
  tests=$(grep -rlE "covers:.*\b${id}\b" src tests player 2>/dev/null | sort -u | tr '\n' ' ')
  echo "tests: ${tests:-none}"
  echo
done
exit $status
