// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The fonts installed on this computer, for the font choice of text items (REQ-108). The
//! scan runs on a worker thread so the window never waits for it; only family names are
//! kept, and only those that are safe to write into an ASS file ([`crate::base::font::valid_font`]).

use crate::base::font::{DEFAULT_FONT, Script, scripts_in, valid_font};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Largest font file that is read (REQ-166: a face is read only when a text uses it).
pub const MAX_FACE_BYTES: u64 = 64 * 1024 * 1024;
/// Most font files kept in memory at once.
pub const KEEP_FACES: usize = 8;

/// A font file that a text uses: its bytes, the face inside it and where it came from.
#[derive(Debug)]
pub struct Face {
    pub bytes: Vec<u8>,
    pub index: u32,
    /// The file (an installed font); `None` for the bundled font.
    pub file: Option<PathBuf>,
}

/// The fonts of this computer, kept after the scan (families and where their files are).
fn database() -> &'static Mutex<fontdb::Database> {
    static DB: OnceLock<Mutex<fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| Mutex::new(fontdb::Database::new()))
}

/// Faces read so far, by file and face index, newest last.
type Kept = Vec<(String, Arc<Face>)>;

/// The faces read so far (at most [`KEEP_FACES`] and [`MAX_FACE_BYTES`] in all).
fn kept() -> &'static Mutex<Kept> {
    static KEPT: OnceLock<Mutex<Kept>> = OnceLock::new();
    KEPT.get_or_init(|| Mutex::new(Vec::new()))
}

/// The bundled font as a face (it is always there).
pub fn bundled() -> Arc<Face> {
    static B: OnceLock<Arc<Face>> = OnceLock::new();
    B.get_or_init(|| {
        Arc::new(Face {
            bytes: crate::ass::FONT.to_vec(),
            index: 0,
            file: None,
        })
    })
    .clone()
}

/// Adds the fonts of a database to the ones this computer is known to have.
pub fn install(db: fontdb::Database) {
    let covered = coverage_of(&db);
    if let Ok(mut d) = database().lock() {
        for f in db.faces() {
            d.push_face_info(f.clone());
        }
    }
    if let Ok(mut c) = coverage().lock() {
        c.get_or_insert_with(BTreeSet::new).extend(covered);
    }
}

/// The scripts some installed font covers, known when the first scan has finished (`None`
/// before: no note is shown then, REQ-187).
fn coverage() -> &'static Mutex<Option<BTreeSet<Script>>> {
    static C: OnceLock<Mutex<Option<BTreeSet<Script>>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(None))
}

/// Runs `f` with the covered scripts set to `covered` (tests of the note): the tests that
/// do so run one after the other, and the real value is put back afterwards.
#[cfg(test)]
pub(crate) fn with_coverage<R>(covered: Option<BTreeSet<Script>>, f: impl FnOnce() -> R) -> R {
    static LOCK: Mutex<()> = Mutex::new(());
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let before = std::mem::replace(&mut *coverage().lock().unwrap(), covered);
    let r = f();
    *coverage().lock().unwrap() = before;
    r
}

/// True once a scan of the fonts has finished and the covered scripts are known.
pub fn scan_finished() -> bool {
    coverage().lock().is_ok_and(|c| c.is_some())
}

/// True when the face `index` of the font file `data` has a glyph for every one of `chars`.
pub fn font_covers(data: &[u8], index: u32, chars: &[char]) -> bool {
    let Ok(font) = skrifa::FontRef::from_index(data, index) else {
        return false;
    };
    let map = skrifa::MetadataProvider::charmap(&font);
    chars.iter().all(|c| map.map(*c).is_some())
}

