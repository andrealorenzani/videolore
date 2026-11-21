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

    /// The table with only `percent` of its effect (REQ-289): every entry is blended with the
    /// colour it was given for (the identity table). 100 is the table itself and 0 changes
    /// nothing.
    pub fn with_strength(&self, percent: u8) -> Lut {
        if percent >= 100 {
            return self.clone();
        }
        let k = f32::from(percent) / 100.0;
        let n = self.size;
        let last = (n - 1) as f32;
        let mut data = Vec::with_capacity(self.data.len());
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    let id = [r as f32 / last, g as f32 / last, b as f32 / last];
                    let v = self.at(r, g, b);
                    data.push([
                        id[0] + (v[0] - id[0]) * k,
                        id[1] + (v[1] - id[1]) * k,
                        id[2] + (v[2] - id[2]) * k,
                    ]);
                }
            }
        }
        Lut::new(n, data)
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
    let mut domain = Domain::default();
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
                    let vals: Vec<f32> = rest.iter().filter_map(|t| t.parse().ok()).collect();
                    if vals.len() != 3
                        || rest.len() != 3
                        || vals.iter().any(|v| !v.is_finite() || v.abs() > MAX_DOMAIN)
                    {
                        return invalid(format!("line {}: bad domain", n + 1));
                    }
                    let slot = if first == "DOMAIN_MIN" {
                        &mut domain.min
                    } else {
                        &mut domain.max
                    };
                    *slot = [vals[0], vals[1], vals[2]];
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
    domain.check()?;
    match (size3, size1) {
        (Some(s), None) => {
            if rows.len() != s * s * s {
                return invalid(format!("{} entries, {} expected", rows.len(), s * s * s));
            }
            Ok(Lut::new(s, domain.apply(s, rows)))
        }
        (None, Some(s)) => {
            if rows.len() != s {
                return invalid(format!("{} entries, {s} expected", rows.len()));
            }
            Ok(domain.apply_lut(from_1d(&rows)))
        }
        _ => invalid("no LUT_3D_SIZE or LUT_1D_SIZE line"),
    }
}

/// Largest absolute DOMAIN_MIN or DOMAIN_MAX value accepted.
pub const MAX_DOMAIN: f32 = 1000.0;

/// The range of input values a `.cube` file covers (`DOMAIN_MIN` and `DOMAIN_MAX`, REQ-291).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Domain {
    min: [f32; 3],
    max: [f32; 3],
}

impl Default for Domain {
    fn default() -> Self {
        Domain {
            min: [0.0; 3],
            max: [1.0; 3],
        }
    }
}

impl Domain {
    /// The minimum must be below the maximum in every channel.
    fn check(&self) -> Result<(), LutError> {
        if (0..3).any(|c| self.min[c] >= self.max[c]) {
            return invalid("DOMAIN_MIN must be below DOMAIN_MAX");
        }
        Ok(())
    }

    fn is_default(&self) -> bool {
        *self == Domain::default()
    }

    /// The table for pixel values 0..=1: a pixel value `x` is looked up at
    /// `(x - min) / (max - min)` of the file's table.
    fn apply(&self, size: usize, rows: Vec<[f32; 3]>) -> Vec<[f32; 3]> {
        if self.is_default() {
            return rows;
        }
        let at = |c: usize, i: usize| {
            let x = i as f32 / (size - 1) as f32;
            ((x - self.min[c]) / (self.max[c] - self.min[c])).clamp(0.0, 1.0)
        };
        let mut out = Vec::with_capacity(rows.len());
        for b in 0..size {
            for g in 0..size {
                for r in 0..size {
                    out.push(trilinear(&rows, size, [at(0, r), at(1, g), at(2, b)]));
                }
            }
        }
        out
    }

    fn apply_lut(&self, lut: Lut) -> Lut {
        if self.is_default() {
            return lut;
        }
        Lut::new(lut.size, self.apply(lut.size, lut.data))
    }
}

