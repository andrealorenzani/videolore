// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Headless UI tests: whole frames are run without a window or painter.

use super::settings as settings_ui;
use super::*;
use crate::app::tests::{FakeDialogs, files, import, settle, state};
use crate::app::{Modal, PendingAction, Popup};
use crate::media::{MediaStatus, ProxyState};
use crate::preview::Frame;
use eframe::egui::{
    Event, Key, Modifiers, PointerButton, Pos2, RawInput, ViewportCommand, ViewportEvent,
    ViewportId, ViewportInfo,
};
use std::sync::Arc;

fn input(w: f32, h: f32, events: Vec<Event>) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(w, h))),
        events,
        ..Default::default()
    }
}

/// Runs one frame; returns the commands sent to the window.
fn frame(
    ctx: &Context,
    app: &mut AppState,
    cache: &mut UiCache,
    raw: RawInput,
) -> Vec<ViewportCommand> {
    let now = Instant::now();
    // Headless: there is no painter, so texture deltas are discarded on purpose.
    let mut out = ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
    let cmds = out
        .viewport_output
        .get(&ViewportId::ROOT)
        .map(|v| v.commands.clone())
        .unwrap_or_default();
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    cmds
}

fn run(ctx: &Context, app: &mut AppState, cache: &mut UiCache, w: f32, h: f32, ev: Vec<Event>) {
    frame(ctx, app, cache, input(w, h, ev));
}

fn loaded() -> (tempfile::TempDir, AppState) {
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let mut app = state(FakeDialogs::default());
    let now = Instant::now();
    import(&mut app, f);
    let a = app.project.media[0].id;
    let b = app.project.media[1].id;
    app.dispatch(Command::Append(Some(a)), now).unwrap();
    app.dispatch(Command::Append(Some(b)), now).unwrap();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    (dir, app)
}

fn key(k: Key, m: Modifiers) -> Event {
    Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: m,
    }
}

fn click_at(ctx: &Context, app: &mut AppState, cache: &mut UiCache, p: Pos2) {
    let press = |pressed| Event::PointerButton {
        pos: p,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(
        ctx,
        app,
        cache,
        1280.0,
        800.0,
        vec![Event::PointerMoved(p), press(true)],
    );
    run(ctx, app, cache, 1280.0, 800.0, vec![press(false)]);
    run(ctx, app, cache, 1280.0, 800.0, vec![]);
}

// covers: REQ-024, REQ-032, REQ-038, REQ-019, REQ-017
#[test]
fn renders_all_panels_at_any_size() {
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    let now = Instant::now();
    app.selected_clip = Some(app.project.timeline.clips[1].id);
    app.selected_media = Some(app.project.media[0].id);
    app.project.media[1].missing = true;
    app.project.media[0].proxy = ProxyState::Generating(40);
    // music on its lane and subtitles on the video track
    app.dispatch(Command::PlaceMusic(Some(app.project.media[2].id)), now)
        .unwrap();
    app.dispatch(Command::Seek(Micros(1_000_000)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    app.frame = Some(Frame {
        pts: Micros::ZERO,
        width: 4,
        height: 2,
        rgba: vec![255; 32],
    });
    app.frame_serial = 1;
    app.status = "hello".into();
    for (w, h) in [(1280.0, 800.0), (800.0, 500.0), (1920.0, 1080.0)] {
        run(&ctx, &mut app, &mut cache, w, h, vec![]);
    }
    assert!(cache.has_texture());
    // every media state has a description
    let mut m = app.project.media[0].clone();
    for (status, proxy, text) in [
        (MediaStatus::Analysing, ProxyState::None, "analysing"),
        (MediaStatus::Rejected("bad".into()), ProxyState::None, "bad"),
        (MediaStatus::Ready, ProxyState::Queued, "queued"),
        (MediaStatus::Ready, ProxyState::Ready("/p".into()), "ready"),
        (MediaStatus::Ready, ProxyState::Failed("x".into()), "failed"),
    ] {
        m.status = status;
        m.proxy = proxy;
        assert!(media::status_line(&m).contains(text), "{text}");
    }
    app.project.media.push(MediaItemRejected::item());
    app.frame_serial = 2;
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    app.frame.as_mut().unwrap().rgba.clear(); // malformed frame is ignored
    app.frame_serial = 3;
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    app.frame = None;
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert!(!cache.has_texture());
    // every dialog
    for modal in [
        Modal::Unsaved(PendingAction::New),
        Modal::Unsaved(PendingAction::Open(None)),
        Modal::Unsaved(PendingAction::Quit),
        Modal::ModelConsent(crate::settings::SubtitleQuality::Balanced),
        Modal::ConfirmRegenerate,
        Modal::ConfirmRemoveSpeechFiles,
        Modal::About,
    ] {
        app.modal = Some(modal);
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    }
    assert!(cache.about.as_deref().unwrap().contains("FFmpeg"));
    app.modal = None;
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert!(cache.about.is_none());
    app.notice = Some("note".into());
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    app.notice = None;
    app.ffmpeg_checking = true;
    app.ffmpeg = Err(crate::app::CHECKING_FFMPEG.into());
    assert!(!modal_open(&app));
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    app.ffmpeg_checking = false;
    app.ffmpeg = Err("ffmpeg was not found".into());
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::Space, Modifiers::NONE)],
    );
    assert!(!app.preview.is_playing());
}

/// A file that could not be used, as shown in the list.
struct MediaItemRejected;

impl MediaItemRejected {
    fn item() -> crate::media::MediaItem {
        let mut m = crate::media::MediaItem::analysing(
            crate::media::MediaId(99),
            std::path::Path::new("/x/notes.txt"),
        );
        m.status = MediaStatus::Rejected("not a video".into());
        m
    }
}

