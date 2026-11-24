// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of videolore player through egui's accessibility tree: every widget
//! with its label and bounds, clicks and drags as real pointer events.

use crate::app::{Command, PlayerState};
use crate::fullscreen::{HIDE_AFTER, HINT_TEXT, HINT_TIME};
use crate::ui::{PlayerCache, frame_ui};
use eframe::egui::accesskit::{Action, Role};
use eframe::egui::{
    Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, pos2, vec2,
};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use videolore::preview::Frame;
use videolore::time::Micros;
use videolore::ui::placement;

#[derive(Debug, Clone)]
struct Node {
    role: Role,
    label: String,
    raw: String,
    rect: Rect,
    disabled: bool,
}

/// A label without the icon (a private-use character and a space) that starts it.
fn without_icon(label: &str) -> String {
    let mut it = label.chars();
    match (it.next(), it.next()) {
        (Some(c), Some(' ')) if ('\u{e000}'..='\u{f8ff}').contains(&c) => it.collect(),
        _ => label.to_string(),
    }
}

struct Harness {
    ctx: Context,
    app: PlayerState,
    cache: PlayerCache,
    size: Vec2,
    nodes: Vec<Node>,
    /// The clock the frames run on (moved by the tests).
    now: Instant,
}

impl Harness {
    fn new(app: PlayerState) -> Harness {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut h = Harness {
            ctx,
            app,
            cache: PlayerCache::default(),
            size: vec2(960.0, 600.0),
            nodes: Vec::new(),
            now: Instant::now(),
        };
        h.idle(3);
        h
    }

