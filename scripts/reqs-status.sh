#!/usr/bin/env bash
# List requirement ids with their status and file, without reading the documents.
# Usage: scripts/reqs-status.sh [status-prefix] [doc-file]
#   scripts/reqs-status.sh                    all requirements, "REQ-NNN status file:line"
#   scripts/reqs-status.sh planned            only the planned ones (any status starting with "planned")
#   scripts/reqs-status.sh implemented docs/business-text.md
#   scripts/reqs-status.sh --counts           count per status
# Read-only. Requirements are the lines "- REQ-NNN (status): text" in docs/business*.md.
set -uo pipefail
cd "$(dirname "$0")/.."
all=$(grep -nE '^[[:space:]]*[-*][[:space:]]*\**REQ-[0-9]+ \([a-z]' docs/business.md docs/business-*.md 2>/dev/null |
  sed -E 's/^([^:]+):([0-9]+):[[:space:]]*[-*][[:space:]]*\**(REQ-[0-9]+) \(([a-z][^)]*)\).*/\3 \4 \1:\2/')
if [ "${1:-}" = "--counts" ]; then
  printf '%s\n' "$all" | awk '{ s=$2; sub(/[ ;,].*/, "", s); c[s]++ } END { for (k in c) print c[k], k }' | sort -rn
  exit 0
fi
prefix=${1:-}
file=${2:-}
out=$all
[ -n "$prefix" ] && out=$(printf '%s\n' "$out" | awk -v p="$prefix" 'index($2, p) == 1')
[ -n "$file" ] && out=$(printf '%s\n' "$out" | grep -F "$file:")
printf '%s\n' "$out" | sed '/^$/d' | sort -t- -k2,2n -u
