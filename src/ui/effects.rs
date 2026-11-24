// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The Effects section (REQ-075): one row per effect, only its name. A click opens the
//! effect's popup, a double-click applies it to the whole video, dragging it onto a clip
//! applies it to that clip (REQ-082). The popup bodies (REQ-076) are here too: Fades,
//! Subtitles, Volume and the volume of one clip.

use super::strings::tr;
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
pub const ASPECT_TOOLTIP: &str = "Drag onto a clip to give that clip its own aspect; double-click to set the frame of the whole video";

/// Tooltip of every other row of the Effects section.
pub const EFFECT_TOOLTIP: &str =
    "Drag onto a clip to apply it to that clip; double-click to apply it to the whole video";

/// The rows of the Effects section: the effect and its name (nothing else is shown).
pub fn rows() -> [(EffectKind, &'static str); 9] {
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
        EffectKind::ChangeSpeed => i::SLIDERS,
    }
}

/// The row below the effects that opens the popup of a new text (REQ-075, REQ-108).
pub const ADD_TEXT_ROW: &str = "Add Text";
/// The row that adds an overlap over the clip under the playhead; it replaces Add Picture
/// (REQ-325).
pub const OVERLAP_MEDIA_ROW: &str = "Overlap media";
pub const OVERLAP_MEDIA_TOOLTIP: &str =
    "Shows a PNG, GIF or video with transparency over the clip under the playhead";
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
    new_object_row(ui, app, cmds);
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
        // every row can be dropped on a clip; Change aspect gives that clip its own aspect
        r.dnd_set_drag_payload(effect);
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
}

/// The two rows of the things that are not effects to drop on a clip, only a click: Add Text
/// and Overlap media (REQ-325). Each is a row of its own, drawn like the effect rows (REQ-336).
/// They are the first rows of the Effects section, so the nine effects below them may scroll.
fn new_object_row(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let w = ui.available_width();
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
    let r = ui
        .add(
            Button::selectable(
                false,
                super::icons::label(super::icons::IMAGE, OVERLAP_MEDIA_ROW),
            )
            .min_size(egui::vec2(w, 0.0)),
        )
        .on_hover_text(OVERLAP_MEDIA_TOOLTIP);
    if r.clicked() {
        cmds.push(Command::AddOverlap(None));
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
         up from black; the sound fades out and in with it. Every fade has the same \
         length, which you can change here.",
    );
    // the length of every fade (REQ-274)
    ui.horizontal(|ui| {
        ui.label(tr!(FadeLength));
        let mut secs = tl.fade_len().as_secs_f64();
        let r = ui.add(
            egui::DragValue::new(&mut secs)
                .range(0.1..=10.0)
                .speed(0.05)
                .max_decimals(2)
                .suffix(" s"),
        );
        if r.drag_stopped() || (r.changed() && !r.dragged()) || r.lost_focus() {
            let len = Micros::from_secs_f64(secs);
            if len != tl.fade_len() {
                cmds.push(Command::SetFadeLength(len));
            }
        }
    });
    let nearest = tl.nearest_boundary(app.playhead);
    let (enabled, why) = match nearest {
        None => (false, "Fades need at least two clips"),
        Some((b, _)) if tl.has_fade(b) => (false, "That boundary already has a fade"),
        Some(_) => (true, ""),
    };
    let r = ui
        .add_enabled(enabled, Button::new(tr!(AddFadeAtTheBoundary)))
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

/// The note of REQ-187 and REQ-188: the scripts of the subtitle lines that no installed font
/// shows (Chinese, Japanese and Korean use the computer's fonts).
pub fn subtitle_script_note(app: &AppState) -> Option<String> {
    crate::fonts::missing_note_in(app.project.subtitles.lines.iter().map(|l| l.text.as_str()))
}

/// Height of one row of the subtitle line list.
const LINE_ROW: f32 = 20.0;

/// The Subtitles popup: style, language, generation, and the list of lines.
/// The warning of a line whose box overlaps another line shown at the same time (REQ-169).
pub const OVERLAPS: &str = "Overlaps another line shown at the same time";

/// A colour button that reports a change once the pointer is up (one undo step per pick).
fn colour_row(ui: &mut Ui, label: &str, c: &mut [u8; 3]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let before = *c;
        let r = egui::color_picker::color_edit_button_srgb(ui, c);
        changed = *c != before && !ui.input(|i| i.pointer.any_down())
            || r.clicked_elsewhere() && *c != before;
    });
    changed
}