    fn frame_with(&mut self, raw: RawInput) {
        let (app, cache, now) = (&mut self.app, &mut self.cache, self.now);
        let mut out = self.ctx.run_ui(raw, |ui| frame_ui(ui, app, cache, now));
        self.nodes = out
            .platform_output
            .accesskit_update
            .take()
            .map(|u| {
                u.nodes
                    .into_iter()
                    .filter_map(|(_, n)| {
                        let b = n.bounds()?;
                        let raw = n.label().or(n.value()).unwrap_or_default().to_string();
                        Some(Node {
                            role: n.role(),
                            label: without_icon(&raw),
                            raw,
                            rect: Rect::from_min_max(
                                pos2(b.x0 as f32, b.y0 as f32),
                                pos2(b.x1 as f32, b.y1 as f32),
                            ),
                            disabled: n.is_disabled(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.textures_delta.clear();
        out.drop_without_applying_deltas();
        let _ = Action::Click;
    }

    fn frame(&mut self, events: Vec<Event>) {
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        self.frame_with(raw);
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(Vec::new());
        }
    }

    fn find(&self, label: &str) -> Option<Node> {
        let exact = self.nodes.iter().find(|n| n.label == label);
        exact
            .or_else(|| self.nodes.iter().find(|n| n.label.starts_with(label)))
            .cloned()
    }

    fn has(&self, label: &str) -> bool {
        self.find(label).is_some()
    }

    fn click(&mut self, p: Pos2) {
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(p), ev(true)]);
        self.frame(vec![ev(false)]);
        self.idle(3);
    }

    fn click_label(&mut self, label: &str) {
        let n = self
            .find(label)
            .unwrap_or_else(|| panic!("no widget \"{label}\" in {:#?}", self.labels()));
        self.click(n.rect.center());
    }

    fn drag(&mut self, from: Pos2, to: Pos2) {
        let ev = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![ev(from, true)]);
        for i in 1..=5 {
            let p = from + (to - from) * (i as f32 / 5.0);
            self.frame(vec![Event::PointerMoved(p)]);
        }
        self.frame(vec![ev(to, false)]);
        self.idle(3);
    }

    fn key(&mut self, k: Key) {
        self.frame(vec![Event::Key {
            key: k,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        self.idle(3);
    }

    fn labels(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter(|n| !n.label.is_empty())
            .map(|n| format!("{:?} {} {:?}", n.role, n.raw, n.rect))
            .collect()
    }

    /// Moves the clock and draws a frame.
    fn wait(&mut self, d: Duration) {
        self.now += d;
        self.idle(3);
    }
}

/// A file dropped on the window.
#[derive(Debug)]
struct Dropped(std::path::PathBuf);

impl eframe::egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

// ---- a player with a scripted ffprobe ----

use super::tests::{Fake, FakeDialogs, info};

fn state(work: &Path) -> PlayerState {
    PlayerState::new(
        Ok(info()),
        Arc::new(Fake),
        Box::new(FakeDialogs {
            video: None,
            srt: None,
        }),
        Box::new(videolore::preview::NullSink),
        videolore::machine::Machine::detect(),
        crate::app::Folders {
            cache_dir: None,
            work_root: work.to_path_buf(),
        },
        Instant::now(),
    )
}

fn open_video(h: &mut Harness, dir: &Path, name: &str) {
    let p = dir.join(name);
    std::fs::write(&p, name).unwrap();
    h.app.dispatch(Command::Open(p), Instant::now());
    let end = Instant::now() + Duration::from_secs(10);
    while h.app.opening() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
        h.idle(1);
    }
    // a still picture, as the preview would deliver
    let (w, hh) = (320u32, 180u32);
    h.app.frame = Some(Frame {
        pts: Micros::ZERO,
        width: w,
        height: hh,
        rgba: vec![90; (w * hh * 4) as usize],
    });
    h.app.frame_serial += 1;
    h.idle(3);
}

// covers: REQ-138
// covers: REQ-142
#[test]
fn the_window_has_menus_and_controls_each_with_an_icon() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    for menu in ["File", "View", "Audio", "Subtitles", "Help"] {
        assert!(h.has(menu), "{menu}: {:#?}", h.labels());
    }
    // the controls: every button shows its icon
    for b in ["Play", "Start", "Sound"] {
        let n = h
            .find(b)
            .unwrap_or_else(|| panic!("{b}: {:#?}", h.labels()));
        assert!(
            n.raw.ends_with(b) && n.raw.chars().count() == b.chars().count() + 2,
            "{:?}",
            n.raw
        );
        assert!(n.disabled, "{b} waits for a video");
    }
    // every item of every menu has an icon too
    for (menu, items) in [
        ("File", vec!["Open…", "Quit"]),
        ("View", vec!["Full screen"]),
        ("Subtitles", vec!["None", "Open .srt…"]),
        ("Help", vec!["About videolore player"]),
    ] {
        h.click_label(menu);
        for it in &items {
            let n = h
                .find(it)
                .unwrap_or_else(|| panic!("{menu} > {it}: {:#?}", h.labels()));
            assert!(
                n.raw
                    .chars()
                    .next()
                    .is_some_and(|c| ('\u{e000}'..='\u{f8ff}').contains(&c)),
                "{menu} > {it} has no icon: {:?}",
                n.raw
            );
        }
        h.key(Key::Escape);
        h.click(pos2(480.0, 300.0));
        h.idle(2);
    }
}

// covers: REQ-138
#[test]
fn file_open_menu_item_and_a_dropped_file_open_a_video_and_a_project_is_refused() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    let movie = d.path().join("clip.mp4");
    std::fs::write(&movie, "x").unwrap();
    // a dropped file opens
    h.frame_with(RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, h.size)),
        dropped_files: vec![Arc::new(Dropped(movie.clone()))],
        ..Default::default()
    });
    let end = Instant::now() + Duration::from_secs(10);
    while h.app.opening() {
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(2));
        h.idle(1);
    }
    assert!(h.app.has_video());
    assert_eq!(h.app.title(), "clip.mp4 — videolore player");
    // a dropped project is refused with a message that OK closes
    let proj = d.path().join("work.vlproj");
    std::fs::write(&proj, "{}").unwrap();
    h.frame_with(RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, h.size)),
        dropped_files: vec![Arc::new(Dropped(proj))],
        ..Default::default()
    });
    h.idle(2);
    assert!(h.has(crate::app::PROJECT_REFUSED), "{:#?}", h.labels());
    assert!(h.app.has_video(), "the video that was open stays");
    h.click_label("OK");
    assert!(h.app.notice.is_none());
}

