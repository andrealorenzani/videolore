#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
#
# Regenerates the committed icon files from assets/icon/videolore.svg (developer tool,
# not part of the build). Needs rsvg-convert, ImageMagick (convert or magick) and python3.
#   icon.png       1024x1024 (macOS bundle, packaging)
#   icon-256.png   256x256   (Linux menu entry and AppImage)
#   icon.ico       16..256   (Windows installer and shortcuts)
#   icon.icns      128..1024 (macOS)
#   icon-256.rgba  raw RGBA 256x256 (window icon, embedded with include_bytes!)
set -euo pipefail
cd "$(dirname "$0")/../assets/icon"
im=convert; command -v magick >/dev/null && im=magick
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
for s in 16 32 48 64 128 256 512 1024; do
    rsvg-convert -w "$s" -h "$s" videolore.svg -o "$tmp/$s.png"
done
cp "$tmp/1024.png" icon.png
cp "$tmp/256.png" icon-256.png
"$im" "$tmp/16.png" "$tmp/32.png" "$tmp/48.png" "$tmp/64.png" "$tmp/128.png" "$tmp/256.png" icon.ico
"$im" icon-256.png -depth 8 rgba:icon-256.rgba
python3 - "$tmp" <<'PY'
import struct, sys, os
tmp = sys.argv[1]
# ICNS: "icns" + total length, then entries (type, length incl. 8-byte header, PNG data).
entries = [(b"ic07", "128"), (b"ic08", "256"), (b"ic09", "512"), (b"ic10", "1024")]
body = b""
for kind, size in entries:
    data = open(os.path.join(tmp, size + ".png"), "rb").read()
    body += kind + struct.pack(">I", len(data) + 8) + data
open("icon.icns", "wb").write(b"icns" + struct.pack(">I", len(body) + 8) + body)
PY
test "$(stat -c %s icon-256.rgba 2>/dev/null || stat -f %z icon-256.rgba)" -eq $((256 * 256 * 4))
echo "icons written to assets/icon/"
