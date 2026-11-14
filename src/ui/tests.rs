// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Headless UI tests: whole frames are run without a window or painter.

use super::*;
use crate::app::tests::{FakeDialogs, files, import, settle, state};
use crate::app::{Modal, PendingAction};
use crate::media::{MediaStatus, ProxyState};
use crate::preview::Frame;
use eframe::egui::{
    Event, Key, Modifiers, PointerButton, Pos2, RawInput, ViewportCommand, ViewportEvent,
    ViewportId, ViewportInfo,
};

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
        Modal::ModelConsent,
        Modal::ConfirmRegenerate,
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
    // fit: the whole 10 s timeline fits in the panel width
    assert!(fit * 10.0 <= 1280.0 * layout::TIMELINE_WIDTH);
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
    let r = layout::panels(Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0)));
    // the video track is in the upper part of the timeline panel
    let video_y = r.timeline.min.y + r.timeline.height() * 0.35;
    click_at(
        &ctx,
        &mut app,
        &mut cache,
        pos2(r.timeline.min.x + 60.0, video_y),
    );
    assert_eq!(app.selected_clip, Some(app.project.timeline.clips[0].id));
    assert!(app.playhead > Micros::ZERO);
    // the music track is below it, in the same panel
    let music_y = r.timeline.min.y + r.timeline.height() * 0.75;
    click_at(
        &ctx,
        &mut app,
        &mut cache,
        pos2(r.timeline.min.x + 80.0, music_y),
    );
    let m = app.project.music.clips[0].id;
    assert_eq!(app.selected_music, Some(m));
    // drag the music clip to the right
    let from = pos2(r.timeline.min.x + 80.0, music_y);
    let to = pos2(r.timeline.min.x + 200.0, music_y);
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
    // the effects panel shows the selected music clip
    app.dispatch(Command::SelectMusic(Some(m)), Instant::now())
        .unwrap();
    run(&ctx, &mut app, &mut cache, 1280.0, 800.0, vec![]);
}

// covers: REQ-020, REQ-016
#[test]
fn volume_and_subtitle_editing_from_the_panel() {
    let (_d, mut app) = loaded();
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
    assert!(modals::consent_text().contains("32 MB"));
    assert!(modals::consent_text().contains("MIT"));
    assert!(modals::consent_text().contains("only time videolore uses the network"));
    let q = modals::unsaved_question(&app, &PendingAction::Quit);
    assert!(q.contains("Untitled") && q.contains("quitting"));
    assert!(effects::LANGUAGES.iter().all(|(c, _)| c.len() == 2));
}
