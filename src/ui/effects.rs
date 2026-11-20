// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Effects section (REQ-075): one row per effect, only its name. A click opens the
//! effect's popup, a double-click applies it to the whole video, dragging it onto a clip
//! applies it to that clip (REQ-082). The popup bodies (REQ-076) are here too: Fades,
//! Subtitles, Volume and the volume of one clip.

use super::{UiCache, button, record};
use crate::app::{AppState, Command, Edge, EffectKind, Instance, Target};
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

/// Tooltip of the Change aspect row: it applies to the whole video only.
pub const ASPECT_TOOLTIP: &str = "Sets the frame of the whole video";

/// Tooltip of every other row of the Effects section.
pub const EFFECT_TOOLTIP: &str =
    "Drag onto a clip to apply it to that clip; double-click to apply it to the whole video";

/// The rows of the Effects section: the effect and its name (nothing else is shown).
pub fn rows() -> [(EffectKind, &'static str); 8] {
    EffectKind::ALL.map(|e| (e, e.name()))
}

/// The icon of an effect's row (REQ-142).
pub fn effect_icon(effect: EffectKind) -> &'static str {
    use super::icons as i;
    match effect {
        EffectKind::Fade => i::CIRCLE_HALF,
        EffectKind::Subtitles => i::SUBTITLES,
        EffectKind::Volume => i::SPEAKER_HIGH,
        EffectKind::CleanAudio => i::WAVEFORM,
        EffectKind::ChangeAspect => i::FRAME_CORNERS,
        EffectKind::Scale => i::ARROWS_OUT,
        EffectKind::Move => i::ARROWS_OUT_CARDINAL,
        EffectKind::ApplyLut => i::PALETTE,
    }
}

/// The row below the effects that opens the popup of a new text (REQ-075, REQ-108).
pub const ADD_TEXT_ROW: &str = "Add Text";
/// Tooltip of [`ADD_TEXT_ROW`].
pub const ADD_TEXT_TOOLTIP: &str = "Opens a popup to add a text at the playhead";

/// Number of fades of every kind (boundaries, start, end).
pub fn fade_count(app: &AppState) -> usize {
    let tl = &app.project.timeline;
    tl.fades.len() + usize::from(tl.start_fade()) + usize::from(tl.end_fade())
}

/// True when the popup of `effect` is open (for the picture effects: the one it shows).
fn row_is_open(app: &AppState, effect: EffectKind) -> bool {
    match effect {
        EffectKind::ChangeAspect => app.popups.instance == Some(Instance::Aspect),
        EffectKind::Scale => {
            matches!(app.popups.instance, Some(Instance::Scale(_)))
                && app.popups.is_open(crate::app::Popup::Instance)
        }
        EffectKind::Move => {
            matches!(app.popups.instance, Some(Instance::Move(_)))
                && app.popups.is_open(crate::app::Popup::Instance)
        }
        EffectKind::ApplyLut => {
            matches!(app.popups.instance, Some(Instance::Lut(_)))
                && app.popups.is_open(crate::app::Popup::Instance)
        }
        e => app.popups.is_open(e.popup()),
    }
}

