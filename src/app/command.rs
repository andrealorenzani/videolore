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
    /// Sets the shadow, the word colours and the Enlarge word size of the subtitle style
    /// (2.1.0, REQ-306 to REQ-309); values are brought inside their ranges.
    SetSubtitleStyle(crate::subtitles::StyleSettings),
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
    /// Settings > Advanced: the video hardware to use, by device name (`None`: Automatic,
    /// REQ-216). A program setting.
    SetGraphicsCard(Option<String>),
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
    /// Back to the default section sizes (and every section shown, in its place).
    ResetLayout,
    /// File > Open in player: the selected media in videolore player (REQ-249).
    OpenInPlayer,
    /// A music clip's own level, mute, fades and Clean audio (REQ-276, REQ-277).
    SetMusicEffects {
        clip: MusicClipId,
        volume: u8,
        muted: bool,
        fade_in: Micros,
        fade_out: Micros,
        clean_audio: bool,
    },
    /// The three answers to a missing LUT at export (REQ-292).
    MissingLutFind,
    MissingLutExportWithout,
    /// How much of the LUT is used (REQ-289).
    SetLutStrength {
        target: FrameTarget,
        percent: u8,
    },
    /// Volume keyframes of a clip, in percent, at clip time (REQ-280).
    SetVolumeKey {
        clip: ClipId,
        at: Micros,
        percent: u8,
    },
    RemoveVolumeKey {
        clip: ClipId,
        at: Micros,
    },
    /// Gives a clip its own Change aspect, or takes it away (REQ-286).
    SetClipAspect {
        clip: ClipId,
        aspect: Option<crate::frame::FrameAspect>,
    },
    /// Speed of a clip in thousandths, 500 to 2000 (REQ-281).
    SetSpeed {
        clip: ClipId,
        speed: u16,
    },
    /// Speed of every clip in thousandths (REQ-281): one copy on each clip (REQ-147), one undo
    /// step.
    SetSpeedAll {
        speed: u16,
    },
    /// Straight or eased move of the volume to the next key (REQ-282).
    SetVolumeKeyEase {
        clip: ClipId,
        at: Micros,
        ease: crate::keyframes::Ease,
    },
    /// Takes the Volume effect off a clip: its keyframes go, the level of the slider stays
    /// (REQ-280).
    RemoveVolume {
        clip: ClipId,
    },
    /// Adds an overlap over the clip under the playhead, from a file (the dialog when none)
    /// (REQ-325).
    AddOverlap(Option<PathBuf>),
    /// Moves or resizes the box of an overlap; `reshaped` is the Ctrl (Command) shape change
    /// (REQ-327).
    SetOverlapBox {
        id: u64,
        place: Pos,
        width: u16,
        height: u16,
        reshaped: bool,
    },
    /// Moves the two ends of an overlap on its clip (offsets from the clip's start).
    SetOverlapTimes {
        id: u64,
        from: Micros,
        len: Micros,
    },
    RemoveOverlap(u64),
    SelectOverlap(Option<u64>),
    /// OK of the warning of an edit that removes overlaps: the edit is done with the removal.
    ConfirmRemoval,
    /// Cancel of that warning: nothing changes.
    CancelRemoval,
    /// The length of every fade (REQ-274) and the side of the fade at a boundary (REQ-275).
    SetFadeLength(Micros),
    SetFadeSide {
        left: ClipId,
        right: ClipId,
        side: crate::timeline::FadeSide,
    },
    /// Marks (or unmarks) a plain 2:1 video as 360° video (REQ-263).
    SetMarked360(MediaId, bool),
    /// Adds or removes a LUT file from the favourites (REQ-294).
    SetFavouriteLut(PathBuf, bool),
    /// When the splash shows (REQ-295).
    SetSplashOption(settings::SplashOption),
    /// The most the preview copies may take (REQ-297).
    SetCacheLimit(u64),
    /// Theme and text size (REQ-243, REQ-244).
    SetTheme(Theme),
    SetTextSize(TextSize),
    /// Toolbar with icons and text, or icons only (REQ-241).
    SetToolbarLabels(bool),
    /// Hides, collapses or docks on the other side Media or Effects (REQ-237, REQ-238).
    SetSectionHidden(Section, bool),
    SetSectionCollapsed(Section, bool),
    SetSectionFlipped(Section, bool),
    /// Workspaces (REQ-239).
    SaveWorkspace(String),
    ApplyWorkspace(String),
    DeleteWorkspace(String),
    /// Adds (`true`) or removes an action (by name) from the toolbar (REQ-240).
    SetToolbarAction(String, bool),
    ResetToolbar,
    /// Shortcuts: wait for the new keys of an action (by name), the keys pressed, give up,
    /// or restore the defaults (REQ-242).
    StartRebind(String),
    Rebind(String, String),
    CancelRebind,
    /// The keys pressed are used by another action (its label): wait for others.
    RebindConflict(String),
    ResetShortcuts,
    /// The window changed (size, place, maximised) or a popup moved (REQ-234, REQ-235).
    WindowChanged(Window),
    PopupMoved(String, (f32, f32)),
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
    /// Moves a movable row of the Applied effects of a clip (REQ-304).
    MoveRow {
        clip: ClipId,
        row: crate::effect_order::Row,
        mv: RowMove,
    },
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
    /// How the value moves on from the selected keyframe (REQ-282).
    SetKeyEase {
        target: FrameTarget,
        effect: FrameFx,
        ease: crate::keyframes::Ease,
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
    /// Normalise speech on or off (REQ-279).
    SetSpeechEnhance {
        target: Target,
        on: bool,
    },
    /// How strong Clean audio is (REQ-278), 0 to 100 %.
    SetCleanStrength {
        target: Target,
        percent: u8,
    },
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
    /// Adds a PNG picture at the playhead (REQ-287); `None` asks for the file.
    AddPicture(Option<std::path::PathBuf>),
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
    /// Sets the width of text boxes, thousandths of the picture width (REQ-116): the right
    /// edge moves, the left one stays.
    ResizeTextWidth {
        ids: Vec<crate::text::TextId>,
        width: u16,
    },
    /// Moves the left or bottom edge of text boxes to a new width or height (thousandths
    /// of the picture): the opposite edge stays (REQ-159, REQ-161).
    ResizeTextBox {
        ids: Vec<crate::text::TextId>,
        edge: TextEdge,
        value: u16,
    },
    /// Sets the readability of text items (REQ-158).
    SetReadability {
        ids: Vec<crate::text::TextId>,
        readability: crate::text::Readability,
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
    /// Arrow keys on a 360 picture that has the focus (REQ-272).
    TurnCamera {
        yaw_cd: i32,
        pitch_cd: i32,
    },
    /// Shift and drag on a 360 picture: turns it round the line of sight (REQ-270).
    RollCamera {
        dx: f32,
        view_w: f32,
    },
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
    /// Moves subtitle lines and text items together so that each ends at `place` (REQ-172).
    MoveSelection {
        lines: Vec<LineId>,
        texts: Vec<crate::text::TextId>,
        place: Pos,
    },
    /// Moves the selected items by the least move that puts each wholly inside the picture
    /// (REQ-175).
    MoveInside,
    /// Moves the anchor of the selected subtitle lines and text items fixed to the sphere to
    /// the centre of the current view (REQ-259).
    ReAnchor,
    /// View > Show boxes on the picture.
    ToggleBoxes,
    /// View > Snap to guides (REQ-198).
    ToggleSnap,
    /// View > Show safe areas (REQ-199).
    ToggleSafeAreas,
    /// Drags the end of a video clip to trim it (REQ-191): `delta` of timeline time moves
    /// the edge, in whole frames, inside the source.
    TrimEdge {
        clip: ClipId,
        edge: crate::timeline::ClipEdge,
        delta: Micros,
    },
    /// "Copy keyframes" of a clip's Scale or Move (REQ-204).
    CopyKeys {
        clip: ClipId,
        effect: FrameFx,
    },
    /// "Paste keyframes to" a clip: replaces its keyframes of that effect (REQ-204).
    PasteKeys {
        clip: ClipId,
        effect: FrameFx,
    },
    /// Moves a keyframe in time (typed time, Left and Right, REQ-203); times are source
    /// times of the clip.
    MoveKey {
        clip: ClipId,
        effect: FrameFx,
        from: Micros,
        to: Micros,
    },
    /// Drags a fade marker from one clip boundary to another (REQ-193).
    MoveFade {
        from: (ClipId, ClipId),
        to: (ClipId, ClipId),
    },
    /// The keys [ and ]: the playhead goes to the previous or next marker and selects it
    /// (REQ-194).
    StepMarker {
        forward: bool,
    },
    /// Delete with subtitle lines selected removes all of them (REQ-197).
    DeleteSubtitles(Vec<LineId>),
    /// Shifts the selected lines earlier or later (REQ-197); `delta` is a multiple of
    /// [`crate::subtitles::SHIFT_STEP`].
    ShiftLines {
        ids: Vec<LineId>,
        delta: Micros,
    },
    /// Asks where to save the subtitles (`.srt`, `.vtt` or `.ass`, REQ-180).
    PickExportSrt,
    /// Saves the subtitles; the extension of the path chooses the format.
    ExportSrt(PathBuf),
    /// Asks for a `.srt` or `.vtt` file and opens the import dialog (REQ-183): for the
    /// range of the clip, or for the whole video (`None`).
    PickImportSubtitles(Option<ClipId>),
    /// Reads the file and opens the import dialog.
    OpenImportSubtitles {
        path: PathBuf,
        clip: Option<ClipId>,
    },
    /// The re-time checkbox of the import dialog (REQ-184).
    SetImportRetime(bool),
    /// Imports the file of the open dialog.
    ConfirmImportSubtitles,
    /// Answer to the subtitle source dialog.
    ChooseSubtitleSource(SourceChoice),
    /// The start-up offer to recover an autosaved copy: open it as unsaved changes (REQ-190).
    RecoverAutosave,
    /// ... delete the copy.
    DiscardAutosave,
    /// ... leave it and open nothing.
    CancelRecovery,
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
            Command::SetFadeLength(_) => "Fade length",
            Command::SetSpeed { .. } | Command::SetSpeedAll { .. } => "Change speed",
            Command::RemoveVolume { .. } => "Remove Volume",
            Command::SetVolumeKeyEase { .. } => "Volume keyframe ease",
            Command::AddOverlap(_) => "Add overlap",
            Command::SetOverlapBox { .. } => "Move overlap",
            Command::SetOverlapTimes { .. } => "Change overlap time",
            Command::RemoveOverlap(_) => "Remove overlap",
            Command::SelectOverlap(_) | Command::ConfirmRemoval | Command::CancelRemoval => "",
            Command::SetClipAspect { .. } => "Clip aspect",
            Command::SetVolumeKey { .. } => "Volume keyframe",
            Command::RemoveVolumeKey { .. } => "Remove volume keyframe",
            Command::SetLutStrength { .. } => "LUT strength",
            Command::SetMusicEffects { .. } => "Music clip sound",
            Command::SetFadeSide { .. } => "Fade side",
            Command::PlaceMusic(_) => "Add music",
            Command::MoveMusic { .. } => "Move music",
            Command::TrimMusic { .. } => "Trim music",
            Command::SplitMusic => "Split music",
            Command::DeleteMusic => "Delete music",
            Command::SetMix(_) => "Volume",
            Command::GenerateSubtitles | Command::ConfirmRegenerate => "Generate subtitles",
            Command::AddSubtitle => "Add subtitle line",
            Command::ConfirmImportSubtitles => "Import subtitles",
            Command::TrimEdge { .. } => "Trim",
            Command::PasteKeys { .. } => "Paste keyframes",
            Command::MoveKey { .. } => "Move keyframe",
            Command::MoveFade { .. } => "Move fade",
            Command::DeleteSubtitles(_) => "Delete subtitle lines",
            Command::ShiftLines { .. } => "Shift subtitle lines",
            Command::AddText(_) => "Add text",
            Command::AddPicture(_) => "Add picture",
            Command::EditText(_) => "Edit text",
            Command::MoveText { .. }
            | Command::SetTextBoxes { .. }
            | Command::MoveTextAnchor { .. } => "Move text",
            Command::ResizeTextWidth { .. }
            | Command::ResizeTextBox { .. }
            | Command::ResizeTextAnchorWidth { .. } => "Resize text",
            Command::SetReadability { .. } => "Text readability",
            Command::MoveLineAnchor { .. } => "Move subtitle",
            Command::DeleteTexts(_) => "Delete text",
            Command::DeleteSubtitle(_) => "Delete subtitle line",
            Command::SetSubtitleText(..) => "Edit subtitle text",
            Command::RetimeSubtitle { .. } => "Change subtitle timing",
            Command::SetPreset(_) | Command::SetSubtitleStyle(_) => "Subtitle style",
            Command::SetLanguage(_) => "Subtitle language",
            Command::ApplyEffect { effect, .. } => effect.undo_name(),
            Command::SetAspect(_) => "Change aspect",
            Command::MoveRow { row, mv, .. } => super::applied::move_label(*row, *mv),
            Command::SetLut { .. } => "Apply LUT",
            Command::RemoveLut(_) => "Remove LUT",
            Command::RemoveAspect | Command::RemoveFrameEffect { .. } => "Remove effect",
            Command::AddKey { .. } => "Add keyframe",
            Command::RemoveKey { .. } => "Remove keyframe",
            Command::SetKeyEase { .. } => "Keyframe easing",
            Command::SetPicture(FrameDrag {
                value: FrameValue::Scale(_),
                ..
            }) => "Scale",
            Command::SetPicture(_) => "Move",
            Command::SetClipVolume { .. } => "Clip volume",
            Command::SetCleanStrength { .. } => "Clean audio strength",
            Command::SetSpeechEnhance { on: true, .. } => "Normalise speech",
            Command::SetSpeechEnhance { on: false, .. } => "Remove normalise speech",
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
            Command::MoveSelection { .. } => "Move",
            Command::MoveInside => "Move inside the picture",
            Command::ReAnchor => "Re-anchor",
            Command::AlignSelected(Axis::Horizontal) => "Align horizontally",
            Command::AlignSelected(Axis::Vertical) => "Align vertically",
            _ => "Edit",
        }
    }
}
