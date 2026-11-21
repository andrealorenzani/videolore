// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Switches a subtitle track of an MP4 or MOV file off by default (REQ-182).
//!
//! ffmpeg's MP4 writer marks the first track of every kind "enabled", whatever
//! `-disposition` says, and players show an enabled subtitle track at once. This clears the
//! enabled bit of the track header (`tkhd`) of each subtitle track, one byte each, in
//! place; the track stays in the file and every player can switch it on. The file is
//! untrusted only as far as it is our own output, but the parser is bounded all the same:
//! boxes are walked inside their parents, the `moov` box is read once (at most
//! [`MAX_MOOV`]), and nothing outside it is touched.

use std::fs::OpenOptions;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// Largest `moov` box that is read.
pub const MAX_MOOV: u64 = 128 * 1024 * 1024;
/// The handler types of timed text and subtitle tracks.
const TEXT_HANDLERS: [&[u8; 4]; 3] = [b"sbtl", b"subt", b"text"];

/// The boxes `(type, payload start, payload end)` that lie in `data[from..to]`.
fn boxes(data: &[u8], from: usize, to: usize) -> Vec<([u8; 4], usize, usize)> {
    let mut out = Vec::new();
    let mut at = from;
    while at + 8 <= to {
        let size =
            u32::from_be_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]) as usize;
        let kind = [data[at + 4], data[at + 5], data[at + 6], data[at + 7]];
        let (header, size) = match size {
            0 => (8, to - at),
            1 if at + 16 <= to => {
                let big = u64::from_be_bytes(data[at + 8..at + 16].try_into().expect("8 bytes"));
                (16, usize::try_from(big).unwrap_or(usize::MAX))
            }
            1 => break,
            n => (8, n),
        };
        if size < header || size > to - at {
            break;
        }
        out.push((kind, at + header, at + size));
        at += size;
    }
    out
}

/// The positions (in `moov`) of the flag byte of each subtitle track's `tkhd`.
fn subtitle_flag_offsets(moov: &[u8]) -> Vec<usize> {
    let mut found = Vec::new();
    for (kind, from, to) in boxes(moov, 0, moov.len()) {
        if &kind != b"trak" {
            continue;
        }
        let parts = boxes(moov, from, to);
        let tkhd = parts.iter().find(|b| &b.0 == b"tkhd");
        let mdia = parts.iter().find(|b| &b.0 == b"mdia");
        let (Some(tkhd), Some(mdia)) = (tkhd, mdia) else {
            continue;
        };
        let handler = boxes(moov, mdia.1, mdia.2)
            .into_iter()
            .find(|b| &b.0 == b"hdlr")
            // version and flags (4), predefined (4), then the handler type
            .and_then(|b| moov.get(b.1 + 8..b.1 + 12));
        let text = handler.is_some_and(|h| TEXT_HANDLERS.iter().any(|t| t.as_slice() == h));
        // the payload starts with the version (1 byte) and the flags (3 bytes)
        if text && tkhd.2 >= tkhd.1 + 4 {
            found.push(tkhd.1 + 3);
        }
    }
    found
}

