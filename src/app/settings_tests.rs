// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the program settings as the user changes them: the Settings window commands,
//! the start-up messages, "How hard videolore works", Preview and Subtitle quality, the
//! speech recognition files and older processors.

use super::tests::{
    FakeDialogs, files, import, info, read_settings, runner, settle, with_fake_speech,
};
use super::*;
use crate::ffmpeg::fake::FakeRunner;
use crate::machine::CpuFeatures;
use crate::preview::NullSink;
use crate::stt::tests::level_body;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;

fn app_with(fake: &Arc<FakeRunner>) -> AppState {
    AppState::with_ffmpeg(
        Ok(info()),
        fake.clone(),
        Box::new(FakeDialogs::default()),
        Box::new(NullSink),
    )
}

/// An app with two clips on the timeline (sources in `dir`).
fn loaded(dir: &Path, fake: &Arc<FakeRunner>) -> AppState {
    let mut app = app_with(fake);
    let f = files(dir);
    import(&mut app, f);
    let now = Instant::now();
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    app.dispatch(Command::Append(Some(app.project.media[1].id)), now)
        .unwrap();
    app
}

/// The ffmpeg calls that wrote `file:<path>` outputs ending with `suffix`.
fn calls_writing(fake: &FakeRunner, suffix: &str) -> Vec<Vec<String>> {
    fake.calls
        .lock()
        .unwrap()
        .iter()
        .filter(|a| a.last().is_some_and(|l| l.ends_with(suffix)))
        .cloned()
        .collect()
}

fn old_processor() -> Machine {
    let mut m = Machine::new(8, 4, 16 * GIB, "linux", "x86_64", CpuFeatures::ALL);
    m.cpu.avx2 = false;
    m
}

// covers: REQ-045
#[test]
fn first_start_creates_the_settings_and_the_message_closes_by_itself() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cfg").join("settings.json");
    let m = Machine::simulated(8, 4, 16 * GIB);
    let (s, notice) = start_settings(Some(&path), &m);
    assert_eq!(notice, Some(StartNotice::Setup));
    assert_eq!(read_settings(&path), s);
    assert_eq!(s.choices(), settings::recommend(&m.fingerprint()));
    let (again, notice) = start_settings(Some(&path), &m);
    assert_eq!(notice, None);
    assert_eq!(again, s);
    // an unreadable file is kept aside and replaced by recommended settings
    std::fs::write(&path, b"{ not json").unwrap();
    let (fixed, notice) = start_settings(Some(&path), &m);
    assert_eq!(notice, Some(StartNotice::Malformed));
    assert_eq!(fixed.choices(), s.choices());
    assert_eq!(
        std::fs::read(settings::backup_path(&path)).unwrap(),
        b"{ not json"
    );
    assert_eq!(start_settings(None, &m).1, None);
    // the set-up message closes itself after a few seconds
    let mut app = app_with(&Arc::new(runner()));
    app.banner = Some(Banner::from_notice(StartNotice::Setup));
    let t0 = Instant::now();
    app.tick(t0);
    assert!(app.banner.is_some());
    app.tick(t0 + SETUP_BANNER_TIME - Duration::from_millis(1));
    assert!(app.banner.is_some());
    app.tick(t0 + SETUP_BANNER_TIME);
    assert!(app.banner.is_none());
    // the others stay until closed
    app.banner = Some(Banner::from_notice(StartNotice::Malformed));
    app.tick(t0 + Duration::from_secs(3600));
    assert!(app.banner.is_some());
    app.dispatch(Command::DismissBanner, t0).unwrap();
    assert!(app.banner.is_none());
}

