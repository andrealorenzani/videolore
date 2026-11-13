#!/usr/bin/env bash
# Feature coverage must be 100%: every implemented requirement has at least one test.
# Requirements: lines in docs/business.md like "- REQ-001 (implemented): ...".
# Tests declare what they cover with a comment "// covers: REQ-001" in src/ or tests/.
set -euo pipefail
cd "$(dirname "$0")/.."
mapfile -t reqs < <(grep -oE 'REQ-[0-9]+ \(implemented\)' docs/business.md | cut -d' ' -f1 | sort -u || true)
missing=()
for r in "${reqs[@]}"; do
  grep -rqE "covers:.*\b$r\b" src tests 2>/dev/null || missing+=("$r")
done
total=${#reqs[@]}; ok=$((total - ${#missing[@]}))
echo "feature coverage: $ok/$total implemented requirements have tests"
if [[ ${#missing[@]} -gt 0 ]]; then echo "missing tests for: ${missing[*]}" >&2; exit 1; fi
