// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The start-up picture option and the preview copies' limit and regeneration (2.0.4,
//! REQ-295, REQ-296, REQ-297).

use super::tests::{FakeDialogs, files, settle, state};
use super::*;
use crate::settings::SplashOption;
use std::time::{Duration, SystemTime};

fn old_copy(dir: &Path, name: char, bytes: usize, age: u64) -> PathBuf {
    let p = dir.join(format!("{}.mkv", name.to_string().repeat(32)));
    std::fs::write(&p, vec![0u8; bytes]).unwrap();
    let t = SystemTime::now() - Duration::from_secs(age);
    std::fs::File::options()
        .write(true)
        .open(&p)
        .unwrap()
        .set_modified(t)
        .unwrap();
    p
}

// covers: REQ-295
#[test]
fn the_splash_follows_the_option_and_remembers_the_version() {
    let version = env!("CARGO_PKG_VERSION");
    for (option, last, shows) in [
        (SplashOption::Always, Some(version), true),
        (SplashOption::OncePerVersion, None, true),
        (SplashOption::OncePerVersion, Some(version), false),
    ] {
        let mut app = state(FakeDialogs::default());
        app.settings.show_splash = option;
        app.settings.last_splash_version = last.map(str::to_string);
        app.start_splash();
        assert_eq!(
            app.popups.is_open(Popup::Splash),
            shows,
            "{option:?} {last:?}"
        );
    }
    // once per version: the first start remembers the version, the next one is quiet
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut app = state(FakeDialogs::default());
    app.settings_path = Some(path.clone());
    app.settings.show_splash = SplashOption::OncePerVersion;
    app.start_splash();
    assert!(app.popups.is_open(Popup::Splash));
    let saved: Settings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved.last_splash_version.as_deref(), Some(version));
    assert_eq!(saved.show_splash, SplashOption::OncePerVersion);
    let mut again = state(FakeDialogs::default());
    again.settings = saved;
    again.start_splash();
    assert!(!again.popups.is_open(Popup::Splash));
    // the choice is a command
    app.dispatch(
        Command::SetSplashOption(SplashOption::Always),
        Instant::now(),
    )
    .unwrap();
    assert_eq!(app.settings.show_splash, SplashOption::Always);
}

// covers: REQ-295
#[test]
fn a_stored_never_reads_as_once_for_each_version() {
    let with = |value: &str| -> Settings {
        let mut v = serde_json::to_value(Settings::default()).unwrap();
        v["show_splash"] = value.into();
        serde_json::from_value(v).unwrap()
    };
    let s = with("Never");
    assert_eq!(s.show_splash, SplashOption::OncePerVersion);
    let s = with("Nonsense");
    assert_eq!(s.show_splash, SplashOption::Always);
}

// covers: REQ-297
#[test]
fn the_oldest_copies_go_first_and_the_open_projects_copies_stay() {
    let dir = tempfile::tempdir().unwrap();
    let cache = ProxyCache::new(dir.path().to_path_buf());
    let a = old_copy(dir.path(), 'a', 600, 3000);
    let b = old_copy(dir.path(), 'b', 600, 2000);
    let c = old_copy(dir.path(), 'c', 600, 1000);
    let mine = old_copy(dir.path(), 'd', 600, 5000);
    let foreign = dir.path().join("keep-me.txt");
    std::fs::write(&foreign, vec![0u8; 5000]).unwrap();
    let partial = dir.path().join(format!("{}.mkv.partial", "e".repeat(32)));
    std::fs::write(&partial, vec![0u8; 600]).unwrap();
    assert_eq!(cache.size(), 3000);
    // the limit of 1900 bytes: the two oldest unprotected go (the protected one is older)
    let freed = cache.enforce_limit(1900, &["d".repeat(32)]);
    assert_eq!(freed, 1200);
    assert!(!a.exists() && !b.exists() && c.exists() && mine.exists());
    assert!(
        foreign.exists() && partial.exists(),
        "only the cache's own finished files go"
    );
    assert_eq!(cache.size(), 1800);
    // nothing to do under the limit or in a missing folder
    assert_eq!(cache.enforce_limit(10_000, &[]), 0);
    assert_eq!(
        ProxyCache::new(dir.path().join("none")).enforce_limit(0, &[]),
        0
    );
    // a link to a file elsewhere is never followed
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        let target = outside.path().join("x.bin");
        std::fs::write(&target, "x").unwrap();
        let link = dir.path().join(format!("{}.mkv", "f".repeat(32)));
        std::os::unix::fs::symlink(&target, &link).unwrap();
        cache.enforce_limit(0, &[]);
        assert!(target.exists());
    }
}

// covers: REQ-297
#[test]
fn the_app_keeps_the_cache_within_the_chosen_limit() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("cache");
    std::fs::create_dir_all(&folder).unwrap();
    let old = old_copy(&folder, 'a', 900, 9000);
    let mut app = state(FakeDialogs::default());
    app.proxy_cache = Some(ProxyCache::new(folder));
    // the limit has a floor: a tiny value is raised to it, and old copies stay under it
    app.dispatch(Command::SetCacheLimit(1), Instant::now())
        .unwrap();
    assert_eq!(
        app.settings.cache_limit,
        crate::settings::CACHE_LIMIT_RANGE.0
    );
    assert!(old.exists());
    app.settings.cache_limit = 500;
    app.enforce_cache_limit();
    assert!(!old.exists());
    app.dispatch(Command::SetCacheLimit(8_000_000_000), Instant::now())
        .unwrap();
    assert_eq!(app.settings.cache_limit, 8_000_000_000);
}

// covers: REQ-296
#[test]
fn clearing_the_copies_makes_the_open_projects_copies_again_in_the_background() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let big = dir.path().join("big.mov");
    std::fs::write(&big, "4k").unwrap();
    let mut app = state(FakeDialogs::default());
    app.proxy_cache = Some(ProxyCache::new(cache));
    app.machine = Machine::simulated(4, 4, 4 * GIB);
    let _ = files(dir.path());
    app.dispatch(Command::Import(vec![big]), Instant::now())
        .unwrap();
    settle(&mut app);
    let first = app.project.media[0]
        .proxy_path()
        .expect("a copy")
        .to_path_buf();
    app.dispatch(Command::ClearProxyCache, Instant::now())
        .unwrap();
    // not waiting for a reopen: the copy is being made again
    assert!(
        !matches!(app.project.media[0].proxy, ProxyState::None),
        "{:?}",
        app.project.media[0].proxy
    );
    settle(&mut app);
    assert_eq!(app.project.media[0].proxy_path(), Some(first.as_path()));
    assert!(first.is_file());
}
