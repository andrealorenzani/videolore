// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The effects and settings panel: selected clip, selected music clip, volumes, subtitles,
//! export, preview copies (proxies), ffmpeg and the shortcut list.

use super::{UiCache, button, keymap};
use crate::app::{AppState, Command};
use crate::music::Mix;
use crate::subtitles::{Preset, ShownLine};
use crate::time::{Micros, timecode};
use eframe::egui::{self, Button, Color32, ProgressBar, Ui};

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

pub fn effects_panel(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.strong("Clip");
        let tl = &app.project.timeline;
        match app.selected_clip.and_then(|id| tl.index_of(id)) {
            None => {
                ui.label("Select a clip on the timeline.");
            }
            Some(i) => clip_settings(ui, app, i, cmds),
        }
        if let Some(id) = app.selected_music
            && app.project.music.get(id).is_some()
        {
            ui.separator();
            music_clip_settings(ui, app, cmds);
        }
        ui.separator();
        mix_settings(ui, app, cache, cmds);
        ui.separator();
        subtitle_settings(ui, app, cache, cmds);
        ui.separator();
        ui.strong("Export");
        match app.output_description() {
            Some(d) => ui.label(format!("Output: {d}")),
            None => ui.label("Add clips to see the output format."),
        };
        if app.is_exporting() {
            let s = &app.export_status;
            let mode = match s.stream_copy {
                Some(true) => "fast (no re-encoding)",
                Some(false) => "re-encoding",
                None => "preparing",
            };
            ui.label(format!("Exporting: {mode}"));
            ui.add(ProgressBar::new(s.progress).show_percentage());
            button(ui, app, "Cancel export", Command::CancelExport, cmds);
        } else {
            button(ui, app, "Export…", Command::PickExport, cmds);
        }
        ui.separator();
        ui.strong("Preview copies");
        let mut on = app.settings.proxies_enabled;
        if ui
            .checkbox(&mut on, "Use small preview copies of big videos")
            .changed()
        {
            cmds.push(Command::SetProxiesEnabled(on));
        }
        ui.label(format!(
            "Stored: {:.1} MB",
            app.proxy_cache_size as f64 / 1_000_000.0
        ));
        button(
            ui,
            app,
            "Clear preview copies",
            Command::ClearProxyCache,
            cmds,
        );
        ui.separator();
        ui.strong("ffmpeg");
        match &app.ffmpeg {
            Ok(i) => ui.label(format!(
                "{} ({}, {})",
                i.ffmpeg.display(),
                i.version,
                i.source
            )),
            Err(e) if app.ffmpeg_checking => ui.label(e),
            Err(e) => ui.colored_label(Color32::LIGHT_RED, e),
        };
        button(ui, app, "Choose ffmpeg…", Command::PickFfmpeg, cmds);
        ui.separator();
        ui.strong("Shortcuts");
        for (k, v) in keymap::HELP {
            ui.label(format!("{k}: {v}"));
        }
    });
}

fn clip_settings(ui: &mut Ui, app: &AppState, i: usize, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let c = tl.clips[i];
    let start = tl.start_of(i);
    let name = app
        .project
        .media(c.media)
        .map(|m| m.name.clone())
        .unwrap_or_default();
    ui.label(format!("Source: {name}"));
    ui.label(format!(
        "In {}  Out {}  Length {}",
        timecode(c.src_in),
        timecode(c.src_out),
        timecode(c.len())
    ));
    let frame = app.fps().frame_duration();
    let inside = app.playhead > start && app.playhead < start + c.len();
    let offset = app.playhead - start;
    let trim = |src_in: Micros, src_out: Micros| Command::SetTrim {
        clip: c.id,
        src_in,
        src_out,
    };
    ui.horizontal(|ui| {
        ui.label("Trim in:");
        button(ui, app, "-1f", trim(c.src_in - frame, c.src_out), cmds);
        button(ui, app, "+1f", trim(c.src_in + frame, c.src_out), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(c.src_in + offset, c.src_out));
        }
    });
    ui.horizontal(|ui| {
        ui.label("Trim out:");
        button(ui, app, "-1f", trim(c.src_in, c.src_out - frame), cmds);
        button(ui, app, "+1f", trim(c.src_in, c.src_out + frame), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(c.src_in, c.src_in + offset));
        }
    });
    ui.horizontal(|ui| {
        if i > 0 {
            let mut on = tl.has_fade(i - 1);
            if ui.checkbox(&mut on, "Fade in").changed() {
                cmds.push(Command::ToggleFade(i - 1));
            }
        }
        if i + 1 < tl.clips.len() {
            let mut on = tl.has_fade(i);
            if ui.checkbox(&mut on, "Fade out").changed() {
                cmds.push(Command::ToggleFade(i));
            }
        }
    });
    button(ui, app, "Merge with next", Command::Merge, cmds);
}

