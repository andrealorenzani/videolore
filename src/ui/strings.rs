// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The interface texts, in one table and in English only (REQ-245, ADR-196). A text of a
//! label, button, menu or tooltip in `src/ui/` is written as `tr!(Name)`; the table below
//! is the place where a language could be added later. Tests check that every key has a
//! text and that no `ui/` file writes a text directly in those calls.

/// The key of one interface text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Key {
    AddFadeAtTheBoundary,
    Style,
    Language,
    SubtitleGenerationIsNotAvailable,
    DownloadingTheSpeechRecognitionFile,
    MatchingSubtitlesToTheSpeech,
    CtrlCmdClickALine,
    ThisClipNoLongerExists,
    ClipSound,
    CleanAudioIsOffFor,
    ThisFadeNoLongerExists,
    ChooseTheFolder,
    N360Video,
    BackToSameAsSource,
    ChangeAspectIsNotApplied,
    Aspect,
    Fit,
    EmptySpace,
    Colour,
    OneCopyPerClipSelect,
    ClickAKeyframeToSelect,
    KeyframeTime,
    PasteKeyframesTo,
    Scale,
    NoRecentProjects,
    ClearList,
    OpenRecent,
    MoreMenu,
    SetPOV,
    SeekDragToMoveThe,
    YouCanAlsoSetVIDEOLORE,
    ChooseFfmpeg,
    Retry,
    Quit,
    OK,
    Remove,
    Cancel,
    SubtitlesToUse,
    RecogniseSpeech,
    Replace,
    Recover,
    Discard,
    Close,
    RemoveFromApplicationMenu,
    Import,
    ClickAMarkerOnThe,
    ThisPointOfViewNo,
    N360PointOfView,
    FadeINOUT,
    ThisSubtitleLineNoLonger,
    SubtitleLine,
    Start,
    End,
    BackToThePlaceOf,
    TheLineIsAtThe,
    SelectAClipOnThe,
    VideoClip,
    N360Video2,
    TrimIn,
    AtPlayhead,
    TrimOut,
    MergeWithNext,
    MusicClip,
    Start2,
    End2,
    CloseMark,
    CloseTheSplashScreenEsc,
    GiveTheNewClipThe,
    For,
    In,
    TextEnterStartsANew,
    Effects,
    TimeToLive,
    Font,
    LoadingFonts,
    Readability,
    NewText,
    ThisTextNoLongerExists,
    TextWord,
    Delete,
    ZoomFromTheWholeProject,
    DragToTrimThisEnd,
    ShiftDragSelectTheSubtitle,
    MoreArrow,
    MoreCommands,
    Expand,
    PreviewCopiesUpTo,
    Never,
    OnceForEachVersion,
    AtEveryStart,
    StartPicture,
    TextSizeLarger,
    TextSizeLarge,
    TextSizeNormal,
    TextSize,
    ResetToolbar,
    IconsAndText,
    CustomiseToolbar,
    ThemeDark,
    ThemeLight,
    ThemeSystem,
    Theme,
    SaveTheCurrentLayout,
    WorkspaceName,
    DeleteThisWorkspace,
    Workspaces,
    Sections,
    ResetShortcuts,
    PressNewKeysToChange,
    Change,
    PressOtherKeys,
    ThoseKeysAreUsedBy,
    PressTheNewKeysFor,
    Collapse,
    HideSection,
    DockLeft,
    DockRight,
    AppearanceAndStart,
    RemoveFromFavourites,
    AddToFavourites,
    ForgetThisLut,
    KeepThisLut,
    Favourites,
    TheFileIsMissing,
    EaseInAndOut,
    CustomRatio,
    UseThisRatio,
    Blur,
    N360Video3,
    ShowThisVideoThroughTheCamera,
    FadeBothSides,
    FadeOutOnly,
    FadeInOnly,
    FadeLength,
    Strength,
    CropPercent,
    CropLeft,
    CropTop,
    CropRight,
    CropBottom,
    ClipLevel,
    FadeInSeconds,
    FadeOutSeconds,
    Mute,
    CleanAudio,
    LutFileMissing,
    FindTheFile,
    ExportWithoutTheLut,
    EaseInAndOut2,
    NormaliseSpeech,
    NormaliseSpeechTip,
    VolumeKeys,
    RemoveThisKeyframe,
}