// covers: REQ-140, REQ-253
#[test]
fn the_subtitles_menu_lists_none_the_tracks_and_the_opened_files() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "tracks.mkv");
    let srt = d.path().join("film.srt");
    std::fs::write(&srt, super::tests::SRT).unwrap();
    h.app.dispatch(Command::OpenSrt(srt), Instant::now());
    h.idle(2);
    h.click_label("Subtitles");
    for name in [
        "None",
        "English — Main",
        "Italian",
        "French (picture)",
        "film.srt (file)",
        "Open .srt…",
    ] {
        assert!(h.has(name), "{name}: {:#?}", h.labels());
    }
    assert!(!h.find("Open .srt…").unwrap().disabled);
    // choosing an item runs its command
    h.click_label("None");
    assert_eq!(h.app.subs.selected, None);
    h.click_label("Subtitles");
    h.click_label("film.srt (file)");
    assert_eq!(h.app.subs.selected, Some(3));
    assert_eq!(h.app.project.subtitles.lines.len(), 2);
    // the picture track is drawn by the graph from the file: it has no lines and takes the
    // lines of the file away; the next choice takes the picture track away (REQ-253)
    h.click_label("Subtitles");
    h.click_label("French (picture)");
    assert_eq!(h.app.subs.selected, Some(2));
    assert!(h.app.project.picture_sub.is_some() && h.app.project.subtitles.lines.is_empty());
    h.click_label("Subtitles");
    h.click_label("film.srt (file)");
    assert!(h.app.project.picture_sub.is_none());
    assert_eq!(h.app.project.subtitles.lines.len(), 2);
}

// covers: REQ-338
#[test]
fn the_audio_menu_lists_the_tracks_and_choosing_one_runs_its_command() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    // no video: nothing to choose
    h.click_label("Audio");
    assert!(h.find("No audio track").unwrap().disabled);
    h.key(Key::Escape);
    h.click(pos2(480.0, 300.0));
    h.idle(2);
    open_video(&mut h, d.path(), "sounds.mkv");
    h.click_label("Audio");
    for name in ["English (stereo)", "Italian — Film (6 channels)"] {
        assert!(h.has(name), "{name}: {:#?}", h.labels());
    }
    h.click_label("Italian — Film (6 channels)");
    assert_eq!(h.app.audio_track(), 1);
    // the menu shows the playing track as the chosen one
    h.click_label("Audio");
    h.click_label("English (stereo)");
    assert_eq!(h.app.audio_track(), 0);
}

// covers: REQ-141
#[test]
fn full_screen_hides_the_menu_shows_the_hint_and_toggles_with_f11_and_f() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    assert!(h.has("File") && h.has("Play"));
    h.key(Key::F11);
    assert!(h.app.fullscreen.on);
    assert!(!h.has("File") && !h.has("Subtitles"), "the menu is hidden");
    assert!(h.has(HINT_TEXT), "{:#?}", h.labels());
    assert!(h.has("Play"), "the controls stay for a moment");
    // OK on the popup closes it; F11 and F toggle both ways
    h.click_label("OK");
    assert!(!h.has(HINT_TEXT) && h.app.fullscreen.on);
    h.key(Key::F11);
    assert!(!h.app.fullscreen.on && h.has("File"));
    h.key(Key::F);
    assert!(
        h.app.fullscreen.on && h.has(HINT_TEXT),
        "the hint shows again"
    );
    h.key(Key::F);
    assert!(!h.app.fullscreen.on);
    // the View menu does it too
    h.click_label("View");
    h.click_label("Full screen");
    assert!(h.app.fullscreen.on);
}

// covers: REQ-141
#[test]
fn escape_closes_the_popup_first_then_leaves_and_the_controls_hide_when_idle() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    h.key(Key::F11);
    assert!(h.has(HINT_TEXT));
    h.key(Key::Escape);
    assert!(
        h.app.fullscreen.on && !h.has(HINT_TEXT),
        "the popup closed first"
    );
    h.key(Key::Escape);
    assert!(!h.app.fullscreen.on, "Esc leaves full screen");
    // the popup closes by itself after 5 seconds
    h.key(Key::F11);
    h.wait(HINT_TIME);
    assert!(h.app.fullscreen.on && !h.has(HINT_TEXT));
    // the controls hide after 2 idle seconds and come back with the pointer
    h.frame(vec![Event::PointerMoved(pos2(300.0, 300.0))]);
    assert!(h.has("Play"));
    h.wait(HIDE_AFTER + Duration::from_millis(100));
    assert!(!h.has("Play"), "{:#?}", h.labels());
    h.frame(vec![Event::PointerMoved(pos2(320.0, 310.0))]);
    assert!(h.has("Play"));
}