/// Trilinear lookup of the table `rows` (red fastest, edge `size`) at `u` in 0..=1 per
/// channel.
fn trilinear(rows: &[[f32; 3]], size: usize, u: [f32; 3]) -> [f32; 3] {
    let last = (size - 1) as f32;
    let pos = u.map(|v| v.clamp(0.0, 1.0) * last);
    let lo = pos.map(|p| (p.floor() as usize).min(size - 2));
    let f = [
        pos[0] - lo[0] as f32,
        pos[1] - lo[1] as f32,
        pos[2] - lo[2] as f32,
    ];
    let mut out = [0f32; 3];
    for corner in 0..8usize {
        let (dr, dg, db) = (corner & 1, (corner >> 1) & 1, (corner >> 2) & 1);
        let w = (if dr == 1 { f[0] } else { 1.0 - f[0] })
            * (if dg == 1 { f[1] } else { 1.0 - f[1] })
            * (if db == 1 { f[2] } else { 1.0 - f[2] });
        let v = rows[(lo[0] + dr) + (lo[1] + dg) * size + (lo[2] + db) * size * size];
        for c in 0..3 {
            out[c] += w * v[c];
        }
    }
    out
}

/// Parses an Autodesk/Lustre `.3dl` file (REQ-290): an optional first line of input
/// points (whose count is the edge of the cube), then one row of three integers per grid
/// point, blue changing fastest. The output depth (8, 10, 12 or 16 bits) is the one that
/// holds the largest value in the file.
pub fn parse_3dl(bytes: &[u8]) -> Result<Lut, LutError> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(LutError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| LutError::NotText)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines: Vec<Vec<f32>> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with("3DMESH")
            || line.starts_with("Mesh")
            || line.starts_with("gamma")
        {
            continue;
        }
        let nums: Result<Vec<f32>, _> = line.split_whitespace().map(str::parse::<f32>).collect();
        let nums = nums.map_err(|_| LutError::Invalid(format!("line {}: not a number", n + 1)))?;
        if nums.iter().any(|v| !v.is_finite() || *v < 0.0) {
            return invalid(format!("line {}: value is not valid", n + 1));
        }
        if lines.len() > MAX_3D * MAX_3D * MAX_3D {
            return invalid("too many entries");
        }
        lines.push(nums);
    }
    // a first line of input points (strictly increasing, as many as the cube's edge) is not a row
    let header = match lines.first() {
        Some(first)
            if first.len() >= 2
                && first.windows(2).all(|w| w[0] < w[1])
                && lines.len() - 1 == first.len().pow(3) =>
        {
            Some(first.len())
        }
        _ => None,
    };
    let body = &lines[usize::from(header.is_some())..];
    let mut max = 0f32;
    let mut rows: Vec<[f32; 3]> = Vec::with_capacity(body.len());
    for (n, v) in body.iter().enumerate() {
        if v.len() != 3 {
            return invalid(format!("entry {}: expected three values", n + 1));
        }
        max = max.max(v[0]).max(v[1]).max(v[2]);
        rows.push([v[0], v[1], v[2]]);
    }
    let size = (rows.len() as f64).cbrt().round() as usize;
    if !(2..=MAX_3D).contains(&size) || size * size * size != rows.len() {
        return invalid(format!("{} entries are not a cube", rows.len()));
    }
    let depth = [255f32, 1023.0, 4095.0, 65535.0]
        .into_iter()
        .find(|d| max <= *d)
        .ok_or_else(|| LutError::Invalid("values larger than 16 bits".into()))?;
    // the file has blue fastest; the cube here has red fastest
    let mut data = vec![[0f32; 3]; rows.len()];
    for (i, v) in rows.iter().enumerate() {
        let (r, g, b) = (i / (size * size), (i / size) % size, i % size);
        data[r + g * size + b * size * size] = v.map(|x| (x / depth).clamp(0.0, 1.0));
    }
    Ok(Lut::new(size, data))
}