fn music_clip_settings(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let Some(m) = app.selected_music.and_then(|id| app.project.music.get(id)) else {
        return;
    };
    ui.strong("Music clip");
    let name = app
        .project
        .media(m.media)
        .map(|x| x.name.clone())
        .unwrap_or_default();
    ui.label(format!(
        "{name}: at {}  length {}",
        timecode(m.start),
        timecode(m.len())
    ));
    let frame = app.fps().frame_duration();
    let inside = app.playhead > m.start && app.playhead < m.end();
    let offset = app.playhead - m.start;
    let trim = |src_in: Micros, src_out: Micros| Command::TrimMusic {
        clip: m.id,
        src_in,
        src_out,
    };
    ui.horizontal(|ui| {
        ui.label("Start:");
        button(ui, app, "-1f", trim(m.src_in - frame, m.src_out), cmds);
        button(ui, app, "+1f", trim(m.src_in + frame, m.src_out), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(m.src_in + offset, m.src_out));
        }
    });
    ui.horizontal(|ui| {
        ui.label("End:");
        button(ui, app, "-1f", trim(m.src_in, m.src_out - frame), cmds);
        button(ui, app, "+1f", trim(m.src_in, m.src_out + frame), cmds);
        if ui.add_enabled(inside, Button::new("at playhead")).clicked() {
            cmds.push(trim(m.src_in, m.src_in + offset));
        }
    });
    ui.horizontal(|ui| {
        button(
            ui,
            app,
            "Move to playhead",
            Command::MoveMusic {
                clip: m.id,
                start: app.playhead,
            },
            cmds,
        );
        button(ui, app, "Split", Command::SplitMusic, cmds);
        button(ui, app, "Delete", Command::DeleteMusic, cmds);
    });
}

/// Volume slider + mute. Dragging changes the value locally; one undo step is recorded
/// when the slider is released.
fn mix_settings(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    ui.strong("Volume");
    let mut mix = cache.mix_drag.unwrap_or(app.project.mix);
    let mut commit = false;
    let mut dragging = false;
    for (label, volume, muted) in [
        ("Video sound", &mut mix.video_volume, &mut mix.video_muted),
        ("Music", &mut mix.music_volume, &mut mix.music_muted),
    ] {
        ui.horizontal(|ui| {
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

fn subtitle_settings(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let subs = &app.project.subtitles;
    ui.strong("Subtitles");
    ui.horizontal(|ui| {
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
    ui.horizontal(|ui| {
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
    ui.horizontal(|ui| {
        if app.is_generating() {
            button(ui, app, "Cancel", Command::CancelSubtitles, cmds);
        } else {
            button(ui, app, "Generate", Command::GenerateSubtitles, cmds);
        }
        button(ui, app, "Add line at playhead", Command::AddSubtitle, cmds);
    });
    if let Some(p) = app.model_progress {
        ui.label("Downloading the speech model…");
        ui.add(ProgressBar::new(p).show_percentage());
    }
    if let Some(p) = app.stt_progress {
        ui.add(ProgressBar::new(p).show_percentage());
    }
    let shown = subs.project(&app.project.timeline);
    if shown.is_empty() {
        ui.label("No subtitles on the timeline.");
    }
    for l in &shown {
        line_row(ui, app, cache, l, cmds);
    }
}

fn line_row(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    l: &ShownLine,
    cmds: &mut Vec<Command>,
) {
    let selected = app.selected_line == Some(l.line);
    let head = format!("{} – {}", timecode(l.start), timecode(l.end));
    if !selected {
        let r = ui.selectable_label(false, format!("{head}  {}", l.text));
        if r.clicked() {
            cmds.push(Command::SelectLine(Some(l.line)));
            cmds.push(Command::Seek(l.start));
        }
        return;
    }
    ui.label(head);
    let text = match &mut cache.line_edit {
        Some((id, t)) if *id == l.line => t,
        _ => {
            let full = app
                .project
                .subtitles
                .get(l.line)
                .map(|x| x.text.clone())
                .unwrap_or_default();
            cache.line_edit = Some((l.line, full));
            &mut cache.line_edit.as_mut().expect("just set").1
        }
    };
    let r = ui.add(egui::TextEdit::singleline(text).desired_width(f32::INFINITY));
    if r.lost_focus() {
        cmds.push(Command::SetSubtitleText(l.line, text.clone()));
    }
    if !r.has_focus() && !r.lost_focus() {
        // not being edited: show the stored text (it may change with undo)
        cache.line_edit = None;
    }
    let retime = |start: Micros, end: Micros| Command::RetimeSubtitle {
        line: l.line,
        clip: l.clip,
        start,
        end,
    };
    ui.horizontal(|ui| {
        ui.label("Start");
        button(ui, app, "-", retime(l.start - NUDGE, l.end), cmds);
        button(ui, app, "+", retime(l.start + NUDGE, l.end), cmds);
        ui.label("End");
        button(ui, app, "-", retime(l.start, l.end - NUDGE), cmds);
        button(ui, app, "+", retime(l.start, l.end + NUDGE), cmds);
        button(ui, app, "Delete", Command::DeleteSubtitle(l.line), cmds);
    });
}
