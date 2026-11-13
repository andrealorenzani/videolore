#!/usr/bin/env bash
# Released docs are frozen: fail if any file under release/ was modified or renamed
# between <base> and HEAD. Adding new snapshots and deleting old ones (manual cleanup) is allowed.
set -euo pipefail
cd "$(dirname "$0")/.."
base="${1:-HEAD~1}"
if ! git cat-file -e "$base^{commit}" 2>/dev/null; then echo "release-guard: no base commit, skipped"; exit 0; fi
changed=$(git diff --name-only --diff-filter=MR "$base" HEAD -- release/)
if [[ -n "$changed" ]]; then echo "released docs must never change:" >&2; echo "$changed" >&2; exit 1; fi
echo "release-guard: release/ untouched"