/// The scripts that some face of `db` covers (a sample of each script's characters), read
/// from the faces' character maps; a script is looked for until one face has it, and a
/// file larger than [`MAX_FACE_BYTES`] is not read.
pub fn coverage_of(db: &fontdb::Database) -> BTreeSet<Script> {
    let mut found = BTreeSet::new();
    for script in Script::ALL {
        let has = db.faces().any(|f| {
            let big = match &f.source {
                fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => {
                    std::fs::metadata(p).is_ok_and(|m| m.len() > MAX_FACE_BYTES)
                }
                fontdb::Source::Binary(_) => false,
            };
            !big && db
                .with_face_data(f.id, |d, i| font_covers(d, i, script.sample()))
                .unwrap_or(false)
        });
        if has {
            found.insert(script);
        }
    }
    found
}

/// The scripts of `text` that no font covers: the bundled faces cover Arabic and
/// Devanagari, `installed` says which scripts an installed font covers (`None`: not known
/// yet, nothing is reported).
pub fn uncovered(text: &str, installed: Option<&BTreeSet<Script>>) -> Vec<Script> {
    let Some(installed) = installed else {
        return Vec::new();
    };
    scripts_in(text)
        .into_iter()
        .filter(|s| !s.is_bundled() && !installed.contains(s))
        .collect()
}

/// The note of REQ-187 for `scripts`, `None` when there are none.
pub fn note_for(scripts: &[Script]) -> Option<String> {
    if scripts.is_empty() {
        return None;
    }
    let names: Vec<&str> = scripts.iter().map(|s| s.name()).collect();
    Some(format!(
        "No installed font shows {} text; install a font for it (some characters may show \
         as empty boxes)",
        names.join(", ")
    ))
}

/// The note for the scripts of `text` that this computer has no font for.
pub fn missing_note(text: &str) -> Option<String> {
    missing_note_in(std::iter::once(text))
}

/// [`missing_note`] for several texts (one note naming every script that is missing).
pub fn missing_note_in<'a>(texts: impl IntoIterator<Item = &'a str>) -> Option<String> {
    let c = coverage().lock().ok()?;
    let mut scripts: Vec<Script> = Vec::new();
    for t in texts {
        for s in uncovered(t, c.as_ref()) {
            if !scripts.contains(&s) {
                scripts.push(s);
            }
        }
    }
    scripts.sort();
    note_for(&scripts)
}

/// The first installed face that covers `script` (read from its file), for the interface
/// to use as a fallback font; `None` when there is none.
pub fn face_for(script: Script) -> Option<Arc<Face>> {
    let (path, index) = {
        let db = database().lock().ok()?;
        let id = db.faces().find_map(|f| {
            let small = match &f.source {
                fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => {
                    std::fs::metadata(p).is_ok_and(|m| m.len() <= MAX_FACE_BYTES)
                }
                fontdb::Source::Binary(_) => false,
            };
            (small
                && db
                    .with_face_data(f.id, |d, i| font_covers(d, i, script.sample()))
                    .unwrap_or(false))
            .then_some(f.id)
        })?;
        let info = db.face(id)?;
        match &info.source {
            fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => (p.clone(), info.index),
            fontdb::Source::Binary(_) => return None,
        }
    };
    let bytes = std::fs::read(&path).ok()?;
    Some(Arc::new(Face {
        bytes,
        index,
        file: Some(path),
    }))
}

/// Adds the fonts found in a folder (tests; the scan does it for the system fonts).
pub fn install_dir(dir: &Path) {
    let mut db = fontdb::Database::new();
    db.load_fonts_dir(dir);
    install(db);
}

