#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Regenerates packaging/notices/rust-crates.txt (the licences of the Rust crates compiled
# into videolore) with cargo-about, using about.toml and the plain-text template about.hbs.
#   scripts/notices.sh           rewrite the file
#   scripts/notices.sh --check   fail if the committed file is out of date (used by CI)
# Needs: cargo install --locked cargo-about --version 0.9.3 --features cli
set -euo pipefail
cd "$(dirname "$0")/.."
out=packaging/notices/rust-crates.txt
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
cargo about generate --locked --all-features --offline about.hbs -o "$tmp"
if [[ "${1:-}" == "--check" ]]; then
    if ! diff -q "$out" "$tmp" >/dev/null; then
        echo "$out is out of date: run scripts/notices.sh and commit the result" >&2
        diff -u "$out" "$tmp" | head -40 >&2 || true
        exit 1
    fi
    echo "$out is up to date"
else
    cat "$tmp" >"$out"
    echo "wrote $out"
fi