// covers: REQ-025
#[test]
fn shortcuts_dispatch_commands() {
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    app.dispatch(Command::Seek(Micros(2_000_000)), Instant::now())
        .unwrap();
    let n = app.project.timeline.clips.len();
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::B, Modifiers::COMMAND)],
    );
    assert_eq!(app.project.timeline.clips.len(), n + 1);
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::Z, Modifiers::COMMAND)],
    );
    assert_eq!(app.project.timeline.clips.len(), n);
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::Y, Modifiers::COMMAND)],
    );
    assert_eq!(app.project.timeline.clips.len(), n + 1);
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![Event::Copy]);
    assert!(app.clipboard.is_some());
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![Event::Paste(String::new())],
    );
    assert_eq!(app.project.timeline.clips.len(), n + 2);
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![Event::Cut]);
    assert_eq!(app.project.timeline.clips.len(), n + 1);
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::Space, Modifiers::NONE)],
    );
    assert!(app.preview.is_playing());
    assert!(app.title().contains('•'));
    // the fake decoder produces no frames: playback ends on the next frame
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.preview.is_playing() && Instant::now() < deadline {
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    }
    assert!(!app.preview.is_playing());
    // disabled commands are ignored
    app.history.clear();
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::Z, Modifiers::COMMAND)],
    );
    assert!(app.status.is_empty() || !app.status.contains("undo"));
}

// covers: REQ-028, REQ-026
#[test]
fn closing_the_window_asks_about_unsaved_changes() {
    let (dir, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    let close = || {
        let mut raw = input(1280.0, 800.0, vec![]);
        raw.viewports.insert(
            ViewportId::ROOT,
            ViewportInfo {
                events: vec![ViewportEvent::Close],
                ..Default::default()
            },
        );
        raw
    };
    // dirty: the close is cancelled and the question is asked
    let sent = frame(&ctx, &mut app, &mut cache, close());
    assert!(sent.contains(&ViewportCommand::CancelClose));
    assert!(!sent.contains(&ViewportCommand::Close));
    assert_eq!(app.modal, Some(Modal::Unsaved(PendingAction::Close)));
    // shortcuts are off while the dialog is open
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::B, Modifiers::COMMAND)],
    );
    assert_eq!(app.project.timeline.clips.len(), 2);
    // "Don't save" closes the window
    app.dispatch(
        Command::ResolveUnsaved(crate::app::UnsavedChoice::Discard),
        Instant::now(),
    )
    .unwrap();
    let sent = frame(&ctx, &mut app, &mut cache, input(1280.0, 800.0, vec![]));
    assert!(sent.contains(&ViewportCommand::Close));
    // once allowed, the next close request goes through
    let sent = frame(&ctx, &mut app, &mut cache, close());
    assert!(!sent.contains(&ViewportCommand::CancelClose));
    // Ctrl/Cmd+S saves, Ctrl/Cmd+Q quits at once when nothing is unsaved
    app.project_path = Some(dir.path().join("p.vlproj"));
    app.dirty = true;
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![key(Key::S, Modifiers::COMMAND)],
    );
    assert!(dir.path().join("p.vlproj").is_file());
    assert!(!app.dirty);
    let sent = frame(
        &ctx,
        &mut app,
        &mut cache,
        input(1280.0, 800.0, vec![key(Key::Q, Modifiers::COMMAND)]),
    );
    assert!(sent.contains(&ViewportCommand::Close));
}

// covers: REQ-012
#[test]
fn zoom_starts_at_fit_and_follows_the_slider() {
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    assert_eq!(app.zoom, 0.0);
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    let fit = app.zoom;
    assert!(fit > 0.0);
    // fit: the whole 10 s timeline fits in the section width
    assert!(fit * 10.0 <= cache.sections.unwrap().timeline.width());
    let frame_level = zoom::PX_PER_FRAME_MAX * app.fps().as_f64() as f32;
    app.dispatch(Command::SetZoom(1e6), Instant::now()).unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert!((app.zoom - frame_level).abs() < 1e-3);
}

// covers: REQ-019, REQ-024
#[test]
fn clicking_the_timeline_and_the_music_lane() {
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    app.dispatch(
        Command::PlaceMusic(Some(app.project.media[2].id)),
        Instant::now(),
    )
    .unwrap();
    app.dispatch(Command::SelectMusic(None), Instant::now())
        .unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    let g = cache.timeline.unwrap();
    // the video track is in the upper part of the timeline section
    let video_y = g.video.center().y;
    click_at(&ctx, &mut app, &mut cache, pos2(g.x0 + 60.0, video_y));
    assert_eq!(app.selected_clip, Some(app.project.timeline.clips[0].id));
    assert!(app.playhead > Micros::ZERO);
    // the music track is below it, in the same section
    let music_y = g.music.center().y;
    assert!(music_y > video_y);
    click_at(&ctx, &mut app, &mut cache, pos2(g.x0 + 80.0, music_y));
    let m = app.project.music.clips[0].id;
    assert_eq!(app.selected_music, Some(m));
    // drag the music clip to the right
    let from = pos2(g.x0 + 80.0, music_y);
    let to = pos2(g.x0 + 200.0, music_y);
    let btn = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![Event::PointerMoved(from), btn(from, true)],
    );
    for i in 1..=6 {
        let p = from + (to - from) * (i as f32 / 6.0);
        run(
            &ctx,
            &mut app,
            &mut cache,
            1280.0,
            800.0,
            vec![Event::PointerMoved(p)],
        );
    }
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![btn(to, false)],
    );
    run(
        &ctx,
        &mut app,
        &mut cache,
        1280.0,
        800.0,
        vec![Event::PointerGone],
    );
    assert!(app.project.music.get(m).unwrap().start > Micros::ZERO);
    // the clip details popup shows the selected music clip
    app.dispatch(Command::SelectMusic(Some(m)), Instant::now())
        .unwrap();
    app.dispatch(Command::OpenPopup(Popup::ClipDetails), Instant::now())
        .unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
}