/// The installed face of family `name` (regular weight and style, the one the renderer is
/// given), read from its file on first use; `None` for the bundled font, for a family that
/// is not installed and for a file that cannot be read (the caller then uses the bundled
/// font, REQ-164).
pub fn resolve(name: &str) -> Option<Arc<Face>> {
    if name.is_empty() || name == DEFAULT_FONT || !valid_font(name) {
        return None;
    }
    let (path, index) = {
        let db = database().lock().ok()?;
        let id = db.query(&fontdb::Query {
            families: &[fontdb::Family::Name(name)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        })?;
        let info = db.face(id)?;
        match &info.source {
            fontdb::Source::File(p) | fontdb::Source::SharedFile(p, _) => (p.clone(), info.index),
            fontdb::Source::Binary(_) => return None,
        }
    };
    let key = format!("{}#{index}", path.display());
    if let Ok(mut k) = kept().lock()
        && let Some(i) = k.iter().position(|(n, _)| *n == key)
    {
        let e = k.remove(i);
        let face = e.1.clone();
        k.push(e);
        return Some(face);
    }
    let len = std::fs::metadata(&path).ok()?.len();
    if len > MAX_FACE_BYTES {
        return None;
    }
    let face = Arc::new(Face {
        bytes: std::fs::read(&path).ok()?,
        index,
        file: Some(path),
    });
    if let Ok(mut k) = kept().lock() {
        k.push((key, face.clone()));
        let mut total: usize = k.iter().map(|(_, f)| f.bytes.len()).sum();
        while k.len() > 1 && (k.len() > KEEP_FACES || total as u64 > MAX_FACE_BYTES) {
            total -= k.remove(0).1.bytes.len();
        }
    }
    Some(face)
}

/// The family names a document writes with `\fn` tags, without duplicates.
fn families_in_doc(doc: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for part in doc.split("\\fn").skip(1) {
        let name: String = part
            .chars()
            .take_while(|c| !matches!(c, '\\' | '}'))
            .collect();
        if valid_font(&name) && !v.contains(&name) {
            v.push(name);
        }
    }
    v
}

/// Most font files copied for one document.
const MAX_COPIES: usize = 16;

/// Copies the files of the installed fonts that an ASS document uses into `dir/fonts`, next
/// to the bundled font, so that the renderer finds exactly the face that was measured, on
/// every system (REQ-163, REQ-165). Names are made from the file's checksum, nothing the
/// project says becomes a path; a font that cannot be read is left out and the renderer
/// then uses the bundled font the document names for it.
pub fn copy_used(dir: &Path, doc: &str) -> std::io::Result<()> {
    use sha2::{Digest, Sha256};
    let mut made = false;
    for name in families_in_doc(doc).into_iter().take(MAX_COPIES) {
        let Some(face) = resolve(&name) else { continue };
        let Some(file) = &face.file else { continue };
        let ext = file
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| e.len() <= 4 && e.chars().all(|c| c.is_ascii_alphanumeric()))
            .unwrap_or("ttf");
        let sum = Sha256::digest(&face.bytes);
        let hex: String = sum.iter().take(8).map(|b| format!("{b:02x}")).collect();
        let target = dir
            .join(crate::ass::FONTS_DIR)
            .join(format!("f-{hex}.{ext}"));
        if !made {
            std::fs::create_dir_all(dir.join(crate::ass::FONTS_DIR))?;
            made = true;
        }
        if !target.is_file() {
            std::fs::write(target, &face.bytes)?;
        }
    }
    Ok(())
}

/// How many font files are kept in memory (tests).
pub fn kept_count() -> usize {
    kept().lock().map(|k| k.len()).unwrap_or(0)
}

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

/// Scans the system fonts (blocking) and remembers where their files are.
pub fn scan() -> Vec<String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let v = families_in(&db);
    install(db);
    v
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

/// The test font of `tests/support/patched_font.rs`.
#[cfg(test)]
pub(crate) mod test_font {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/patched_font.rs"
    ));
}

