// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Effects section (REQ-075): one row per effect type with a count; a click opens the
//! effect's popup. The popup bodies (REQ-076) are here too: Fades, Subtitles, Volume.

use super::{UiCache, button, record};
use crate::app::{AppState, Command, Instance, Popup};
use crate::music::Mix;
use crate::subtitles::Preset;
use crate::time::{Micros, timecode};
use eframe::egui::{self, Button, ProgressBar, Ui};

/// Languages offered for subtitles (the speech model knows many more; these are the
/// common ones). `None` = detect.
pub const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("it", "Italian"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("pt", "Portuguese"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("ru", "Russian"),
    ("uk", "Ukrainian"),
    ("el", "Greek"),
    ("tr", "Turkish"),
    ("sv", "Swedish"),
];

/// Step of the subtitle timing buttons.
pub const NUDGE: Micros = Micros(100_000);

/// "1 fade", "3 fades".
pub fn count(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// "Video 80% · Music muted".
pub fn volume_text(m: &Mix) -> String {
    let level = |v: u8, muted: bool| {
        if muted {
            "muted".to_string()
        } else {
            format!("{v}%")
        }
    };
    format!(
        "Video {} · Music {}",
        level(m.video_volume, m.video_muted),
        level(m.music_volume, m.music_muted)
    )
}

/// The rows of the Effects section: popup, name, description, count.
pub fn rows(app: &AppState) -> [(Popup, &'static str, &'static str, String); 3] {
    let tl = &app.project.timeline;
    let lines = app.project.subtitles.project(tl).len();
    [
        (
            Popup::Fades,
            "Fade through black",
            "Black between two clips, picture and sound",
            count(tl.fades.len(), "fade", "fades"),
        ),
        (
            Popup::Subtitles,
            "Subtitles",
            "Words shown over the video",
            count(lines, "line", "lines"),
        ),
        (
            Popup::Volume,
            "Volume",
            "Video sound and music levels",
            volume_text(&app.project.mix),
        ),
    ]
}

pub fn effects_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let w = ui.available_width();
    for (popup, name, description, n) in rows(app) {
        let text = format!("{name}\n{description}\n{n}");
        let r = ui
            .add(
                Button::selectable(app.popups.is_open(popup), text)
                    .wrap()
                    .min_size(egui::vec2(w, 0.0)),
            )
            .on_hover_text(format!("Open the {name} settings"));
        if r.clicked() {
            cmds.push(Command::OpenPopup(popup));
        }
    }
}

/// Text of a fade in lists and the chooser.
pub fn fade_text(app: &AppState, b: usize) -> String {
    let tl = &app.project.timeline;
    let name = |i: usize| {
        app.project
            .media(tl.clips[i].media)
            .map(|m| m.name.clone())
            .unwrap_or_default()
    };
    format!(
        "{}  {} → {}",
        timecode(tl.start_of(b + 1)),
        name(b),
        name(b + 1)
    )
}

/// The Fades popup: what a fade is, add one, the list of fades, remove them all.
pub fn fades_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    ui.label(
        "A fade through black darkens the end of a clip to black and brings the next one \
         up from black; the sound fades out and in with it. Every fade has the same fixed \
         length.",
    );
    let nearest = tl.nearest_boundary(app.playhead);
    let (enabled, why) = match nearest {
        None => (false, "Fades need at least two clips"),
        Some((b, _)) if tl.has_fade(b) => (false, "That boundary already has a fade"),
        Some(_) => (true, ""),
    };
    let r = ui
        .add_enabled(
            enabled,
            Button::new("Add fade at the boundary nearest the playhead"),
        )
        .on_disabled_hover_text(why);
    if r.clicked()
        && let Some((b, _)) = nearest
    {
        cmds.push(Command::SetFade {
            left: tl.clips[b].id,
            right: tl.clips[b + 1].id,
            on: true,
        });
    }
    ui.separator();
    ui.strong(count(tl.fades.len(), "fade", "fades"));
    if tl.fades.is_empty() {
        ui.weak("No fades yet: click a \"+\" handle on the timeline, or the button above.");
    }
    for b in 0..tl.clips.len().saturating_sub(1) {
        if !tl.has_fade(b) {
            continue;
        }
        let i = Instance::Fade(tl.clips[b].id, tl.clips[b + 1].id);
        let on = app.popups.instance == Some(i);
        let text = fade_text(app, b);
        if ui
            .add(Button::selectable(on, text.as_str()).truncate())
            .on_hover_text(text)
            .clicked()
        {
            cmds.push(Command::OpenInstance(i));
        }
    }
    ui.separator();
    button(ui, app, "Remove all fades", Command::RemoveAllFades, cmds);
}

