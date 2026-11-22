#!/usr/bin/env bash
# Where a Rust item is defined and which files use it.
#   scripts/symbol.sh EffectKind        definitions (fn, struct, enum, trait, type, const, static, mod, macro)
#   scripts/symbol.sh -u EffectKind     also the number of uses per file
# Read-only; a plain word search, so a name used by several items lists them all.
set -euo pipefail
cd "$(dirname "$0")/.."
uses=0
if [[ "${1:-}" == "-u" ]]; then uses=1; shift; fi
[[ $# -eq 1 && "$1" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || { echo "usage: $0 [-u] Name" >&2; exit 2; }
name=$1
dirs=(src player/src tests player/tests)
echo "-- defined:"
grep -rnE "^\s*(pub(\([a-z]+\))? )?((const |async |unsafe )*fn|struct|enum|trait|type|const|static|mod|macro_rules!) $name\b" "${dirs[@]}" 2>/dev/null | sed 's/^/  /' | cut -c1-200 || true
if [[ $uses -eq 1 ]]; then
  echo "-- used (count per file):"
  grep -rcw "$name" "${dirs[@]}" 2>/dev/null | grep -v ':0$' | sort -t: -k2 -rn | sed 's/^/  /' || true
fi