/// Installs, for the whole test run, a copy of the bundled font under `family` (nine
/// characters) with the given OS/2 win ascent (the folder lives as long as the process, so a
/// face that was dropped from the memory can be read again).
#[cfg(test)]
pub(crate) fn install_patched(family: &str, win_ascent: u16) {
    static DIRS: Mutex<Vec<tempfile::TempDir>> = Mutex::new(Vec::new());
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("x.ttf"),
        test_font::patched_font(family, win_ascent),
    )
    .unwrap();
    install_dir(dir.path());
    DIRS.lock().unwrap().push(dir);
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

    /// A folder with a copy of the bundled font under `family` (nine characters).
    fn font_dir(family: &str, win_ascent: u16) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("x.ttf"),
            test_font::patched_font(family, win_ascent),
        )
        .unwrap();
        dir
    }

    // covers: REQ-165
    #[test]
    fn an_installed_font_is_found_by_its_family_name_and_a_missing_one_is_not() {
        let dir = font_dir("Look Sans", 1124);
        install_dir(dir.path());
        let f = resolve("Look Sans").expect("the family installed for the test is found");
        assert_eq!(f.file.as_deref(), Some(dir.path().join("x.ttf").as_path()));
        assert!(crate::measure::Shaped::new(&f.bytes, f.index, "x").is_some());
        // the bundled font, an empty name, a missing family and a hostile name are no face
        assert!(resolve("").is_none() && resolve(DEFAULT_FONT).is_none());
        assert!(resolve("No Such Family").is_none());
        assert!(resolve("a;b").is_none());
        assert_eq!(bundled().file, None);
    }

    // covers: REQ-166
    #[test]
    fn a_slow_scan_never_blocks_the_caller_and_a_face_is_read_only_when_used() {
        // the scan runs on its own thread: starting it returns at once, whatever it costs
        let t = std::time::Instant::now();
        let f = Fonts::start_with(|| {
            std::thread::sleep(std::time::Duration::from_millis(400));
            vec!["Slow".to_string()]
        });
        assert!(t.elapsed() < std::time::Duration::from_millis(200));
        assert!(f.families().is_none(), "the list is not there yet");
        // installing the fonts of a folder keeps where their files are, not their bytes
        let dir = font_dir("Lazy Sans", 1124);
        install_dir(dir.path());
        let mine = |dir: &Path| {
            kept()
                .lock()
                .unwrap()
                .iter()
                .any(|(k, _)| k.starts_with(&dir.display().to_string()))
        };
        assert!(!mine(dir.path()), "nothing is read until a text uses it");
        let first = resolve("Lazy Sans").unwrap();
        assert!(mine(dir.path()));
        let second = resolve("Lazy Sans").unwrap();
        assert!(
            Arc::ptr_eq(&first, &second),
            "the second use is not another read"
        );
        // memory is bounded: few files and few bytes, however many fonts are used
        let mut dirs = Vec::new();
        for i in 0..KEEP_FACES + 1 {
            let name = format!("Bnd Sans{i}");
            let d = font_dir(&name, 1124 - i as u16);
            install_dir(d.path());
            let _ = resolve(&name);
            dirs.push(d);
        }
        assert!(kept_count() <= KEEP_FACES);
    }

    // covers: REQ-165
    #[test]
    fn the_files_of_the_fonts_a_document_uses_are_copied_next_to_the_bundled_one() {
        let dir = font_dir("Copy Sans", 1124);
        install_dir(dir.path());
        let work = tempfile::tempdir().unwrap();
        let doc = "Dialogue: 1,0:00:00.00,0:00:01.00,Text,,0,0,0,,{\\an5\\fnCopy Sans\\b0\\fs20}Hi\n\
                   Dialogue: 1,0:00:00.00,0:00:01.00,Text,,0,0,0,,{\\fnNoto Sans\\b1}Hi\n\
                   Dialogue: 1,0:00:00.00,0:00:01.00,Text,,0,0,0,,{\\fnGone Font}Hi";
        copy_used(work.path(), doc).unwrap();
        let names: Vec<String> = std::fs::read_dir(work.path().join("fonts"))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names.len(), 1, "{names:?}");
        assert!(names[0].starts_with("f-") && names[0].ends_with(".ttf"));
        // again: no second copy
        copy_used(work.path(), doc).unwrap();
        assert_eq!(
            std::fs::read_dir(work.path().join("fonts"))
                .unwrap()
                .count(),
            1
        );
        // a document with no installed font makes no folder
        let none = tempfile::tempdir().unwrap();
        copy_used(none.path(), "{\\fnNoto Sans}x").unwrap();
        assert!(!none.path().join("fonts").exists());
    }

    // covers: REQ-163
    #[test]
    fn a_font_that_is_not_a_font_file_gives_no_face_for_measuring() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bad.ttf"), b"junk, not a font").unwrap();
        install_dir(dir.path());
        // fontdb skips what it cannot parse: the family is not installed
        assert!(resolve("junk").is_none());
        // and a truncated font does not shape: the text is measured with the bundled font
        let cut = test_font::patched_font("Cutt Sans", 1124);
        let bad = tempfile::tempdir().unwrap();
        std::fs::write(bad.path().join("c.ttf"), &cut[..cut.len() / 2]).unwrap();
        install_dir(bad.path());
        let l = crate::text::layout_in("Hello there", "Cutt Sans", 800.0, 100.0, 10.0, 80.0);
        let b = crate::text::layout_in("Hello there", "", 800.0, 100.0, 10.0, 80.0);
        assert_eq!(l.px, b.px);
    }

    // covers: REQ-187
    #[test]
    fn the_bundled_faces_cover_their_scripts_and_not_the_cjk_ones() {
        use crate::ass::{FONT, FONT_ARABIC, FONT_DEVANAGARI};
        for (bytes, script) in [
            (FONT_ARABIC, Script::Arabic),
            (FONT_DEVANAGARI, Script::Devanagari),
        ] {
            assert!(font_covers(bytes, 0, script.sample()), "{script:?}");
        }
        assert!(
            font_covers(FONT, 0, &['a', 'Z', 'й', 'ω']),
            "Latin, Greek, Cyrillic"
        );
        for script in [Script::Han, Script::Kana, Script::Hangul, Script::Arabic] {
            assert!(!font_covers(FONT, 0, script.sample()), "{script:?}");
        }
        assert!(!font_covers(b"not a font", 0, &['a']));
        assert!(!font_covers(FONT, 9, &['a']), "no such face");
        // a folder with only the bundled faces covers Arabic and Devanagari, no CJK
        let dir = tempfile::tempdir().unwrap();
        for (name, bytes) in crate::ass::BUNDLED_FACES {
            std::fs::write(dir.path().join(name), bytes).unwrap();
        }
        let mut db = fontdb::Database::new();
        db.load_fonts_dir(dir.path());
        let covered = coverage_of(&db);
        assert_eq!(
            covered.into_iter().collect::<Vec<_>>(),
            [Script::Arabic, Script::Devanagari]
        );
    }

    // covers: REQ-187, REQ-188
    #[test]
    fn a_note_names_the_scripts_no_installed_font_covers() {
        let none: BTreeSet<Script> = BTreeSet::new();
        let cjk: BTreeSet<Script> = [Script::Han, Script::Kana, Script::Hangul].into();
        let text = "漢字 かな 한글 مرحبا नमस्ते hello";
        // without an installed CJK font: the note names Chinese, Japanese and Korean
        let missing = uncovered(text, Some(&none));
        assert_eq!(missing, [Script::Han, Script::Kana, Script::Hangul]);
        let note = note_for(&missing).unwrap();
        assert!(note.starts_with("No installed font shows Chinese, Japanese, Korean text"));
        assert!(note.contains("install a font for it"));
        // with one: no note; bundled scripts never need one
        assert!(uncovered(text, Some(&cjk)).is_empty());
        assert!(uncovered("مرحبا नमस्ते", Some(&none)).is_empty());
        assert!(note_for(&[]).is_none());
        // before the scan has finished nothing is reported
        assert!(uncovered(text, None).is_empty());
        // one script at a time
        assert_eq!(uncovered("かな", Some(&none)), [Script::Kana]);
    }
}
