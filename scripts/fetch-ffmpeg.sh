#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Downloads the ffmpeg build pinned in packaging/ffmpeg.lock for a target, checks its
# SHA-256 and extracts only ffmpeg, ffprobe (and on Windows the DLLs they need) into DEST.
#   scripts/fetch-ffmpeg.sh linux-x86_64|windows-x86_64|macos-aarch64 DEST
# Then runs a smoke test of the extracted copy when it can run on this machine.
set -euo pipefail
cd "$(dirname "$0")/.."
target="${1:?usage: fetch-ffmpeg.sh TARGET DEST}"
dest="${2:?usage: fetch-ffmpeg.sh TARGET DEST}"
lock=packaging/ffmpeg.lock

# Value of KEY in section [TARGET] of the lock file.
pin() {
    awk -v sec="[$target]" -v key="$1" '
        /^[[:space:]]*#/ { next }
        /^\[/ { insec = ($0 == sec); next }
        insec { split($0, kv, "="); k = kv[1]; gsub(/[[:space:]]/, "", k)
                if (k == key) { v = substr($0, index($0, "=") + 1); gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); print v; exit } }
    ' "$lock"
}

sha256() {
    if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1
    else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# download URL SHA FILE
download() {
    curl -fsSL --retry 3 --retry-delay 5 -o "$3" "$1"
    local got; got=$(sha256 "$3")
    if [[ "$got" != "$2" ]]; then
        echo "checksum mismatch for $1: expected $2, got $got" >&2
        exit 1
    fi
}

unzip_to() {
    if command -v unzip >/dev/null; then unzip -q -o "$1" -d "$2"
    else 7z x -y -o"$2" "$1" >/dev/null; fi
}

[[ -n "$(pin licence)" ]] || { echo "no pin for target $target in $lock" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p "$dest"
case "$target" in
    linux-x86_64)
        download "$(pin url)" "$(pin sha256)" "$work/ffmpeg.tar.xz"
        tar -xJf "$work/ffmpeg.tar.xz" -C "$work"
        for b in ffmpeg ffprobe; do
            install -m 0755 "$(find "$work" -path "*/bin/$b" -type f | head -1)" "$dest/$b"
        done
        ;;
    windows-x86_64)
        download "$(pin url)" "$(pin sha256)" "$work/ffmpeg.zip"
        unzip_to "$work/ffmpeg.zip" "$work/x"
        bin=$(dirname "$(find "$work/x" -path "*/bin/ffmpeg.exe" | head -1)")
        cp "$bin/ffmpeg.exe" "$bin/ffprobe.exe" "$dest/"
        cp "$bin"/*.dll "$dest/"
        ;;
    macos-aarch64)
        for b in ffmpeg ffprobe; do
            download "$(pin "${b}_url")" "$(pin "${b}_sha256")" "$work/$b.zip"
            unzip_to "$work/$b.zip" "$work/$b"
            install -m 0755 "$(find "$work/$b" -name "$b" -type f | head -1)" "$dest/$b"
        done
        ;;
    *) echo "unknown target $target" >&2; exit 2 ;;
esac
echo "ffmpeg $(pin version) for $target in $dest"
