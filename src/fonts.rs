// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The fonts installed on this computer, for the font choice of text items (REQ-108). The
//! scan runs on a worker thread so the window never waits for it; only family names are
//! kept, and only those that are safe to write into an ASS file ([`crate::text::valid_font`]).

use crate::text::{DEFAULT_FONT, valid_font};
use std::sync::{Arc, Mutex};

/// Family names found by `db`, sorted, without duplicates, safe to use. The font of the
/// bundled subtitle font comes first.
pub fn families_of(names: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut v: Vec<String> = names
        .into_iter()
        .filter(|n| !n.is_empty() && valid_font(n))
        .collect();
    v.sort_by_cached_key(|n| n.to_lowercase());
    v.dedup();
    v.retain(|n| n != DEFAULT_FONT);
    v.insert(0, DEFAULT_FONT.to_string());
    v
}

/// The usable families of a font database.
pub fn families_in(db: &fontdb::Database) -> Vec<String> {
    families_of(
        db.faces()
            .flat_map(|f| f.families.iter().map(|(n, _)| n.clone())),
    )
}

/// Scans the system fonts (blocking).
pub fn scan() -> Vec<String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    families_in(&db)
}

/// The list of families, filled by a background scan.
#[derive(Debug, Clone, Default)]
pub struct Fonts {
    list: Arc<Mutex<Option<Vec<String>>>>,
}

impl Fonts {
    /// Starts the scan on a worker thread.
    pub fn start() -> Fonts {
        Fonts::start_with(scan)
    }

    /// Starts a scan that is made by `scan`.
    pub fn start_with(scan: impl FnOnce() -> Vec<String> + Send + 'static) -> Fonts {
        let f = Fonts::default();
        let list = f.list.clone();
        std::thread::spawn(move || {
            let v = scan();
            if let Ok(mut l) = list.lock() {
                *l = Some(v);
            }
        });
        f
    }

    /// A list that is already known (tests).
    pub fn with(list: Vec<String>) -> Fonts {
        Fonts {
            list: Arc::new(Mutex::new(Some(families_of(list)))),
        }
    }

    /// The families, or `None` while the scan runs.
    pub fn families(&self) -> Option<Vec<String>> {
        self.list.lock().ok().and_then(|l| l.clone())
    }

    /// False only when the scan has finished and does not know `name` (the bundled font is
    /// always there; an empty name means it).
    pub fn is_installed(&self, name: &str) -> bool {
        name.is_empty()
            || name == DEFAULT_FONT
            || self.families().is_none_or(|l| l.iter().any(|f| f == name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-108
    #[test]
    fn families_are_sorted_safe_and_start_with_the_bundled_font() {
        let v = families_of(
            [
                "zeta",
                "Alpha",
                "bad;name",
                "",
                "alpha b",
                "Noto Sans",
                "Alpha",
                "x\ny",
            ]
            .map(String::from),
        );
        assert_eq!(v, ["Noto Sans", "Alpha", "alpha b", "zeta"]);
    }

    // covers: REQ-108
    #[test]
    fn a_missing_family_is_reported_only_after_the_scan() {
        let f = Fonts::default();
        assert!(f.families().is_none());
        assert!(f.is_installed("Anything"));
        let f = Fonts::with(vec!["Here".into()]);
        assert!(f.is_installed("Here") && f.is_installed("") && f.is_installed("Noto Sans"));
        assert!(!f.is_installed("Gone"));
        assert_eq!(f.families().unwrap().len(), 2);
    }

    // covers: REQ-108
    #[test]
    fn a_scan_of_a_font_folder_finds_its_families_on_a_worker_thread() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.ttf"), crate::ass::FONT).unwrap();
        let path = dir.path().to_path_buf();
        let f = Fonts::start_with(move || {
            let mut db = fontdb::Database::new();
            db.load_fonts_dir(&path);
            families_in(&db)
        });
        for _ in 0..600 {
            if f.families().is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let v = f.families().expect("scan finished");
        assert_eq!(v, [DEFAULT_FONT]);
        assert!(v.iter().all(|n| valid_font(n)));
    }
}