// covers: REQ-046
#[test]
fn choices_apply_at_once_and_are_program_settings() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    let path = dir.path().join("settings.json");
    app.settings_path = Some(path.clone());
    let now = Instant::now();
    let proj = dir.path().join("p.vlproj");
    app.dispatch(Command::SaveProjectAs(proj.clone()), now)
        .unwrap();
    app.dispatch(Command::OpenSettings, now).unwrap();
    assert!(app.settings_open);
    assert!(app.is_recommended());
    // How hard videolore works: applied without restart
    assert_eq!(app.scheduler().cap(), 3);
    app.dispatch(Command::SetEffort(Effort::Maximum), now)
        .unwrap();
    assert_eq!(app.scheduler().cap(), 6);
    assert_eq!(app.budget.stt_threads, 8);
    assert!(!app.is_recommended());
    app.dispatch(Command::SetEffort(Effort::Light), now)
        .unwrap();
    assert_eq!(app.scheduler().cap(), 3);
    assert_eq!(app.budget.threads, Some(8));
    app.dispatch(Command::SetSubtitleQuality(SubtitleQuality::Fast), now)
        .unwrap();
    app.dispatch(Command::SetPreviewQuality(PreviewQuality::Smoother), now)
        .unwrap();
    let saved = read_settings(&path);
    assert_eq!(saved.effort, Effort::Light);
    assert_eq!(saved.subtitle_quality, SubtitleQuality::Fast);
    assert_eq!(saved.preview_quality, PreviewQuality::Smoother);
    // never part of the project
    assert!(!app.dirty);
    app.dispatch(Command::SaveProject, now).unwrap();
    let text = std::fs::read_to_string(&proj).unwrap();
    for word in ["effort", "preview_quality", "subtitle_quality", "smoother"] {
        assert!(!text.contains(word), "{word} in the project file");
    }
    app.dispatch(Command::CloseSettings, now).unwrap();
    assert!(!app.settings_open);
    // greyed-out choices cannot be picked, with a plain reason
    app.machine = Machine::simulated(8, 4, 3 * GIB);
    assert!(!app.is_enabled(&Command::SetSubtitleQuality(SubtitleQuality::MostAccurate)));
    let e = app
        .dispatch(
            Command::SetSubtitleQuality(SubtitleQuality::MostAccurate),
            now,
        )
        .unwrap_err();
    assert_eq!(e, settings::MEMORY_REASON);
    assert_eq!(app.settings.subtitle_quality, SubtitleQuality::Fast);
}

// covers: REQ-047
#[test]
fn use_recommended_settings_restores_the_recommendation_only() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    let path = dir.path().join("settings.json");
    app.settings_path = Some(path.clone());
    with_fake_speech(&mut app, dir.path());
    let store = app.model_store.clone().unwrap();
    std::fs::create_dir_all(&store.dir).unwrap();
    std::fs::write(store.path(SubtitleQuality::Fast), level_body(b"x", 0)).unwrap();
    let cache_dir = dir.path().join("proxies");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cached = cache_dir.join(format!("{}.mkv", "a".repeat(32)));
    std::fs::write(&cached, b"copy").unwrap();
    app.proxy_cache = Some(ProxyCache::new(cache_dir));
    app.settings.ffmpeg = Some("/opt/ff/ffmpeg".into());
    let fetches = Arc::new(AtomicUsize::new(0));
    app.fetcher = Some(Arc::new(super::tests::LevelFetcher {
        store: store.clone(),
        body: b"x".to_vec(),
        calls: Arc::clone(&fetches),
    }));
    let now = Instant::now();
    app.dispatch(Command::SetEffort(Effort::Maximum), now)
        .unwrap();
    app.dispatch(Command::SetPreviewQuality(PreviewQuality::Original), now)
        .unwrap();
    let project_before = app.project.doc();
    // the machine is read again
    app.detect = || Machine::simulated(4, 2, 8 * GIB);
    app.dispatch(Command::UseRecommended, now).unwrap();
    let fp = Machine::simulated(4, 2, 8 * GIB).fingerprint();
    assert_eq!(app.settings.choices(), settings::recommend(&fp));
    assert_eq!(app.settings.machine, Some(fp));
    assert!(app.is_recommended());
    assert_eq!(
        app.settings_summary,
        vec![
            "Preview quality: Original → Automatic".to_string(),
            "Subtitle quality: Fast → Balanced".to_string(),
            "How hard videolore works: Maximum speed → Balanced".to_string(),
        ]
    );
    assert_eq!(read_settings(&path).choices(), app.settings.choices());
    // untouched: ffmpeg choice, project, preview copies, speech files; nothing downloaded
    assert_eq!(app.settings.ffmpeg, Some("/opt/ff/ffmpeg".into()));
    assert_eq!(app.project.doc(), project_before);
    assert!(cached.exists());
    assert!(store.path(SubtitleQuality::Fast).exists());
    assert_eq!(fetches.load(Ordering::SeqCst), 0);
    assert!(app.modal.is_none() && !app.is_generating());
    // again: nothing to change
    app.dispatch(Command::UseRecommended, now).unwrap();
    assert_eq!(
        app.settings_summary,
        vec![settings::ALREADY_RECOMMENDED.to_string()]
    );
}

