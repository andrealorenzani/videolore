// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The streamed `.cube` reader (2.1.0, ADR-203): one pass over the file, one line at a time
//! in a buffer of at most 256 bytes, values quantised to 16 bits as they are read. The text
//! of the file is never held, so a 450 MB file of 256 points an edge needs only its table.
//! Imports std, thiserror and the types of the parent module only.

use std::io::{BufRead, Read};

use super::{
    Domain, Lut, LutError, MAX_1D, MAX_3D, MAX_BYTES, MAX_DOMAIN, MAX_LINE, from_1d, invalid, q16,
};

/// Parses `.cube` bytes (a 3D or a 1D LUT) into a 3D cube.
pub fn parse(bytes: &[u8]) -> Result<Lut, LutError> {
    if bytes.len() as u64 > MAX_BYTES {
        return Err(LutError::TooLarge);
    }
    parse_reader(std::io::Cursor::new(bytes), bytes.len() as u64, &mut |_| {
        true
    })
}

/// Parses a `.cube` stream of about `len_hint` bytes. `progress` gets the bytes read so far
/// every MiB and stops the reading (`LutError::Cancelled`) when it returns `false`.
pub fn parse_reader(
    mut r: impl BufRead,
    len_hint: u64,
    progress: &mut dyn FnMut(u64) -> bool,
) -> Result<Lut, LutError> {
    let mut size3: Option<usize> = None;
    let mut size1: Option<usize> = None;
    let mut rows: Vec<[u16; 3]> = Vec::new();
    let mut domain = Domain::default();
    let mut buf: Vec<u8> = Vec::with_capacity(MAX_LINE + 2);
    let (mut n, mut read, mut next_report) = (0usize, 0u64, 1u64 << 20);
    loop {
        buf.clear();
        let got = (&mut r)
            .take(MAX_LINE as u64 + 2)
            .read_until(b'\n', &mut buf)
            .map_err(|e| LutError::Read(e.to_string()))?;
        if got == 0 {
            break;
        }
        n += 1;
        read += got as u64;
        if read >= next_report {
            next_report = read + (1 << 20);
            if !progress(read) {
                return Err(LutError::Cancelled);
            }
        }
        while matches!(buf.last(), Some(b'\n' | b'\r')) {
            buf.pop();
        }
        if buf.len() > MAX_LINE {
            return invalid(format!("line {n} is longer than {MAX_LINE} bytes"));
        }
        let text = std::str::from_utf8(&buf).map_err(|_| LutError::NotText)?;
        let text = if n == 1 {
            text.strip_prefix('\u{feff}').unwrap_or(text)
        } else {
            text
        };
        let line = text.split('#').next().unwrap_or("").trim();
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
                        return invalid(format!("line {n}: bad size"));
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
                        return invalid(format!("line {n}: size given twice"));
                    }
                    if v < lo || v > hi {
                        return invalid(format!("size {v} is outside {lo}..={hi}"));
                    }
                    *slot = Some(v);
                    let want = size3.map(|s| s * s * s).or(size1).unwrap_or(0);
                    // a file that cannot hold the entries does not get the room for them
                    rows.reserve_exact(want.min((len_hint / 6) as usize + 1));
                }
                "DOMAIN_MIN" | "DOMAIN_MAX" => {
                    let vals: Vec<f32> = rest.iter().filter_map(|t| t.parse().ok()).collect();
                    if vals.len() != 3
                        || rest.len() != 3
                        || vals.iter().any(|v| !v.is_finite() || v.abs() > MAX_DOMAIN)
                    {
                        return invalid(format!("line {n}: bad domain"));
                    }
                    let slot = if first == "DOMAIN_MIN" {
                        &mut domain.min
                    } else {
                        &mut domain.max
                    };
                    *slot = [vals[0], vals[1], vals[2]];
                }
                other => {
                    return invalid(format!(
                        "line {n}: unknown keyword {:?}",
                        super::clip(other)
                    ));
                }
            }
            continue;
        }
        let mut v = [0u16; 3];
        let mut count = 0;
        for tok in std::iter::once(first).chain(it) {
            if count == 3 {
                return invalid(format!("line {n}: more than three values"));
            }
            let x: f32 = tok
                .parse()
                .map_err(|_| LutError::Invalid(format!("line {n}: not a number")))?;
            if !x.is_finite() {
                return invalid(format!("line {n}: value is not finite"));
            }
            v[count] = q16(x);
            count += 1;
        }
        if count != 3 {
            return invalid(format!("line {n}: expected three values"));
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
            if domain.is_default() {
                Ok(Lut::from_u16(s, rows))
            } else {
                let f = rows
                    .iter()
                    .map(|v| v.map(|c| f32::from(c) / 65535.0))
                    .collect();
                Ok(Lut::new(s, domain.apply(s, f)))
            }
        }
        (None, Some(s)) => {
            if rows.len() != s {
                return invalid(format!("{} entries, {s} expected", rows.len()));
            }
            let f: Vec<[f32; 3]> = rows
                .iter()
                .map(|v| v.map(|c| f32::from(c) / 65535.0))
                .collect();
            Ok(domain.apply_lut(from_1d(&f)))
        }
        _ => invalid("no LUT_3D_SIZE or LUT_1D_SIZE line"),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::{BufReader, Read};

    /// A generated `.cube` file of `n` points an edge, read without being held in memory.
    pub(crate) struct Gen {
        n: usize,
        next: usize,
        pending: Vec<u8>,
        pos: usize,
    }

    impl Gen {
        pub(crate) fn new(n: usize) -> Gen {
            Gen {
                n,
                next: 0,
                pending: format!("TITLE gen\nLUT_3D_SIZE {n}\n").into_bytes(),
                pos: 0,
            }
        }
    }

    impl Read for Gen {
        fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
            let mut put = 0;
            while put < out.len() {
                if self.pos == self.pending.len() {
                    if self.next == self.n * self.n * self.n {
                        break;
                    }
                    let (r, g, b) = (
                        self.next % self.n,
                        self.next / self.n % self.n,
                        self.next / self.n / self.n,
                    );
                    let m = (self.n - 1) as f32;
                    // a sepia-like table: fixed digits keep the lines 27 bytes long
                    self.pending = format!(
                        "{:.6} {:.6} {:.6}\n",
                        r as f32 / m,
                        g as f32 / m * 0.9,
                        b as f32 / m * 0.7
                    )
                    .into_bytes();
                    self.pos = 0;
                    self.next += 1;
                }
                let k = (self.pending.len() - self.pos).min(out.len() - put);
                out[put..put + k].copy_from_slice(&self.pending[self.pos..self.pos + k]);
                self.pos += k;
                put += k;
            }
            Ok(put)
        }
    }

    // covers: REQ-300
    #[test]
    fn a_144_point_cube_of_the_size_of_the_maintainers_files_is_read_in_a_stream() {
        let n = 144;
        let mut last = 0;
        let lut = parse_reader(BufReader::new(Gen::new(n)), 80_000_000, &mut |r| {
            last = r;
            true
        })
        .unwrap();
        assert_eq!(lut.size(), n);
        assert!(last > 70_000_000, "progress reported up to {last}");
        assert_eq!(lut.at(n - 1, 0, 0)[0], 1.0);
        let g = lut.at(0, n - 1, 0)[1];
        assert!((g - 0.9).abs() < 1e-4, "{g}");
        assert_eq!(lut.bytes(), n * n * n * 6);
        assert_eq!(lut.cube_len(), "LUT_3D_SIZE 144\n".len() + n * n * n * 27);
    }

    // covers: REQ-300
    #[test]
    fn the_edge_limit_is_256_points() {
        let ok = "LUT_3D_SIZE 256\n0 0 0\n";
        let e = parse(ok.as_bytes()).unwrap_err().to_string();
        assert!(e.contains("16777216 expected"), "{e}");
        let e = parse(b"LUT_3D_SIZE 257\n").unwrap_err().to_string();
        assert!(e.contains("outside 2..=256"), "{e}");
    }

    // covers: REQ-300
    #[test]
    fn a_broken_line_is_refused_with_its_number_and_a_stop_is_honoured() {
        let mut t = String::from("TITLE x\nLUT_3D_SIZE 2\n");
        t.push_str("0 0 0\n1 0 0\n0 1 0\nnot a number\n");
        let e = parse(t.as_bytes()).unwrap_err().to_string();
        assert!(e.contains("line 6"), "{e}");
        let r = parse_reader(BufReader::new(Gen::new(40)), 2_000_000, &mut |_| false);
        assert_eq!(r, Err(LutError::Cancelled));
    }

    // covers: REQ-300
    #[test]
    fn random_streams_never_panic() {
        let mut x = 12345u64;
        for _ in 0..200 {
            let mut v = Vec::new();
            for _ in 0..(x % 400) {
                x = x
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                v.push((x >> 33) as u8);
            }
            let _ = parse(&v);
        }
    }

    // covers: REQ-300
    #[test]
    fn the_maintainers_luts_are_read_when_the_folder_exists() {
        let dir = std::path::Path::new("/home/andrea/Videos/tools/LUTS/My_Creations");
        let Ok(rd) = std::fs::read_dir(dir) else {
            return; // read only, optional: the repository tests use generated tables
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("cube") {
                continue;
            }
            let lut = crate::lut::read(&p).unwrap();
            assert!(lut.size() >= 2, "{p:?}");
        }
    }
}