// covers: REQ-020, REQ-016, REQ-076
#[test]
fn volume_and_subtitle_editing_from_the_popups() {
    let (_d, mut app) = loaded();
    app.dispatch(Command::OpenPopup(Popup::Volume), Instant::now())
        .unwrap();
    let ctx = Context::default();
    // a volume being dragged is not an undo step until released
    let mut cache = UiCache {
        mix_drag: Some(crate::music::Mix {
            video_volume: 30,
            ..Default::default()
        }),
        ..Default::default()
    };
    let undo = app.history.undo_len();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert_eq!(app.history.undo_len(), undo);
    cache.mix_drag = Some(app.project.mix);
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert!(cache.mix_drag.is_none());
    // a selected subtitle line is edited in place
    app.dispatch(Command::Seek(Micros(1_000_000)), Instant::now())
        .unwrap();
    app.dispatch(Command::AddSubtitle, Instant::now()).unwrap();
    let id = app.selected_line.unwrap();
    let clip = app.project.timeline.clips[0].id;
    app.dispatch(
        Command::OpenInstance(crate::app::Instance::Line(id, clip)),
        Instant::now(),
    )
    .unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    assert!(
        cache.line_edit.is_none(),
        "not focused: shows the stored text"
    );
    cache.line_edit = Some((id, "Edited".into()));
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    app.dispatch(
        Command::SetSubtitleText(id, "Edited".into()),
        Instant::now(),
    )
    .unwrap();
    assert_eq!(app.project.subtitles.get(id).unwrap().text, "Edited");
    app.dispatch(Command::SelectLine(None), Instant::now())
        .unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    settle(&mut app);
}

#[test]
fn helpers() {
    let (_d, app) = loaded();
    let tl = &app.project.timeline;
    assert_eq!(timeline::drop_index(tl, Micros(1_000_000)), 0);
    assert_eq!(timeline::drop_index(tl, Micros(4_000_000)), 1);
    assert_eq!(timeline::drop_index(tl, Micros(9_000_000)), 1);
    assert_eq!(timeline::drop_index(tl, Micros(99_000_000)), 1);
    let r = fit_rect(
        Rect::from_min_size(Pos2::ZERO, vec2(200.0, 100.0)),
        vec2(16.0, 9.0),
    );
    assert!((r.width() / r.height() - 16.0 / 9.0).abs() < 1e-3);
    assert_eq!(r.height(), 100.0);
    let z = fit_rect(Rect::from_min_size(Pos2::ZERO, vec2(2.0, 2.0)), Vec2::ZERO);
    assert_eq!(z.area(), 0.0);
    let (v, m) = timeline::track_rects(Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0)));
    assert!(v.max.y < m.min.y);
    let fast = &crate::stt::builtin_models()[0];
    let text = modals::consent_text(crate::settings::SubtitleQuality::Fast, fast);
    assert!(text.contains("32 MB"));
    assert!(text.contains("MIT"));
    assert!(text.contains("\"Fast\""));
    assert!(text.contains("only times videolore uses the network"));
    let q = modals::unsaved_question(&app, &PendingAction::Quit);
    assert!(q.contains("Untitled") && q.contains("quitting"));
    assert!(effects::LANGUAGES.iter().all(|(c, _)| c.len() == 2));
}