/// Shadow, word colours and Enlarge word size of the style (REQ-306 to REQ-309).
fn style_settings(ui: &mut Ui, subs: &crate::subtitles::Subtitles, cmds: &mut Vec<Command>) {
    use crate::subtitles::{ENLARGE_MAX, ENLARGE_MIN};
    let mut st = subs.style;
    let mut done = false;
    ui.horizontal(|ui| {
        ui.label(tr!(ShadowLabel));
        let r = ui.add(
            egui::DragValue::new(&mut st.shadow.px)
                .range(0..=crate::base::shadow::SHADOW_MAX)
                .suffix(" px"),
        );
        done |= r.drag_stopped() || (r.changed() && !r.dragged());
        let before = st.shadow.colour;
        let c = egui::color_picker::color_edit_button_srgb(ui, &mut st.shadow.colour);
        done |= st.shadow.colour != before && !ui.input(|i| i.pointer.any_down())
            || c.clicked_elsewhere() && st.shadow.colour != before;
    });
    if matches!(subs.preset, Preset::WordHighlight | Preset::Typewriter) {
        done |= colour_row(ui, "Not yet highlighted", &mut st.not_yet);
        done |= colour_row(ui, "Highlighted", &mut st.highlighted);
    }
    if subs.preset == Preset::EnlargeWord {
        ui.horizontal(|ui| {
            ui.label(tr!(EnlargeBy));
            let r = ui.add(
                egui::DragValue::new(&mut st.enlarge)
                    .range(ENLARGE_MIN..=ENLARGE_MAX)
                    .suffix(" px"),
            );
            done |= r.drag_stopped() || (r.changed() && !r.dragged());
        });
    }
    if done && st != subs.style {
        cmds.push(Command::SetSubtitleStyle(st));
    }
}

pub fn subtitles_body(ui: &mut Ui, app: &AppState, cache: &mut UiCache, cmds: &mut Vec<Command>) {
    let subs = &app.project.subtitles;
    // the fonts of the scripts the lines use (REQ-188)
    super::scripts::install_for(
        ui.ctx(),
        &super::scripts::scripts_of_lines(subs.lines.iter().map(|l| l.text.as_str())),
    );
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(Style));
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
    style_settings(ui, subs, cmds);
    subtitle_audio_choice(ui, app, cmds);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(Language));
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
        ui.label(tr!(SubtitleGenerationIsNotAvailable));
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
        button(ui, app, "Export subtitles…", Command::PickExportSrt, cmds);
    });
    ui.horizontal_wrapped(|ui| {
        let clip = app.target_clip();
        button(
            ui,
            app,
            "Import subtitles…",
            Command::PickImportSubtitles(None),
            cmds,
        );
        if let Some(c) = clip
            && let Some(file) = app.import_offer(c)
        {
            let name = crate::media::display_name(file);
            button(
                ui,
                app,
                &format!("Import {name}"),
                Command::OpenImportSubtitles {
                    path: file.to_path_buf(),
                    clip: Some(c),
                },
                cmds,
            );
        }
    });
    if let Some(note) = subtitle_script_note(app) {
        ui.label(super::icons::label(super::icons::WARNING, &note));
    }
    if let Some(p) = app.model_progress {
        ui.label(tr!(DownloadingTheSpeechRecognitionFile));
        ui.add(ProgressBar::new(p).show_percentage());
    }
    if let Some(p) = app.stt_progress {
        if app.stt_matching {
            ui.label(tr!(MatchingSubtitlesToTheSpeech));
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
        .on_hover_text(tr!(CtrlCmdClickALine));
    // the gestures that select several (REQ-171)
    ui.weak(super::status::SELECTION_HINT);
    let overlaps = cache.overlapping_lines(app).clone();
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
                let warn = overlaps.contains(&l.line);
                let text = if warn {
                    format!(
                        "{} {}  {}",
                        super::icons::WARNING,
                        timecode(l.start),
                        l.text
                    )
                } else {
                    format!("{}  {}", timecode(l.start), l.text)
                };
                let tip = if warn {
                    format!("{text}\n{OVERLAPS}")
                } else {
                    text.clone()
                };
                if ui
                    .add(Button::selectable(on, text.as_str()).truncate())
                    .on_hover_text(tip)
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
        ui.label(tr!(ThisClipNoLongerExists));
        return;
    };
    let (mut volume, mut muted) = match cache.clip_volume_drag {
        Some((id, v, m)) if id == clip => (v, m),
        _ => (c.volume, c.muted),
    };
    let mut commit = false;
    let mut dragging = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(ClipSound));
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

/// The Audio track choice of a clip (REQ-340): its media's tracks, the clip's own chosen.
fn audio_track_choice(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let Some(c) = app.project.timeline.get(clip) else {
        return;
    };
    let tracks = app.audio_track_labels(c.media);
    if tracks.is_empty() {
        return;
    }
    let current = tracks
        .iter()
        .find(|(o, _)| *o == c.audio_track)
        .map_or_else(|| tracks[0].1.clone(), |(_, l)| l.clone());
    ui.horizontal_wrapped(|ui| {
        ui.label(AUDIO_TRACK);
        egui::ComboBox::from_id_salt("clip-audio-track")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (ordinal, label) in &tracks {
                    if ui
                        .selectable_label(c.audio_track == *ordinal, label)
                        .clicked()
                    {
                        cmds.push(Command::SetClipAudioTrack {
                            clip,
                            track: *ordinal,
                        });
                    }
                }
            });
    });
}

