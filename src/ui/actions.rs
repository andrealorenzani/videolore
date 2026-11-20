// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The single table of user actions: label, keyboard shortcuts and tooltip. The menus,
//! the toolbar, the tooltips, the shortcuts window and the key bindings all read it, so
//! they can never disagree.

use crate::app::{AppState, Axis, Command, EffectKind, Popup, Target, ZOOM_STEP};
use crate::time::Micros;
use eframe::egui::{Key, KeyboardShortcut, Modifiers};

/// Every action of the menus and the toolbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionId {
    New,
    Open,
    Save,
    SaveAs,
    Import,
    Export,
    ExportSubtitles,
    Quit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Split,
    Merge,
    MoveLeft,
    MoveRight,
    AddToTimeline,
    AddMusic,
    ClipDetails,
    Settings,
    PlayPause,
    GoStart,
    GoEnd,
    PrevFrame,
    NextFrame,
    Fades,
    Subtitles,
    Volume,
    CleanAudio,
    ChangeAspect,
    Scale,
    Move,
    ApplyLut,
    ToggleFadeAtPlayhead,
    GenerateSubtitles,
    AddSubtitleLine,
    AddText,
    ApplyClipFade,
    ApplyClipSubtitles,
    ApplyClipVolume,
    ApplyClipCleanAudio,
    ApplyClipScale,
    ApplyClipMove,
    ApplyClipLut,
    ApplyWholeFade,
    ApplyWholeSubtitles,
    ApplyWholeVolume,
    ApplyWholeCleanAudio,
    ApplyWholeChangeAspect,
    ApplyWholeScale,
    ApplyWholeMove,
    ApplyWholeLut,
    AlignHorizontal,
    AlignVertical,
    MoveInside,
    ResetObjects,
    SetPov,
    ToggleBoxes,
    ZoomIn,
    ZoomOut,
    ZoomFit,
    ResetLayout,
    Shortcuts,
    About,
}

/// One action of the table.
#[derive(Debug, Clone, Copy)]
pub struct Action {
    pub id: ActionId,
    pub label: &'static str,
    /// The bindings; the first one is shown in menus and tooltips.
    pub shortcuts: &'static [KeyboardShortcut],
    pub tooltip: &'static str,
}

const fn cmd(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND, key)
}

const fn cmd_shift(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), key)
}

const fn plain(key: Key) -> KeyboardShortcut {
    KeyboardShortcut::new(Modifiers::NONE, key)
}

macro_rules! action {
    ($id:ident, $label:expr, [$($sc:expr),*], $tip:expr) => {
        Action {
            id: ActionId::$id,
            label: $label,
            shortcuts: &[$($sc),*],
            tooltip: $tip,
        }
    };
}

