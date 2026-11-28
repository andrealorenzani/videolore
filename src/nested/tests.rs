// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

use super::*;
use crate::project::save;
use crate::timeline::tests::media;

/// A project file `name` in `dir` with one 4 s clip of a video file that exists, and the
/// projects `imports` as further media (the first of them on the timeline too).
pub(crate) fn write_project(dir: &Path, name: &str, imports: &[&Path], used: bool) -> PathBuf {
    let video = dir.join(format!("{name}.mp4"));
    std::fs::write(&video, name).unwrap();
    let mut p = Project::default();
    let mut m = media(1, 4, true);
    m.id = p.new_media_id();
    m.path = video;
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    for (i, from) in imports.iter().enumerate() {
        let mut item = media(1, 4, true);
        item.id = p.new_media_id();
        item.path = dir.join("not-made-yet.mkv");
        item.from = Some(from.to_path_buf());
        p.add_media(item.clone());
        if used && i == 0 {
            p.timeline.append(&item).unwrap();
        }
    }
    let path = dir.join(format!("{name}.vlproj"));
    save(&p, &path).unwrap();
    path
}

// covers: REQ-373
#[test]
fn a_project_never_imports_itself_directly_or_through_others() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let a = d.join("a.vlproj");
    let b = write_project(d, "b", &[&a], true);
    let c = write_project(d, "c", &[&b], true);
    write_project(d, "a", &[], true);
    // itself, also written another way
    for itself in [a.clone(), d.join(".").join("a.vlproj")] {
        assert_eq!(
            read(&itself, Some(&a)).unwrap_err(),
            NestedError::Itself("a.vlproj".into())
        );
    }
    // through one and through two others
    for other in [&b, &c] {
        let e = read(other, Some(&a)).unwrap_err();
        assert_eq!(e, NestedError::Itself("a.vlproj".into()));
        assert!(e.to_string().contains("cannot import itself"));
    }
    // a project that is not in the chain may be imported
    assert!(read(&c, Some(&d.join("z.vlproj"))).is_ok());
    assert!(read(&c, None).is_ok());
    // a link outside the timeline may not lead back either
    let e = write_project(d, "e", &[&a], false);
    assert!(matches!(read(&e, Some(&a)), Err(NestedError::Itself(_))));
    // ... and a broken one outside the timeline is not a reason to refuse
    let f = write_project(d, "f", &[&d.join("gone.vlproj")], false);
    assert!(read(&f, None).is_ok());
    // two projects importing each other, whoever imports them
    write_project(d, "x", &[&d.join("y.vlproj")], true);
    let y = write_project(d, "y", &[&d.join("x.vlproj")], true);
    assert!(matches!(read(&y, None), Err(NestedError::Itself(_))));
}

// covers: REQ-373
#[test]
fn the_chain_of_imports_is_limited() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let mut last = write_project(d, "p0", &[], true);
    for i in 1..MAX_DEPTH {
        last = write_project(d, &format!("p{i}"), &[&last], true);
    }
    assert!(read(&last, Some(&d.join("me.vlproj"))).is_ok(), "8 deep");
    let ninth = write_project(d, "p8", &[&last], true);
    assert!(matches!(read(&ninth, None), Err(NestedError::TooDeep(_))));
    // many projects side by side are limited too
    let leaf = write_project(d, "leaf", &[], true);
    let wide = vec![leaf.as_path(); MAX_PROJECTS];
    let w = write_project(d, "wide", &wide, false);
    assert!(matches!(read(&w, None), Err(NestedError::TooMany(_))));
}

