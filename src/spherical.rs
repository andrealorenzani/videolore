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
    marker_box_with(None)
}

/// The initial view of a 360 source: yaw, pitch and roll in hundredths of a degree.
pub type Pose = [i32; 3];

/// The marker box with the initial view of the source when it had one (REQ-262).
pub fn marker_box_with(pose: Option<Pose>) -> Vec<u8> {
    let xml = match pose.filter(|p| *p != [0, 0, 0]) {
        Some([y, p, r]) => XML.replace(
            "</rdf:SphericalVideo>",
            &format!(
                "<GSpherical:InitialViewHeadingDegrees>{:.2}</GSpherical:InitialViewHeadingDegrees>\
                 <GSpherical:InitialViewPitchDegrees>{:.2}</GSpherical:InitialViewPitchDegrees>\
                 <GSpherical:InitialViewRollDegrees>{:.2}</GSpherical:InitialViewRollDegrees>\
                 </rdf:SphericalVideo>",
                f64::from(y) / 100.0,
                f64::from(p) / 100.0,
                f64::from(r) / 100.0
            ),
        ),
        None => XML.to_string(),
    };
    let len = 8 + UUID.len() + xml.len();
    let mut b = Vec::with_capacity(len);
    b.extend_from_slice(&(len as u32).to_be_bytes());
    b.extend_from_slice(b"uuid");
    b.extend_from_slice(&UUID);
    b.extend_from_slice(xml.as_bytes());
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
fn marked_moov(moov: &[u8], moov_first: bool, pose: Option<Pose>) -> io::Result<Option<Vec<u8>>> {
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
    let marker = marker_box_with(pose);
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
    mark_equirectangular_with(path, None)
}

/// [`mark_equirectangular`] with the initial view of the source (REQ-262).
pub fn mark_equirectangular_with(path: &Path, pose: Option<Pose>) -> io::Result<bool> {
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
    let Some(new_moov) = marked_moov(&moov, moov_first, pose)? else {
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

/// The initial view written in the Spherical Video V1 XML of an MP4 or MOV file (REQ-269):
/// ffprobe does not report it for this kind of file. Only the movie box is read, with the
/// same limits as the writer; anything not understood is `None`.
pub fn read_pose(path: &Path) -> Option<Pose> {
    let mut f = File::open(path).ok()?;
    let total = f.metadata().ok()?.len();
    let mut at = 0u64;
    for _ in 0..MAX_BOXES {
        if total.saturating_sub(at) < 8 {
            return None;
        }
        let mut h = [0u8; 8];
        f.seek(SeekFrom::Start(at)).ok()?;
        f.read_exact(&mut h).ok()?;
        let size = u64::from(u32::from_be_bytes([h[0], h[1], h[2], h[3]]));
        if size < 8 || size > total - at {
            return None;
        }
        if &h[4..8] == b"moov" {
            if size > MAX_MOOV {
                return None;
            }
            let mut moov = vec![0u8; size as usize];
            f.seek(SeekFrom::Start(at)).ok()?;
            f.read_exact(&mut moov).ok()?;
            let k = moov.windows(16).position(|w| w == UUID)?;
            let end = (k + 16 + 4096).min(moov.len());
            let xml = String::from_utf8_lossy(&moov[k + 16..end]).into_owned();
            let tag = |name: &str| -> Option<f64> {
                let open = format!("<GSpherical:{name}>");
                let from = xml.find(&open)? + open.len();
                let to = xml[from..].find('<')? + from;
                let v: f64 = xml[from..to].trim().parse().ok()?;
                (v.is_finite() && v.abs() <= 360.0).then_some(v)
            };
            let cd = |v: Option<f64>| (v.unwrap_or(0.0) * 100.0).round() as i32;
            let pose = [
                cd(tag("InitialViewHeadingDegrees")),
                cd(tag("InitialViewPitchDegrees")),
                cd(tag("InitialViewRollDegrees")),
            ];
            return (pose != [0, 0, 0]).then_some(pose);
        }
        at += size;
    }
    None
}

// ---- Matroska and WebM (REQ-261) ----
//
// ffmpeg writes a Void element (padding) between the SeekHead and the Info of a Matroska
// file. The Projection of the video track is added by growing the Tracks element and
// shrinking that Void by the same number of bytes: nothing after Tracks moves (clusters,
// cues, tags keep their places), and only the SeekHead entries of the elements between the
// Void and Tracks (which moved earlier) are changed, with the same number of bytes. The
// file keeps its size and is rewritten in place at those few places. Only our own element
// is written.

const MKV_SEGMENT: [u8; 4] = [0x18, 0x53, 0x80, 0x67];
const MKV_SEEKHEAD: [u8; 4] = [0x11, 0x4D, 0x9B, 0x74];
const MKV_TRACKS: [u8; 4] = [0x16, 0x54, 0xAE, 0x6B];
const MKV_VOID: [u8; 1] = [0xEC];
const MKV_TRACK_ENTRY: [u8; 1] = [0xAE];
const MKV_VIDEO: [u8; 1] = [0xE0];
const MKV_PROJECTION: [u8; 2] = [0x76, 0x70];
/// Largest start of a Matroska file looked at (the headers come first).
const MKV_HEAD: u64 = 1 << 20;

/// An element found in a buffer: its id bytes, and where its header and body are.
#[derive(Debug, Clone)]
struct Ebml {
    id: Vec<u8>,
    start: usize,
    body: usize,
    end: usize,
}

fn ebml_len(first: u8) -> Option<usize> {
    let n = first.leading_zeros() as usize + 1;
    (n <= 8 && first != 0).then_some(n)
}

/// Reads the element at `at` in `b`; `None` for a damaged or unknown-size element.
fn ebml_at(b: &[u8], at: usize, limit: usize) -> Option<Ebml> {
    let first = *b.get(at)?;
    let il = ebml_len(first)?.min(4);
    if ebml_len(first)? > 4 {
        return None;
    }
    let sl_at = at + il;
    let sf = *b.get(sl_at)?;
    let sl = ebml_len(sf)?;
    let mut size = u64::from(sf) & ((1u64 << (8 - sl)) - 1);
    let mut all_ones = size == (1u64 << (8 - sl)) - 1;
    for k in 1..sl {
        let byte = *b.get(sl_at + k)?;
        all_ones &= byte == 0xFF;
        size = (size << 8) | u64::from(byte);
    }
    if all_ones {
        return None;
    }
    let body = sl_at + sl;
    let end = body.checked_add(usize::try_from(size).ok()?)?;
    (end <= limit).then(|| Ebml {
        id: b[at..at + il].to_vec(),
        start: at,
        body,
        end,
    })
}

fn ebml_children(b: &[u8], from: usize, to: usize) -> Option<Vec<Ebml>> {
    let mut out = Vec::new();
    let mut at = from;
    while at < to {
        if out.len() > 4096 {
            return None;
        }
        let e = ebml_at(b, at, to)?;
        at = e.end;
        out.push(e);
    }
    Some(out)
}

/// The size field of `n` bytes of body in the fewest bytes.
fn ebml_size(n: usize) -> Vec<u8> {
    let mut len = 1;
    while len < 8 && n as u64 >= (1u64 << (7 * len)) - 1 {
        len += 1;
    }
    let mut v = vec![0u8; len];
    let marked = (1u64 << (7 * len)) | n as u64;
    for (i, byte) in v.iter_mut().enumerate() {
        *byte = (marked >> (8 * (len - 1 - i))) as u8;
    }
    v
}

fn ebml_element(id: &[u8], body: &[u8]) -> Vec<u8> {
    let mut v = id.to_vec();
    v.extend(ebml_size(body.len()));
    v.extend_from_slice(body);
    v
}

/// The Tracks body with a Projection (equirectangular) in the first video track's Video
/// element; `None` when there is no such track or it has a Projection already.
fn tracks_with_projection(b: &[u8], tracks: &Ebml, pose: Option<Pose>) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    let mut done = false;
    for entry in ebml_children(b, tracks.body, tracks.end)? {
        if done || entry.id != MKV_TRACK_ENTRY {
            body.extend_from_slice(&b[entry.start..entry.end]);
            continue;
        }
        let kids = ebml_children(b, entry.body, entry.end)?;
        let is_video = kids
            .iter()
            .any(|k| k.id == [0x83] && b.get(k.body..k.end) == Some(&[1u8][..]));
        let Some(video) = kids.iter().find(|k| k.id == MKV_VIDEO).filter(|_| is_video) else {
            body.extend_from_slice(&b[entry.start..entry.end]);
            continue;
        };
        if ebml_children(b, video.body, video.end)?
            .iter()
            .any(|k| k.id == MKV_PROJECTION)
        {
            return None;
        }
        // Projection { ProjectionType 1 = equirectangular }
        let mut inner = vec![0x76, 0x71, 0x81, 0x01];
        if let Some([y, p, r]) = pose.filter(|p| *p != [0, 0, 0]) {
            // ProjectionPoseYaw, Pitch, Roll: 32-bit floats in degrees
            for (id, v) in [(0x7673u16, y), (0x7674, p), (0x7675, r)] {
                inner.extend(id.to_be_bytes());
                inner.push(0x84);
                inner.extend((f64::from(v) as f32 / 100.0).to_be_bytes());
            }
        }
        let projection = ebml_element(&MKV_PROJECTION, &inner);
        let mut new_video = b[video.body..video.end].to_vec();
        new_video.extend(projection);
        let mut new_entry = Vec::new();
        for k in &kids {
            if k.start == video.start {
                new_entry.extend(ebml_element(&MKV_VIDEO, &new_video));
            } else {
                new_entry.extend_from_slice(&b[k.start..k.end]);
            }
        }
        body.extend(ebml_element(&MKV_TRACK_ENTRY, &new_entry));
        done = true;
    }
    done.then_some(body)
}

/// Marks the Matroska or WebM file at `path` as an equirectangular 360° video (REQ-261).
/// Returns whether the file was changed: not for a file that is not Matroska, has no
/// padding to use, has no video track or is marked already.
pub fn mark_matroska(path: &Path) -> io::Result<bool> {
    mark_matroska_with(path, None)
}

/// [`mark_matroska`] with the initial view of the source (REQ-262): yaw, pitch and roll
/// elements of the Projection.
pub fn mark_matroska_with(path: &Path, pose: Option<Pose>) -> io::Result<bool> {
    use std::fs::OpenOptions;
    let mut f = OpenOptions::new().read(true).write(true).open(path)?;
    let total = f.metadata()?.len();
    let want = total.min(MKV_HEAD) as usize;
    let mut b = vec![0u8; want];
    f.read_exact(&mut b)?;
    // the EBML header, then the Segment
    let Some(head) = ebml_at(&b, 0, want) else {
        return Ok(false);
    };
    if head.id != [0x1A, 0x45, 0xDF, 0xA3] {
        return Ok(false);
    }
    // a Segment of unknown or longer size than we read: only its start is needed
    let seg_at = head.end;
    let Some(first) = b.get(seg_at..seg_at + 4) else {
        return Ok(false);
    };
    if first != MKV_SEGMENT {
        return Ok(false);
    }
    let sl = match b.get(seg_at + 4).copied().and_then(ebml_len) {
        Some(n) => n,
        None => return Ok(false),
    };
    let seg_body = seg_at + 4 + sl;
    // the top-level children until Tracks
    let mut kids: Vec<Ebml> = Vec::new();
    let mut at = seg_body;
    let tracks = loop {
        let Some(e) = ebml_at(&b, at, want) else {
            return Ok(false);
        };
        at = e.end;
        if e.id == MKV_TRACKS {
            break e;
        }
        kids.push(e);
        if kids.len() > 64 {
            return Ok(false);
        }
    };
    // the padding before Tracks, after the SeekHead
    let Some(void) = kids.iter().rev().find(|k| k.id == MKV_VOID).cloned() else {
        return Ok(false);
    };
    let Some(new_body) = tracks_with_projection(&b, &tracks, pose) else {
        return Ok(false);
    };
    let new_tracks = ebml_element(&MKV_TRACKS, &new_body);
    let grow = new_tracks.len() - (tracks.end - tracks.start);
    let void_len = void.end - void.start;
    // the padding shrinks by what Tracks grows: a Void of 0 bytes is not there, one of 1
    // byte cannot be written
    if grow > void_len || void_len - grow == 1 {
        return Ok(false);
    }
    let left = void_len - grow;
    let mut region = Vec::new();
    if left >= 2 {
        // a Void element of `left` bytes: id, size, zeros
        let mut v = vec![0xEC];
        let mut size_len = 1;
        while size_len < 8 && ((left - 1 - size_len) as u64) >= (1u64 << (7 * size_len)) - 1 {
            size_len += 1;
        }
        let data = left - 1 - size_len;
        let marked = (1u64 << (7 * size_len)) | data as u64;
        for i in 0..size_len {
            v.push((marked >> (8 * (size_len - 1 - i))) as u8);
        }
        v.resize(left, 0);
        region.extend(v);
    }
    let between = &b[void.end..tracks.start];
    region.extend_from_slice(between);
    region.extend(new_tracks);
    debug_assert_eq!(region.len(), tracks.end - void.start);
    // the SeekHead entries of the elements that moved earlier: from the end of the padding
    // up to the start of Tracks (positions are counted from the start of the Segment body)
    let moved = (void.end - seg_body) as u64..=(tracks.start - seg_body) as u64;
    let mut patches: Vec<(usize, Vec<u8>)> = Vec::new();
    for sh in kids.iter().filter(|k| k.id == MKV_SEEKHEAD) {
        for seek in ebml_children(&b, sh.body, sh.end).unwrap_or_default() {
            for item in ebml_children(&b, seek.body, seek.end).unwrap_or_default() {
                // SeekPosition
                if item.id != [0x53, 0xAC] || item.end - item.body == 0 || item.end - item.body > 8
                {
                    continue;
                }
                let mut v = 0u64;
                for byte in &b[item.body..item.end] {
                    v = (v << 8) | u64::from(*byte);
                }
                if moved.contains(&v) {
                    let n = v - grow as u64;
                    let width = item.end - item.body;
                    let bytes = n.to_be_bytes();
                    patches.push((item.body, bytes[8 - width..].to_vec()));
                }
            }
        }
    }
    f.seek(SeekFrom::Start(void.start as u64))?;
    f.write_all(&region)?;
    for (at, bytes) in patches {
        f.seek(SeekFrom::Start(at as u64))?;
        f.write_all(&bytes)?;
    }
    f.sync_all()?;
    Ok(true)
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

    // covers: REQ-261
    #[test]
    fn files_that_are_not_matroska_or_have_no_padding_are_left_alone() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("x.mkv");
        for bytes in [
            &b""[..],
            b"not a file at all",
            &[0x1A, 0x45, 0xDF, 0xA3, 0x80][..],
        ] {
            std::fs::write(&p, bytes).unwrap();
            assert!(!mark_matroska(&p).unwrap());
            assert_eq!(std::fs::read(&p).unwrap(), bytes);
        }
        assert_eq!(ebml_size(5), vec![0x85]);
        assert_eq!(ebml_size(126), vec![0xFE]);
        assert_eq!(ebml_size(127), vec![0x40, 0x7F]);
    }
}
