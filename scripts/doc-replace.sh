#!/usr/bin/env bash
# Replace text in the docs WITHOUT reading it first: an agent that knows the new text never has to read the old one.
#   scripts/doc-replace.sh req REQ-NNN <new line|->            replace the definition line of a requirement (docs/business*.md)
#   scripts/doc-replace.sh adr ADR-NNN <new line|-> [FILE]     replace the definition line of an ADR (ai-docs/architecture-adr*.md, or FILE,
#                                                              e.g. docs/architecture-decisions.md, where it is "- **...(ADR-NNN...")
#   scripts/doc-replace.sh line FILE <ID> <new line|->         same for any id (REQ-/ADR-/QG-...) that starts a line of FILE
#   scripts/doc-replace.sh text [--all] <old> <new> FILE...    literal (no regex) replacement inside the files: library versions, the README
#                                                              badge, a number, a word. Without --all it must match exactly once in total.
#   scripts/doc-replace.sh show-version-hint                   how to change the project version (it is scripts/version.sh set)
# "-" as new line reads it from stdin (it may have several lines). The new line is written exactly as given: include the
# "- REQ-NNN (status): ..." prefix. Prints only "file:Lfirst-Llast replaced, +N lines": never the old text.
# When the line count changes (+N != 0) the line ranges of the owner's ai-docs index move: run scripts/doc-index.sh for that file.
# Fails (nothing changed) when the id is not found, found twice, or the file is under releases/. The project version itself: scripts/version.sh set.
set -uo pipefail
cd "$(dirname "$0")/.."
die() { echo "doc-replace: $*" >&2; exit 1; }
usage() { sed -n '2,13p' "$0" >&2; exit 2; }
guard() { case "$1" in releases/*|./releases/*) die "never modify files under releases/";; esac; [ -f "$1" ] || die "no such file: $1"; }
newtext() { if [ "$1" = "-" ]; then cat; else printf '%s\n' "$1"; fi; }

# replace_line FILE ANCHOR_REGEX(ERE, matched at line start) NEWFILE
replace_line() {
  local file=$1 re=$2 new=$3 hits n first
  guard "$file"
  mapfile -t hits < <(grep -nE -- "$re" "$file" | cut -d: -f1)
  [ "${#hits[@]}" -eq 0 ] && die "definition not found in $file"
  [ "${#hits[@]}" -gt 1 ] && die "found ${#hits[@]} definitions in $file (lines ${hits[*]}): nothing changed"
  first=${hits[0]}; n=$(wc -l < "$new")
  local tmp; tmp=$(mktemp)
  awk -v ln="$first" -v nf="$new" 'NR==ln { while ((getline l < nf) > 0) print l; next } { print }' "$file" > "$tmp" || { rm -f "$tmp"; die "write failed"; }
  cat "$tmp" > "$file"; rm -f "$tmp"
  echo "$file:L$first-L$((first + n - 1)) replaced, +$((n - 1)) lines"
}
id_re() { printf '^[[:space:]]*([-*][[:space:]]+)?(\\*\\*)?%s([^0-9A-Za-z]|$)' "$1"; }

mode=${1:-}; [ -n "$mode" ] || usage; shift
case "$mode" in
  req)
    [ $# -eq 2 ] || usage; [[ $1 =~ ^REQ-[0-9]+$ ]] || die "bad id: $1"
    new=$(mktemp); newtext "$2" > "$new"
    mapfile -t files < <(grep -lE -- "$(id_re "$1")" docs/business.md docs/business-*.md 2>/dev/null)
    [ "${#files[@]}" -eq 1 ] || { rm -f "$new"; die "$1 defined in ${#files[@]} files of docs/business*.md"; }
    replace_line "${files[0]}" "$(id_re "$1")" "$new"; rc=$?; rm -f "$new"; exit $rc ;;
  adr)
    [ $# -ge 2 ] || usage; [[ $1 =~ ^ADR-[0-9]+$ ]] || die "bad id: $1"
    new=$(mktemp); newtext "$2" > "$new"
    if [ -n "${3:-}" ]; then
      re="^[[:space:]]*([-*][[:space:]]+)?(\\*\\*)?.*\\(($1)[^0-9]|$(id_re "$1")"
      replace_line "$3" "$re" "$new"; rc=$?
    else
      mapfile -t files < <(grep -lE -- "^$1([^0-9]|$)" ai-docs/architecture-adr*.md 2>/dev/null)
      [ "${#files[@]}" -eq 1 ] || { rm -f "$new"; die "$1 defined in ${#files[@]} ai-docs ADR files"; }
      replace_line "${files[0]}" "^$1([^0-9]|$)" "$new"; rc=$?
    fi
    rm -f "$new"; exit $rc ;;
  line)
    [ $# -eq 3 ] || usage
    new=$(mktemp); newtext "$3" > "$new"
    replace_line "$1" "$(id_re "$2")" "$new"; rc=$?; rm -f "$new"; exit $rc ;;
  text)
    all=0; [ "${1:-}" = "--all" ] && { all=1; shift; }
    [ $# -ge 3 ] || usage
    old=$1; new=$2; shift 2
    [ -n "$old" ] || die "empty old text"
    total=0
    for f in "$@"; do guard "$f"; c=$(OLD="$old" awk 'BEGIN{o=ENVIRON["OLD"]} { s=$0; while ((i=index(s,o))>0) {n++; s=substr(s,i+length(o))} } END{print n+0}' "$f"); total=$((total + c)); done
    [ "$total" -eq 0 ] && die "text not found"
    [ "$all" -eq 0 ] && [ "$total" -gt 1 ] && die "found $total times: use --all or a longer text; nothing changed"
    for f in "$@"; do
      tmp=$(mktemp)
      OLD="$old" NEW="$new" awk 'BEGIN{o=ENVIRON["OLD"]; w=ENVIRON["NEW"]} { s=$0; r=""; while ((i=index(s,o))>0) { r=r substr(s,1,i-1) w; s=substr(s,i+length(o)) } print r s }' "$f" > "$tmp" || { rm -f "$tmp"; die "write failed"; }
      cat "$tmp" > "$f"; rm -f "$tmp"
    done
    echo "replaced $total occurrence(s) in $# file(s), +0 lines" ;;
  show-version-hint) echo "project version: scripts/version.sh set X.Y.Z; any other version (library, badge): scripts/doc-replace.sh text <old> <new> FILE..." ;;
  *) usage ;;
esac
