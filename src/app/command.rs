// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! [`Command`]: every user operation, and the name of the undo step it makes.

use super::*;

/// Every user operation.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    NewProject,
    PickOpenProject,
    OpenProject(PathBuf),
    SaveProject,
    PickSaveProjectAs,
    SaveProjectAs(PathBuf),
    /// The window is being closed.
    RequestClose,
    Quit,
    ResolveUnsaved(UnsavedChoice),
    PickImport,
    Import(Vec<PathBuf>),
    RemoveMedia(MediaId),
    SelectMedia(Option<MediaId>),
    /// Appends the given media (or the selected one) at the end of the track.
    Append(Option<MediaId>),
    SelectClip(Option<ClipId>),
    Split,
    Cut,
    Copy,
    Paste,
    Delete,
    Merge,
    SetTrim {
        clip: ClipId,
        src_in: Micros,
        src_out: Micros,
    },
    MoveClip {
        clip: ClipId,
        to: usize,
    },
    MoveLeft,
    MoveRight,
    /// Toggles the fade at boundary `b` (between clips `b` and `b + 1`).
    ToggleFade(usize),
    /// Toggles the fade at the boundary nearest to the playhead.
    ToggleFadeAtPlayhead,
    /// Puts the given audio file (or the selected one) on the music track at the playhead.
    PlaceMusic(Option<MediaId>),
    SelectMusic(Option<MusicClipId>),
    MoveMusic {
        clip: MusicClipId,
        start: Micros,
    },
    TrimMusic {
        clip: MusicClipId,
        src_in: Micros,
        src_out: Micros,
    },
    /// Splits the music clip under the playhead.
    SplitMusic,
    /// Deletes the selected music clip.
    DeleteMusic,
    SetMix(Mix),
    GenerateSubtitles,
    /// The user confirmed that manual subtitle edits may be replaced.
    ConfirmRegenerate,
    ModelConsent(ConsentChoice),
    CancelSubtitles,
    /// Adds a subtitle line at the playhead.
    AddSubtitle,
    DeleteSubtitle(LineId),
    SetSubtitleText(LineId, String),
    RetimeSubtitle {
        line: LineId,
        clip: ClipId,
        start: Micros,
        end: Micros,
    },
    SelectLine(Option<LineId>),
    SetPreset(Preset),
    /// A language code, or `None` to detect it.
    SetLanguage(Option<String>),
    OpenSettings,
    CloseSettings,
    SetPreviewQuality(PreviewQuality),
    SetSubtitleQuality(SubtitleQuality),
    SetEffort(Effort),
    SetExport(ExportChoice),
    /// Forgets the hardware problems and checks the video hardware again.
    TryHardwareAgain,
    /// Detects the machine again and applies the recommended choices.
    UseRecommended,
    /// Asks before removing the downloaded speech recognition files.
    RemoveSpeechFiles,
    ConfirmRemoveSpeechFiles,
    /// Uses the bundled (or system) ffmpeg again instead of the chosen one.
    ResetFfmpeg,
    /// The banner's button.
    BannerAction(BannerAction),
    DismissBanner,
    ClearProxyCache,
    Undo,
    Redo,
    PlayPause,
    Seek(Micros),
    StepFrames(i64),
    SetZoom(f32),
    /// File > Export…: opens the Export dialog (REQ-118).
    PickExport,
    /// The dialog's choices changed.
    SetExportChoice(Box<export::choice::ExportChoice>),
    /// Export in the dialog: starts the export (asks before replacing a file).
    StartExport(Box<export::choice::ExportChoice>),
    /// Answer "Replace" of the overwrite question.
    ConfirmOverwrite,
    /// The "..." button of the dialog: choose the folder.
    PickExportFolder,
    /// Exports to a file in a chosen way.
    ExportAs(PathBuf, render::ExportMode),
    Export(PathBuf),
    CancelExport,
    PickFfmpeg,
    SetFfmpeg(PathBuf),
    RetryFfmpeg,
    ShowAbout,
    CloseModal,
    RemoveDesktopIntegration,
    DismissNotice,
    OpenPopup(Popup),
    ClosePopup(Popup),
    /// Selects a fade or line, moves the playhead to its start and opens its popup.
    OpenInstance(Instance),
    /// Shows the chooser of overlapping markers (an empty list closes it).
    ChooseInstance(Vec<Instance>),
    /// Adds (`on`) or removes the fade between two adjacent clips.
    SetFade {
        left: ClipId,
        right: ClipId,
        on: bool,
    },
    /// Removes every fade (one undo step).
    RemoveAllFades,
    /// Plays one subtitle line and stops at its end.
    PlayLine(LineId),
    /// Multiplies the timeline zoom.
    ZoomBy(f32),
    /// Fits the whole project in the timeline.
    ZoomFit,
    /// New section sizes (saved in the settings).
    SetLayout(SavedLayout),
    /// Back to the default section sizes.
    ResetLayout,
    OpenRecent(PathBuf),
    ClearRecent,
    /// Files dropped on the window.
    DropFiles(Vec<PathBuf>),
    /// Applies an effect to a clip or to the whole video (and opens its popup).
    ApplyEffect {
        effect: EffectKind,
        target: Target,
    },
    /// Opens the popup of a picture effect of the whole video (a click on its row).
    OpenFrameEffect(EffectKind),
    /// Asks for a LUT file and applies it to a clip or the whole video (REQ-135).
    PickLut(FrameTarget),
    /// Applies the LUT file at `path` (the file chosen in the dialog).
    SetLut {
        target: FrameTarget,
        path: PathBuf,
    },
    /// Removes the Apply LUT of a clip or the whole video.
    RemoveLut(FrameTarget),
    /// Sets Change aspect of the whole video (REQ-124).
    SetAspect(FrameAspect),
    /// Removes Change aspect.
    RemoveAspect,
    /// Adds a keyframe at the playhead with the value shown there (REQ-129).
    AddKey {
        target: FrameTarget,
        effect: FrameFx,
    },
    /// Removes the selected keyframe.
    RemoveKey {
        target: FrameTarget,
        effect: FrameFx,
    },
    /// Removes Scale or Move (all its keyframes).
    RemoveFrameEffect {
        target: FrameTarget,
        effect: FrameFx,
    },
    /// Selects a keyframe of the open Scale or Move popup.
    SelectKey(Option<Micros>),
    /// A value being dragged in the preview (not an edit).
    DragPicture(Option<FrameDrag>),
    /// Sets the selected keyframe, or makes one at the playhead (REQ-127..129).
    SetPicture(FrameDrag),
    /// Sound level (0–100 %) and mute of one video clip.
    /// Switches Clean audio on or off for a clip or the whole video (REQ-114).
    SetCleanAudio {
        target: Target,
        on: bool,
    },
    SetClipVolume {
        clip: ClipId,
        volume: u8,
        muted: bool,
    },
    /// Adds or removes the fade at the very start or end of the video.
    SetEdgeFade {
        edge: Edge,
        on: bool,
    },
    /// Adds a subtitle line to the selection, or takes it out (Ctrl/Cmd+click).
    ToggleLine(LineId),
    /// Adds a text item at the playhead from the fields of a draft (REQ-108).
    AddText(crate::text::TextItem),
    /// Replaces a text item with the edited one (REQ-108, REQ-110).
    EditText(crate::text::TextItem),
    /// Moves a text item in time and sets its time to live (REQ-110).
    MoveText {
        id: crate::text::TextId,
        start: Micros,
        ttl: Micros,
    },
    /// Removes text items (REQ-110).
    DeleteTexts(Vec<crate::text::TextId>),
    /// Moves the box of text items in the preview (REQ-111, REQ-117).
    SetTextBoxes {
        ids: Vec<crate::text::TextId>,
        place: Pos,
    },
    /// Sets the width of text boxes, thousandths of the picture width (REQ-116).
    ResizeTextWidth {
        ids: Vec<crate::text::TextId>,
        width: u16,
    },
    /// Sets the width, as an angle, of text boxes fixed to the sphere (REQ-116).
    ResizeTextAnchorWidth {
        ids: Vec<crate::text::TextId>,
        width_cd: u32,
    },
    /// Moves texts fixed to the sphere to another point of it (REQ-112).
    MoveTextAnchor {
        ids: Vec<crate::text::TextId>,
        yaw_cd: i32,
        pitch_cd: i32,
    },
    /// Moves subtitle lines fixed to the sphere to another point of it (REQ-112).
    MoveLineAnchor {
        ids: Vec<LineId>,
        yaw_cd: i32,
        pitch_cd: i32,
    },
    /// Selects a text item, moves the playhead to it and opens its popup (REQ-110).
    OpenText(crate::text::TextId),
    /// Selects only this text item (a click on its box in the preview).
    SelectText(crate::text::TextId),
    /// Opens the popup of a new text (REQ-108).
    OpenNewText,
    /// Adds a text item to the selection or takes it out (Ctrl/Cmd/Shift+click).
    ToggleText(crate::text::TextId),
    /// Selects the lines whose timeline start is in `[from, to]` (`add`: keeps the
    /// current selection too) and moves the playhead to the first one.
    SelectRange {
        from: Micros,
        to: Micros,
        add: bool,
    },
    ClearSelection,
    /// Gives the lines their own place on screen (`None`: the style's place).
    SetLinePositions {
        ids: Vec<LineId>,
        pos: Option<Pos>,
    },
    /// The selected lines go back to the style's place.
    ResetPosition,
    /// Where lines are while being dragged in the preview (not an edit).
    DragLines(Option<(Vec<LineId>, Pos)>),
    /// Turns the camera of the 360 clip under the playhead: the picture was dragged by
    /// `dx`/`dy` pixels in a preview `view_w` pixels wide (not an edit).
    DragCamera {
        dx: f32,
        dy: f32,
        view_w: f32,
    },
    /// Wheel or pinch zoom of the 360 clip under the playhead (positive zooms in).
    ZoomCamera(f32),
    /// Stores the camera shown at the playhead as a point of view of its 360 source.
    SetPov,
    /// Changes a point of view: its time (`old_t` finds it) and view.
    EditPov {
        media: MediaId,
        old_t: Micros,
        pov: crate::camera::Pov,
    },
    RemovePov {
        media: MediaId,
        t: Micros,
    },
    /// A corner drag finished: `ids` get `size` (thousandths), `line` its new centre.
    ResizeLines {
        ids: Vec<LineId>,
        line: LineId,
        size: u16,
        pos: Pos,
    },
    /// Lines being resized in the preview (not an edit).
    DragResize(Option<ResizeDrag>),
    /// Centres the selected lines on the picture along one axis.
    AlignSelected(Axis),
    /// Selected lines back to the style's place and size; else the 360 camera to front.
    ResetObjects,
    /// View > Show boxes on the picture.
    ToggleBoxes,
    /// Asks where to save the subtitles as an `.srt` file.
    PickExportSrt,
    ExportSrt(PathBuf),
    /// Answer to the subtitle source dialog.
    ChooseSubtitleSource(SourceChoice),
}

