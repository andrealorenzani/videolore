#!/usr/bin/env bash
# Line coverage of the Rust code of the whole workspace (the editor and videolore player;
# src/main.rs and player/src/main.rs excluded: they only wire their libraries).
#   scripts/coverage.sh                    print coverage, e.g. 86.2
#   scripts/coverage.sh --update-readme    also rewrite the README coverage badge
#   scripts/coverage.sh --fail-under 85    exit 1 if below the threshold
#   scripts/coverage.sh --badge 86.2       ONLY rewrite the README badge with a value already
#                                          measured by the Principal Engineer (runs no tests)
# Running the tests (every mode except --badge) is reserved to the Principal Engineer and CI.
# Needs: cargo install cargo-llvm-cov && rustup component add llvm-tools-preview
set -euo pipefail
cd "$(dirname "$0")/.."
update=0; min=""
badge() {
  local pct="$1"
  [[ "$pct" =~ ^[0-9]{1,3}\.[0-9]$ ]] || { echo "badge value must have one decimal, e.g. 86.2" >&2; exit 2; }
  local color; color=$(python3 -c "p=$pct; print('brightgreen' if p>=95 else 'green' if p>=85 else 'red')")
  sed -i -E "s#badge/coverage-[0-9.]+%25-[a-z]+#badge/coverage-${pct}%25-${color}#" README.md
}
if [[ "${1:-}" == "--badge" ]]; then badge "${2:?usage: --badge 86.2}"; echo "badge set to $2%"; exit 0; fi
while [[ $# -gt 0 ]]; do
  case "$1" in
    --update-readme) update=1 ;;
    --fail-under) min="$2"; shift ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac; shift
done
json=$(cargo llvm-cov --quiet --workspace --ignore-filename-regex 'src/main\.rs$' --json --summary-only)
# Truncate (not round) to one decimal, so 84.96 is reported as 84.9 and never looks like a pass.
pct=$(python3 -c 'import json,sys,math; t=json.loads(sys.stdin.read())["data"][0]["totals"]["lines"]; p=100.0 if t["count"]==0 else t["percent"]; print(f"{math.floor(p*10)/10:.1f}")' <<<"$json")
echo "$pct"
if [[ $update -eq 1 ]]; then badge "$pct"; fi
if [[ -n "$min" ]] && python3 -c "import sys; sys.exit(0 if $pct < $min else 1)"; then
  echo "coverage $pct% is below $min%" >&2; exit 1
fi