// covers: REQ-048
#[test]
fn a_new_machine_with_custom_settings_offers_the_recommended_ones() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let old = Machine::simulated(8, 4, 16 * GIB);
    let new = Machine::simulated(16, 8, 64 * GIB);
    let mut custom = Settings::with(
        settings::recommend(&old.fingerprint()),
        Some(old.fingerprint()),
    );
    custom.effort = Effort::Light;
    settings::save(&custom, &path).unwrap();
    let (s, notice) = start_settings(Some(&path), &new);
    assert_eq!(notice, Some(StartNotice::MachineChanged));
    assert_eq!(s.effort, Effort::Light);
    let mut app = app_with(&Arc::new(runner()));
    app.settings = s;
    app.settings_path = Some(path.clone());
    app.detect = || Machine::simulated(16, 8, 64 * GIB);
    app.banner = notice.map(Banner::from_notice);
    let b = app.banner.clone().unwrap();
    assert_eq!(b.action, Some(BannerAction::UseRecommended));
    assert_eq!(b.lifetime, None);
    app.dispatch(
        Command::BannerAction(BannerAction::UseRecommended),
        Instant::now(),
    )
    .unwrap();
    assert!(app.banner.is_none());
    assert!(app.is_recommended());
    assert_eq!(read_settings(&path).effort, Effort::Balanced);
    // recommended settings on a new machine are simply recomputed
    let (s, notice) = start_settings(Some(&path), &old);
    assert_eq!(notice, Some(StartNotice::Setup));
    assert_eq!(s.choices(), settings::recommend(&old.fingerprint()));
}

// covers: REQ-050
#[test]
fn how_hard_applies_to_the_next_job_and_never_restarts_a_running_one() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    app.machine = Machine::simulated(8, 8, 16 * GIB);
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap(); // forces a re-encode
    app.dispatch(Command::SetEffort(Effort::Balanced), now)
        .unwrap();
    app.dispatch(Command::Export(dir.path().join("one.mp4")), now)
        .unwrap();
    // changing the level while the export runs does not restart it
    app.dispatch(Command::SetEffort(Effort::Light), now)
        .unwrap();
    app.wait_export(now);
    let first = calls_writing(&fake, ".one.mp4.videolore-partial");
    assert_eq!(first.len(), 1, "started once");
    assert!(!first[0].contains(&"-threads".to_string()));
    // the next export uses the new level: half the cores
    app.dispatch(Command::Export(dir.path().join("two.mp4")), now)
        .unwrap();
    app.wait_export(now);
    let second = calls_writing(&fake, ".two.mp4.videolore-partial");
    assert_eq!(second.len(), 1);
    assert!(second[0].windows(2).any(|w| w == ["-threads", "4"]));
    assert!(
        second[0]
            .windows(2)
            .any(|w| w == ["-filter_complex_threads", "4"])
    );
    // speech recognition threads follow the level too
    if stt::available() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = Arc::clone(&seen);
        with_fake_speech(&mut app, dir.path());
        let store = app.model_store.clone().unwrap();
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.path(SubtitleQuality::Fast),
            level_body(b"tiny fake model", 0),
        )
        .unwrap();
        app.transcriber = Arc::new(move |_, threads| {
            s2.lock().unwrap().push(threads);
            Ok(Box::new(crate::stt::tests::FakeTranscriber::default())
                as Box<dyn stt::Transcriber>)
        });
        for e in Effort::ALL {
            app.dispatch(Command::SetEffort(e), now).unwrap();
            app.dispatch(Command::GenerateSubtitles, now).unwrap();
            if app.modal == Some(Modal::ConfirmRegenerate) {
                app.dispatch(Command::ConfirmRegenerate, now).unwrap();
            }
            settle(&mut app);
        }
        assert_eq!(*seen.lock().unwrap(), vec![4, 7, 8]);
    }
}

// covers: REQ-051
#[test]
fn preview_quality_never_changes_the_export() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    let mut exports = Vec::new();
    for (i, q) in PreviewQuality::ALL.into_iter().enumerate() {
        app.dispatch(Command::SetPreviewQuality(q), now).unwrap();
        assert_eq!(app.preview.proxies, q != PreviewQuality::Original);
        let out = dir.path().join(format!("o{i}.mp4"));
        app.dispatch(Command::Export(out), now).unwrap();
        app.wait_export(now);
        let mut args = calls_writing(&fake, &format!(".o{i}.mp4.videolore-partial"))
            .pop()
            .unwrap();
        args.pop(); // the output file
        exports.push(args);
    }
    assert_eq!(exports[0], exports[1]);
    assert_eq!(exports[1], exports[2]);
}