/// Parses a Cinespace `.csp` file (REQ-290): the three input tables (applied before the
/// cube, piecewise linear), then the cube, red changing fastest.
pub fn parse_csp(bytes: &[u8]) -> Result<Lut, LutError> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(LutError::TooLarge);
    }
    let text = std::str::from_utf8(bytes).map_err(|_| LutError::NotText)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .enumerate();
    let bad = |n: usize, what: &str| LutError::Invalid(format!("line {}: {what}", n + 1));
    match lines.next() {
        Some((_, l)) if l.starts_with("CSPLUTV100") => {}
        _ => return invalid("not a CSP file"),
    }
    match lines.next() {
        Some((_, "3D")) => {}
        _ => return invalid("only 3D CSP tables are supported"),
    }
    // optional metadata block
    let mut rest: Vec<(usize, &str)> = Vec::new();
    let mut in_meta = false;
    for (n, l) in lines {
        match l {
            "BEGIN METADATA" => in_meta = true,
            "END METADATA" => in_meta = false,
            _ if in_meta => {}
            _ => rest.push((n, l)),
        }
    }
    let mut it = rest.into_iter();
    let num =
        |it: &mut std::vec::IntoIter<(usize, &str)>, count: usize| -> Result<Vec<f32>, LutError> {
            let (n, l) = it
                .next()
                .ok_or_else(|| LutError::Invalid("the file ends too early".into()))?;
            let v: Result<Vec<f32>, _> = l.split_whitespace().map(str::parse::<f32>).collect();
            let v = v.map_err(|_| bad(n, "not a number"))?;
            if v.len() != count || v.iter().any(|x| !x.is_finite()) {
                return Err(bad(n, "unexpected values"));
            }
            Ok(v)
        };
    let mut shapers: Vec<(Vec<f32>, Vec<f32>)> = Vec::new();
    for _ in 0..3 {
        let count = num(&mut it, 1)?[0];
        if !(2.0..=4096.0).contains(&count) || count.fract() != 0.0 {
            return invalid("bad input table size");
        }
        let n = count as usize;
        let xs = num(&mut it, n)?;
        let ys = num(&mut it, n)?;
        if xs.windows(2).any(|w| w[0] >= w[1]) {
            return invalid("an input table is not increasing");
        }
        shapers.push((xs, ys));
    }
    let dims = num(&mut it, 3)?;
    let size = dims[0] as usize;
    if dims.iter().any(|d| *d as usize != size) || !(2..=MAX_3D).contains(&size) {
        return invalid("only cubes with equal edges of 2 to 65 are supported");
    }
    let mut rows: Vec<[f32; 3]> = Vec::with_capacity(size * size * size);
    for _ in 0..size * size * size {
        let v = num(&mut it, 3)?;
        rows.push([
            v[0].clamp(0.0, 1.0),
            v[1].clamp(0.0, 1.0),
            v[2].clamp(0.0, 1.0),
        ]);
    }
    if it.next().is_some() {
        return invalid("more entries than the declared size");
    }
    // the input tables: a pixel value goes through its table, then into the cube
    let through = |c: usize, x: f32| -> f32 {
        let (xs, ys) = &shapers[c];
        if x <= xs[0] {
            return ys[0];
        }
        for i in 1..xs.len() {
            if x <= xs[i] {
                let f = (x - xs[i - 1]) / (xs[i] - xs[i - 1]);
                return ys[i - 1] + f * (ys[i] - ys[i - 1]);
            }
        }
        ys[ys.len() - 1]
    };
    let identity = shapers
        .iter()
        .all(|(xs, ys)| xs.len() == 2 && xs[0] == 0.0 && xs[1] == 1.0 && ys == xs);
    if identity {
        return Ok(Lut::new(size, rows));
    }
    let g = |i: usize| i as f32 / (size - 1) as f32;
    let mut data = Vec::with_capacity(rows.len());
    for b in 0..size {
        for gg in 0..size {
            for r in 0..size {
                let u = [through(0, g(r)), through(1, g(gg)), through(2, g(b))];
                data.push(trilinear(&rows, size, u));
            }
        }
    }
    Ok(Lut::new(size, data))
}

