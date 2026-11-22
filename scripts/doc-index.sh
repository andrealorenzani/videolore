#!/usr/bin/env bash
# Index of the headings of human documents, in the format of the ai-docs indexes:
#   - docs/file.md § Heading — L<first>-<last> (<n> lines)
# Usage: scripts/doc-index.sh [file ...]     (default: every docs/*.md except docs/changelog.md)
# Read-only. Use it to find a line range, then read only that range (sed -n 'A,Bp').
set -uo pipefail
cd "$(dirname "$0")/.."
files=("$@")
[ "${#files[@]}" -eq 0 ] && mapfile -t files < <(ls docs/*.md | grep -v '/changelog.md$')
for f in "${files[@]}"; do
  [ -f "$f" ] || { echo "no such file: $f" >&2; continue; }
  total=$(wc -l < "$f")
  grep -nE '^#{1,3} ' "$f" | awk -F: -v f="$f" -v total="$total" '
    { line[NR] = $1; head[NR] = substr($0, index($0, ":") + 1) }
    END {
      for (i = 1; i <= NR; i++) {
        end = (i < NR) ? line[i + 1] - 1 : total
        t = head[i]; sub(/^#+ */, "", t)
        printf "- %s § %s — L%d-%d (%d lines)\n", f, t, line[i], end, end - line[i] + 1
      }
    }'
done