/// Every text painted in one frame.
fn painted_text(ctx: &Context, app: &mut AppState, cache: &mut UiCache, ev: Vec<Event>) -> String {
    fn walk(s: &eframe::epaint::Shape, out: &mut String) {
        match s {
            eframe::epaint::Shape::Text(t) => {
                out.push_str(t.galley.text());
                out.push('\n');
            }
            eframe::epaint::Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let now = Instant::now();
    let mut out = ctx.run_ui(input(1280.0, 2400.0, ev), |ui| {
        frame_ui(ui, app, cache, now)
    });
    let mut text = String::new();
    for c in &out.shapes {
        walk(&c.shape, &mut text);
    }
    out.textures_delta.clear();
    out.drop_without_applying_deltas();
    text
}

// covers: REQ-046
#[test]
fn settings_window_shows_the_three_choices_in_plain_words() {
    use crate::settings::{self, Effort, PreviewQuality, SubtitleQuality};
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    // opened from the keyboard (and from the command bar button)
    painted_text(
        &ctx,
        &mut app,
        &mut cache,
        vec![key(Key::Comma, Modifiers::COMMAND)],
    );
    assert!(app.popups.is_open(Popup::Settings));
    let mut text = String::new();
    for _ in 0..10 {
        text = painted_text(&ctx, &mut app, &mut cache, vec![]);
    }
    for t in [
        settings::PREVIEW_TITLE,
        settings::SUBTITLE_TITLE,
        settings::EFFORT_TITLE,
        settings::RECOMMENDED_STATUS,
        settings::USE_RECOMMENDED,
    ] {
        assert!(text.contains(t), "{t} missing");
    }
    for l in PreviewQuality::ALL.map(|q| q.label()) {
        assert!(text.contains(l), "{l}");
    }
    for l in SubtitleQuality::ALL.map(|q| q.label()) {
        assert!(text.contains(l), "{l}");
    }
    // the last choices may be below the fold of the scrollable window
    assert!(text.contains(Effort::Light.label()));
    // the technical details stay in the closed Advanced section
    assert!(!text.to_lowercase().contains("ffmpeg:"));
    let adv = settings_ui::advanced_lines(&app).join("\n");
    assert!(adv.contains("ffmpeg") && adv.contains("cores") && adv.contains("Processor"));
    // custom settings, an old processor, a chosen ffmpeg, a summary: all drawn
    app.settings.effort = Effort::Maximum;
    app.settings.ffmpeg = Some("/opt/ff/ffmpeg".into());
    app.settings_summary = vec!["Subtitle quality: Fast → Balanced".into()];
    app.machine.arch = "x86_64";
    app.machine.cpu.avx2 = false;
    app.ffmpeg = Err("ffmpeg was not found".into());
    app.ffmpeg_checking = true;
    let text = painted_text(&ctx, &mut app, &mut cache, vec![]);
    assert!(text.contains(settings::CUSTOM_STATUS));
    assert!(text.contains(settings::TOO_OLD_REASON));
    assert!(
        settings_ui::advanced_lines(&app)
            .iter()
            .any(|l| l.contains("not supported") && l.contains("AVX2"))
    );
    assert!(
        settings_ui::advanced_lines(&app)
            .iter()
            .any(|l| l.contains("/opt/ff/ffmpeg"))
    );
    // closing
    app.dispatch(Command::CloseSettings, Instant::now())
        .unwrap();
    let text = painted_text(&ctx, &mut app, &mut cache, vec![]);
    assert!(!text.contains(settings::EFFORT_TITLE));
}

// covers: REQ-060, REQ-064
#[test]
fn settings_window_offers_the_export_choice() {
    use crate::app::HwProbe;
    use crate::settings::{self, ExportChoice};
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    app.popups.show(Popup::Settings);
    for _ in 0..3 {
        painted_text(&ctx, &mut app, &mut cache, vec![]);
    }
    // the Export choice is at the bottom of the main section: scroll down to it
    let down = || {
        vec![
            Event::PointerMoved(pos2(640.0, 1200.0)),
            Event::MouseWheel {
                unit: eframe::egui::MouseWheelUnit::Point,
                delta: vec2(0.0, -400.0),
                phase: eframe::egui::TouchPhase::Move,
                source: eframe::egui::MouseWheelSource::Unknown,
                modifiers: Modifiers::NONE,
            },
        ]
    };
    let mut text = String::new();
    for _ in 0..10 {
        text = painted_text(&ctx, &mut app, &mut cache, down());
    }
    assert!(text.contains(settings::EXPORT_TITLE), "{text}");
    assert!(text.contains(ExportChoice::Faster.label()));
    assert!(text.contains(ExportChoice::BestQuality.label()));
    assert!(!text.contains(settings::NO_HARDWARE_NOTE));
    // Faster: custom settings; with no usable hardware found, the plain note
    app.dispatch(Command::SetExport(ExportChoice::Faster), Instant::now())
        .unwrap();
    app.hw.probe = HwProbe::Done(Default::default());
    for _ in 0..3 {
        text = painted_text(&ctx, &mut app, &mut cache, down());
    }
    assert!(app.settings.export == ExportChoice::Faster && !app.is_recommended());
    assert!(
        text.contains("This computer has no video hardware"),
        "{text}"
    );
    // the Advanced section lists the hardware state and the turned-off line
    app.settings
        .turn_hardware_off(crate::hwenc::Codec::Hevc, crate::hwenc::Vendor::Nvenc, "x");
    let adv = settings_ui::advanced_lines(&app);
    assert!(
        adv.iter()
            .any(|l| l == "Video hardware for H.264: none found")
    );
    assert!(adv.iter().any(|l| l == settings_ui::HARDWARE_OFF_LINE));
    // "Use recommended settings" brings Best quality back
    app.dispatch(Command::UseRecommended, Instant::now())
        .unwrap();
    assert_eq!(app.settings.export, ExportChoice::BestQuality);
    assert!(app.is_recommended());
}

// covers: REQ-045, REQ-048
#[test]
fn banner_is_drawn_without_blocking_the_window() {
    use crate::app::{Banner, BannerAction};
    use crate::settings::StartNotice;
    let (_d, mut app) = loaded();
    let ctx = Context::default();
    let mut cache = UiCache::default();
    app.banner = Some(Banner::from_notice(StartNotice::MachineChanged));
    painted_text(&ctx, &mut app, &mut cache, vec![]);
    let text = painted_text(&ctx, &mut app, &mut cache, vec![]);
    assert!(text.contains(crate::settings::MACHINE_CHANGED_NOTICE));
    assert!(text.contains(crate::settings::USE_RECOMMENDED));
    assert!(!modal_open(&app), "shortcuts keep working");
    app.banner = Some(Banner::from_notice(StartNotice::Setup));
    painted_text(&ctx, &mut app, &mut cache, vec![]);
    let text = painted_text(&ctx, &mut app, &mut cache, vec![]);
    assert!(text.contains(crate::settings::SETUP_NOTICE));
    assert_eq!(
        Banner::from_notice(StartNotice::MachineChanged).action,
        Some(BannerAction::UseRecommended)
    );
}

// covers: REQ-052
#[test]
fn consent_offers_the_fast_file_when_it_makes_sense() {
    use crate::app::ConsentChoice;
    use crate::settings::SubtitleQuality::*;
    let (d, mut app) = loaded();
    let store = crate::stt::tests::store_for(d.path(), b"m");
    app.model_store = Some(store.clone());
    let choices = |app: &AppState, l| -> Vec<ConsentChoice> {
        modals::consent_choices(app, l)
            .into_iter()
            .map(|c| c.1)
            .collect()
    };
    // nothing installed: the small download is offered
    assert_eq!(
        choices(&app, Balanced),
        vec![
            ConsentChoice::Download,
            ConsentChoice::DownloadFastInstead,
            ConsentChoice::LocalFile,
            ConsentChoice::Cancel
        ]
    );
    assert!(
        modals::consent_choices(&app, Balanced)[1]
            .0
            .contains("Fast one instead")
    );
    assert_eq!(
        choices(&app, Fast),
        vec![
            ConsentChoice::Download,
            ConsentChoice::LocalFile,
            ConsentChoice::Cancel
        ]
    );
    // Fast installed: it can be used instead, without download
    std::fs::create_dir_all(&store.dir).unwrap();
    std::fs::write(store.path(Fast), crate::stt::tests::level_body(b"m", 0)).unwrap();
    assert_eq!(
        choices(&app, MostAccurate)[1],
        ConsentChoice::UseFastInstead
    );
    // only another big file installed: no Fast alternative
    std::fs::remove_file(store.path(Fast)).unwrap();
    std::fs::write(store.path(Balanced), crate::stt::tests::level_body(b"m", 1)).unwrap();
    assert_eq!(choices(&app, MostAccurate).len(), 3);
    let text = modals::consent_text(MostAccurate, store.spec(MostAccurate));
    assert!(text.contains("Most accurate") && text.contains("MIT"));
    // the dialog draws
    let ctx = Context::default();
    let mut cache = UiCache::default();
    app.modal = Some(crate::app::Modal::ModelConsent(MostAccurate));
    painted_text(&ctx, &mut app, &mut cache, vec![]);
    let painted = painted_text(&ctx, &mut app, &mut cache, vec![]);
    assert!(painted.contains("Download a speech recognition file?"));
}

/// Export that streams progress for `secs` (until killed); everything else is the usual
/// fake.
struct SlowExport {
    inner: crate::ffmpeg::fake::FakeRunner,
    secs: f32,
    /// Spawned commands that used the video hardware.
    hardware: Arc<std::sync::atomic::AtomicUsize>,
}

struct Ticking {
    left: usize,
    killed: Arc<std::sync::atomic::AtomicBool>,
}

impl std::io::Read for Ticking {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.left == 0 || self.killed.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(0);
        }
        self.left -= 1;
        std::thread::sleep(std::time::Duration::from_millis(20));
        let line = b"out_time_us=100000\n";
        buf[..line.len()].copy_from_slice(line);
        Ok(line.len())
    }
}

