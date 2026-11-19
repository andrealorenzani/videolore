// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! LUT files (`.cube`) for the Apply LUT effect (REQ-135, REQ-136).
//!
//! A LUT file is untrusted input. It is parsed here into a normalised 3D cube; the
//! normalised text ([`Lut::to_cube`]) is what is written into the private Work folder and
//! read by ffmpeg, so a user's file name or content never reaches a filter string.

use std::path::Path;
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

/// Largest `.cube` file read (16 MiB).
pub const MAX_BYTES: u64 = 16 * 1024 * 1024;
/// Largest 3D cube edge.
pub const MAX_3D: usize = 65;
/// Largest 1D table.
pub const MAX_1D: usize = 4096;
/// Edge of the 3D cube a 1D LUT is converted to.
pub const CUBE_FROM_1D: usize = 33;
/// A stored LUT path is cleared when longer than this (characters).
pub const MAX_PATH_CHARS: usize = 4096;

/// Why a LUT file was refused. The text is shown to the user.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LutError {
    #[error("the file could not be read: {0}")]
    Read(String),
    #[error("the LUT file is larger than 16 MiB")]
    TooLarge,
    #[error("the LUT file is not a regular file")]
    NotRegular,
    #[error("the LUT file is not UTF-8 text")]
    NotText,
    #[error("not a valid .cube LUT: {0}")]
    Invalid(String),
}

fn invalid<T>(s: impl Into<String>) -> Result<T, LutError> {
    Err(LutError::Invalid(s.into()))
}

/// A 3D LUT, red index fastest (the `.cube` order), values in 0..=1.
#[derive(Debug, Clone, PartialEq)]
pub struct Lut {
    size: usize,
    data: Vec<[f32; 3]>,
    /// The normalised `.cube` text, made once (a graph is built for every preview picture).
    cube: String,
    hash: String,
}

impl Lut {
    fn new(size: usize, data: Vec<[f32; 3]>) -> Lut {
        let mut cube = format!("LUT_3D_SIZE {size}\n");
        for v in &data {
            cube.push_str(&format!("{:.6} {:.6} {:.6}\n", v[0], v[1], v[2]));
        }
        let hash = Sha256::digest(cube.as_bytes())
            .iter()
            .take(16)
            .map(|b| format!("{b:02x}"))
            .collect();
        Lut {
            size,
            data,
            cube,
            hash,
        }
    }

    /// Edge length of the cube.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The normalised 3D cube as `.cube` text (what ffmpeg reads).
    pub fn to_cube(&self) -> String {
        self.cube.clone()
    }

    /// Fingerprint of the normalised content (changes when the file's meaning changes).
    pub fn hash(&self) -> &str {
        &self.hash
    }

    /// Output colour for the input at grid point `(r, g, b)` (tests and sampling).
    pub fn at(&self, r: usize, g: usize, b: usize) -> [f32; 3] {
        self.data[r + g * self.size + b * self.size * self.size]
    }
}