/// The English table, in the order of [`Key`].
pub const ENGLISH: &[(Key, &str)] = &[
    (
        Key::AddFadeAtTheBoundary,
        "Add fade at the boundary nearest the playhead",
    ),
    (Key::Style, "Style:"),
    (Key::Language, "Language:"),
    (
        Key::SubtitleGenerationIsNotAvailable,
        "Subtitle generation is not available in this build.",
    ),
    (
        Key::DownloadingTheSpeechRecognitionFile,
        "Downloading the speech recognition file…",
    ),
    (
        Key::MatchingSubtitlesToTheSpeech,
        "Matching subtitles to the speech",
    ),
    (
        Key::CtrlCmdClickALine,
        "Ctrl/Cmd+click a line to select several",
    ),
    (Key::ThisClipNoLongerExists, "This clip no longer exists."),
    (Key::ClipSound, "Clip sound"),
    (Key::CleanAudioIsOffFor, "Clean audio is off for this clip."),
    (Key::ThisFadeNoLongerExists, "This fade no longer exists."),
    (Key::ChooseTheFolder, "Choose the folder"),
    (Key::N360Video, "360 video"),
    (Key::BackToSameAsSource, "Back to \\\"Same as source\\\""),
    (
        Key::ChangeAspectIsNotApplied,
        "Change aspect is not applied.",
    ),
    (Key::Aspect, "Aspect"),
    (Key::Fit, "Fit"),
    (Key::EmptySpace, "Empty space"),
    (Key::Colour, "Colour"),
    (
        Key::OneCopyPerClipSelect,
        "One copy per clip; select the one to edit:",
    ),
    (
        Key::ClickAKeyframeToSelect,
        "Click a keyframe to select it; click elsewhere to move the playhead",
    ),
    (Key::KeyframeTime, "Keyframe time"),
    (Key::PasteKeyframesTo, "Paste keyframes to"),
    (Key::Scale, "Scale"),
    (Key::NoRecentProjects, "No recent projects"),
    (Key::ClearList, "Clear list"),
    (Key::OpenRecent, "Open recent"),
    (Key::MoreMenu, "⋯"),
    (Key::SetPOV, "Set POV"),
    (Key::SeekDragToMoveThe, "Seek: drag to move the playhead"),
    (
        Key::YouCanAlsoSetVIDEOLORE,
        "You can also set VIDEOLORE_FFMPEG / VIDEOLORE_FFPROBE.",
    ),
    (Key::ChooseFfmpeg, "Choose ffmpeg…"),
    (Key::Retry, "Retry"),
    (Key::Quit, "Quit"),
    (Key::OK, "OK"),
    (Key::Remove, "Remove"),
    (Key::Cancel, "Cancel"),
    (Key::SubtitlesToUse, "Subtitles to use:"),
    (Key::RecogniseSpeech, "Recognise speech"),
    (Key::Replace, "Replace"),
    (Key::Recover, "Recover"),
    (Key::Discard, "Discard"),
    (Key::Close, "Close"),
    (
        Key::RemoveFromApplicationMenu,
        "Remove from application menu",
    ),
    (Key::Import, "Import"),
    (Key::ClickAMarkerOnThe, "Click a marker on the timeline."),
    (
        Key::ThisPointOfViewNo,
        "This point of view no longer exists.",
    ),
    (Key::N360PointOfView, "360° point of view"),
    (Key::FadeINOUT, "Fade IN/OUT"),
    (
        Key::ThisSubtitleLineNoLonger,
        "This subtitle line no longer exists.",
    ),
    (Key::SubtitleLine, "Subtitle line"),
    (Key::Start, "Start"),
    (Key::End, "End"),
    (
        Key::BackToThePlaceOf,
        "Back to the place of the subtitle style",
    ),
    (
        Key::TheLineIsAtThe,
        "The line is at the place of the subtitle style",
    ),
    (
        Key::SelectAClipOnThe,
        "Select a clip on the timeline (double-click opens this window).",
    ),
    (Key::VideoClip, "Video clip"),
    (Key::N360Video2, "360° video"),
    (Key::TrimIn, "Trim in:"),
    (Key::AtPlayhead, "at playhead"),
    (Key::TrimOut, "Trim out:"),
    (Key::MergeWithNext, "Merge with next"),
    (Key::MusicClip, "Music clip"),
    (Key::Start2, "Start:"),
    (Key::End2, "End:"),
    (Key::CloseMark, "✕"),
    (
        Key::CloseTheSplashScreenEsc,
        "Close the splash screen (Esc)",
    ),
    (
        Key::GiveTheNewClipThe,
        "Give the new clip the same effect as the other clips",
    ),
    (Key::For, "for"),
    (Key::In, "in"),
    (Key::TextEnterStartsANew, "Text (Enter starts a new line)"),
    (Key::Effects, "Effects"),
    (Key::TimeToLive, "Time to live"),
    (Key::Font, "Font"),
    (Key::LoadingFonts, "Loading fonts…"),
    (Key::Readability, "Readability"),
    (Key::NewText, "New text"),
    (Key::ThisTextNoLongerExists, "This text no longer exists."),
    (Key::TextWord, "Text"),
    (Key::Delete, "Delete"),
    (
        Key::ZoomFromTheWholeProject,
        "Zoom: from the whole project to single frames",
    ),
    (Key::DragToTrimThisEnd, "Drag to trim this end of the clip"),
    (
        Key::ShiftDragSelectTheSubtitle,
        "Shift+drag: select the subtitle lines starting here",
    ),
    (Key::MoreArrow, "»"),
    (Key::MoreCommands, "More commands"),
    (Key::Expand, "Expand"),
    (Key::PreviewCopiesUpTo, "Keep preview copies up to"),
    (Key::Never, "Never"),
    (Key::OnceForEachVersion, "Once for each version"),
    (Key::AtEveryStart, "At every start"),
    (Key::StartPicture, "Picture at start"),
    (Key::TextSizeLarger, "Larger (150%)"),
    (Key::TextSizeLarge, "Large (125%)"),
    (Key::TextSizeNormal, "Normal"),
    (Key::TextSize, "Text size"),
    (Key::ResetToolbar, "Reset toolbar"),
    (Key::IconsAndText, "Icons and text"),
    (Key::CustomiseToolbar, "Customise toolbar"),
    (Key::ThemeDark, "Dark"),
    (Key::ThemeLight, "Light"),
    (Key::ThemeSystem, "System"),
    (Key::Theme, "Theme"),
    (Key::SaveTheCurrentLayout, "Save the current layout"),
    (Key::WorkspaceName, "Workspace name"),
    (Key::DeleteThisWorkspace, "Delete this workspace"),
    (Key::Workspaces, "Workspaces"),
    (Key::Sections, "Sections"),
    (Key::ResetShortcuts, "Reset shortcuts"),
    (
        Key::PressNewKeysToChange,
        "Press the new keys for this command",
    ),
    (Key::Change, "Change"),
    (Key::PressOtherKeys, "Press other keys, or cancel."),
    (Key::ThoseKeysAreUsedBy, "Those keys are already used by"),
    (Key::PressTheNewKeysFor, "Press the new keys for"),
    (Key::Collapse, "Collapse"),
    (Key::HideSection, "Hide this section"),
    (Key::DockLeft, "Dock on the left"),
    (Key::DockRight, "Dock on the right"),
    (Key::AppearanceAndStart, "Appearance and start-up"),
    (Key::RemoveFromFavourites, "Remove from favourites"),
    (Key::AddToFavourites, "Add to favourites"),
    (Key::ForgetThisLut, "Forget this LUT file"),
    (Key::KeepThisLut, "Keep this LUT file in the list below"),
    (Key::Favourites, "Favourites"),
    (Key::TheFileIsMissing, "missing"),
    (Key::EaseInAndOut, "Ease in and out (slow at both ends)"),
    (Key::CustomRatio, "Custom ratio"),
    (Key::UseThisRatio, "Use this ratio"),
    (Key::Blur, "Blur"),
    (Key::N360Video3, "360° video"),
    (
        Key::ShowThisVideoThroughTheCamera,
        "Show this video through the 360° camera",
    ),
    (Key::FadeBothSides, "Both clips"),
    (Key::FadeOutOnly, "Only the clip before fades out"),
    (Key::FadeInOnly, "Only the clip after fades in"),
    (Key::FadeLength, "Fade length"),
    (Key::Strength, "Strength"),
    (Key::CropPercent, "Crop (%)"),
    (Key::CropLeft, "left"),
    (Key::CropTop, "top"),
    (Key::CropRight, "right"),
    (Key::CropBottom, "bottom"),
    (Key::ClipLevel, "Level"),
    (Key::FadeInSeconds, "Fade in"),
    (Key::FadeOutSeconds, "Fade out"),
    (Key::Mute, "Mute"),
    (Key::CleanAudio, "Clean audio"),
    (Key::LutFileMissing, "A LUT file is missing"),
    (Key::FindTheFile, "Find the file…"),
    (Key::ExportWithoutTheLut, "Export without the LUT"),
    (Key::EaseInAndOut2, "Ease in and out"),
    (Key::NormaliseSpeech, "Normalise speech"),
    (
        Key::NormaliseSpeechTip,
        "Makes quiet words as clear as loud ones",
    ),
    (Key::VolumeKeys, "Volume keyframes"),
    (Key::RemoveThisKeyframe, "Remove this keyframe"),
];