/// Zoom step of Zoom in / Zoom out.
pub const ZOOM_STEP: f32 = std::f32::consts::SQRT_2;

impl Command {
    /// The name of the undo step this command makes, if it edits the project.
    pub fn undo_name(&self) -> &'static str {
        match self {
            Command::Append(_) => "Add to timeline",
            Command::Split => "Split",
            Command::Cut => "Cut",
            Command::Delete => "Delete",
            Command::Paste => "Paste",
            Command::Merge => "Merge",
            Command::SetTrim { .. } => "Trim",
            Command::MoveClip { .. } => "Move clip",
            Command::MoveLeft => "Move left",
            Command::MoveRight => "Move right",
            Command::SetFade { on: true, .. } => "Add fade",
            Command::SetFade { on: false, .. } => "Remove fade",
            Command::ToggleFade(_) | Command::ToggleFadeAtPlayhead => "Fade",
            Command::RemoveAllFades => "Remove all fades",
            Command::PlaceMusic(_) => "Add music",
            Command::MoveMusic { .. } => "Move music",
            Command::TrimMusic { .. } => "Trim music",
            Command::SplitMusic => "Split music",
            Command::DeleteMusic => "Delete music",
            Command::SetMix(_) => "Volume",
            Command::GenerateSubtitles | Command::ConfirmRegenerate => "Generate subtitles",
            Command::AddSubtitle => "Add subtitle line",
            Command::AddText(_) => "Add text",
            Command::EditText(_) => "Edit text",
            Command::MoveText { .. }
            | Command::SetTextBoxes { .. }
            | Command::MoveTextAnchor { .. } => "Move text",
            Command::ResizeTextWidth { .. } | Command::ResizeTextAnchorWidth { .. } => {
                "Resize text"
            }
            Command::MoveLineAnchor { .. } => "Move subtitle",
            Command::DeleteTexts(_) => "Delete text",
            Command::DeleteSubtitle(_) => "Delete subtitle line",
            Command::SetSubtitleText(..) => "Edit subtitle text",
            Command::RetimeSubtitle { .. } => "Change subtitle timing",
            Command::SetPreset(_) => "Subtitle style",
            Command::SetLanguage(_) => "Subtitle language",
            Command::ApplyEffect {
                effect: EffectKind::Fade,
                ..
            } => "Fade IN/OUT",
            Command::ApplyEffect {
                effect: EffectKind::Subtitles,
                ..
            } => "Create subtitles",
            Command::ApplyEffect {
                effect: EffectKind::CleanAudio,
                ..
            } => "Clean audio",
            Command::ApplyEffect {
                effect: EffectKind::ChangeAspect,
                ..
            }
            | Command::SetAspect(_) => "Change aspect",
            Command::ApplyEffect {
                effect: EffectKind::Scale,
                ..
            } => "Scale",
            Command::ApplyEffect {
                effect: EffectKind::Move,
                ..
            } => "Move",
            Command::SetLut { .. } => "Apply LUT",
            Command::RemoveLut(_) => "Remove LUT",
            Command::RemoveAspect | Command::RemoveFrameEffect { .. } => "Remove effect",
            Command::AddKey { .. } => "Add keyframe",
            Command::RemoveKey { .. } => "Remove keyframe",
            Command::SetPicture(FrameDrag {
                value: FrameValue::Scale(_),
                ..
            }) => "Scale",
            Command::SetPicture(_) => "Move",
            Command::ApplyEffect { .. } => "Volume",
            Command::SetClipVolume { .. } => "Clip volume",
            Command::SetCleanAudio { on: true, .. } => "Clean audio",
            Command::SetCleanAudio { on: false, .. } => "Remove clean audio",
            Command::SetEdgeFade { on: true, .. } => "Add fade",
            Command::SetEdgeFade { on: false, .. } => "Remove fade",
            Command::SetLinePositions { pos: Some(_), .. } => "Move subtitle",
            Command::SetLinePositions { pos: None, .. }
            | Command::ResetPosition
            | Command::ResetObjects => "Reset",
            Command::SetPov => "Set POV",
            Command::EditPov { .. } => "Edit POV",
            Command::RemovePov { .. } => "Remove POV",
            Command::ResizeLines { .. } => "Resize subtitle",
            Command::AlignSelected(Axis::Horizontal) => "Align horizontally",
            Command::AlignSelected(Axis::Vertical) => "Align vertically",
            _ => "Edit",
        }
    }
}
