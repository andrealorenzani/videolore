#!/usr/bin/env bash
# Print one section of a document: from the heading that matches, up to the next heading of the same or a higher level.
# Usage: scripts/doc-section.sh <file> <heading text, case-insensitive substring>
#   scripts/doc-section.sh docs/code-hardware.md "Hardware check"
# Prints "file: L<first>-<last>" on stderr. Read-only. Fails when no heading or several match (use a longer text).
set -uo pipefail
cd "$(dirname "$0")/.."
[ $# -eq 2 ] || { echo "usage: $0 <file> <heading>" >&2; exit 2; }
file=$1; want=$2
[ -f "$file" ] || { echo "no such file: $file" >&2; exit 2; }
mapfile -t hits < <(grep -niE '^#{1,6} ' "$file" | grep -iF -- "$want")
[ "${#hits[@]}" -eq 0 ] && { echo "no heading matches '$want' in $file" >&2; exit 1; }
[ "${#hits[@]}" -gt 1 ] && { printf 'several headings match, use a longer text:\n%s\n' "${hits[*]}" >&2; exit 1; }
start=${hits[0]%%:*}
level=$(sed -n "${start}p" "$file" | sed -E 's/^(#+).*/\1/' | tr -d '\n' | wc -c)
total=$(wc -l < "$file")
end=$total
if [ "$start" -lt "$total" ]; then
  next=$(awk -v s="$start" -v lv="$level" 'NR > s && /^#+ / { n = match($0, /[^#]/) - 1; if (n <= lv) { print NR - 1; exit } }' "$file")
  [ -n "$next" ] && end=$next
fi
echo "$file: L$start-$end" >&2
sed -n "${start},${end}p" "$file"