/// The Audio track choice of the Subtitles popup (REQ-339): the tracks of the clip the
/// effect applies to (the first clip for the whole video); the speech is taken from it.
fn subtitle_audio_choice(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let clip = app
        .target_clip()
        .or_else(|| app.project.timeline.clips.first().map(|c| c.id));
    let Some(media) = clip
        .and_then(|id| app.project.timeline.get(id))
        .map(|c| c.media)
    else {
        return;
    };
    let tracks = app.audio_track_labels(media);
    if tracks.is_empty() {
        return;
    }
    let chosen = app.popups.subtitle_audio;
    let current = tracks
        .iter()
        .find(|(o, _)| *o == chosen)
        .map_or_else(|| tracks[0].1.clone(), |(_, l)| l.clone());
    ui.horizontal_wrapped(|ui| {
        ui.label(AUDIO_TRACK);
        egui::ComboBox::from_id_salt("subtitle-audio-track")
            .selected_text(current)
            .show_ui(ui, |ui| {
                for (ordinal, label) in &tracks {
                    if ui.selectable_label(chosen == *ordinal, label).clicked() {
                        cmds.push(Command::SetSubtitleAudio(*ordinal));
                    }
                }
            });
    });
}

pub const AUDIO_TRACK: &str = "Audio track";

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
    audio_track_choice(ui, app, clip, cmds);
    ui.weak("The video's volume (Volume, whole video) multiplies this level.");
    let tl = &app.project.timeline;
    let keys = tl.volume_keys(clip);
    if keys.is_empty() {
        clip_volume_controls(ui, app, cache, clip, cmds);
    } else if let Some(c) = tl.get(clip) {
        // while the effect is set the keyframes set the volume: the slider is shown, off
        ui.horizontal_wrapped(|ui| {
            ui.label(tr!(ClipSound));
            let mut v = c.volume;
            ui.add_enabled(false, egui::Slider::new(&mut v, 0..=100).suffix("%"));
        });
        ui.weak("The keyframes set the volume; the slider is off while Volume is set.");
    }
    volume_keys_controls(ui, app, clip, cmds);
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

pub const ADD_KEYFRAME: &str = "Add keyframe";
const ADD_KEYFRAME_TIP: &str = "Adds a keyframe at the playhead, at the volume there";
const KEYFRAME_THERE: &str = "There is a keyframe at the playhead already";
pub const EASE: &str = "Ease";
const EASE_TIP: &str = "Moves slowly at both ends to the next keyframe";
const REMOVE_KEY: &str = "✕";

/// The keyframes of the Volume effect (REQ-280, REQ-282): add one at the playhead with the
/// level there, take one away, straight or eased move to the next one, remove the effect.
pub fn volume_keys_controls(
    ui: &mut Ui,
    app: &AppState,
    clip: crate::timeline::ClipId,
    cmds: &mut Vec<Command>,
) {
    let tl = &app.project.timeline;
    let Some(i) = tl.index_of(clip) else { return };
    let c = tl.clips[i];
    let keys = tl.volume_keys(clip);
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(VolumeKeys));
        let at = (app.playhead - tl.start_of(i)).clamp(Micros::ZERO, c.len());
        let level = crate::keyframes::at(keys, at).unwrap_or(if c.muted { 0 } else { c.volume });
        let can_add = keys.iter().all(|k| k.t != at);
        if ui
            .add_enabled(can_add, Button::new(ADD_KEYFRAME))
            .on_hover_text(ADD_KEYFRAME_TIP)
            .on_disabled_hover_text(KEYFRAME_THERE)
            .clicked()
        {
            cmds.push(Command::SetVolumeKey {
                clip,
                at,
                percent: level,
            });
        }
    });
    for k in keys {
        ui.horizontal_wrapped(|ui| {
            let mut v = f32::from(k.v);
            let r = ui.add(
                egui::DragValue::new(&mut v)
                    .range(0.0..=100.0)
                    .speed(0.5)
                    .max_decimals(0)
                    .prefix(format!("{}  ", timecode(k.t)))
                    .suffix(" %"),
            );
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && v.round() != f32::from(k.v) {
                cmds.push(Command::SetVolumeKey {
                    clip,
                    at: k.t,
                    percent: v.round() as u8,
                });
            }
            let eased = k.ease == crate::keyframes::Ease::EaseInOut;
            if ui
                .selectable_label(eased, EASE)
                .on_hover_text(EASE_TIP)
                .clicked()
            {
                cmds.push(Command::SetVolumeKeyEase {
                    clip,
                    at: k.t,
                    ease: if eased {
                        crate::keyframes::Ease::Straight
                    } else {
                        crate::keyframes::Ease::EaseInOut
                    },
                });
            }
            if ui
                .small_button(REMOVE_KEY)
                .on_hover_text(tr!(RemoveThisKeyframe))
                .clicked()
            {
                cmds.push(Command::RemoveVolumeKey { clip, at: k.t });
            }
        });
    }
    if !keys.is_empty() {
        button(
            ui,
            app,
            "Remove effect",
            Command::RemoveVolume { clip },
            cmds,
        );
    }
}