/// The table, in menu order.
pub const ACTIONS: &[Action] = &[
    action!(New, "New", [cmd(Key::N)], "Start a new project"),
    action!(Open, "Open…", [cmd(Key::O)], "Open a project"),
    action!(Save, "Save", [cmd(Key::S)], "Save the project"),
    action!(
        SaveAs,
        "Save as…",
        [cmd_shift(Key::S)],
        "Save the project in a new file"
    ),
    action!(
        Import,
        "Import media…",
        [cmd(Key::I)],
        "Add video and audio files to the media list"
    ),
    action!(
        Export,
        "Export…",
        [cmd(Key::E)],
        "Make the final video file"
    ),
    action!(
        ExportSubtitles,
        "Export subtitles (.srt)…",
        [],
        "Save the subtitles of the timeline as an .srt file"
    ),
    action!(Quit, "Quit", [cmd(Key::Q)], "Close videolore"),
    action!(Undo, "Undo", [cmd(Key::Z)], "Undo the last change"),
    action!(
        Redo,
        "Redo",
        [cmd_shift(Key::Z), cmd(Key::Y)],
        "Redo the change you undid"
    ),
    action!(
        Cut,
        "Cut",
        [cmd(Key::X)],
        "Remove the selected clip and keep it for pasting"
    ),
    action!(
        Copy,
        "Copy",
        [cmd(Key::C)],
        "Keep a copy of the selected clip"
    ),
    action!(
        Paste,
        "Paste",
        [cmd(Key::V)],
        "Insert the cut or copied clip at the playhead"
    ),
    action!(
        Delete,
        "Delete",
        [plain(Key::Delete), plain(Key::Backspace)],
        "Remove the selected clip"
    ),
    action!(
        Split,
        "Split at playhead",
        [cmd(Key::B)],
        "Cut the clip in two at the playhead"
    ),
    action!(
        Merge,
        "Merge",
        [],
        "Join the selected clip with the next part of the same file"
    ),
    action!(
        MoveLeft,
        "Move left",
        [],
        "Move the selected clip one place earlier"
    ),
    action!(
        MoveRight,
        "Move right",
        [],
        "Move the selected clip one place later"
    ),
    action!(
        AddToTimeline,
        "Add selected media to timeline",
        [],
        "Put the selected video at the end of the timeline"
    ),
    action!(
        AddMusic,
        "Add selected media to music track",
        [],
        "Put the selected audio file on the music track at the playhead"
    ),
    action!(
        ClipDetails,
        "Clip details…",
        [],
        "Trim, fade and merge the selected clip"
    ),
    action!(
        Settings,
        "Settings…",
        [cmd(Key::Comma)],
        "Preview, subtitle, effort and export choices"
    ),
    action!(
        PlayPause,
        "Play/Pause",
        [plain(Key::Space)],
        "Play or pause the preview"
    ),
    action!(
        GoStart,
        "Go to start",
        [plain(Key::Home)],
        "Move the playhead to the start"
    ),
    action!(
        GoEnd,
        "Go to end",
        [plain(Key::End)],
        "Move the playhead to the end"
    ),
    action!(
        PrevFrame,
        "Previous frame",
        [plain(Key::ArrowLeft)],
        "Move the playhead one frame back"
    ),
    action!(
        NextFrame,
        "Next frame",
        [plain(Key::ArrowRight)],
        "Move the playhead one frame forward"
    ),
    action!(Fades, "Fades…", [], "Fade IN/OUT between clips"),
    action!(
        Subtitles,
        "Subtitles…",
        [],
        "Subtitle style, language and lines"
    ),
    action!(Volume, "Volume…", [], "Video sound and music volume"),
    action!(
        CleanAudio,
        "Clean audio…",
        [],
        "Reduce the background noise of the video's sound"
    ),
    action!(
        ChangeAspect,
        "Change aspect…",
        [],
        "The shape of the whole video's frame and what fills the empty space"
    ),
    action!(
        Scale,
        "Scale…",
        [],
        "Make the picture bigger or smaller, with keyframes"
    ),
    action!(
        Move,
        "Move…",
        [],
        "Move the picture in the frame, with keyframes"
    ),
    action!(
        ApplyLut,
        "Apply LUT…",
        [],
        "Change the colours of the picture with a LUT file"
    ),
    action!(
        ToggleFadeAtPlayhead,
        "Add/remove fade at playhead",
        [],
        "Add or remove the fade at the clip boundary nearest the playhead"
    ),
    action!(
        GenerateSubtitles,
        "Generate subtitles",
        [],
        "Make subtitles from the speech in the clips"
    ),
    action!(
        AddSubtitleLine,
        "Add subtitle line at playhead",
        [],
        "Add a subtitle line at the playhead"
    ),
    action!(
        AddText,
        "Add text at playhead",
        [],
        "Opens a popup to add a text at the playhead"
    ),
    action!(
        ApplyClipFade,
        "Fade IN/OUT",
        [],
        "Fade the selected clip at both of its edges"
    ),
    action!(
        ApplyClipSubtitles,
        "Subtitles",
        [],
        "Create subtitles for the selected clip only"
    ),
    action!(
        ApplyClipVolume,
        "Volume",
        [],
        "Set the sound level of the selected clip"
    ),
    action!(
        ApplyClipCleanAudio,
        "Clean audio",
        [],
        "Reduce the background noise in the sound of the selected clip"
    ),
    action!(
        ApplyClipScale,
        "Scale",
        [],
        "Make the picture of the selected clip bigger or smaller"
    ),
    action!(
        ApplyClipMove,
        "Move",
        [],
        "Move the picture of the selected clip in the frame"
    ),
    action!(
        ApplyClipLut,
        "Apply LUT",
        [],
        "Change the colours of the selected clip with a LUT file"
    ),
    action!(
        ApplyWholeFade,
        "Fade IN/OUT",
        [],
        "Fade from black at the start and to black at the end of the video"
    ),
    action!(
        ApplyWholeSubtitles,
        "Subtitles",
        [],
        "Create subtitles for every clip"
    ),
    action!(
        ApplyWholeVolume,
        "Volume",
        [],
        "Set the sound levels of the whole video"
    ),
    action!(
        ApplyWholeCleanAudio,
        "Clean audio",
        [],
        "Reduce the background noise in the sound of the whole video"
    ),
    action!(
        ApplyWholeChangeAspect,
        "Change aspect",
        [],
        "Change the shape of the whole video's frame"
    ),
    action!(
        ApplyWholeScale,
        "Scale",
        [],
        "Make the picture of the whole video bigger or smaller"
    ),
    action!(
        ApplyWholeMove,
        "Move",
        [],
        "Move the picture of the whole video in the frame"
    ),
    action!(
        ApplyWholeLut,
        "Apply LUT",
        [],
        "Change the colours of the whole video with a LUT file"
    ),
    action!(
        AlignHorizontal,
        "Align horizontally",
        [cmd_shift(Key::H)],
        "Centre the selected objects on the picture, left to right"
    ),
    action!(
        AlignVertical,
        "Align vertically",
        [cmd_shift(Key::J)],
        "Centre the selected objects on the picture, top to bottom"
    ),
    action!(
        MoveInside,
        "Move inside the picture",
        [],
        "Move the selected items by the least distance that puts each wholly inside the picture"
    ),
    action!(
        ResetObjects,
        "Reset",
        [cmd_shift(Key::R)],
        "Put the selected objects back at their original place and size; a 360° view back to the front"
    ),
    action!(
        SetPov,
        "Set POV",
        [plain(Key::P)],
        "Save the 360° view shown now as a point of view at the playhead"
    ),
    action!(
        ToggleBoxes,
        "Show boxes on the picture",
        [],
        "Show or hide the boxes around the objects you can move on the picture"
    ),
    action!(
        ZoomIn,
        "Zoom in",
        [cmd(Key::Equals), cmd(Key::Plus)],
        "Show the timeline in more detail"
    ),
    action!(
        ZoomOut,
        "Zoom out",
        [cmd(Key::Minus)],
        "Show more of the timeline"
    ),
    action!(
        ZoomFit,
        "Fit project",
        [cmd(Key::Num0)],
        "Show the whole project in the timeline"
    ),
    action!(
        ResetLayout,
        "Reset layout",
        [],
        "Give the sections their original sizes"
    ),
    action!(
        Shortcuts,
        "Keyboard shortcuts",
        [plain(Key::F1)],
        "List every keyboard shortcut"
    ),
    action!(
        About,
        "About videolore and third-party notices",
        [],
        "Version, licence and the components videolore uses"
    ),
];