/// The Volume popup: two sliders and two mute switches. Dragging changes the value
/// locally; one undo step is recorded when the slider is released.
pub fn volume_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let mut mix = cache.mix_drag.unwrap_or(app.project.mix);
    let mut commit = false;
    let mut dragging = false;
    for (label, volume, muted) in [
        ("Video sound", &mut mix.video_volume, &mut mix.video_muted),
        ("Music", &mut mix.music_volume, &mut mix.music_muted),
    ] {
        ui.horizontal_wrapped(|ui| {
            ui.label(label);
            let r = ui.add(egui::Slider::new(volume, 0..=100).suffix("%"));
            dragging |= r.dragged();
            commit |= r.drag_stopped() || (r.changed() && !r.dragged());
            commit |= ui.checkbox(muted, "Mute").changed();
        });
    }
    if commit {
        cache.mix_drag = None;
        cmds.push(Command::SetMix(mix));
    } else if dragging {
        cache.mix_drag = Some(mix);
    } else if cache.mix_drag.is_some_and(|m: Mix| m == app.project.mix) {
        cache.mix_drag = None;
    }
}

/// Height of one row of the subtitle line list.
const LINE_ROW: f32 = 20.0;

/// The Subtitles popup: style, language, generation, and the list of lines.
pub fn subtitles_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let subs = &app.project.subtitles;
    ui.horizontal_wrapped(|ui| {
        ui.label("Style:");
        egui::ComboBox::from_id_salt("preset")
            .selected_text(subs.preset.name())
            .show_ui(ui, |ui| {
                for p in Preset::ALL {
                    if ui.selectable_label(subs.preset == p, p.name()).clicked() {
                        cmds.push(Command::SetPreset(p));
                    }
                }
            });
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Language:");
        let current = match (&subs.language, subs.language_overridden) {
            (Some(l), true) => l.clone(),
            (Some(l), false) => format!("detected: {l}"),
            (None, _) => "detect".into(),
        };
        egui::ComboBox::from_id_salt("language")
            .selected_text(current)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(!subs.language_overridden, "Detect")
                    .clicked()
                {
                    cmds.push(Command::SetLanguage(None));
                }
                for (code, name) in LANGUAGES {
                    let on = subs.language_overridden && subs.language.as_deref() == Some(*code);
                    if ui.selectable_label(on, *name).clicked() {
                        cmds.push(Command::SetLanguage(Some(code.to_string())));
                    }
                }
            });
    });
    if !crate::stt::available() {
        ui.label("Subtitle generation is not available in this build.");
    }
    ui.horizontal_wrapped(|ui| {
        if app.is_generating() {
            button(ui, app, "Cancel", Command::CancelSubtitles, cmds);
        } else {
            button(ui, app, "Generate", Command::GenerateSubtitles, cmds);
        }
        button(ui, app, "Add line at playhead", Command::AddSubtitle, cmds);
    });
    if let Some(p) = app.model_progress {
        ui.label("Downloading the speech recognition file…");
        ui.add(ProgressBar::new(p).show_percentage());
    }
    if let Some(p) = app.stt_progress {
        ui.add(ProgressBar::new(p).show_percentage());
    }
    let shown = subs.project(&app.project.timeline);
    ui.separator();
    ui.strong(count(shown.len(), "line", "lines"));
    if shown.is_empty() {
        ui.weak("No subtitles on the timeline.");
        return;
    }
    let out = egui::ScrollArea::vertical()
        .id_salt("subtitle-lines")
        .max_height(240.0)
        .auto_shrink([false, true])
        .show_rows(ui, LINE_ROW, shown.len(), |ui, range| {
            for l in &shown[range] {
                let i = Instance::Line(l.line, l.clip);
                let on = app.popups.instance == Some(i) || app.selected_line == Some(l.line);
                let text = format!("{}  {}", timecode(l.start), l.text);
                if ui
                    .add(Button::selectable(on, text.as_str()).truncate())
                    .on_hover_text(text)
                    .clicked()
                {
                    cmds.push(Command::OpenInstance(i));
                }
            }
        });
    record(
        cache,
        out.inner_rect,
        out.state.clamped_offset(),
        out.content_size,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_volume_texts() {
        assert_eq!(count(1, "fade", "fades"), "1 fade");
        assert_eq!(count(3, "fade", "fades"), "3 fades");
        assert_eq!(count(0, "line", "lines"), "0 lines");
        let m = Mix {
            video_volume: 80,
            video_muted: false,
            music_volume: 50,
            music_muted: true,
        };
        assert_eq!(volume_text(&m), "Video 80% · Music muted");
        assert!(LANGUAGES.iter().all(|(c, _)| c.len() == 2));
    }
}