/// Parses `.cube` bytes (a 3D or a 1D LUT) into a 3D cube.
pub fn parse(bytes: &[u8]) -> Result<Lut, LutError> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(LutError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| LutError::NotText)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut size3: Option<usize> = None;
    let mut size1: Option<usize> = None;
    let mut rows: Vec<[f32; 3]> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut it = line.split_whitespace();
        let first = it.next().unwrap_or("");
        let numeric = first
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'));
        if !numeric {
            let rest: Vec<&str> = it.collect();
            match first {
                "TITLE" => {}
                "LUT_3D_SIZE" | "LUT_1D_SIZE" => {
                    let v = rest.first().and_then(|t| t.parse::<usize>().ok());
                    let Some(v) = v.filter(|_| rest.len() == 1) else {
                        return invalid(format!("line {}: bad size", n + 1));
                    };
                    let (slot, other, lo, hi) = if first == "LUT_3D_SIZE" {
                        (&mut size3, size1, 2, MAX_3D)
                    } else {
                        (&mut size1, size3, 2, MAX_1D)
                    };
                    if other.is_some() {
                        return invalid("a file cannot hold both a 1D and a 3D table");
                    }
                    if slot.is_some() {
                        return invalid(format!("line {}: size given twice", n + 1));
                    }
                    if v < lo || v > hi {
                        return invalid(format!("size {v} is outside {lo}..={hi}"));
                    }
                    *slot = Some(v);
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    let want = if first == "DOMAIN_MIN" { 0.0 } else { 1.0 };
                    let vals: Vec<f32> = rest.iter().filter_map(|t| t.parse().ok()).collect();
                    if vals.len() != 3 || rest.len() != 3 || vals.iter().any(|v| *v != want) {
                        return invalid("only the default domain 0 to 1 is supported");
                    }
                }
                other => {
                    return invalid(format!("line {}: unknown keyword {:?}", n + 1, clip(other)));
                }
            }
            continue;
        }
        let mut v = [0f32; 3];
        let mut count = 0;
        for tok in std::iter::once(first).chain(it) {
            if count == 3 {
                return invalid(format!("line {}: more than three values", n + 1));
            }
            let x: f32 = tok
                .parse()
                .map_err(|_| LutError::Invalid(format!("line {}: not a number", n + 1)))?;
            if !x.is_finite() {
                return invalid(format!("line {}: value is not finite", n + 1));
            }
            v[count] = x.clamp(0.0, 1.0);
            count += 1;
        }
        if count != 3 {
            return invalid(format!("line {}: expected three values", n + 1));
        }
        let want = size3.map(|s| s * s * s).or(size1).unwrap_or(0);
        if rows.len() >= want {
            return invalid("more entries than the declared size");
        }
        rows.push(v);
    }
    match (size3, size1) {
        (Some(s), None) => {
            if rows.len() != s * s * s {
                return invalid(format!("{} entries, {} expected", rows.len(), s * s * s));
            }
            Ok(Lut::new(s, rows))
        }
        (None, Some(s)) => {
            if rows.len() != s {
                return invalid(format!("{} entries, {s} expected", rows.len()));
            }
            Ok(from_1d(&rows))
        }
        _ => invalid("no LUT_3D_SIZE or LUT_1D_SIZE line"),
    }
}

fn clip(s: &str) -> String {
    s.chars().take(24).collect()
}

/// Per-channel linear sampling of a 1D table at `x` in 0..=1.
fn sample(rows: &[[f32; 3]], c: usize, x: f32) -> f32 {
    let p = x.clamp(0.0, 1.0) * (rows.len() - 1) as f32;
    let i = (p.floor() as usize).min(rows.len() - 2);
    let f = p - i as f32;
    rows[i][c] * (1.0 - f) + rows[i + 1][c] * f
}

fn from_1d(rows: &[[f32; 3]]) -> Lut {
    let n = CUBE_FROM_1D;
    let g = |i: usize| i as f32 / (n - 1) as f32;
    let mut data = Vec::with_capacity(n * n * n);
    for b in 0..n {
        for gg in 0..n {
            for r in 0..n {
                data.push([
                    sample(rows, 0, g(r)),
                    sample(rows, 1, g(gg)),
                    sample(rows, 2, g(b)),
                ]);
            }
        }
    }
    Lut::new(n, data)
}

/// Reads and parses a LUT file (regular file, at most 16 MiB).
pub fn read(path: &Path) -> Result<Lut, LutError> {
    use std::io::ErrorKind;
    let bytes = crate::project::read_limited(path, MAX_BYTES).map_err(|e| match e.kind() {
        ErrorKind::InvalidData => LutError::TooLarge,
        ErrorKind::InvalidInput => LutError::NotRegular,
        _ => LutError::Read(e.to_string()),
    })?;
    parse(&bytes)
}

type CacheEntry = (String, u64, Option<std::time::SystemTime>, Arc<Lut>);

/// Parsed LUTs by path, re-read when the file's size or modification time changes, so the
/// preview and the export do not parse a large cube again and again (at most 8 kept).
static CACHE: Mutex<Vec<CacheEntry>> = Mutex::new(Vec::new());

/// [`read`] through the cache. A changed file is read and validated again.
pub fn cached(path: &str) -> Result<Arc<Lut>, LutError> {
    let meta = std::fs::metadata(path).map_err(|e| LutError::Read(e.to_string()))?;
    let stamp = (meta.len(), meta.modified().ok());
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(e) = c.iter().find(|e| e.0 == path && (e.1, e.2) == stamp) {
        return Ok(e.3.clone());
    }
    let lut = Arc::new(read(Path::new(path))?);
    c.retain(|e| e.0 != path);
    if c.len() >= 8 {
        c.remove(0);
    }
    c.push((path.to_string(), stamp.0, stamp.1, lut.clone()));
    Ok(lut)
}

