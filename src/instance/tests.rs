// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the one-window protocol: lock, heartbeat, inbox and their limits.

use super::*;

fn project(dir: &Path, name: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, b"{}").unwrap();
    p
}

fn first(dir: &Path, now: u64) -> Instance {
    match claim(dir, None, now) {
        Claim::First(i) => i,
        other => panic!("{other:?}"),
    }
}

// covers: REQ-230
#[test]
fn a_second_start_with_a_project_hands_it_to_the_running_window() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("instance");
    let p = project(tmp.path(), "a.vlproj");
    let mut win = first(&dir, 1_000);
    // a second start with a project: no second window, the project is in the inbox
    assert!(matches!(claim(&dir, Some(&p), 1_500), Claim::Forwarded));
    let t = Instant::now();
    assert_eq!(win.poll(t, 1_600), vec![p.clone()]);
    // read once: the file is gone
    assert!(win.poll(t + BEAT_EVERY, 1_900).is_empty());
    // a second start with no project (the user started the program again): a window as usual
    assert!(matches!(claim(&dir, None, 1_700), Claim::Alone));
    // a project that is not a file (or a relative path that is not there) opens its own window
    assert!(matches!(
        claim(&dir, Some(&tmp.path().join("missing.vlproj")), 1_800),
        Claim::Alone
    ));
    assert!(matches!(claim(&dir, Some(tmp.path()), 1_800), Claim::Alone));
    // the private folder
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&dir).unwrap().permissions().mode();
        assert_eq!(mode & 0o077, 0, "{mode:o}");
    }
}

// covers: REQ-230
#[test]
fn with_no_running_window_or_a_stale_heartbeat_a_new_window_opens() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("instance");
    let p = project(tmp.path(), "a.vlproj");
    // nobody runs: the start becomes the first window (and may open its project itself)
    let win = first(&dir, 10_000);
    drop(win);
    // the lock went with the process: the next start is the first again
    let mut win = first(&dir, 20_000);
    // the heartbeat is old (the window is stuck or was suspended): not reachable
    assert!(matches!(
        claim(&dir, Some(&p), 20_000 + 2_001),
        Claim::Alone
    ));
    // fresh again after its next beat
    win.poll(Instant::now(), 30_000);
    assert!(matches!(claim(&dir, Some(&p), 30_500), Claim::Forwarded));
    // a damaged heartbeat is no heartbeat
    std::fs::write(dir.join(names::INSTANCE_HEARTBEAT), "soon").unwrap();
    assert!(matches!(claim(&dir, Some(&p), 30_600), Claim::Alone));
    assert!(is_fresh(Some(1_000), 3_000));
    assert!(!is_fresh(Some(1_000), 3_001));
    assert!(!is_fresh(None, 0));
    // a folder that cannot be made (a file in the way): a window as usual
    let blocked = tmp.path().join("blocked");
    std::fs::write(&blocked, b"x").unwrap();
    assert!(matches!(claim(&blocked.join("sub"), None, 0), Claim::Alone));
}

// covers: REQ-230, REQ-081
#[test]
fn several_projects_are_read_in_order_and_the_running_window_opens_the_first() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("instance");
    let mut win = first(&dir, 1_000);
    let ps: Vec<PathBuf> = (0..3)
        .map(|i| project(tmp.path(), &format!("p{i}.vlproj")))
        .collect();
    for (i, p) in ps.iter().enumerate() {
        assert!(matches!(
            claim(&dir, Some(p), 1_100 + i as u64),
            Claim::Forwarded
        ));
    }
    assert_eq!(win.poll(Instant::now(), 1_500), ps);
    // the program opens only the first of several, like a drop of several projects
    let other = tmp.path().join("other");
    let slot = Mutex::new(Some(first(&other, 1_000)));
    for (i, p) in ps.iter().enumerate() {
        assert!(matches!(
            claim(&other, Some(p), 1_100 + i as u64),
            Claim::Forwarded
        ));
    }
    assert_eq!(poll_slot(&slot), Some(ps[0].clone()));
    assert_eq!(poll_slot(&slot), None, "the others were dropped");
    // not a window that holds the lock: nothing
    assert_eq!(poll_slot(&Mutex::new(None)), None);
    assert_eq!(poll_current(), None);
    // the real start-up decision never fails: on this computer it is one of the roles
    let _ = default_dir();
}

// covers: REQ-230
#[test]
fn what_the_inbox_holds_is_untrusted_and_bounded() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("instance");
    let mut win = first(&dir, 1_000);
    let inbox = dir.join(names::INSTANCE_INBOX);
    std::fs::create_dir_all(&inbox).unwrap();
    let good = project(tmp.path(), "good.vlproj");
    let put = |name: &str, content: &[u8]| std::fs::write(inbox.join(name), content).unwrap();
    put("1-1.path", good.to_str().unwrap().as_bytes());
    put("2-1.path", b"relative/p.vlproj"); // not absolute
    put(
        "3-1.path",
        format!("{}\nrm -rf /", good.display()).as_bytes(),
    ); // control characters
    put("4-1.path", tmp.path().to_str().unwrap().as_bytes()); // a folder
    put("5-1.path", &[0xff, 0xfe, 0x00]); // not UTF-8
    put("6-1.path", "x".repeat(MAX_INBOX_PATH + 1).as_bytes()); // too long
    put("7-1.path", b""); // empty
    put("notes.txt", good.to_str().unwrap().as_bytes()); // not an inbox name
    put("8-1.path.partial", good.to_str().unwrap().as_bytes()); // not finished
    put("-.path", good.to_str().unwrap().as_bytes());
    let out = win.poll(Instant::now(), 2_000);
    assert_eq!(out, vec![good.clone()]);
    // what was read is gone; what is not an inbox file stays untouched
    assert!(!inbox.join("1-1.path").exists() && !inbox.join("6-1.path").exists());
    assert!(inbox.join("notes.txt").exists() && inbox.join("8-1.path.partial").exists());
    // a full inbox is not written to: the start opens its own window
    for i in 0..MAX_INBOX_FILES {
        put(&format!("{}-9.path", 100 + i), b"x");
    }
    assert!(matches!(claim(&dir, Some(&good), 2_100), Claim::Alone));
    // names
    assert!(names::is_inbox_file("1700000000000-4242.path"));
    for bad in ["x-1.path", "1-.path", "1.path", "1-2.txt", "1-2-3.path", ""] {
        assert!(!names::is_inbox_file(bad), "{bad}");
    }
    // a path text is checked on its own too
    assert_eq!(valid_inbox_path(good.to_str().unwrap()), Some(good));
    assert_eq!(valid_inbox_path(""), None);
}

// covers: REQ-230
#[test]
fn the_window_looks_at_the_inbox_every_quarter_second() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("instance");
    let mut win = first(&dir, 1_000);
    let p = project(tmp.path(), "a.vlproj");
    let t0 = Instant::now();
    // the first look
    assert!(win.poll(t0, 1_000).is_empty());
    assert!(matches!(claim(&dir, Some(&p), 1_050), Claim::Forwarded));
    // too soon: not read yet (and the heartbeat is not written again)
    assert!(win.poll(t0 + Duration::from_millis(100), 1_100).is_empty());
    assert_eq!(read_beat(&dir), Some(1_000));
    // after a quarter second: read, and the heartbeat is new
    assert_eq!(win.poll(t0 + BEAT_EVERY, 1_300), vec![p]);
    assert_eq!(read_beat(&dir), Some(1_300));
}
