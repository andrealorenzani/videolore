// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Marks an MP4/MOV file as a 360° video (Spherical Video V1, equirectangular): a `uuid`
//! box with a small XML text is added to the video track so that players show the file as
//! a sphere. ffmpeg cannot write this itself (measured with 6.1: a re-encode drops the
//! source's spherical side data), so the movie box is rewritten here. Only the small
//! `moov` box is read into memory; the rest of the file is copied as it is. The file is
//! untrusted input of our own export: every size is checked, nothing panics.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

/// `ffcc8263-f855-4a93-8814-587a02521fdd`, the id of the Spherical Video V1 box.
const UUID: [u8; 16] = [
    0xff, 0xcc, 0x82, 0x63, 0xf8, 0x55, 0x4a, 0x93, 0x88, 0x14, 0x58, 0x7a, 0x02, 0x52, 0x1f, 0xdd,
];
const XML: &str = "<?xml version=\"1.0\"?><rdf:SphericalVideo \
xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\" \
xmlns:GSpherical=\"http://ns.google.com/videos/1.0/spherical/\">\
<GSpherical:Spherical>true</GSpherical:Spherical>\
<GSpherical:Stitched>true</GSpherical:Stitched>\
<GSpherical:StitchingSoftware>videolore</GSpherical:StitchingSoftware>\
<GSpherical:ProjectionType>equirectangular</GSpherical:ProjectionType>\
</rdf:SphericalVideo>";
/// Biggest movie box we are willing to rewrite.
const MAX_MOOV: u64 = 256 * 1024 * 1024;
/// Most top-level boxes read.
const MAX_BOXES: usize = 100_000;

fn bad(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_string())
}

/// The bytes of the box that marks a file as equirectangular.
pub fn marker_box() -> Vec<u8> {
    let len = 8 + UUID.len() + XML.len();
    let mut b = Vec::with_capacity(len);
    b.extend_from_slice(&(len as u32).to_be_bytes());
    b.extend_from_slice(b"uuid");
    b.extend_from_slice(&UUID);
    b.extend_from_slice(XML.as_bytes());
    b
}

/// One box inside a buffer: its type, where it starts and ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Atom {
    kind: [u8; 4],
    start: usize,
    end: usize,
}

impl Atom {
    fn body(&self) -> usize {
        self.start + 8
    }
}

/// The boxes of `buf[from..to]`; stops with an error at a size that does not fit (a box
/// with a 64-bit size or one that runs to the end of the file is not understood).
fn atoms(buf: &[u8], from: usize, to: usize) -> io::Result<Vec<Atom>> {
    let mut v = Vec::new();
    let mut at = from;
    while at < to {
        if to - at < 8 || v.len() > MAX_BOXES {
            return Err(bad("truncated box"));
        }
        let size = u32::from_be_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]) as usize;
        if size < 8 || size > to - at {
            return Err(bad("box size outside its parent"));
        }
        v.push(Atom {
            kind: [buf[at + 4], buf[at + 5], buf[at + 6], buf[at + 7]],
            start: at,
            end: at + size,
        });
        at += size;
    }
    Ok(v)
}

fn child(buf: &[u8], parent: &Atom, kind: &[u8; 4]) -> io::Result<Vec<Atom>> {
    Ok(atoms(buf, parent.body(), parent.end)?
        .into_iter()
        .filter(|a| &a.kind == kind)
        .collect())
}

