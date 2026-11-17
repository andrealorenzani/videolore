#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 Andrea Lorenzani
# Developer-only: rebuilds tests/fixtures/spherical-360.mp4 (a 5-frame 128x64 clip with the
# spherical-video v1 box, equirectangular) and tests/fixtures/flat-2to1.mp4 (the same
# picture without it). Needs ffmpeg and python3. The committed files are what tests use.
set -euo pipefail
cd "$(dirname "$0")/../tests/fixtures"
ffmpeg -v error -y -f lavfi -i "testsrc2=size=128x64:rate=5:duration=1" -pix_fmt yuv420p \
  -c:v libx264 -preset ultrafast -crf 40 flat-2to1.mp4
python3 -I - <<'PY'
import struct
d = bytearray(open('flat-2to1.mp4', 'rb').read())
def boxes(start, end):
    i = start
    while i < end:
        size, kind = struct.unpack('>I4s', d[i:i + 8])
        yield i, size, kind.decode('latin1')
        i += size
moov = next((i, s) for i, s, t in boxes(0, len(d)) if t == 'moov')
trak = next((i, s) for i, s, t in boxes(moov[0] + 8, moov[0] + moov[1]) if t == 'trak')
xml = (b'<?xml version="1.0"?><rdf:SphericalVideo xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" '
       b'xmlns:GSpherical="http://ns.google.com/videos/1.0/spherical/"><GSpherical:Spherical>true</GSpherical:Spherical>'
       b'<GSpherical:Stitched>true</GSpherical:Stitched><GSpherical:StitchingSoftware>videolore</GSpherical:StitchingSoftware>'
       b'<GSpherical:ProjectionType>equirectangular</GSpherical:ProjectionType></rdf:SphericalVideo>')
box = struct.pack('>I4s', 24 + len(xml), b'uuid') + bytes.fromhex('ffcc8263f8554a938814587a02521fdd') + xml
d[trak[0] + trak[1]:trak[0] + trak[1]] = box
d[trak[0]:trak[0] + 4] = struct.pack('>I', trak[1] + len(box))
d[moov[0]:moov[0] + 4] = struct.pack('>I', moov[1] + len(box))
open('spherical-360.mp4', 'wb').write(d)
PY