/// Esc closes the topmost popup (handled apart from the table: it is not a command).
pub const CLOSE_POPUP: KeyboardShortcut = plain(Key::Escape);
/// What Esc does, for the shortcuts window.
pub const CLOSE_POPUP_LABEL: &str =
    "Close the topmost popup (none open: clear the selected subtitle lines)";

fn apply(effect: EffectKind, target: Target) -> Command {
    Command::ApplyEffect { effect, target }
}

/// The table entry of `id`.
pub fn action(id: ActionId) -> &'static Action {
    ACTIONS
        .iter()
        .find(|a| a.id == id)
        .expect("every action is in the table")
}

impl ActionId {
    /// The command this action runs now (Export becomes Cancel export while exporting).
    pub fn command(self, app: &AppState) -> Command {
        use ActionId::*;
        match self {
            New => Command::NewProject,
            Open => Command::PickOpenProject,
            Save => Command::SaveProject,
            SaveAs => Command::PickSaveProjectAs,
            Import => Command::PickImport,
            Export if app.is_exporting() => Command::CancelExport,
            Export => Command::PickExport,
            ExportSubtitles => Command::PickExportSrt,
            Quit => Command::Quit,
            Undo => Command::Undo,
            Redo => Command::Redo,
            Cut => Command::Cut,
            Copy => Command::Copy,
            Paste => Command::Paste,
            Delete if app.selection.primary_text().is_some() => {
                Command::DeleteTexts(app.selection.texts())
            }
            Delete => Command::Delete,
            Split => Command::Split,
            Merge => Command::Merge,
            MoveLeft => Command::MoveLeft,
            MoveRight => Command::MoveRight,
            AddToTimeline => Command::Append(None),
            AddMusic => Command::PlaceMusic(None),
            ClipDetails => Command::OpenPopup(Popup::ClipDetails),
            Settings => Command::OpenSettings,
            PlayPause => Command::PlayPause,
            GoStart => Command::Seek(Micros::ZERO),
            GoEnd => Command::Seek(app.project.timeline.duration()),
            PrevFrame => Command::StepFrames(-1),
            NextFrame => Command::StepFrames(1),
            Fades => Command::OpenPopup(Popup::Fades),
            Subtitles => Command::OpenPopup(Popup::Subtitles),
            Volume => Command::OpenPopup(Popup::Volume),
            CleanAudio => Command::OpenPopup(Popup::CleanAudio),
            ChangeAspect => Command::OpenFrameEffect(EffectKind::ChangeAspect),
            Scale => Command::OpenFrameEffect(EffectKind::Scale),
            Move => Command::OpenFrameEffect(EffectKind::Move),
            ApplyLut => Command::OpenFrameEffect(EffectKind::ApplyLut),
            ToggleFadeAtPlayhead => Command::ToggleFadeAtPlayhead,
            GenerateSubtitles => Command::GenerateSubtitles,
            AddSubtitleLine => Command::AddSubtitle,
            AddText => Command::OpenNewText,
            ApplyClipFade => apply(EffectKind::Fade, Target::Selected),
            ApplyClipSubtitles => apply(EffectKind::Subtitles, Target::Selected),
            ApplyClipVolume => apply(EffectKind::Volume, Target::Selected),
            ApplyClipCleanAudio => apply(EffectKind::CleanAudio, Target::Selected),
            ApplyClipScale => apply(EffectKind::Scale, Target::Selected),
            ApplyClipMove => apply(EffectKind::Move, Target::Selected),
            ApplyClipLut => apply(EffectKind::ApplyLut, Target::Selected),
            ApplyWholeLut => apply(EffectKind::ApplyLut, Target::WholeVideo),
            ApplyWholeChangeAspect => apply(EffectKind::ChangeAspect, Target::WholeVideo),
            ApplyWholeScale => apply(EffectKind::Scale, Target::WholeVideo),
            ApplyWholeMove => apply(EffectKind::Move, Target::WholeVideo),
            ApplyWholeFade => apply(EffectKind::Fade, Target::WholeVideo),
            ApplyWholeSubtitles => apply(EffectKind::Subtitles, Target::WholeVideo),
            ApplyWholeVolume => apply(EffectKind::Volume, Target::WholeVideo),
            ApplyWholeCleanAudio => apply(EffectKind::CleanAudio, Target::WholeVideo),
            AlignHorizontal => Command::AlignSelected(Axis::Horizontal),
            AlignVertical => Command::AlignSelected(Axis::Vertical),
            MoveInside => Command::MoveInside,
            ResetObjects => Command::ResetObjects,
            SetPov => Command::SetPov,
            ToggleBoxes => Command::ToggleBoxes,
            ZoomIn => Command::ZoomBy(ZOOM_STEP),
            ZoomOut => Command::ZoomBy(1.0 / ZOOM_STEP),
            ZoomFit => Command::ZoomFit,
            ResetLayout => Command::ResetLayout,
            Shortcuts => Command::OpenPopup(Popup::Shortcuts),
            About => Command::ShowAbout,
        }
    }