// covers: REQ-138
#[test]
fn space_and_the_buttons_play_pause_mute_and_seek() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    assert!(!h.app.is_playing());
    h.key(Key::Space);
    assert!(h.app.is_playing());
    assert!(h.has("Pause"));
    h.click_label("Pause");
    assert!(!h.app.is_playing());
    h.key(Key::M);
    assert!(h.app.muted());
    h.click_label("Sound");
    assert!(!h.app.muted());
    h.key(Key::ArrowRight);
    assert_eq!(h.app.playhead, Micros(5_000_000));
    h.key(Key::ArrowLeft);
    assert_eq!(h.app.playhead, Micros::ZERO);
    h.app
        .dispatch(Command::Seek(Micros(2_000_000)), Instant::now());
    h.click_label("Start");
    assert_eq!(h.app.playhead, Micros::ZERO);
    assert!(h.has("00:00:00.000 / 00:00:05.000"), "{:#?}", h.labels());
    // About opens with the player's own list and closes with Esc
    h.click_label("Help");
    h.click_label("About videolore player");
    assert!(h.app.show_about);
    h.idle(2);
    assert!(
        h.nodes
            .iter()
            .any(|n| n.raw.contains("videolore player") && n.raw.contains("GPL"))
    );
    h.key(Key::Escape);
    assert!(!h.app.show_about);
    // Ctrl+O asks for a file (the test dialog says no); Ctrl+Q quits
    h.frame(vec![Event::Key {
        key: Key::O,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Key {
        key: Key::Q,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    assert!(h.app.quit);
}

fn picture(h: &Harness) -> Rect {
    h.cache.picture.expect("a picture is drawn")
}

// covers: REQ-139
#[test]
fn dragging_a_360_picture_turns_the_camera_and_a_flat_one_does_not() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    let r = picture(&h);
    h.drag(r.center(), r.center() + vec2(60.0, 20.0));
    assert_eq!(
        h.app.camera(),
        videolore::camera::Camera::FRONT,
        "a flat video has no turning"
    );
    open_video(&mut h, d.path(), "sphere.mp4");
    let r = picture(&h);
    h.drag(r.center(), r.center() + vec2(60.0, 20.0));
    let cam = h.app.camera();
    assert_ne!(cam, videolore::camera::Camera::FRONT);
    assert!(cam.yaw_cd < 0 && cam.pitch_cd > 0, "{cam:?}");
    // the wheel zooms
    let before = cam.fov_cd;
    h.frame(vec![
        Event::PointerMoved(r.center()),
        Event::MouseWheel {
            unit: eframe::egui::MouseWheelUnit::Point,
            delta: vec2(0.0, 40.0),
            phase: eframe::egui::TouchPhase::Move,
            source: eframe::egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.idle(2);
    assert_ne!(h.app.camera().fov_cd, before);
}

// covers: REQ-140
#[test]
fn a_subtitle_line_is_dragged_to_another_place_on_the_picture() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    let srt = d.path().join("film.srt");
    std::fs::write(&srt, super::tests::SRT).unwrap();
    h.app.dispatch(Command::OpenSrt(srt), Instant::now());
    h.app
        .dispatch(Command::Seek(Micros(1_500_000)), Instant::now());
    h.idle(3);
    let frame = h.app.output_size().unwrap();
    let boxes = crate::ui::line_boxes(&h.ctx, &h.app);
    assert_eq!(boxes.len(), 1, "the line shown at 1.5 s");
    let (id, rect) = boxes[0];
    let pic = picture(&h);
    let from = placement::to_screen(rect, pic, frame).center();
    let to = pic.center() + vec2(0.0, -pic.height() * 0.3);
    h.drag(from, to);
    let line = h.app.project.subtitles.get(id).unwrap();
    let pos = line.position.expect("the line has its own place now");
    let (fx, fy) = pos.fraction();
    assert!(
        (fx - 0.5).abs() < 0.1 && (fy - 0.2).abs() < 0.1,
        "{fx} {fy}"
    );
    // a flat video: dragging a line did not turn any camera
    assert_eq!(h.app.camera(), videolore::camera::Camera::FRONT);
    // reopening the file gives the default place
    open_video(&mut h, d.path(), "a.mp4");
    assert!(h.app.project.subtitles.lines.is_empty());
}

#[test]
fn clicking_the_picture_plays_and_a_double_click_goes_full_screen() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    let c = picture(&h).center();
    h.click(c);
    assert!(h.app.is_playing());
    h.click(c);
    assert!(!h.app.is_playing());
    let ev = |pressed| Event::PointerButton {
        pos: c,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    for e in [ev(true), ev(false), ev(true), ev(false)] {
        h.frame(vec![Event::PointerMoved(c), e]);
    }
    h.idle(2);
    assert!(h.app.fullscreen.on);
}

// covers: REQ-138
#[test]
fn at_the_smallest_window_every_control_and_popup_is_inside_the_window() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    h.size = vec2(800.0, 500.0);
    open_video(&mut h, d.path(), "tracks.mkv");
    let screen = Rect::from_min_size(Pos2::ZERO, h.size).expand(1.0);
    let outside = |h: &Harness| -> Vec<String> {
        h.nodes
            .iter()
            .filter(|n| {
                matches!(n.role, Role::Button | Role::Slider | Role::Window)
                    && !n.label.is_empty()
                    && !screen.contains_rect(n.rect)
            })
            .map(|n| format!("{:?} {} {:?}", n.role, n.raw, n.rect))
            .collect()
    };
    assert!(outside(&h).is_empty(), "{:#?}", outside(&h));
    // the About window, a notice and the full-screen hint fit too
    h.app.dispatch(Command::ShowAbout, Instant::now());
    h.idle(3);
    assert!(outside(&h).is_empty(), "about: {:#?}", outside(&h));
    h.app.dispatch(Command::CloseAbout, Instant::now());
    h.app.notice = Some(crate::app::PROJECT_REFUSED.into());
    h.idle(3);
    assert!(outside(&h).is_empty(), "notice: {:#?}", outside(&h));
    h.app.notice = None;
    h.app.dispatch(Command::ToggleFullScreen, Instant::now());
    h.idle(3);
    assert!(outside(&h).is_empty(), "full screen: {:#?}", outside(&h));
    // the Subtitles menu with its entries
    h.app.dispatch(Command::ToggleFullScreen, Instant::now());
    h.idle(3);
    h.click_label("Subtitles");
    assert!(outside(&h).is_empty(), "menu: {:#?}", outside(&h));
}

// ---- the start screen (1.6.2, REQ-150) ----

fn with_splash(d: &Path) -> Harness {
    let mut app = state(d);
    app.show_splash();
    Harness::new(app)
}

// covers: REQ-150
#[test]
fn the_start_screen_has_its_picture_rectangle_and_summary_and_everything_fits() {
    let d = tempfile::tempdir().unwrap();
    let h = with_splash(d.path());
    assert!(h.app.splash && h.has("Close"));
    let r = h.cache.splash_rects.expect("drawn");
    // 400 wide and as high as the 487 x 505 picture (1 px tolerance)
    assert!((r.panel.width() - 400.0).abs() <= 1.0, "{:?}", r.panel);
    assert!((r.panel.height() - r.image.height()).abs() <= 1.0);
    assert!((r.image.width() - 487.0).abs() <= 1.0 && (r.image.height() - 505.0).abs() <= 1.0);
    // the version rectangle is 308 x 74 at x 78, y 388
    assert!((r.text.width() - 308.0).abs() <= 1.0 && (r.text.height() - 74.0).abs() <= 1.0);
    assert!((r.text.min.x - r.image.min.x - 78.0).abs() <= 1.0);
    assert!((r.text.min.y - r.image.min.y - 388.0).abs() <= 1.0);
    // "Version 1.6" at its fitted size is inside the rectangle
    let text = videolore::splash::current_version_text();
    let size =
        videolore::ui::splash_window::fit_version(&h.ctx, &text, 1.0, &videolore::splash::PLAYER);
    assert!(size > 20.0, "{size}");
    let g = h.ctx.fonts_mut(|f| {
        f.layout_no_wrap(
            text,
            eframe::egui::FontId::proportional(size),
            eframe::egui::Color32::WHITE,
        )
    });
    let m = 2.0 * videolore::splash::TEXT_MARGIN + 0.5;
    assert!(g.size().x <= r.text.width() - m + 1.0 && g.size().y <= r.text.height() - m + 1.0);
    // the summary of this version: heading and lines, all fitting, no scroll
    assert!(
        r.summary_used.height() <= r.summary_room.height() + 0.5,
        "{r:?}"
    );
    let lines = videolore::splash::summary_lines(&videolore::splash::PLAYER);
    assert!(lines.len() >= 2 && lines.len() <= 6);
    assert!(h.has(lines[0]), "{:#?}", h.labels());
    assert!(lines[0] == "New in this version");
}

// covers: REQ-154
#[test]
fn the_player_summary_fits_its_panel_and_at_the_floor_size() {
    let d = tempfile::tempdir().unwrap();
    let h = with_splash(d.path());
    let r = h.cache.splash_rects.expect("drawn");
    assert!(
        r.summary_used.height() <= r.summary_room.height() + 0.5,
        "{r:?}"
    );
    assert!(
        r.summary_used.width() <= r.summary_room.width() + 0.5,
        "{r:?}"
    );
    let lines = videolore::splash::summary_lines(&videolore::splash::PLAYER);
    for l in &lines {
        assert!(h.labels().iter().any(|x| x.contains(&l[..15])), "{l}");
    }
    let s = r.image.width() / 487.0;
    assert!(videolore::ui::splash_window::summary_fits_at_floor(
        &h.ctx,
        &lines,
        r.summary_room,
        s
    ));
}

// covers: REQ-150
#[test]
fn each_way_of_closing_the_start_screen_removes_it_and_the_next_start_shows_it_again() {
    let d = tempfile::tempdir().unwrap();
    let mut h = with_splash(d.path());
    h.click_label("Close");
    assert!(!h.app.splash && h.cache.splash_rects.is_none());
    h.app.show_splash();
    h.idle(3);
    assert!(h.app.splash);
    h.key(Key::Escape);
    assert!(!h.app.splash);
    h.app.show_splash();
    h.idle(3);
    let x = h.find("Close window").expect("the window's close control");
    h.click(x.rect.center());
    assert!(!h.app.splash);
    assert!(with_splash(d.path()).app.splash, "the next start");
    // a start with a video shows none: opening one closes it
    let mut h = with_splash(d.path());
    open_video(&mut h, d.path(), "a.mp4");
    assert!(!h.app.splash && !h.has("Close"));
    // a small window scales both parts together and the text still fits
    let mut app = state(d.path());
    app.show_splash();
    let mut h = Harness::new(app);
    h.size = vec2(800.0, 500.0);
    h.idle(3);
    let r = h.cache.splash_rects.unwrap();
    assert!(r.image.height() < 505.0 && r.summary_used.height() <= r.summary_room.height() + 0.5);
}

// covers: REQ-150
#[test]
fn escape_closes_the_start_screen_before_the_full_screen_hint_and_the_controls_still_work() {
    let d = tempfile::tempdir().unwrap();
    let mut h = Harness::new(state(d.path()));
    open_video(&mut h, d.path(), "a.mp4");
    h.key(Key::F11);
    assert!(h.app.fullscreen.on && h.has(HINT_TEXT));
    h.app.show_splash();
    h.idle(3);
    assert!(h.app.splash);
    // Space and F11 work with the start screen open
    h.key(Key::Space);
    assert!(h.app.is_playing() && h.app.splash);
    h.key(Key::Space);
    // Esc closes the start screen first, then the hint, then leaves full screen
    h.key(Key::Escape);
    assert!(!h.app.splash && h.app.fullscreen.hint_visible() && h.app.fullscreen.on);
    h.key(Key::Escape);
    assert!(!h.app.fullscreen.hint_visible() && h.app.fullscreen.on);
    h.key(Key::Escape);
    assert!(!h.app.fullscreen.on);
    h.app.show_splash();
    h.idle(3);
    h.key(Key::F11);
    assert!(h.app.fullscreen.on && h.app.splash);
}

// covers: REQ-254, REQ-250
#[test]
fn the_file_menu_offers_the_playlist_and_the_editor_and_next_previous_appear_with_a_list() {
    let d = tempfile::tempdir().unwrap();
    let mut app = crate::tests::player(d.path());
    let now = Instant::now();
    for n in ["a.mp4", "b.mp4"] {
        let f = d.path().join(n);
        std::fs::write(&f, n).unwrap();
        app.dispatch(Command::AddToPlaylistPath(f), now);
        let end = Instant::now() + Duration::from_secs(10);
        while !(app.has_video() && app.idle_for_tests()) || app.opening() {
            app.tick(Instant::now());
            assert!(Instant::now() < end);
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let mut h = Harness::new(app);
    assert!(h.find("Next").is_some_and(|n| !n.disabled));
    assert!(
        h.find("Previous").is_some_and(|n| n.disabled),
        "at the first video"
    );
    h.click_label("File");
    for item in ["Add to playlist…", "Open folder…", "Open in editor"] {
        assert!(h.find(item).is_some(), "{item}");
    }
    h.click_label("Next");
}
