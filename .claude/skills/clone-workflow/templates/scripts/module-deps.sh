#!/usr/bin/env bash
# Which top-level modules each module uses (`crate::<module>` paths in its own
# files and sub-modules; comment lines and test files left out). A quick text-based view.
#   scripts/module-deps.sh            one line per module: "module: used modules"
#   scripts/module-deps.sh config     only that module
#   scripts/module-deps.sh -r config  the modules that use it
#   scripts/module-deps.sh --mermaid  the whole graph as mermaid "a --> b" lines
# Read-only.
set -euo pipefail
cd "$(dirname "$0")/.."
mode=list; want=""
case "${1:-}" in
  --mermaid) mode=mermaid ;;
  -r) mode=rev; want="${2:?usage: $0 -r module}" ;;
  "") ;;
  *) want="$1" ;;
esac
files_of() {
  local m=$1
  { [[ -f "src/$m.rs" ]] && echo "src/$m.rs"; [[ -d "src/$m" ]] && find "src/$m" -name '*.rs'; } \
    | grep -vE '(_tests|/tests)\.rs$' || true
}
uses_of() {
  local m=$1 f
  for f in $(files_of "$m"); do
    # stop at a `#[cfg(test)]` test module (its uses are not the module's)
    awk 'prev ~ /^#\[cfg\(test\)\]/ && /^(pub(\([a-z]+\))? )?mod / {exit} {if (NR>1) print prev; prev=$0} END {print prev}' "$f" \
      | grep -vE '^\s*(//|#\[cfg\(test\)\])' | grep -oE 'crate::[a-z_0-9]+' | sed 's/crate:://' || true
  done | { grep -vx "$m" || true; } | sort -u | tr '\n' ' '
}
modules=$( { ls src/*.rs | sed 's#src/##; s#\.rs$##'; find src -mindepth 1 -maxdepth 1 -type d | sed 's#src/##'; } \
  | grep -vxE 'main|lib' | sort -u)
for m in $modules; do
  u=$(uses_of "$m")
  case $mode in
    list) if [[ -z "$want" || "$want" == "$m" ]]; then echo "$m: $u"; fi ;;
    rev) if grep -qw -- "$want" <<<"$u"; then echo "$m"; fi ;;
    mermaid) for d in $u; do echo "  $m --> $d"; done ;;
  esac
done