// covers: REQ-051
#[test]
fn preview_quality_decides_which_sources_get_a_preview_copy() {
    use crate::media::MediaFormat;
    let f = |w, h| MediaFormat {
        vcodec: Some("h264".into()),
        width: w,
        height: h,
        ..Default::default()
    };
    let strong = Machine::simulated(16, 8, 64 * GIB);
    let weak = Machine::simulated(4, 4, 4 * GIB);
    let table = [
        ((3840, 2160), [true, true, false], [true, true, false]),
        ((1920, 1080), [false, true, false], [true, true, false]),
        ((1280, 720), [false, false, false], [false, false, false]),
        ((1281, 720), [false, true, false], [true, true, false]),
    ];
    for ((w, h), on_strong, on_weak) in table {
        for (i, q) in PreviewQuality::ALL.into_iter().enumerate() {
            assert_eq!(
                proxy::wants_proxy(&f(w, h), q, &strong),
                on_strong[i],
                "{w}x{h} {q:?}"
            );
            assert_eq!(proxy::wants_proxy(&f(w, h), q, &weak), on_weak[i]);
        }
    }
    let audio = MediaFormat::default();
    assert!(
        PreviewQuality::ALL
            .iter()
            .all(|q| !proxy::wants_proxy(&audio, *q, &weak))
    );
}

// covers: REQ-052, REQ-044
#[test]
fn better_subtitle_levels_ask_before_downloading() {
    if !stt::available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    with_fake_speech(&mut app, dir.path());
    let store = app.model_store.clone().unwrap();
    let now = Instant::now();
    app.dispatch(Command::SetSubtitleQuality(SubtitleQuality::Balanced), now)
        .unwrap();
    // nothing installed: consent for the Balanced file
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(
        app.modal,
        Some(Modal::ModelConsent(SubtitleQuality::Balanced))
    );
    // declining changes nothing
    app.dispatch(Command::ModelConsent(ConsentChoice::Cancel), now)
        .unwrap();
    assert!(app.modal.is_none() && !app.is_generating());
    assert_eq!(app.settings.subtitle_quality, SubtitleQuality::Balanced);
    assert!(!store.dir.exists());
    // "Download the Fast one instead": only the small file arrives, the setting stays
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    app.dispatch(
        Command::ModelConsent(ConsentChoice::DownloadFastInstead),
        now,
    )
    .unwrap();
    settle(&mut app);
    assert!(store.is_installed(SubtitleQuality::Fast));
    assert!(!store.is_installed(SubtitleQuality::Balanced));
    let n = app.project.subtitles.lines.len();
    assert!(n > 0);
    assert_eq!(app.settings.subtitle_quality, SubtitleQuality::Balanced);
    // changing the level never regenerates by itself
    let lines = app.project.subtitles.lines.clone();
    app.dispatch(
        Command::SetSubtitleQuality(SubtitleQuality::MostAccurate),
        now,
    )
    .unwrap();
    settle(&mut app);
    assert_eq!(app.project.subtitles.lines, lines);
    // regenerating at another level with manual edits asks first (REQ-044)
    let first = app.project.subtitles.lines[0].id;
    app.dispatch(Command::SetSubtitleText(first, "mine".into()), now)
        .unwrap();
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(app.modal, Some(Modal::ConfirmRegenerate));
    app.dispatch(Command::ConfirmRegenerate, now).unwrap();
    assert_eq!(
        app.modal,
        Some(Modal::ModelConsent(SubtitleQuality::MostAccurate))
    );
    // "Use Fast instead": this time only, no download
    app.dispatch(Command::ModelConsent(ConsentChoice::UseFastInstead), now)
        .unwrap();
    settle(&mut app);
    assert!(!store.is_installed(SubtitleQuality::MostAccurate));
    assert_eq!(app.settings.subtitle_quality, SubtitleQuality::MostAccurate);
    assert_eq!(app.project.subtitles.lines.len(), n);
    // consenting downloads that level's file and generates with it
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    settle(&mut app);
    assert!(store.is_installed(SubtitleQuality::MostAccurate));
    assert!(app.speech_files_size > 0);
    // once installed, no question at all
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert!(app.modal.is_none());
    settle(&mut app);
    // "Use Fast instead" needs the Fast file
    store.remove_downloaded();
    app.modal = Some(Modal::ModelConsent(SubtitleQuality::Balanced));
    assert!(
        app.dispatch(Command::ModelConsent(ConsentChoice::UseFastInstead), now)
            .is_err()
    );
}