/// Adds `n` to every chunk offset of every track of the movie box in `moov`.
fn shift_offsets(moov: &mut [u8], n: u64) -> io::Result<()> {
    let top = Atom {
        kind: *b"moov",
        start: 0,
        end: moov.len(),
    };
    let mut spots: Vec<(usize, usize, bool)> = Vec::new();
    for trak in child(moov, &top, b"trak")? {
        for mdia in child(moov, &trak, b"mdia")? {
            for minf in child(moov, &mdia, b"minf")? {
                for stbl in child(moov, &minf, b"stbl")? {
                    for a in atoms(moov, stbl.body(), stbl.end)? {
                        let wide = &a.kind == b"co64";
                        if &a.kind == b"stco" || wide {
                            spots.push((a.body(), a.end, wide));
                        }
                    }
                }
            }
        }
    }
    for (body, end, wide) in spots {
        if end < body + 8 {
            return Err(bad("short offset table"));
        }
        let count = u32::from_be_bytes([
            moov[body + 4],
            moov[body + 5],
            moov[body + 6],
            moov[body + 7],
        ]) as usize;
        let width = if wide { 8 } else { 4 };
        if count.checked_mul(width).is_none_or(|b| body + 8 + b > end) {
            return Err(bad("offset table does not fit its box"));
        }
        for i in 0..count {
            let at = body + 8 + i * width;
            if wide {
                let v = u64::from_be_bytes(moov[at..at + 8].try_into().map_err(|_| bad("offset"))?);
                let v = v.checked_add(n).ok_or_else(|| bad("offset overflow"))?;
                moov[at..at + 8].copy_from_slice(&v.to_be_bytes());
            } else {
                let v = u64::from(u32::from_be_bytes(
                    moov[at..at + 4].try_into().map_err(|_| bad("offset"))?,
                )) + n;
                let v = u32::try_from(v).map_err(|_| bad("offset overflow"))?;
                moov[at..at + 4].copy_from_slice(&v.to_be_bytes());
            }
        }
    }
    Ok(())
}

/// The movie box with the marker added to its video track, or `None` when it already has
/// one or has no video track. `moov_first`: the media data comes after the movie box, so
/// its chunk offsets grow by the size of the marker.
fn marked_moov(moov: &[u8], moov_first: bool) -> io::Result<Option<Vec<u8>>> {
    let top = Atom {
        kind: *b"moov",
        start: 0,
        end: moov.len(),
    };
    let mut target = None;
    for trak in child(moov, &top, b"trak")? {
        for mdia in child(moov, &trak, b"mdia")? {
            for hdlr in child(moov, &mdia, b"hdlr")? {
                // version/flags (4), predefined (4), handler type (4)
                let at = hdlr.body() + 8;
                if hdlr.end >= at + 4 && &moov[at..at + 4] == b"vide" && target.is_none() {
                    target = Some(trak);
                }
            }
        }
        let marked = atoms(moov, trak.body(), trak.end)?.into_iter().any(|a| {
            &a.kind == b"uuid" && a.end >= a.body() + 16 && moov[a.body()..a.body() + 16] == UUID
        });
        if marked {
            return Ok(None);
        }
    }
    let Some(trak) = target else {
        return Ok(None);
    };
    let marker = marker_box();
    let n = marker.len();
    let mut out = Vec::with_capacity(moov.len() + n);
    out.extend_from_slice(&moov[..trak.end]);
    out.extend_from_slice(&marker);
    out.extend_from_slice(&moov[trak.end..]);
    let grow = |out: &mut Vec<u8>, at: usize| -> io::Result<()> {
        let old = u32::from_be_bytes(out[at..at + 4].try_into().map_err(|_| bad("size"))?);
        let new = old
            .checked_add(n as u32)
            .ok_or_else(|| bad("movie box too big"))?;
        out[at..at + 4].copy_from_slice(&new.to_be_bytes());
        Ok(())
    };
    grow(&mut out, 0)?;
    grow(&mut out, trak.start)?;
    if moov_first {
        shift_offsets(&mut out, n as u64)?;
    }
    Ok(Some(out))
}

