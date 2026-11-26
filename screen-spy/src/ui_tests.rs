// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Interface tests of the recorder through egui's accessibility tree: every widget with
//! its label and bounds, clicks as real pointer events.

use crate::app::Phase;
use crate::app_tests::{Options, Rig, rig, rig_with};
use crate::plan::{AudioFormat, VideoFormat};
use crate::ui::{DEFAULT_SIZE, MIN_SIZE, RecorderCache, WAV_NOTE, clock, frame_ui};
use eframe::egui::{
    Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2, ViewportCommand,
    ViewportEvent, ViewportId, pos2, vec2,
};
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(Debug, Clone)]
struct Node {
    label: String,
    value: String,
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
    rig: Rig,
    cache: RecorderCache,
    size: Vec2,
    nodes: Vec<Node>,
    commands: Vec<ViewportCommand>,
}

impl Harness {
    fn new(rig: Rig, size: [f32; 2]) -> Harness {
        let ctx = Context::default();
        ctx.enable_accesskit();
        let mut h = Harness {
            ctx,
            rig,
            cache: RecorderCache::default(),
            size: size.into(),
            nodes: Vec::new(),
            commands: Vec::new(),
        };
        h.idle(3);
        h
    }

    fn frame_with(&mut self, events: Vec<Event>, close: bool) {
        let mut raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        if close {
            raw.viewports
                .entry(ViewportId::ROOT)
                .or_default()
                .events
                .push(ViewportEvent::Close);
        }
        let (rig, cache) = (&mut self.rig, &mut self.cache);
        let mut out = self
            .ctx
            .run_ui(raw, |ui| frame_ui(ui, &mut rig.app, cache, rig.now));
        if let Some(v) = out.viewport_output.get(&ViewportId::ROOT) {
            self.commands.extend(v.commands.iter().cloned());
        }
        self.nodes = out
            .platform_output
            .accesskit_update
            .take()
            .map(|u| {
                u.nodes
                    .into_iter()
                    .filter_map(|(_, n)| {
                        let b = n.bounds()?;
                        Some(Node {
                            label: without_icon(n.label().or(n.value()).unwrap_or_default()),
                            value: n.value().unwrap_or_default().to_string(),
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
    }

    fn idle(&mut self, n: usize) {
        for _ in 0..n {
            self.frame_with(Vec::new(), false);
        }
    }

    fn find(&self, label: &str) -> Option<Node> {
        let exact = self.nodes.iter().find(|n| n.label == label);
        exact
            .or_else(|| {
                self.nodes
                    .iter()
                    .find(|n| n.label.starts_with(label) || n.value.starts_with(label))
            })
            .cloned()
    }

    fn must(&self, label: &str) -> Node {
        self.find(label).unwrap_or_else(|| {
            let all: Vec<_> = self.nodes.iter().map(|n| (&n.label, &n.value)).collect();
            panic!("no widget \"{label}\" in {all:#?}")
        })
    }

    fn click_label(&mut self, label: &str) {
        let p = self.must(label).rect.center();
        let ev = |pressed| Event::PointerButton {
            pos: p,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        self.frame_with(vec![Event::PointerMoved(p), ev(true)], false);
        self.frame_with(vec![ev(false)], false);
        self.idle(3);
        // the checks before a recording run on their own thread
        let end = std::time::Instant::now() + Duration::from_secs(10);
        while self.rig.app.phase() == Phase::Starting
            && !self.rig.world.sound.hold.load(Ordering::SeqCst)
        {
            assert!(std::time::Instant::now() < end);
            self.idle(1);
        }
    }

    /// Every one of these is inside the window.
    fn inside(&self, labels: &[&str]) {
        let window = Rect::from_min_size(Pos2::ZERO, self.size);
        for l in labels {
            let n = self.must(l);
            assert!(window.contains_rect(n.rect), "{l} {:?}", n.rect);
        }
    }
}

const PARTS: [&str; 8] = [
    "Preview",
    "Screen",
    "Output",
    "File name",
    "Choose…",
    "Video format",
    "Audio format",
    "Start recording",
];

// covers: REQ-342, REQ-344, REQ-345
#[test]
fn all_four_parts_are_visible_at_the_default_and_the_smallest_size() {
    for size in [DEFAULT_SIZE, MIN_SIZE] {
        let h = Harness::new(rig(), size);
        h.inside(&PARTS);
        h.inside(&[WAV_NOTE, "Screen 1, 1920x1080", "screen-recording-"]);
    }
    // the note of the blocked session leaves room for everything
    let wayland = rig_with(Options {
        env: vec![("WAYLAND_DISPLAY", "wayland-0")],
        ..Options::default()
    });
    let h = Harness::new(wayland, MIN_SIZE);
    h.inside(&PARTS);
    h.inside(&["Screen recording needs an X11 session", "No preview"]);
}

// covers: REQ-342
#[test]
fn the_file_menu_holds_about_and_quit() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.click_label("File");
    h.click_label("About videolore screen spy");
    assert!(h.rig.app.show_about);
    h.must("videolore screen spy ");
    // the About window closes before the menu is used again
    h.rig.run(crate::app::Command::CloseAbout);
    h.idle(3);
    h.click_label("File");
    h.click_label("Quit");
    assert!(h.rig.app.quit);
    assert!(h.commands.contains(&ViewportCommand::Close));
}

// covers: REQ-344, REQ-347, REQ-348
#[test]
fn the_lists_offer_the_screens_the_four_video_and_the_three_sound_formats() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.click_label("Screen 1, 1920x1080");
    h.click_label("Screen 2, 1280x1024");
    assert_eq!(h.rig.app.chosen, Some(20));
    h.click_label(VideoFormat::Hd1080.label());
    for f in VideoFormat::ALL {
        h.must(f.label());
    }
    h.click_label(VideoFormat::Uhd.label());
    assert_eq!(h.rig.app.video, VideoFormat::Uhd);
    h.click_label("AAC");
    h.must("MP3");
    h.click_label("WAV");
    assert_eq!(h.rig.app.audio, AudioFormat::Wav);
    h.click_label("Choose…");
    assert_eq!(h.rig.world.asked.borrow().len(), 1);
}

// covers: REQ-345
#[test]
fn typing_in_the_box_sets_the_file_name() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.click_label("screen-recording-");
    h.frame_with(
        vec![
            Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::COMMAND,
            },
            Event::Text("demo".into()),
        ],
        false,
    );
    h.idle(2);
    assert_eq!(h.rig.app.name, "demo");
}

// covers: REQ-349, REQ-346, REQ-344
#[test]
fn the_button_starts_and_stops_and_locks_the_choices() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    // an existing file: the question, first cancelled, then replaced
    h.rig.run(crate::app::Command::SetName("old".into()));
    std::fs::write(h.rig.dir.path().join("old.mp4"), "x").unwrap();
    h.click_label("Start recording");
    h.must("Replace the file?");
    h.click_label("Cancel");
    assert_eq!(h.rig.app.phase(), Phase::Previewing);
    h.click_label("Start recording");
    h.click_label("Replace");
    assert_eq!(h.rig.app.phase(), Phase::Recording);
    for locked in [
        "Screen 1, 1920x1080",
        "Choose…",
        "AAC",
        VideoFormat::Hd1080.label(),
    ] {
        assert!(h.must(locked).disabled, "{locked}");
    }
    h.rig.now += Duration::from_secs(65);
    h.idle(2);
    h.must("Recording 01:05");
    h.click_label("Stop recording");
    assert_eq!(h.rig.app.phase(), Phase::Stopping);
    assert!(h.must("Stop recording").disabled);
    h.rig.world.end(true, "");
    h.idle(3);
    assert!(!h.must("Choose…").disabled);
    h.must("Start recording");
    assert_eq!(clock(Duration::from_secs(3605)), "1:00:05");
}

// covers: REQ-351
#[test]
fn the_windows_close_control_asks_while_recording() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.click_label("Start recording");
    h.frame_with(Vec::new(), true);
    assert!(h.commands.contains(&ViewportCommand::CancelClose));
    h.idle(2);
    h.must("Stop the recording and close?");
    h.click_label("Cancel");
    assert_eq!(h.rig.app.phase(), Phase::Recording);
    // Ctrl+Q asks too
    h.frame_with(
        vec![Event::Key {
            key: Key::Q,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::COMMAND,
        }],
        false,
    );
    h.idle(2);
    h.commands.clear();
    h.click_label("Stop and close");
    assert!(!h.commands.contains(&ViewportCommand::Close), "not yet");
    h.rig.world.end(true, "");
    h.idle(3);
    assert!(h.commands.contains(&ViewportCommand::Close));
    // the close the program asked for goes through
    h.commands.clear();
    h.frame_with(Vec::new(), true);
    assert!(!h.commands.contains(&ViewportCommand::CancelClose));
    // without a recording the control closes at once
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.frame_with(Vec::new(), true);
    assert!(h.rig.app.quit);
    assert!(!h.commands.contains(&ViewportCommand::CancelClose));
}

// covers: REQ-357, REQ-343
#[test]
fn messages_are_shown_in_a_popup_and_the_preview_picture_is_drawn() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    // the picture arrives from its thread
    let end = std::time::Instant::now() + Duration::from_secs(5);
    while h.rig.app.frame.is_none() {
        assert!(std::time::Instant::now() < end);
        h.idle(1);
    }
    h.idle(2);
    h.rig
        .world
        .sound
        .fail(Some("The microphone is not available"));
    h.click_label("Start recording");
    h.must("The microphone is not available");
    h.click_label("OK");
    assert_eq!(h.rig.app.message, None);
    assert_eq!(h.size, vec2(DEFAULT_SIZE[0], DEFAULT_SIZE[1]));
}

// covers: REQ-349, REQ-342
#[test]
fn while_the_start_is_checked_the_button_cancels_and_the_window_closes_freely() {
    let mut h = Harness::new(rig(), DEFAULT_SIZE);
    h.rig.world.sound.hold.store(true, Ordering::SeqCst);
    h.click_label("Start recording");
    assert_eq!(h.rig.app.phase(), Phase::Starting);
    assert!(h.must("Choose…").disabled);
    h.must("Checking ffmpeg and the sound");
    h.click_label("Cancel start");
    h.rig.settle();
    h.idle(3);
    h.must("Start recording");
    h.click_label("Start recording");
    h.commands.clear();
    h.frame_with(Vec::new(), true);
    assert!(h.rig.app.quit);
    assert!(!h.commands.contains(&ViewportCommand::CancelClose));
}