/// Parses LUT bytes by the file's extension: `.cube`, `.3dl` or `.csp` (REQ-290).
pub fn parse_by_extension(ext: &str, bytes: &[u8]) -> Result<Lut, LutError> {
    match ext.to_ascii_lowercase().as_str() {
        "3dl" => parse_3dl(bytes),
        "csp" => parse_csp(bytes),
        _ => parse(bytes),
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
    let bytes = crate::base::io::read_limited(path, MAX_BYTES).map_err(|e| match e.kind() {
        ErrorKind::InvalidData => LutError::TooLarge,
        ErrorKind::InvalidInput => LutError::NotRegular,
        _ => LutError::Read(e.to_string()),
    })?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("cube");
    parse_by_extension(ext, &bytes)
}

type CacheEntry = (String, u64, Option<std::time::SystemTime>, Arc<Lut>);

/// Parsed LUTs by path, re-read when the file's size or modification time changes, so the
/// preview and the export do not parse a large cube again and again (at most 8 kept).
static CACHE: Mutex<Vec<CacheEntry>> = Mutex::new(Vec::new());

/// LUT tables kept in the projects opened in this run, by the path the clips use (REQ-293):
/// used when the file itself is gone.
static EMBEDDED: Mutex<Vec<(String, Arc<Lut>)>> = Mutex::new(Vec::new());

/// Remembers the copy of a LUT that a project carries (at most 16 are kept).
pub fn embed(path: &str, lut: Lut) {
    let mut e = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner());
    e.retain(|x| x.0 != path);
    if e.len() >= 16 {
        e.remove(0);
    }
    e.push((path.to_string(), Arc::new(lut)));
}

/// True when a copy of the LUT at `path` is kept from an opened project.
pub fn has_embedded(path: &str) -> bool {
    EMBEDDED
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .any(|x| x.0 == path)
}