// covers: REQ-371
#[test]
fn an_unreadable_project_or_a_missing_source_is_refused_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let junk = d.join("junk.vlproj");
    std::fs::write(&junk, "not a project").unwrap();
    for bad in [junk, d.join("none.vlproj"), d.to_path_buf()] {
        let e = read(&bad, None).unwrap_err();
        assert!(matches!(e, NestedError::Unreadable { .. }), "{e}");
    }
    assert!(
        read(&d.join("none.vlproj"), None)
            .unwrap_err()
            .to_string()
            .contains("none.vlproj")
    );
    let p = write_project(d, "p", &[], true);
    std::fs::remove_file(d.join("p.mp4")).unwrap();
    let e = read(&p, None).unwrap_err();
    assert_eq!(
        e.to_string(),
        "'p.vlproj' cannot be imported: its file '1.mp4' is missing or cannot be used"
    );
    // a project with nothing on the timeline
    let empty = d.join("empty.vlproj");
    save(&Project::default(), &empty).unwrap();
    assert_eq!(
        read(&empty, None).unwrap_err(),
        NestedError::Empty("empty.vlproj".into())
    );
    // an imported project used on the timeline must be readable
    let q = write_project(d, "q", &[&d.join("gone.vlproj")], true);
    assert!(matches!(
        read(&q, None),
        Err(NestedError::Unreadable { .. })
    ));
}

#[test]
fn a_missing_overlay_or_lut_file_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let path = write_project(d, "p", &[], true);
    let (mut p, _) = crate::project::load(&path).unwrap();
    let clip = p.timeline.clips[0].id;
    p.timeline
        .set_lut(
            crate::timeline::FrameTarget::Clip(clip),
            Some(d.join("look.cube").to_string_lossy().into_owned()),
        )
        .unwrap();
    save(&p, &path).unwrap();
    let e = read(&path, None).unwrap_err().to_string();
    assert!(e.contains("look.cube"), "{e}");
}

// covers: REQ-371
#[test]
fn the_key_changes_with_the_project_its_files_and_what_it_imports() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let child = write_project(d, "child", &[], true);
    let top = write_project(d, "top", &[&child], true);
    let key = || read(&top, None).unwrap().key;
    let first = key();
    assert_eq!(first, key(), "nothing changed");
    assert_eq!(first.len(), 32);
    assert!(crate::base::names::is_cache_file(&format!("{first}.mkv")));
    // the project below changed
    let (mut p, _) = crate::project::load(&child).unwrap();
    let id = p.timeline.clips[0].id;
    p.timeline.set_clip_volume(id, 50, false).unwrap();
    save(&p, &child).unwrap();
    let second = key();
    assert_ne!(first, second);
    // a file it uses was replaced
    std::fs::write(d.join("top.mp4"), "a longer file").unwrap();
    assert_ne!(second, key());
}

// covers: REQ-371
#[test]
fn a_rebuilt_item_keeps_the_edits_inside_the_new_length() {
    let s = |n: i64| Micros(n * 1_000_000);
    let mut p = Project::default();
    let m = media(1, 6, true);
    p.add_media(m.clone());
    p.timeline.append(&m).unwrap();
    p.timeline.split(s(2), Micros(1)).unwrap();
    p.timeline.append(&m).unwrap();
    let mut tail = p.timeline.clips[2];
    tail.src_in = s(5);
    p.timeline.clips[2] = tail;
    let ranges = |p: &Project| -> Vec<(Micros, Micros)> {
        p.timeline
            .clips
            .iter()
            .map(|c| (c.src_in, c.src_out))
            .collect()
    };
    // longer: the clips that went to the end still do, the others stay
    let mut longer = p.clone();
    replace_media(&mut longer, m.id, media(7, 8, true));
    assert_eq!(longer.media[0].id, m.id);
    assert_eq!(ranges(&longer), [(s(0), s(2)), (s(2), s(8)), (s(5), s(8))]);
    // shorter: every clip stays inside, keeping its length where it can
    let mut shorter = p.clone();
    replace_media(&mut shorter, m.id, media(1, 3, true));
    assert_eq!(ranges(&shorter), [(s(0), s(2)), (s(2), s(3)), (s(2), s(3))]);
    // the saved file opens again
    let dir = tempfile::tempdir().unwrap();
    save(&shorter, &dir.path().join("s.vlproj")).unwrap();
    crate::project::load(&dir.path().join("s.vlproj")).unwrap();
    // an unknown media changes nothing
    let mut same = p.clone();
    replace_media(&mut same, MediaId(99), media(99, 1, true));
    assert_eq!(same, p);
}