/// Marks the MP4/MOV file at `path` as an equirectangular 360° video, rewriting it through
/// a temporary file next to it. Returns whether the file was changed: not for a file that
/// is not an MP4 (Matroska, WebM), has no video track or is marked already.
pub fn mark_equirectangular(path: &Path) -> io::Result<bool> {
    let mut f = File::open(path)?;
    let total = f.metadata()?.len();
    // the top-level boxes, read by their headers only
    let mut boxes: Vec<([u8; 4], u64, u64)> = Vec::new();
    let mut at = 0u64;
    while at < total {
        if boxes.len() > MAX_BOXES || total - at < 8 {
            return Ok(false);
        }
        let mut h = [0u8; 8];
        f.seek(SeekFrom::Start(at))?;
        f.read_exact(&mut h)?;
        let size = u64::from(u32::from_be_bytes([h[0], h[1], h[2], h[3]]));
        if size < 8 || size > total - at {
            // 64-bit sizes and boxes to the end of the file: not understood, left alone
            return Ok(false);
        }
        boxes.push(([h[4], h[5], h[6], h[7]], at, size));
        at += size;
    }
    let Some(&(_, moov_at, moov_len)) = boxes.iter().find(|b| &b.0 == b"moov") else {
        return Ok(false);
    };
    if moov_len > MAX_MOOV {
        return Ok(false);
    }
    let moov_first = boxes.iter().any(|b| &b.0 == b"mdat" && b.1 > moov_at);
    let mut moov = vec![0u8; moov_len as usize];
    f.seek(SeekFrom::Start(moov_at))?;
    f.read_exact(&mut moov)?;
    let Some(new_moov) = marked_moov(&moov, moov_first)? else {
        return Ok(false);
    };
    let tmp = path.with_extension("spherical-partial");
    let result = (|| -> io::Result<()> {
        let mut out = crate::base::private::create_private_file(&tmp)?;
        f.seek(SeekFrom::Start(0))?;
        io::copy(&mut (&mut f).take(moov_at), &mut out)?;
        out.write_all(&new_moov)?;
        f.seek(SeekFrom::Start(moov_at + moov_len))?;
        io::copy(&mut f, &mut out)?;
        out.sync_all()?;
        Ok(())
    })();
    match result.and_then(|()| std::fs::rename(&tmp, path)) {
        Ok(()) => Ok(true),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }

    fn stco(offsets: &[u32]) -> Vec<u8> {
        let mut b = vec![0, 0, 0, 0];
        b.extend_from_slice(&(offsets.len() as u32).to_be_bytes());
        for o in offsets {
            b.extend_from_slice(&o.to_be_bytes());
        }
        atom(b"stco", &b)
    }

    fn co64(offsets: &[u64]) -> Vec<u8> {
        let mut b = vec![0, 0, 0, 0];
        b.extend_from_slice(&(offsets.len() as u32).to_be_bytes());
        for o in offsets {
            b.extend_from_slice(&o.to_be_bytes());
        }
        atom(b"co64", &b)
    }

    fn trak(handler: &[u8; 4], table: Vec<u8>) -> Vec<u8> {
        let mut hdlr = vec![0u8; 8];
        hdlr.extend_from_slice(handler);
        hdlr.extend_from_slice(&[0u8; 12]);
        let stbl = atom(b"stbl", &table);
        let minf = atom(b"minf", &stbl);
        let mut mdia = atom(b"hdlr", &hdlr);
        mdia.extend_from_slice(&minf);
        atom(b"trak", &atom(b"mdia", &mdia))
    }

    /// ftyp, moov (a video track whose chunks are at `chunk` and an audio track at
    /// `chunk + 100`) and an mdat of 200 bytes, in the given order.
    fn mp4(moov_first: bool, wide: bool) -> Vec<u8> {
        let ftyp = atom(b"ftyp", b"isom\0\0\0\0isom");
        let mdat = atom(b"mdat", &[7u8; 200]);
        let build = |chunk: u64| {
            let t = |c: u64| if wide { co64(&[c]) } else { stco(&[c as u32]) };
            let mut body = trak(b"vide", t(chunk));
            body.extend_from_slice(&trak(b"soun", t(chunk + 100)));
            atom(b"moov", &body)
        };
        let moov_len = build(0).len() as u64;
        let mut f = ftyp.clone();
        if moov_first {
            let chunk = (ftyp.len() as u64) + moov_len + 8;
            f.extend_from_slice(&build(chunk));
            f.extend_from_slice(&mdat);
        } else {
            let chunk = ftyp.len() as u64 + 8;
            f.extend_from_slice(&mdat);
            f.extend_from_slice(&build(chunk));
        }
        f
    }

    fn offsets(file: &[u8]) -> Vec<u64> {
        // every stco/co64 value of the file, found by scanning for the box names
        let mut v = Vec::new();
        for i in 0..file.len().saturating_sub(15) {
            if &file[i..i + 4] == b"stco" {
                v.push(u64::from(u32::from_be_bytes(
                    file[i + 12..i + 16].try_into().unwrap(),
                )));
            }
            if &file[i..i + 4] == b"co64" && i + 20 <= file.len() {
                v.push(u64::from_be_bytes(file[i + 12..i + 20].try_into().unwrap()));
            }
        }
        v
    }

    fn mark(bytes: &[u8]) -> (bool, Vec<u8>) {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.mp4");
        std::fs::write(&p, bytes).unwrap();
        let changed = mark_equirectangular(&p).unwrap();
        assert!(!d.path().join("a.spherical-partial").exists());
        (changed, std::fs::read(&p).unwrap())
    }

    // covers: REQ-103
    #[test]
    fn the_marker_goes_into_the_video_track_and_chunk_offsets_follow() {
        for wide in [false, true] {
            for moov_first in [true, false] {
                let f = mp4(moov_first, wide);
                let (changed, out) = mark(&f);
                assert!(changed);
                let n = marker_box().len();
                assert_eq!(out.len(), f.len() + n);
                // the chunks still point at the media data (the 7s)
                let at = if moov_first { n as u64 } else { 0 };
                let before = offsets(&f);
                let after = offsets(&out);
                assert_eq!(after.len(), 2);
                for (b, a) in before.iter().zip(&after) {
                    assert_eq!(*a, b + at);
                }
                // the marker is inside the first (video) track: after its last child
                let xml = XML.as_bytes();
                let pos = out.windows(xml.len()).position(|w| w == xml).unwrap();
                let vide = out.windows(4).position(|w| w == b"vide").unwrap();
                let soun = out.windows(4).position(|w| w == b"soun").unwrap();
                assert!(vide < pos && pos < soun);
                // the file is still a list of boxes that fits exactly
                let all = atoms(&out, 0, out.len()).unwrap();
                assert_eq!(all.len(), 3);
                // marking again changes nothing
                let (again, same) = mark(&out);
                assert!(!again);
                assert_eq!(same, out);
            }
        }
    }

    // covers: REQ-103
    #[test]
    fn files_that_are_not_understood_are_left_alone() {
        // Matroska, an empty file, garbage, a box that runs past the file, no video track
        for bytes in [
            b"\x1a\x45\xdf\xa3\x01\x00\x00\x00".to_vec(),
            Vec::new(),
            vec![0u8; 5],
            vec![0, 0, 0, 200, b'f', b't', b'y', b'p'],
            atom(b"ftyp", b"isom"),
        ] {
            let (changed, out) = mark(&bytes);
            assert!(!changed);
            assert_eq!(out, bytes);
        }
        let mut f = Vec::new();
        f.extend_from_slice(&atom(b"ftyp", b"isom"));
        f.extend_from_slice(&atom(b"moov", &trak(b"soun", stco(&[1]))));
        assert!(!mark(&f).0, "no video track");
        // a movie box that lies about its tables is an error, not a panic
        let mut bad_table = stco(&[1, 2]);
        bad_table[15] = 9; // 9 entries claimed
        let mut g = atom(b"ftyp", b"isom");
        g.extend_from_slice(&atom(b"moov", &trak(b"vide", bad_table)));
        g.extend_from_slice(&atom(b"mdat", &[0; 8]));
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("b.mp4");
        std::fs::write(&p, &g).unwrap();
        assert!(mark_equirectangular(&p).is_err());
        assert_eq!(std::fs::read(&p).unwrap(), g, "the file is untouched");
        assert!(!d.path().join("b.spherical-partial").exists());
        // a box inside the movie box that is bigger than it
        let mut h = atom(b"ftyp", b"isom");
        let mut broken = atom(b"moov", &atom(b"trak", &[0u8; 8]));
        broken[8] = 0xff;
        h.extend_from_slice(&broken);
        let p = d.path().join("c.mp4");
        std::fs::write(&p, &h).unwrap();
        assert!(mark_equirectangular(&p).is_err());
        assert!(mark_equirectangular(&d.path().join("missing.mp4")).is_err());
    }
}