// covers: REQ-053
#[test]
fn removing_speech_files_frees_the_space_and_keeps_the_subtitles() {
    if !stt::available() {
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    with_fake_speech(&mut app, dir.path());
    let store = app.model_store.clone().unwrap();
    let now = Instant::now();
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    app.dispatch(Command::ModelConsent(ConsentChoice::Download), now)
        .unwrap();
    settle(&mut app);
    let lines = app.project.subtitles.lines.clone();
    assert!(!lines.is_empty());
    let on_disk = std::fs::metadata(store.path(SubtitleQuality::Fast))
        .unwrap()
        .len();
    app.dispatch(Command::OpenSettings, now).unwrap();
    assert_eq!(app.speech_files_size, on_disk);
    // asks first
    app.dispatch(Command::RemoveSpeechFiles, now).unwrap();
    assert_eq!(app.modal, Some(Modal::ConfirmRemoveSpeechFiles));
    app.dispatch(Command::CloseModal, now).unwrap();
    assert!(store.is_installed(SubtitleQuality::Fast));
    app.dispatch(Command::RemoveSpeechFiles, now).unwrap();
    app.dispatch(Command::ConfirmRemoveSpeechFiles, now)
        .unwrap();
    assert!(!store.path(SubtitleQuality::Fast).exists());
    assert_eq!(app.speech_files_size, 0);
    assert_eq!(store.downloaded_bytes(), 0);
    // subtitles still show and export
    assert_eq!(app.project.subtitles.lines, lines);
    assert!(
        !app.project
            .subtitles
            .project(&app.project.timeline)
            .is_empty()
    );
    app.dispatch(Command::Export(dir.path().join("subs.mp4")), now)
        .unwrap();
    app.wait_export(now);
    assert!(app.status.starts_with("Exported"), "{}", app.status);
    let export = calls_writing(&fake, ".subs.mp4.videolore-partial");
    assert!(export[0].iter().any(|a| a.contains("subtitles=f=subs.ass")));
    // next generation asks for the file again
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(app.modal, Some(Modal::ModelConsent(SubtitleQuality::Fast)));
}

// covers: REQ-054
#[test]
fn an_old_processor_gets_a_message_and_never_reaches_the_speech_engine() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    with_fake_speech(&mut app, dir.path());
    let loads = Arc::new(AtomicUsize::new(0));
    let l2 = Arc::clone(&loads);
    app.transcriber = Arc::new(move |_, _| {
        l2.fetch_add(1, Ordering::SeqCst);
        Err(stt::SttError::NotAvailable)
    });
    app.machine = old_processor();
    let now = Instant::now();
    if !stt::available() {
        return;
    }
    app.dispatch(Command::GenerateSubtitles, now).unwrap();
    assert_eq!(app.notice.as_deref(), Some(TOO_OLD_FOR_SUBTITLES));
    assert!(app.modal.is_none(), "no download is offered");
    assert!(!app.is_generating());
    assert!(app.generate_subtitles(None).is_ok());
    assert_eq!(loads.load(Ordering::SeqCst), 0);
    // every level is greyed out, Fast is what gets recommended
    let fp = app.fingerprint();
    assert!(settings::availability(&fp).iter().all(Option::is_some));
    assert_eq!(settings::recommend(&fp).subtitles, SubtitleQuality::Fast);
    for q in SubtitleQuality::ALL {
        assert!(!app.is_enabled(&Command::SetSubtitleQuality(q)));
    }
    // the rest of the app works: lines by hand, export
    app.notice = None;
    app.dispatch(Command::AddSubtitle, now).unwrap();
    app.dispatch(Command::Export(dir.path().join("old.mp4")), now)
        .unwrap();
    app.wait_export(now);
    assert!(app.status.starts_with("Exported"));
}

// covers: REQ-059
#[test]
fn export_temporary_files_are_private_and_gone_afterwards() {
    let dir = tempfile::tempdir().unwrap();
    let fake = Arc::new(runner());
    let mut app = loaded(dir.path(), &fake);
    app.work_root = dir.path().join("work");
    let now = Instant::now();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let out = dir.path().join("final.mp4");
    app.dispatch(Command::Export(out.clone()), now).unwrap();
    app.wait_export(now);
    assert!(out.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&out).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o644, "the finished export is a normal file");
    }
    assert_eq!(std::fs::read_dir(&app.work_root).unwrap().count(), 0);
    assert!(!crate::export::partial_path(&out).exists());
}