pub fn effects_panel(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let w = ui.available_width();
    for (effect, name) in rows() {
        let popup = effect.popup();
        let r = ui
            .add(
                Button::selectable(
                    row_is_open(app, effect),
                    super::icons::label(effect_icon(effect), name),
                )
                .min_size(egui::vec2(w, 0.0))
                .sense(egui::Sense::click_and_drag()),
            )
            .on_hover_text(if effect == EffectKind::ChangeAspect {
                ASPECT_TOOLTIP
            } else {
                EFFECT_TOOLTIP
            });
        // Change aspect cannot be dropped on a clip (REQ-075, REQ-130)
        if effect != EffectKind::ChangeAspect {
            r.dnd_set_drag_payload(effect);
        }
        if r.dragged() && egui::DragAndDrop::has_payload_of_type::<EffectKind>(ui.ctx()) {
            // what is being dragged follows the pointer
            if let Some(p) = ui.ctx().pointer_interact_pos() {
                egui::Area::new(egui::Id::unique("videolore-effect-drag"))
                    .order(egui::Order::Tooltip)
                    .fixed_pos(p + egui::vec2(12.0, 4.0))
                    .interactable(false)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| ui.label(name));
                    });
            }
        }
        // (a quick third click still counts: it must not just reopen the popup)
        if r.double_clicked() || r.triple_clicked() {
            cmds.push(Command::ApplyEffect {
                effect,
                target: Target::WholeVideo,
            });
        } else if r.clicked() {
            cmds.push(if effect.is_frame() {
                Command::OpenFrameEffect(effect)
            } else {
                Command::OpenPopup(popup)
            });
        }
    }
    // not an effect to drop on a clip: only a click
    let r = ui
        .add(
            Button::selectable(
                app.popups.is_open(crate::app::Popup::Text),
                super::icons::label(super::icons::TEXT_T, ADD_TEXT_ROW),
            )
            .min_size(egui::vec2(w, 0.0)),
        )
        .on_hover_text(ADD_TEXT_TOOLTIP);
    if r.clicked() {
        cmds.push(Command::OpenNewText);
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
        "A Fade IN/OUT darkens the end of a clip to black and brings the next one \
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
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Fade the whole video",
            Command::ApplyEffect {
                effect: EffectKind::Fade,
                target: Target::WholeVideo,
            },
            cmds,
        );
        button(
            ui,
            app,
            "Fade the selected clip",
            Command::ApplyEffect {
                effect: EffectKind::Fade,
                target: Target::Selected,
            },
            cmds,
        );
    });
    ui.separator();
    ui.strong(count(fade_count(app), "fade", "fades"));
    if !tl.has_any_fade() {
        ui.weak("No fades yet: click a \"+\" handle on the timeline, or the button above.");
    }
    for (on, i, text) in [
        (
            tl.start_fade(),
            Instance::FadeStart,
            "Fade from black at the start",
        ),
        (tl.end_fade(), Instance::FadeEnd, "Fade to black at the end"),
    ] {
        if on
            && ui
                .add(Button::selectable(app.popups.instance == Some(i), text).truncate())
                .clicked()
        {
            cmds.push(Command::OpenInstance(i));
        }
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
    let mut mix = cache.mix_drag.unwrap_or(app.volume_mix());
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
    } else if cache.mix_drag.is_some_and(|m: Mix| m == app.volume_mix()) {
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
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Create for the selected clip",
            Command::ApplyEffect {
                effect: EffectKind::Subtitles,
                target: Target::Selected,
            },
            cmds,
        );
        button(
            ui,
            app,
            "Export subtitles (.srt)…",
            Command::PickExportSrt,
            cmds,
        );
    });
    if let Some(p) = app.model_progress {
        ui.label("Downloading the speech recognition file…");
        ui.add(ProgressBar::new(p).show_percentage());
    }
    if let Some(p) = app.stt_progress {
        if app.stt_matching {
            ui.label("Matching subtitles to the speech");
        }
        ui.add(ProgressBar::new(p).show_percentage());
    }
    if !app.selection.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.label(count(
                app.selection.len(),
                "line selected",
                "lines selected",
            ));
            button(ui, app, "Reset position", Command::ResetPosition, cmds);
        });
    }
    let shown = subs.project(&app.project.timeline);
    ui.separator();
    ui.strong(count(shown.len(), "line", "lines"))
        .on_hover_text("Ctrl/Cmd+click a line to select several");
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
                let on = app.popups.instance == Some(i)
                    || app.selection.primary_line() == Some(l.line)
                    || app.selection.contains(&l.line);
                let text = format!("{}  {}", timecode(l.start), l.text);
                if ui
                    .add(Button::selectable(on, text.as_str()).truncate())
                    .on_hover_text(text)
                    .clicked()
                {
                    if ui.input(|x| x.modifiers.command) {
                        cmds.push(Command::ToggleLine(l.line));
                    } else {
                        cmds.push(Command::OpenInstance(i));
                    }
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

/// Level slider and mute of one video clip (REQ-084, REQ-078): one undo step when the
/// slider is released or the switch changes.
pub fn clip_volume_controls(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let Some(c) = app.project.timeline.get(clip) else {
        ui.label("This clip no longer exists.");
        return;
    };
    let (mut volume, mut muted) = match cache.clip_volume_drag {
        Some((id, v, m)) if id == clip => (v, m),
        _ => (c.volume, c.muted),
    };
    let mut commit = false;
    let mut dragging = false;
    ui.horizontal_wrapped(|ui| {
        ui.label("Clip sound");
        let r = ui.add(egui::Slider::new(&mut volume, 0..=100).suffix("%"));
        dragging |= r.dragged();
        commit |= r.drag_stopped() || (r.changed() && !r.dragged());
        commit |= ui.checkbox(&mut muted, "Mute").changed();
    });
    if commit {
        cache.clip_volume_drag = None;
        cmds.push(Command::SetClipVolume {
            clip,
            volume,
            muted,
        });
    } else if dragging {
        cache.clip_volume_drag = Some((clip, volume, muted));
    } else if cache
        .clip_volume_drag
        .is_some_and(|(id, v, m)| id == clip && (v, m) == (c.volume, c.muted))
    {
        cache.clip_volume_drag = None;
    }
}

/// The popup of one clip's volume.
pub fn clip_volume_body(
    ui: &mut Ui,
    app: &AppState,
    cache: &mut UiCache,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    if let Some(name) = app.clip_name(clip) {
        ui.add(egui::Label::new(format!("Clip: {name}")).truncate());
    }
    ui.weak("The video's volume (Volume, whole video) multiplies this level.");
    clip_volume_controls(ui, app, cache, clip, cmds);
    button(
        ui,
        app,
        "Reset to 100%",
        Command::SetClipVolume {
            clip,
            volume: 100,
            muted: false,
        },
        cmds,
    );
}

/// What Clean audio says in its popups.
pub const CLEAN_AUDIO_TEXT: &str = "Reduces background noise in the video's sound, such as \
wind, music or people chatting, while the speech stays. It is not perfect and has no \
strength setting. It works in the preview and in the export, before the clip's own volume; \
the music track is not changed.";

/// The popup of Clean audio on one clip (REQ-114): what it does, and Remove.
pub fn clean_audio_clip_body(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let Some(c) = app.project.timeline.get(clip) else {
        ui.label("This clip no longer exists.");
        return;
    };
    if let Some(name) = app.clip_name(clip) {
        ui.add(egui::Label::new(format!("Clip: {name}")).truncate());
    }
    ui.weak(CLEAN_AUDIO_TEXT);
    if c.clean_audio {
        button(
            ui,
            app,
            "Remove",
            Command::SetCleanAudio {
                target: Target::Clip(clip),
                on: false,
            },
            cmds,
        );
    } else {
        ui.label("Clean audio is off for this clip.");
        button(
            ui,
            app,
            "Apply",
            Command::SetCleanAudio {
                target: Target::Clip(clip),
                on: true,
            },
            cmds,
        );
    }
}

/// The Clean audio popup of the whole video: how many clips have it, apply and remove.
pub fn clean_audio_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let clips = &app.project.timeline.clips;
    let on = clips.iter().filter(|c| c.clean_audio).count();
    ui.weak(CLEAN_AUDIO_TEXT);
    ui.label(format!("{on} of {} clips have Clean audio.", clips.len()));
    ui.horizontal_wrapped(|ui| {
        button(
            ui,
            app,
            "Apply to whole video",
            Command::SetCleanAudio {
                target: Target::WholeVideo,
                on: true,
            },
            cmds,
        );
        button(
            ui,
            app,
            "Remove from whole video",
            Command::SetCleanAudio {
                target: Target::WholeVideo,
                on: false,
            },
            cmds,
        );
    });
}

/// The popup of the fade at the very start or end of the video.
pub fn edge_fade_body(ui: &mut Ui, app: &AppState, edge: Edge, cmds: &mut Vec<Command>) {
    let tl = &app.project.timeline;
    let on = match edge {
        Edge::Start => tl.start_fade(),
        Edge::End => tl.end_fade(),
    };
    if !on {
        ui.label("This fade no longer exists.");
        return;
    }
    ui.strong(match edge {
        Edge::Start => "Fade from black at the start of the video",
        Edge::End => "Fade to black at the end of the video",
    });
    ui.weak(
        "It stays at the edge of the video whatever is edited. Picture and sound fade together.",
    );
    button(
        ui,
        app,
        "Remove",
        Command::SetEdgeFade { edge, on: false },
        cmds,
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
