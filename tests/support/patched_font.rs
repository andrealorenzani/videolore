// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

// A test font made from the bundled one (shared by the unit tests and the ffmpeg tests with
// `include!`): the same glyphs under another family name and with another line height, so
// that text set in it is wider than in the bundled font at the same size.

/// The bundled font with its family renamed to `family` (exactly nine characters, the
/// length of "Noto Sans") and its OS/2 `usWinAscent` set to `win_ascent` (1124 in the
/// bundled font; a smaller value makes the renderer's glyphs bigger, because the size it is
/// given is the height of a line, the Windows ascender plus descender).
pub fn patched_font(family: &str, win_ascent: u16) -> Vec<u8> {
    assert_eq!(family.chars().count(), 9, "the name keeps its length");
    let mut b = videolore::ass::FONT.to_vec();
    let swap = |b: &mut Vec<u8>, from: &[u8], to: &[u8]| {
        let mut i = 0;
        while i + from.len() <= b.len() {
            if &b[i..i + from.len()] == from {
                b[i..i + from.len()].copy_from_slice(to);
                i += from.len();
            } else {
                i += 1;
            }
        }
    };
    let utf16 = |s: &str| s.encode_utf16().flat_map(u16::to_be_bytes).collect::<Vec<u8>>();
    swap(&mut b, &utf16("Noto Sans"), &utf16(family));
    swap(&mut b, b"Noto Sans", family.as_bytes());
    let tables = u16::from_be_bytes([b[4], b[5]]) as usize;
    for i in 0..tables {
        let r = 12 + 16 * i;
        if &b[r..r + 4] == b"OS/2" {
            let off = u32::from_be_bytes([b[r + 8], b[r + 9], b[r + 10], b[r + 11]]) as usize;
            b[off + 74..off + 76].copy_from_slice(&win_ascent.to_be_bytes());
        }
    }
    b
}