/// The text of `key`.
pub fn get(key: Key) -> &'static str {
    ENGLISH[key as usize].1
}

/// `tr!(Name)`: the text of `Key::Name`.
macro_rules! tr {
    ($name:ident) => {
        $crate::ui::strings::get($crate::ui::strings::Key::$name)
    };
}
pub(crate) use tr;

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-245
    #[test]
    fn every_key_has_an_english_text_in_order() {
        for (i, (k, text)) in ENGLISH.iter().enumerate() {
            assert_eq!(*k as usize, i, "{k:?} is out of order");
            assert!(!text.is_empty(), "{k:?} has no text");
        }
        assert_eq!(get(Key::Collapse), "Collapse");
    }

    /// No file of `src/ui/` writes a text directly in a label, button, menu or tooltip
    /// call: the texts come from the table.
    // covers: REQ-245
    #[test]
    fn no_interface_text_is_written_outside_the_table() {
        let call = [
            ".label(",
            ".button(",
            ".small_button(",
            ".heading(",
            ".strong(",
            ".menu_button(",
            ".on_hover_text(",
            ".on_disabled_hover_text(",
            "Button::new(",
            "RichText::new(",
        ];
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut found = Vec::new();
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.ends_with(".rs") || name.ends_with("tests.rs") || name == "strings.rs" {
                continue;
            }
            let src = std::fs::read_to_string(e.path()).unwrap();
            // the code before the unit tests of the file
            let code = src.split("#[cfg(test)]").next().unwrap_or("");
            for (n, line) in code.lines().enumerate() {
                for c in call {
                    if let Some(i) = line.find(c)
                        && line[i + c.len()..].trim_start().starts_with('"')
                    {
                        found.push(format!("{name}:{}: {}", n + 1, line.trim()));
                    }
                }
            }
        }
        assert!(
            found.is_empty(),
            "texts outside the table:\n{}",
            found.join("\n")
        );
    }
}