/// A stored LUT path is acceptable: not empty, at most 4096 characters, no NUL.
pub fn path_ok(p: &str) -> bool {
    !p.is_empty() && p.chars().count() <= MAX_PATH_CHARS && !p.contains('\0')
}

/// The name shown in the popup and in messages.
pub fn display_name(path: &str) -> String {
    let n = Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string());
    crate::media::sanitize_for_display(&n)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn identity(n: usize) -> String {
        let mut s = format!("TITLE \"id\"\n# comment\nLUT_3D_SIZE {n}\n");
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let f = |i: usize| i as f32 / (n - 1) as f32;
                    s.push_str(&format!("{} {} {}\n", f(r), f(g), f(b)));
                }
            }
        }
        s
    }

    // covers: REQ-136
    #[test]
    fn valid_3d_identity_and_channel_order() {
        let l = parse(identity(2).as_bytes()).unwrap();
        assert_eq!(l.size(), 2);
        assert_eq!(l.at(1, 0, 0), [1.0, 0.0, 0.0]);
        assert_eq!(l.at(0, 1, 0), [0.0, 1.0, 0.0]);
        assert_eq!(l.at(0, 0, 1), [0.0, 0.0, 1.0]);
        assert!(l.to_cube().starts_with("LUT_3D_SIZE 2\n"));
        // the normalised text parses to the same table and the same hash
        let again = parse(l.to_cube().as_bytes()).unwrap();
        assert_eq!(again, l);
        assert_eq!(again.hash(), l.hash());
        assert_eq!(l.hash().len(), 32);
    }

    // covers: REQ-136
    #[test]
    fn one_d_becomes_a_33_cube_with_per_channel_sampling() {
        let t = "LUT_1D_SIZE 2\n0 0 1\n1 0.5 0\n";
        let l = parse(t.as_bytes()).unwrap();
        assert_eq!(l.size(), CUBE_FROM_1D);
        let last = CUBE_FROM_1D - 1;
        assert_eq!(l.at(0, 0, 0), [0.0, 0.0, 1.0]);
        assert_eq!(l.at(last, last, last), [1.0, 0.5, 0.0]);
        let mid = l.at(16, 16, 16);
        assert!((mid[0] - 0.5).abs() < 0.02 && (mid[1] - 0.25).abs() < 0.02);
        // each channel depends on its own input only
        let a = l.at(last, 0, 0);
        assert_eq!(a[1], 0.0);
        assert_eq!(a[0], 1.0);
    }

    // covers: REQ-136
    #[test]
    fn refusals() {
        let bad: Vec<(&str, String)> = vec![
            ("empty", String::new()),
            ("no size", "0 0 0\n".into()),
            ("size 1", "LUT_3D_SIZE 1\n0 0 0\n".into()),
            ("size 66", "LUT_3D_SIZE 66\n".into()),
            ("1d too big", "LUT_1D_SIZE 4097\n".into()),
            ("size text", "LUT_3D_SIZE x\n".into()),
            ("size extra", "LUT_3D_SIZE 2 3\n".into()),
            ("twice", "LUT_3D_SIZE 2\nLUT_3D_SIZE 2\n".into()),
            ("mixed", "LUT_3D_SIZE 2\nLUT_1D_SIZE 2\n".into()),
            ("mixed 1d first", "LUT_1D_SIZE 2\nLUT_3D_SIZE 2\n".into()),
            ("domain", format!("DOMAIN_MAX 2 2 2\n{}", identity(2))),
            ("domain min", format!("DOMAIN_MIN -1 0 0\n{}", identity(2))),
            ("domain short", format!("DOMAIN_MIN 0 0\n{}", identity(2))),
            ("keyword", "WHAT 1\nLUT_3D_SIZE 2\n".into()),
            ("too few", "LUT_3D_SIZE 2\n0 0 0\n".into()),
            ("too many", format!("{}0 0 0\n", identity(2))),
            ("1d too few", "LUT_1D_SIZE 3\n0 0 0\n".into()),
            ("nan", "LUT_3D_SIZE 2\nNaN 0 0\n".into()),
            ("inf", "LUT_1D_SIZE 2\ninf 0 0\n0 0 0\n".into()),
            ("two values", "LUT_3D_SIZE 2\n0 0\n".into()),
            ("four values", "LUT_3D_SIZE 2\n0 0 0 0\n".into()),
            ("word value", "LUT_3D_SIZE 2\n0 x 0\n".into()),
        ];
        for (name, t) in bad {
            let r = parse(t.as_bytes());
            assert!(matches!(r, Err(LutError::Invalid(_))), "{name}: {r:?}");
            assert!(!r.unwrap_err().to_string().is_empty());
        }
        assert_eq!(parse(&[0xff, 0xfe, 0x00]), Err(LutError::NotText));
        let big = vec![b'#'; MAX_BYTES as usize + 1];
        assert_eq!(parse(&big), Err(LutError::TooLarge));
    }

    // covers: REQ-136
    #[test]
    fn values_clamped_and_bom_comments_accepted() {
        let t = "\u{feff}LUT_3D_SIZE 2 # edge\n".to_string()
            + &identity(2)
                .replace("LUT_3D_SIZE 2\n", "")
                .replace("1 0 0", "5 -3 0.5");
        let l = parse(t.as_bytes()).unwrap();
        assert_eq!(l.at(1, 0, 0), [1.0, 0.0, 0.5]);
        let l2 =
            parse(b"DOMAIN_MIN 0 0 0\nDOMAIN_MAX 1 1 1\nLUT_1D_SIZE 2\n0 0 0\n1 1 1\n").unwrap();
        assert_eq!(l2.size(), CUBE_FROM_1D);
    }

    // covers: REQ-136
    #[test]
    fn random_bytes_never_panic() {
        let mut x: u64 = 0x1234_5678_9abc_def1;
        let mut next = || {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            x
        };
        let words = [
            "LUT_3D_SIZE",
            "LUT_1D_SIZE",
            "DOMAIN_MIN",
            "TITLE",
            "0.5",
            "1",
            "-",
            "nan",
            "\n",
            " ",
            "#",
        ];
        for _ in 0..400 {
            let mut buf = Vec::new();
            for _ in 0..(next() % 60) {
                if next() % 3 == 0 {
                    buf.push((next() & 0xff) as u8);
                } else {
                    buf.extend_from_slice(words[(next() % words.len() as u64) as usize].as_bytes());
                    buf.push(b' ');
                }
            }
            let _ = parse(&buf);
        }
    }

    // covers: REQ-136
    #[test]
    fn read_files_and_errors() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("a.cube");
        std::fs::write(&p, identity(3)).unwrap();
        assert_eq!(read(&p).unwrap().size(), 3);
        assert!(matches!(
            read(&d.path().join("none.cube")),
            Err(LutError::Read(_))
        ));
        assert_eq!(read(d.path()), Err(LutError::NotRegular));
        let f = std::fs::File::create(d.path().join("big.cube")).unwrap();
        f.set_len(MAX_BYTES + 1).unwrap();
        assert_eq!(read(&d.path().join("big.cube")), Err(LutError::TooLarge));
    }

    // covers: REQ-136
    #[test]
    fn cache_returns_the_same_table_until_the_file_changes() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("c.cube");
        std::fs::write(&p, identity(2)).unwrap();
        let path = p.to_string_lossy().to_string();
        let a = cached(&path).unwrap();
        assert!(Arc::ptr_eq(&a, &cached(&path).unwrap()));
        std::fs::write(&p, identity(3)).unwrap();
        assert_eq!(cached(&path).unwrap().size(), 3);
        std::fs::write(&p, "broken").unwrap();
        assert!(matches!(cached(&path), Err(LutError::Invalid(_))));
        assert!(matches!(cached("/no/such.cube"), Err(LutError::Read(_))));
        // the cache keeps at most 8 files
        for i in 0..10 {
            let q = d.path().join(format!("{i}.cube"));
            std::fs::write(&q, identity(2)).unwrap();
            cached(&q.to_string_lossy()).unwrap();
        }
        assert!(CACHE.lock().unwrap().len() <= 8);
    }

    #[test]
    fn path_rules_and_names() {
        assert!(path_ok("/a/b.cube"));
        assert!(!path_ok(""));
        assert!(!path_ok("a\0b"));
        assert!(!path_ok(&"x".repeat(MAX_PATH_CHARS + 1)));
        assert_eq!(display_name("/a/b/warm.cube"), "warm.cube");
    }
}