struct SlowProcess {
    left: usize,
    out: Option<std::path::PathBuf>,
    killed: Arc<std::sync::atomic::AtomicBool>,
}

impl crate::ffmpeg::Process for SlowProcess {
    fn take_stdout(&mut self) -> Option<Box<dyn std::io::Read + Send>> {
        Some(Box::new(Ticking {
            left: self.left,
            killed: Arc::clone(&self.killed),
        }))
    }
    fn kill(&mut self) {
        self.killed.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    fn wait(&mut self) -> std::io::Result<crate::ffmpeg::CmdOutput> {
        if let Some(o) = self.out.take() {
            std::fs::write(o, b"video")?;
        }
        Ok(crate::ffmpeg::CmdOutput {
            success: !self.killed.load(std::sync::atomic::Ordering::SeqCst),
            ..Default::default()
        })
    }
}

impl crate::ffmpeg::Runner for SlowExport {
    fn run(
        &self,
        p: &std::path::Path,
        a: &[std::ffi::OsString],
    ) -> std::io::Result<crate::ffmpeg::CmdOutput> {
        self.inner.run(p, a)
    }
    fn run_within(
        &self,
        p: &std::path::Path,
        a: &[std::ffi::OsString],
        t: std::time::Duration,
    ) -> std::io::Result<crate::ffmpeg::CmdOutput> {
        self.inner.run_within(p, a, t)
    }
    fn spawn_in(
        &self,
        p: &std::path::Path,
        a: &[std::ffi::OsString],
        c: Option<&std::path::Path>,
    ) -> std::io::Result<Box<dyn crate::ffmpeg::Process>> {
        let last = a
            .last()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default();
        if a.iter().any(|x| x.to_string_lossy().ends_with("_nvenc")) {
            self.hardware
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
        if last.ends_with(".videolore-partial") {
            return Ok(Box::new(SlowProcess {
                left: (self.secs * 50.0) as usize,
                out: last.strip_prefix("file:").map(Into::into),
                killed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            }));
        }
        self.inner.spawn_in(p, a, c)
    }
}

/// The responsiveness gates keep every core busy: they run one after the other, so that
/// their load does not add up (the 200 ms limit is for one export and one speech job).
static BUSY_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn one_gate_at_a_time() -> std::sync::MutexGuard<'static, ()> {
    BUSY_GATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Keeps every core busy until cancelled (speech recognition at "Maximum speed").
struct BusyTranscriber;

impl crate::stt::Transcriber for BusyTranscriber {
    fn transcribe(
        &mut self,
        _pcm: &[f32],
        _offset: Micros,
        _language: Option<&str>,
        cancel: &Arc<std::sync::atomic::AtomicBool>,
    ) -> Result<crate::stt::Transcript, crate::stt::SttError> {
        let n = std::thread::available_parallelism().map_or(2, |n| n.get());
        let deadline = Instant::now() + std::time::Duration::from_secs(3);
        std::thread::scope(|s| {
            for _ in 0..n {
                s.spawn(|| {
                    let mut x = 0u64;
                    while Instant::now() < deadline
                        && !cancel.load(std::sync::atomic::Ordering::Relaxed)
                    {
                        x = std::hint::black_box(x.wrapping_mul(31).wrapping_add(7));
                    }
                });
            }
        });
        Ok(crate::stt::Transcript::default())
    }
}

// covers: REQ-056
#[test]
fn stays_responsive_while_exporting_and_transcribing() {
    let _gate = one_gate_at_a_time();
    use crate::settings::{Effort, SubtitleQuality};
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let hardware = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let fake = SlowExport {
        inner: crate::app::tests::runner().holding_streams(),
        secs: 3.0,
        hardware: Arc::clone(&hardware),
    };
    let mut app = AppState::with_ffmpeg(
        Ok(crate::app::tests::info()),
        Arc::new(fake),
        Box::new(FakeDialogs::default()),
        Box::new(crate::preview::NullSink),
    );
    let now = Instant::now();
    import(&mut app, f);
    app.dispatch(Command::Append(Some(app.project.media[0].id)), now)
        .unwrap();
    app.dispatch(Command::Append(Some(app.project.media[1].id)), now)
        .unwrap();
    app.dispatch(Command::ToggleFade(0), now).unwrap();
    app.dispatch(Command::SetEffort(Effort::Maximum), now)
        .unwrap();
    // the very first frames build the font atlas (not what is measured here)
    let ctx = Context::default();
    let mut cache = UiCache::default();
    for _ in 0..3 {
        app.popups.show(Popup::Settings);
        run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
    }
    // speech recognition keeping every core busy
    let store = crate::stt::tests::store_for(dir.path(), b"m");
    std::fs::create_dir_all(&store.dir).unwrap();
    std::fs::write(
        store.path(SubtitleQuality::Fast),
        crate::stt::tests::level_body(b"m", 0),
    )
    .unwrap();
    app.model_store = Some(store);
    app.settings.subtitle_quality = SubtitleQuality::Fast;
    app.transcriber =
        Arc::new(|_, _| Ok(Box::new(BusyTranscriber) as Box<dyn crate::stt::Transcriber>));
    if crate::stt::available() {
        app.dispatch(Command::GenerateSubtitles, now).unwrap();
        assert!(app.is_generating());
    }
    // Export: Faster, with working (simulated) video hardware
    app.settings.export = crate::settings::ExportChoice::Faster;
    app.hw.probe = crate::app::HwProbe::Done(crate::hwenc::HwStatus {
        h264: Some(crate::hwenc::Candidate::new(crate::hwenc::Vendor::Nvenc)),
        ..Default::default()
    });
    app.dispatch(Command::Export(dir.path().join("busy.mp4")), now)
        .unwrap();
    assert!(app.is_exporting());
    let mut times = Vec::new();
    let timeline = cache.timeline.unwrap().video;
    let mut worst = std::time::Duration::ZERO;
    for i in 0..30 {
        let before_playing = app.preview.is_playing();
        let before_head = app.playhead;
        let (events, check): (Vec<Event>, u8) = match i % 3 {
            0 => (vec![key(Key::Space, Modifiers::NONE)], 0),
            1 => {
                let x = if (i / 3) % 2 == 0 { 120.0 } else { 520.0 };
                let p = pos2(timeline.min.x + x, timeline.center().y);
                (
                    vec![
                        Event::PointerMoved(p),
                        Event::PointerButton {
                            pos: p,
                            button: PointerButton::Primary,
                            pressed: true,
                            modifiers: Modifiers::NONE,
                        },
                        Event::PointerButton {
                            pos: p,
                            button: PointerButton::Primary,
                            pressed: false,
                            modifiers: Modifiers::NONE,
                        },
                    ],
                    1,
                )
            }
            _ => (vec![key(Key::Comma, Modifiers::COMMAND)], 2),
        };
        // the Settings window opened by the previous round would cover the timeline
        app.popups.close(Popup::Settings);
        let t = Instant::now();
        frame(&ctx, &mut app, &mut cache, input(1280.0, 800.0, events));
        let took = t.elapsed();
        times.push(took.as_millis());
        worst = worst.max(took);
        match check {
            0 => assert_ne!(app.preview.is_playing(), before_playing, "frame {i}"),
            1 => assert!(
                app.playhead != before_head || app.preview.is_playing(),
                "frame {i}"
            ),
            _ => assert!(app.popups.is_open(Popup::Settings), "frame {i}"),
        }
    }
    assert!(
        worst <= std::time::Duration::from_millis(200),
        "slowest frame {worst:?} ({times:?} ms)"
    );
    assert!(app.is_exporting(), "the export was still running meanwhile");
    assert_eq!(
        hardware.load(std::sync::atomic::Ordering::SeqCst),
        1,
        "a hardware export"
    );
    app.dispatch(Command::CancelExport, Instant::now()).unwrap();
    app.dispatch(Command::CancelSubtitles, Instant::now())
        .unwrap();
    app.wait_export(Instant::now());
}

// covers: REQ-056, REQ-066, REQ-073, REQ-021
#[test]
fn borders_and_popups_stay_responsive_and_export_cancels_from_the_status_bar() {
    let _gate = one_gate_at_a_time();
    use super::ux_tests::Harness;
    use crate::settings::{Effort, SubtitleQuality};
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let fake = SlowExport {
        inner: crate::app::tests::runner().holding_streams(),
        secs: 5.0,
        hardware: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let mut app = AppState::with_ffmpeg(
        Ok(crate::app::tests::info()),
        Arc::new(fake),
        Box::new(FakeDialogs::default()),
        Box::new(crate::preview::NullSink),
    );
    let now = Instant::now();
    import(&mut app, f);
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::SetEffort(Effort::Maximum), now)
        .unwrap();
    for p in [Popup::Fades, Popup::Subtitles, Popup::Volume] {
        app.dispatch(Command::OpenPopup(p), now).unwrap();
    }
    // the first frames build the font atlas (not what is measured here)
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.idle(3);
    let store = crate::stt::tests::store_for(dir.path(), b"m");
    std::fs::create_dir_all(&store.dir).unwrap();
    std::fs::write(
        store.path(SubtitleQuality::Fast),
        crate::stt::tests::level_body(b"m", 0),
    )
    .unwrap();
    h.app.model_store = Some(store);
    h.app.settings.subtitle_quality = SubtitleQuality::Fast;
    h.app.transcriber =
        Arc::new(|_, _| Ok(Box::new(BusyTranscriber) as Box<dyn crate::stt::Transcriber>));
    if crate::stt::available() {
        h.app.dispatch(Command::GenerateSubtitles, now).unwrap();
    }
    let out = dir.path().join("busy.mp4");
    h.app.dispatch(Command::Export(out.clone()), now).unwrap();
    assert!(h.app.is_exporting());
    // drag the Timeline | Effects border for 30 frames, timing each one
    let from = h.cache.sections.unwrap().splitters[1].1.center();
    let btn = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let mut worst = Duration::ZERO;
    let mut times = Vec::new();
    for i in 0..30 {
        let p = from - vec2(4.0 * i as f32, 0.0);
        let events = match i {
            0 => vec![Event::PointerMoved(p)],
            1 => vec![btn(p, true)],
            29 => vec![btn(p, false)],
            _ => vec![Event::PointerMoved(p)],
        };
        let t = Instant::now();
        h.frame(events);
        let took = t.elapsed();
        times.push(took.as_millis());
        worst = worst.max(took);
        if (3..29).contains(&i) {
            assert!(h.cache.layout_drag.is_some(), "frame {i}");
        }
    }
    assert!(
        worst <= Duration::from_millis(200),
        "slowest frame {worst:?} ({times:?} ms)"
    );
    h.idle(1);
    assert!(h.app.settings.layout.is_some(), "the drag was applied");
    assert!(
        h.app.is_exporting(),
        "the export was still running meanwhile"
    );
    // the status bar shows the export with its progress and Cancel
    let bar_top = h.size.y - 30.0;
    assert!(
        h.nodes
            .iter()
            .any(|n| n.label.starts_with("Exporting") && n.rect.min.y > bar_top),
        "{:#?}",
        h.labels()
    );
    let cancel = h
        .nodes
        .iter()
        .filter(|n| n.label == "Cancel" && n.rect.min.y > bar_top)
        .max_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x))
        .cloned()
        .expect("Cancel in the status bar");
    h.click(cancel.rect.center());
    h.app.wait_export(Instant::now());
    assert!(!h.app.is_exporting());
    assert_eq!(h.app.status, "Export cancelled");
    // REQ-021: nothing is left behind
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains("busy"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
    h.app
        .dispatch(Command::CancelSubtitles, Instant::now())
        .unwrap();
}

/// Dragging an effect onto a clip and a subtitle in the preview while an export runs and
/// subtitles are being matched to the speech: every frame stays short.
// covers: REQ-056, REQ-082, REQ-089, REQ-095
#[test]
fn effect_and_subtitle_drags_stay_responsive_while_exporting_and_matching() {
    let _gate = one_gate_at_a_time();
    use super::ux_tests::Harness;
    use crate::app::{EffectKind, Instance, Modal, SourceChoice, Target};
    use crate::settings::{Effort, SubtitleQuality};
    let dir = tempfile::tempdir().unwrap();
    let f = files(dir.path());
    let fake = SlowExport {
        inner: crate::app::tests::runner().holding_streams(),
        secs: 5.0,
        hardware: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let mut app = AppState::with_ffmpeg(
        Ok(crate::app::tests::info()),
        Arc::new(fake),
        Box::new(FakeDialogs::default()),
        Box::new(crate::preview::NullSink),
    );
    let now = Instant::now();
    import(&mut app, f);
    for i in [0, 1] {
        app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
            .unwrap();
    }
    app.dispatch(Command::SetEffort(Effort::Maximum), now)
        .unwrap();
    app.dispatch(Command::Seek(Micros(1_000_000)), now).unwrap();
    app.dispatch(Command::AddSubtitle, now).unwrap();
    let line = app.selected_line.unwrap();
    app.dispatch(Command::ClearSelection, now).unwrap();
    app.frame = Some(crate::preview::Frame {
        pts: Micros::ZERO,
        width: 64,
        height: 36,
        rgba: vec![0; 64 * 36 * 4],
    });
    app.frame_serial += 1;
    let mut h = Harness::new(app, 1280.0, 800.0);
    h.idle(3);
    // matching the video's subtitles to the speech, every core busy
    let store = crate::stt::tests::store_for(dir.path(), b"m");
    std::fs::create_dir_all(&store.dir).unwrap();
    std::fs::write(
        store.path(SubtitleQuality::Fast),
        crate::stt::tests::level_body(b"m", 0),
    )
    .unwrap();
    h.app.model_store = Some(store);
    h.app.settings.subtitle_quality = SubtitleQuality::Fast;
    h.app.transcriber =
        Arc::new(|_, _| Ok(Box::new(BusyTranscriber) as Box<dyn crate::stt::Transcriber>));
    h.app.project.media[1].format.subtitle_tracks = Some(vec![crate::media::SubtitleTrack {
        ordinal: 0,
        codec: crate::media::TextCodec::Srt,
        language: Some("eng".into()),
        title: None,
    }]);
    let clips: Vec<_> = h.app.project.timeline.clips.iter().map(|c| c.id).collect();
    if crate::stt::available() {
        h.app
            .dispatch(
                Command::ApplyEffect {
                    effect: EffectKind::Subtitles,
                    target: Target::Clip(clips[1]),
                },
                now,
            )
            .unwrap();
        assert!(matches!(h.app.modal, Some(Modal::SubtitleSource { .. })));
        h.app
            .dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
            .unwrap();
        assert!(h.app.is_generating() && h.app.stt_matching);
        h.app
            .dispatch(Command::ClosePopup(Popup::Subtitles), now)
            .unwrap();
    }
    h.app
        .dispatch(Command::Export(dir.path().join("busy.mp4")), now)
        .unwrap();
    assert!(h.app.is_exporting());
    h.idle(2);
    let row = h.find("Volume").unwrap().rect.center();
    let clip = h.cache.timeline.unwrap().video;
    let target = pos2(
        h.cache.timeline.unwrap().x_of(Micros(2_000_000)),
        clip.center().y,
    );
    let pic = h.find("Drag a subtitle to move it").unwrap().rect;
    let grab = pos2(pic.center().x, pic.min.y + pic.height() * 0.91);
    let btn = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    let mut worst = Duration::ZERO;
    let mut times = Vec::new();
    let mut timed = |h: &mut Harness, events: Vec<Event>| {
        let t = Instant::now();
        h.frame(events);
        let took = t.elapsed();
        times.push(took.as_millis());
        worst = worst.max(took);
    };
    // the effect row dragged onto the first clip
    timed(&mut h, vec![Event::PointerMoved(row)]);
    timed(&mut h, vec![btn(row, true)]);
    for i in 1..=10 {
        let p = row + (target - row) * (i as f32 / 10.0);
        timed(&mut h, vec![Event::PointerMoved(p)]);
    }
    timed(&mut h, vec![btn(target, false)]);
    timed(&mut h, vec![]);
    assert_eq!(
        h.app.popups.instance,
        Some(Instance::ClipVolume(clips[0])),
        "the drop applied Volume to the clip"
    );
    h.app
        .dispatch(Command::ClosePopup(Popup::Instance), now)
        .unwrap();
    // the subtitle dragged across the preview
    let to = pic.center();
    timed(&mut h, vec![Event::PointerMoved(grab)]);
    timed(&mut h, vec![btn(grab, true)]);
    for i in 1..=12 {
        let p = grab + (to - grab) * (i as f32 / 12.0);
        timed(&mut h, vec![Event::PointerMoved(p)]);
    }
    assert!(h.app.drag_pos.is_some(), "the line follows the pointer");
    timed(&mut h, vec![btn(to, false)]);
    timed(&mut h, vec![]);
    assert!(
        worst <= Duration::from_millis(200),
        "slowest frame {worst:?} ({times:?} ms)"
    );
    assert!(
        h.app
            .project
            .subtitles
            .get(line)
            .unwrap()
            .position
            .is_some()
    );
    assert!(h.app.is_exporting(), "the export was still running");
    h.app.dispatch(Command::CancelExport, now).unwrap();
    h.app.wait_export(Instant::now());
    h.app
        .dispatch(Command::CancelSubtitles, Instant::now())
        .unwrap();
}

/// Looking around a 360 picture and resizing a box while an export runs and subtitles
/// are being matched to the speech, on 4K x 2K and 8K x 4K spheres: every frame of the
/// interface stays short (REQ-056 gate, extended by REQ-104).
// covers: REQ-104, REQ-056, REQ-101, REQ-097
#[test]
fn look_around_and_resize_stay_responsive_on_big_spheres_while_exporting_and_matching() {
    let _gate = one_gate_at_a_time();
    use super::ux_tests::Harness;
    use crate::app::{EffectKind, Modal, SourceChoice, Target};
    use crate::settings::{Effort, SubtitleQuality};
    for (w, hgt) in [(3840u32, 1920u32), (7680, 3840)] {
        let dir = tempfile::tempdir().unwrap();
        let f = files(dir.path());
        let fake = SlowExport {
            inner: crate::app::tests::runner().holding_streams(),
            secs: 5.0,
            hardware: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        };
        let mut app = AppState::with_ffmpeg(
            Ok(crate::app::tests::info()),
            Arc::new(fake),
            Box::new(FakeDialogs::default()),
            Box::new(crate::preview::NullSink),
        );
        let now = Instant::now();
        import(&mut app, f);
        app.project.media[0].format.width = w;
        app.project.media[0].format.height = hgt;
        app.project.media[0].format.spherical = Some(true);
        for i in [0, 1] {
            app.dispatch(Command::Append(Some(app.project.media[i].id)), now)
                .unwrap();
        }
        app.dispatch(Command::SetEffort(Effort::Maximum), now)
            .unwrap();
        app.dispatch(Command::Seek(Micros(1_000_000)), now).unwrap();
        app.dispatch(Command::AddSubtitle, now).unwrap();
        let line = app.selected_line.unwrap();
        app.dispatch(
            Command::SetLinePositions {
                ids: vec![line],
                pos: Some(crate::subtitles::Pos { x: 5000, y: 5000 }),
            },
            now,
        )
        .unwrap();
        app.dispatch(Command::SetSubtitleText(line, "HHHH".into()), now)
            .unwrap();
        app.dispatch(Command::ClearSelection, now).unwrap();
        app.frame = Some(crate::preview::Frame {
            pts: Micros::ZERO,
            width: 64,
            height: 36,
            rgba: vec![0; 64 * 36 * 4],
        });
        app.frame_serial += 1;
        let mut h = Harness::new(app, 1280.0, 800.0);
        h.idle(3);
        let store = crate::stt::tests::store_for(dir.path(), b"m");
        std::fs::create_dir_all(&store.dir).unwrap();
        std::fs::write(
            store.path(SubtitleQuality::Fast),
            crate::stt::tests::level_body(b"m", 0),
        )
        .unwrap();
        h.app.model_store = Some(store);
        h.app.settings.subtitle_quality = SubtitleQuality::Fast;
        h.app.transcriber =
            Arc::new(|_, _| Ok(Box::new(BusyTranscriber) as Box<dyn crate::stt::Transcriber>));
        h.app.project.media[1].format.subtitle_tracks = Some(vec![crate::media::SubtitleTrack {
            ordinal: 0,
            codec: crate::media::TextCodec::Srt,
            language: Some("eng".into()),
            title: None,
        }]);
        let clip = h.app.project.timeline.clips[1].id;
        if crate::stt::available() {
            h.app
                .dispatch(
                    Command::ApplyEffect {
                        effect: EffectKind::Subtitles,
                        target: Target::Clip(clip),
                    },
                    now,
                )
                .unwrap();
            assert!(matches!(h.app.modal, Some(Modal::SubtitleSource { .. })));
            h.app
                .dispatch(Command::ChooseSubtitleSource(SourceChoice::Use(0)), now)
                .unwrap();
            assert!(h.app.is_generating());
            h.app
                .dispatch(Command::ClosePopup(Popup::Subtitles), now)
                .unwrap();
        }
        h.app
            .dispatch(Command::Export(dir.path().join("busy.mp4")), now)
            .unwrap();
        assert!(h.app.is_exporting());
        h.idle(2);
        let pic = h.find("Drag to look around").unwrap().rect;
        let btn = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        let mut worst = Duration::ZERO;
        let mut times = Vec::new();
        let mut timed = |h: &mut Harness, events: Vec<Event>| {
            let t = Instant::now();
            h.frame(events);
            let took = t.elapsed();
            times.push(took.as_millis());
            worst = worst.max(took);
        };
        // (1) the camera dragged across the picture, with wheel zoom
        let from = pic.min + vec2(pic.width() * 0.2, pic.height() * 0.3);
        timed(&mut h, vec![Event::PointerMoved(from)]);
        timed(&mut h, vec![btn(from, true)]);
        for i in 1..=15 {
            let p = from + vec2(8.0 * i as f32, 3.0 * i as f32);
            timed(&mut h, vec![Event::PointerMoved(p)]);
        }
        timed(&mut h, vec![btn(from + vec2(120.0, 45.0), false)]);
        for _ in 0..5 {
            timed(
                &mut h,
                vec![Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, 40.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        let m = h.app.project.media[0].id;
        assert!(h.app.live.contains_key(&m), "the camera turned");
        // (2) a corner of the subtitle's box dragged
        let hs = {
            let raw = RawInput {
                screen_rect: Some(h.screen()),
                ..Default::default()
            };
            let now = Instant::now();
            let (app, cache) = (&mut h.app, &mut h.cache);
            let mut out = h.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
            let v: Vec<Rect> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r) if (r.rect.width() - 7.0).abs() < 0.01 => Some(r.rect),
                    _ => None,
                })
                .collect();
            out.textures_delta.clear();
            out.drop_without_applying_deltas();
            v
        };
        let corner = hs
            .iter()
            .max_by(|a, b| (a.center().x + a.center().y).total_cmp(&(b.center().x + b.center().y)))
            .expect("handles")
            .center();
        timed(&mut h, vec![Event::PointerMoved(corner)]);
        timed(&mut h, vec![btn(corner, true)]);
        for i in 1..=10 {
            timed(
                &mut h,
                vec![Event::PointerMoved(
                    corner + vec2(4.0 * i as f32, 2.0 * i as f32),
                )],
            );
        }
        assert!(h.app.resize_drag.is_some(), "the box follows the pointer");
        timed(&mut h, vec![btn(corner + vec2(40.0, 20.0), false)]);
        timed(&mut h, vec![]);
        assert!(h.app.project.subtitles.get(line).unwrap().size.is_some());
        assert!(
            worst <= Duration::from_millis(200),
            "{w}x{hgt}: slowest frame {worst:?} ({times:?} ms)"
        );
        assert!(h.app.is_exporting(), "the export was still running");
        h.app.dispatch(Command::CancelExport, now).unwrap();
        h.app.wait_export(Instant::now());
        h.app
            .dispatch(Command::CancelSubtitles, Instant::now())
            .unwrap();
    }
}