/// Clears the "enabled" bit of every subtitle track of the MP4 or MOV file at `path`;
/// returns how many tracks were changed. A file without a readable `moov` box is left as it
/// is (`Ok(0)`).
pub fn disable_subtitle_tracks(path: &Path) -> io::Result<usize> {
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let len = f.metadata()?.len();
    // find the moov box among the top-level boxes without reading the media data
    let mut at = 0u64;
    let mut head = [0u8; 16];
    while at + 8 <= len {
        f.seek(SeekFrom::Start(at))?;
        let got = f.read(&mut head)?;
        if got < 8 {
            break;
        }
        let small = u64::from(u32::from_be_bytes([head[0], head[1], head[2], head[3]]));
        let (header, size) = match small {
            0 => (8, len - at),
            1 if got >= 16 => (
                16,
                u64::from_be_bytes(head[8..16].try_into().expect("8 bytes")),
            ),
            1 => break,
            n => (8, n),
        };
        if size < header || size > len - at {
            break;
        }
        if &head[4..8] == b"moov" {
            if size - header > MAX_MOOV {
                return Ok(0);
            }
            let mut moov = vec![0u8; usize::try_from(size - header).map_err(io::Error::other)?];
            f.seek(SeekFrom::Start(at + header))?;
            f.read_exact(&mut moov)?;
            let offsets = subtitle_flag_offsets(&moov);
            let mut changed = 0;
            for o in offsets {
                // bit 0 of the flags: the track is enabled
                if moov[o] & 1 != 0 {
                    f.seek(SeekFrom::Start(at + header + o as u64))?;
                    f.write_all(&[moov[o] & !1])?;
                    changed += 1;
                }
            }
            f.sync_all()?;
            return Ok(changed);
        }
        at += size;
    }
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bx(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut v = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(payload);
        v
    }

    fn trak(handler: &[u8; 4], flags: u8) -> Vec<u8> {
        let tkhd = bx(b"tkhd", &[0, 0, 0, flags, 0, 0, 0, 0]);
        let mut h = vec![0u8; 8];
        h.extend_from_slice(handler);
        h.extend_from_slice(&[0u8; 12]);
        let mdia = bx(b"mdia", &bx(b"hdlr", &h));
        bx(b"trak", &[tkhd, mdia].concat())
    }

    fn file(tracks: &[Vec<u8>]) -> Vec<u8> {
        let moov = bx(b"moov", &tracks.concat());
        [bx(b"ftyp", b"isom0000"), bx(b"mdat", &[7u8; 100]), moov].concat()
    }

    fn flags_of(data: &[u8]) -> Vec<u8> {
        // the third payload byte of each tkhd is the last flag byte
        let mut v = Vec::new();
        let mut i = 0;
        while let Some(p) = data[i..].windows(4).position(|w| w == b"tkhd") {
            v.push(data[i + p + 4 + 3]);
            i += p + 4;
        }
        v
    }

    // covers: REQ-182
    #[test]
    fn only_the_subtitle_tracks_are_switched_off() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.mp4");
        std::fs::write(
            &p,
            file(&[
                trak(b"vide", 3),
                trak(b"soun", 3),
                trak(b"sbtl", 3),
                trak(b"text", 7),
            ]),
        )
        .unwrap();
        assert_eq!(disable_subtitle_tracks(&p).unwrap(), 2);
        assert_eq!(flags_of(&std::fs::read(&p).unwrap()), [3, 3, 2, 6]);
        // a second run finds nothing to change
        assert_eq!(disable_subtitle_tracks(&p).unwrap(), 0);
    }

    // covers: REQ-182
    #[test]
    fn damaged_or_foreign_files_are_left_alone_and_never_panic() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.mp4");
        let good = file(&[trak(b"sbtl", 3)]);
        for cut in 0..good.len() {
            std::fs::write(&p, &good[..cut]).unwrap();
            let _ = disable_subtitle_tracks(&p);
        }
        for junk in [
            vec![],
            vec![0u8; 7],
            vec![0, 0, 0, 1, b'm', b'o', b'o', b'v'],
            [vec![0, 0, 0, 0], b"moov".to_vec(), vec![0xff; 40]].concat(),
            [vec![0xff, 0xff, 0xff, 0xff], b"moov".to_vec()].concat(),
            [
                vec![0, 0, 0, 1],
                b"moov".to_vec(),
                u64::MAX.to_be_bytes().to_vec(),
            ]
            .concat(),
        ] {
            std::fs::write(&p, &junk).unwrap();
            let _ = disable_subtitle_tracks(&p);
            assert_eq!(std::fs::read(&p).unwrap(), junk, "unchanged");
        }
        // no moov at all
        std::fs::write(&p, bx(b"ftyp", b"isom")).unwrap();
        assert_eq!(disable_subtitle_tracks(&p).unwrap(), 0);
        assert!(disable_subtitle_tracks(&d.path().join("none.mp4")).is_err());
    }
}