/// The popup of Change speed (REQ-281): 50 to 200 %, for one clip or, from the whole-video
/// entry, for every clip at once (one copy on each clip, one undo step). Not keyframed.
pub fn speed_body(
    ui: &mut Ui,
    app: &AppState,
    target: crate::timeline::FrameTarget,
    cmds: &mut Vec<Command>,
) {
    use crate::timeline::FrameTarget;
    let tl = &app.project.timeline;
    let current = match target {
        FrameTarget::Clip(id) => tl.get(id).map(|c| c.speed),
        FrameTarget::Whole => tl.clips.first().map(|c| c.speed),
    };
    let Some(current) = current else {
        ui.label(tr!(ThisClipNoLongerExists));
        return;
    };
    ui.weak(
        "The clip plays faster or slower; its sound keeps its pitch and its length on the \
         timeline changes. The overlaps of the clip change with it.",
    );
    let apply = |speed: u16| match target {
        FrameTarget::Clip(clip) => Command::SetSpeed { clip, speed },
        FrameTarget::Whole => Command::SetSpeedAll { speed },
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!(SpeedPercent));
        let mut pct = u32::from(current) / 10;
        let r = ui.add(egui::DragValue::new(&mut pct).range(50..=200));
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && pct * 10 != u32::from(current) {
            cmds.push(apply((pct * 10) as u16));
        }
    });
    if matches!(target, FrameTarget::Whole) {
        ui.weak("The value is set on every clip.");
    }
    button(
        ui,
        app,
        "Remove effect",
        apply(crate::timeline::NORMAL_SPEED),
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
        ui.label(tr!(ThisClipNoLongerExists));
        return;
    };
    if let Some(name) = app.clip_name(clip) {
        ui.add(egui::Label::new(format!("Clip: {name}")).truncate());
    }
    ui.weak(CLEAN_AUDIO_TEXT);
    if c.clean_audio {
        strength_slider(ui, c.clean_strength, Target::Clip(clip), cmds);
        speech_switch(ui, c.speech_enhance, Target::Clip(clip), cmds);
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
        ui.label(tr!(CleanAudioIsOffFor));
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

/// The Strength of Clean audio, 0 to 100 % (REQ-278): 0 is the sound as it was.
fn speech_switch(ui: &mut Ui, now: bool, target: Target, cmds: &mut Vec<Command>) {
    let mut on = now;
    if ui
        .checkbox(&mut on, tr!(NormaliseSpeech))
        .on_hover_text(tr!(NormaliseSpeechTip))
        .changed()
    {
        cmds.push(Command::SetSpeechEnhance { target, on });
    }
}

fn strength_slider(ui: &mut Ui, now: u8, target: Target, cmds: &mut Vec<Command>) {
    ui.horizontal(|ui| {
        ui.label(tr!(Strength));
        let mut v = f32::from(now);
        let r = ui.add(
            egui::Slider::new(&mut v, 0.0..=100.0)
                .step_by(1.0)
                .suffix(" %"),
        );
        if (r.drag_stopped() || (r.changed() && !r.dragged())) && v.round() as u8 != now {
            cmds.push(Command::SetCleanStrength {
                target,
                percent: v.round() as u8,
            });
        }
    });
}

/// The Clean audio popup of the whole video: how many clips have it, apply and remove.
pub fn clean_audio_body(ui: &mut Ui, app: &AppState, cmds: &mut Vec<Command>) {
    let clips = &app.project.timeline.clips;
    let on = clips.iter().filter(|c| c.clean_audio).count();
    ui.weak(CLEAN_AUDIO_TEXT);
    ui.label(format!("{on} of {} clips have Clean audio.", clips.len()));
    if on > 0 {
        let first = clips
            .iter()
            .find(|c| c.clean_audio)
            .map_or(100, |c| c.clean_strength);
        strength_slider(ui, first, Target::WholeVideo, cmds);
        let speech = clips.iter().any(|c| c.speech_enhance);
        speech_switch(ui, speech, Target::WholeVideo, cmds);
    }
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
        ui.label(tr!(ThisFadeNoLongerExists));
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