    /// The icon of the action (REQ-142): every action has one, so the menus and the toolbar
    /// never show an item without it (the match is exhaustive).
    pub fn icon(self) -> &'static str {
        use super::icons as i;
        use ActionId::*;
        match self {
            New => i::FILE_PLUS,
            Open => i::FOLDER_OPEN,
            Save => i::FLOPPY_DISK,
            SaveAs => i::FLOPPY_DISK_BACK,
            Import => i::DOWNLOAD_SIMPLE,
            Export => i::EXPORT,
            ExportSubtitles => i::SUBTITLES,
            Quit => i::SIGN_OUT,
            Undo => i::ARROW_U_UP_LEFT,
            Redo => i::ARROW_U_UP_RIGHT,
            Cut => i::SCISSORS,
            Copy => i::COPY,
            Paste => i::CLIPBOARD,
            Delete => i::TRASH,
            Split => i::SPLIT_HORIZONTAL,
            Merge => i::ARROWS_MERGE,
            MoveLeft => i::ARROW_LEFT,
            MoveRight => i::ARROW_RIGHT,
            AddToTimeline => i::FILM_STRIP,
            AddMusic => i::MUSIC_NOTE,
            ClipDetails => i::SLIDERS,
            Settings => i::GEAR,
            PlayPause => i::PLAY,
            GoStart => i::SKIP_BACK,
            GoEnd => i::SKIP_FORWARD,
            PrevFrame => i::CARET_LEFT,
            NextFrame => i::CARET_RIGHT,
            Fades | ApplyClipFade | ApplyWholeFade | ToggleFadeAtPlayhead => i::CIRCLE_HALF,
            Subtitles | ApplyClipSubtitles | ApplyWholeSubtitles | AddSubtitleLine => i::SUBTITLES,
            Volume | ApplyClipVolume | ApplyWholeVolume => i::SPEAKER_HIGH,
            CleanAudio | ApplyClipCleanAudio | ApplyWholeCleanAudio => i::WAVEFORM,
            ChangeAspect | ApplyWholeChangeAspect => i::FRAME_CORNERS,
            Scale | ApplyClipScale | ApplyWholeScale => i::ARROWS_OUT,
            Move | ApplyClipMove | ApplyWholeMove => i::ARROWS_OUT_CARDINAL,
            ApplyLut | ApplyClipLut | ApplyWholeLut => i::PALETTE,
            GenerateSubtitles => i::MAGIC_WAND,
            AddText => i::TEXT_T,
            AlignHorizontal => i::ALIGN_CENTER_HORIZONTAL,
            AlignVertical => i::ALIGN_CENTER_VERTICAL,
            MoveInside => i::CORNERS_OUT,
            ResetObjects => i::ARROW_COUNTER_CLOCKWISE,
            SetPov => i::CROSSHAIR,
            ToggleBoxes => i::BOUNDING_BOX,
            ZoomIn => i::MAGNIFYING_GLASS_PLUS,
            ZoomOut => i::MAGNIFYING_GLASS_MINUS,
            ZoomFit => i::CORNERS_OUT,
            ResetLayout => i::SIDEBAR,
            Shortcuts => i::KEYBOARD,
            About => i::INFO,
        }
    }

    /// The label shown now: "Undo Split", "Cancel export"…
    pub fn label(self, app: &AppState) -> String {
        match self {
            ActionId::Undo => app.undo_text(),
            ActionId::Redo => app.redo_text(),
            ActionId::Export if app.is_exporting() => "Cancel export".into(),
            _ => action(self).label.into(),
        }
    }

    /// The tooltip shown now (names the undo step, and the export being cancelled).
    pub fn tooltip(self, app: &AppState) -> String {
        match self {
            ActionId::Undo | ActionId::Redo => self.label(app),
            ActionId::Export if app.is_exporting() => "Stop the export (no file is left)".into(),
            ActionId::ToggleBoxes if app.settings.show_boxes => {
                "Boxes are shown on the picture: click to hide them".into()
            }
            ActionId::ToggleBoxes => "Boxes are hidden: click to show them on the picture".into(),
            _ => action(self).tooltip.into(),
        }
    }
}