/// [`read`] through the cache. A changed file is read and validated again.
pub fn cached(path: &str) -> Result<Arc<Lut>, LutError> {
    let meta = match std::fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            // the file is gone: the copy kept in the project is used (REQ-293)
            let embedded = EMBEDDED.lock().unwrap_or_else(|e| e.into_inner());
            return match embedded.iter().find(|x| x.0 == path) {
                Some(x) => Ok(x.1.clone()),
                None => Err(LutError::Read(e.to_string())),
            };
        }
    };
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
            ("domain max 0", format!("DOMAIN_MAX 0 0 0\n{}", identity(2))),
            ("domain min 2", format!("DOMAIN_MIN 2 0 0\n{}", identity(2))),
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

    fn fixture(name: &str) -> Vec<u8> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lut")
            .join(name);
        std::fs::read(p).unwrap()
    }

    /// The most two tables differ by at any grid point, in levels of 8 bits.
    fn levels_apart(a: &Lut, b: &Lut) -> f32 {
        assert_eq!(a.size(), b.size());
        let n = a.size();
        let mut worst = 0f32;
        for r in 0..n {
            for g in 0..n {
                for bl in 0..n {
                    let (x, y) = (a.at(r, g, bl), b.at(r, g, bl));
                    for c in 0..3 {
                        worst = worst.max((x[c] - y[c]).abs() * 255.0);
                    }
                }
            }
        }
        worst
    }

    // covers: REQ-290
    #[test]
    fn a_3dl_and_a_csp_file_make_the_same_table_as_the_cube() {
        let cube = parse(&fixture("look.cube")).unwrap();
        for (name, ext) in [("look.3dl", "3dl"), ("look.csp", "CSP")] {
            let lut = parse_by_extension(ext, &fixture(name)).unwrap();
            assert!(levels_apart(&cube, &lut) <= 2.0, "{name}");
        }
        // through the file reader: the extension decides
        let d = tempfile::tempdir().unwrap();
        for name in ["look.3dl", "look.csp", "look.cube"] {
            std::fs::write(d.path().join(name), fixture(name)).unwrap();
            assert_eq!(read(&d.path().join(name)).unwrap().size(), 3, "{name}");
        }
    }

    // covers: REQ-290
    #[test]
    fn broken_3dl_and_csp_files_are_refused_with_a_message() {
        for (ext, name) in [("3dl", "broken.3dl"), ("csp", "broken.csp")] {
            match parse_by_extension(ext, &fixture(name)) {
                Err(LutError::Invalid(m)) => assert!(!m.is_empty()),
                other => panic!("{name}: {other:?}"),
            }
        }
        for (ext, bad) in [
            ("3dl", "0 1 2\n9999999 0 0\n"),
            ("3dl", "0 1\n1 2 3\n4 5 6\n"),
            ("3dl", "x y z\n"),
            ("csp", "not a csp"),
            ("csp", "CSPLUTV100\n1D\n"),
            ("csp", "CSPLUTV100\n3D\n2\n1 0\n0 1\n"),
        ] {
            assert!(
                parse_by_extension(ext, bad.as_bytes()).is_err(),
                "{ext} {bad:?}"
            );
        }
        assert_eq!(
            parse_by_extension("3dl", &[0xff, 0xfe]),
            Err(LutError::NotText)
        );
        // a csp input table is applied: a table that doubles the input
        let mut csp = String::from("CSPLUTV100\n3D\n");
        for _ in 0..3 {
            csp.push_str("2\n0.0 1.0\n0.0 0.5\n");
        }
        csp.push_str("2 2 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    csp.push_str(&format!("{r} {g} {b}\n"));
                }
            }
        }
        let lut = parse_csp(csp.as_bytes()).unwrap();
        assert_eq!(lut.at(1, 1, 1), [0.5, 0.5, 0.5]);
    }

    // covers: REQ-291
    #[test]
    fn a_domain_scales_the_input_and_a_minimum_not_below_the_maximum_is_refused() {
        // a ramp over the domain 0..2: a pixel of value 1 is half way through the table,
        // so the output is 0.5 where the plain table would give 1
        let lut = parse(&fixture("domain.cube")).unwrap();
        assert_eq!(lut.size(), 5);
        let n = lut.size() - 1;
        for i in 0..=n {
            let x = i as f32 / n as f32;
            let want = x / 2.0;
            let got = lut.at(i, i, i);
            for c in got {
                assert!((c - want).abs() < 0.005, "{i}: {c} vs {want}");
            }
        }
        // the same table written without a domain and with the input scaled by hand
        let plain = parse(identity(3).as_bytes()).unwrap();
        assert_eq!(plain.at(2, 2, 2), [1.0, 1.0, 1.0]);
        for bad in [
            "DOMAIN_MIN 1 0 0\nDOMAIN_MAX 1 1 1\nLUT_3D_SIZE 2\n",
            "DOMAIN_MIN 0 0 0\nDOMAIN_MAX 0 0 0\nLUT_3D_SIZE 2\n",
            "DOMAIN_MIN 2 2 2\nDOMAIN_MAX 1 1 1\nLUT_3D_SIZE 2\n",
            "DOMAIN_MIN 0 0\nDOMAIN_MAX 1 1 1\nLUT_3D_SIZE 2\n",
            "DOMAIN_MIN 0 0 0\nDOMAIN_MAX 5000 1 1\nLUT_3D_SIZE 2\n",
        ] {
            let text = format!("{bad}{}", "0 0 0\n".repeat(8));
            assert!(
                matches!(parse(text.as_bytes()), Err(LutError::Invalid(_))),
                "{bad}"
            );
        }
    }

    // covers: REQ-289
    #[test]
    fn the_strength_blends_the_table_with_no_change() {
        let lut = parse(&fixture("look.cube")).unwrap();
        let none = lut.with_strength(0);
        let id = parse(identity(3).as_bytes()).unwrap();
        assert!(levels_apart(&none, &id) <= 0.5, "0 % changes nothing");
        assert_eq!(lut.with_strength(100), lut, "100 % is the table");
        let half = lut.with_strength(50);
        let n = lut.size();
        for r in 0..n {
            for g in 0..n {
                for b in 0..n {
                    let (a, c, h) = (id.at(r, g, b), lut.at(r, g, b), half.at(r, g, b));
                    for k in 0..3 {
                        let avg = (a[k] + c[k]) / 2.0;
                        assert!((h[k] - avg).abs() * 255.0 <= 2.0, "{r}{g}{b}{k}");
                    }
                }
            }
        }
        assert_ne!(
            half.hash(),
            lut.hash(),
            "another table, another file for ffmpeg"
        );
    }
}
