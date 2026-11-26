#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Regenerates packaging/notices/rust-crates.txt (the licences of the Rust crates compiled
# into videolore) and packaging/notices/rust-crates-player.txt (the same for videolore
# player, whose graph has no speech-to-text) and rust-crates-screen-spy.txt (videolore
# screen spy) with cargo-about, using about.toml and the plain-text template about.hbs.
#   scripts/notices.sh           rewrite the three files
#   scripts/notices.sh --check   fail if a committed file is out of date (used by CI)
# Needs: cargo install --locked cargo-about --version 0.9.3 --features cli
set -euo pipefail
cd "$(dirname "$0")/.."
check=0
[[ "${1:-}" == "--check" ]] && check=1
tmp=$(mktemp)
trap 'rm -f "$tmp"' EXIT
# notices OUT MANIFEST FEATURES...: the notice file of one product
notices() {
    local out="$1" manifest="$2"; shift 2
    cargo about generate --locked --offline --manifest-path "$manifest" "$@" about.hbs -o "$tmp"
    if [[ $check -eq 1 ]]; then
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
}
notices packaging/notices/rust-crates.txt Cargo.toml --all-features
notices packaging/notices/rust-crates-player.txt player/Cargo.toml
notices packaging/notices/rust-crates-screen-spy.txt screen-spy/Cargo.toml
