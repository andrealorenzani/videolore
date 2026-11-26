#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 {{AUTHOR}}
#
# Turns a saved `cargo test` output into GitHub Actions error annotations, so the reason of a
# CI failure can be read from the public check-run annotations without opening the log.
#
#   scripts/test-annotations.sh <cargo-test-output.txt>
#
# For every failed test: one `::error` with the test name, the file/line of the panic and the
# captured output (panic message included). Compiler errors, crashed test binaries (signals)
# and the final summary lines are annotated too. Prints nothing for a clean output.
set -euo pipefail
[[ $# -eq 1 && -f "$1" ]] || { echo "usage: $0 <cargo-test-output.txt>" >&2; exit 2; }

max_lines=40   # captured lines kept per failed test
max_notes=9    # GitHub shows at most 10 error annotations per step; keep one for the summary

# Data and properties of a workflow command must be escaped (% CR LF, plus : and , in properties).
esc_data() { local s=${1//'%'/'%25'}; s=${s//$'\r'/'%0D'}; printf '%s' "${s//$'\n'/'%0A'}"; }
esc_prop() { local s; s=$(esc_data "$1"); s=${s//':'/'%3A'}; printf '%s' "${s//','/'%2C'}"; }

notes=0
note() { # title, message, [file], [line]
  (( notes < max_notes )) || return 0
  notes=$((notes + 1))
  local props="title=$(esc_prop "$1")"
  [[ -n "${3:-}" ]] && props="file=$(esc_prop "$3"),line=${4:-1},$props"
  printf '::error %s::%s\n' "$props" "$(esc_data "$2")"
}

# Strip colours (CI sets CARGO_TERM_COLOR=always) and carriage returns.
mapfile -t lines < <(sed -e 's/\x1b\[[0-9;]*[A-Za-z]//g' -e 's/\r$//' "$1")

test_name="" body="" file="" line="" kept=0 summary=""
flush() {
  [[ -n "$test_name" ]] || return 0
  while [[ $body == *$'\n' ]]; do body=${body%$'\n'}; done
  note "test $test_name failed" "$body" "$file" "$line"
  test_name="" body="" file="" line="" kept=0
}
for l in "${lines[@]}"; do
  if [[ $l =~ ^----\ (.+)\ std(out|err)\ ----$ ]]; then
    flush
    test_name=${BASH_REMATCH[1]}
    continue
  fi
  if [[ -n "$test_name" ]]; then
    if [[ $l == "failures:" || $l =~ ^test\ result: ]]; then
      flush
    else
      if [[ -z "$file" && $l =~ panicked\ at\ ([^:]+):([0-9]+): ]]; then
        file=${BASH_REMATCH[1]} line=${BASH_REMATCH[2]}
      fi
      [[ -z "$body" && -z "$l" ]] && continue # skip leading blank lines
      if (( kept < max_lines )); then body+="$l"$'\n'; kept=$((kept + 1)); fi
      continue
    fi
  fi
  if [[ $l =~ process\ didn\'t\ exit\ successfully ]]; then
    note "test binary failed or crashed" "$l"
  elif [[ $l =~ ^error(\[E[0-9]+\])?:\ (.+)$ ]]; then
    note "cargo: error" "$l"
  elif [[ $l =~ ^test\ result:\ FAILED ]]; then
    summary+="$l"$'\n'
  fi
done
flush
if [[ -n "$summary" ]]; then
  notes=$((max_notes - 1)) # the summary always gets the last slot
  note "cargo test summary" "${summary%$'\n'}"
fi
