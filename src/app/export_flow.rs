// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Export from the application: the Export dialog's context, the chosen format, the
//! output path and the start of the export job (REQ-118 to REQ-122).

use super::*;

impl AppState {
    pub(super) fn export_extension(&self) -> String {
        render::timeline_media(&self.project)
            .ok()
            .and_then(|m| {
                let f: Vec<_> = m.iter().map(|m| &m.format).collect();
                crate::format::choose_output(&f)
            })
            .map(|f| crate::format::encode_settings(&f, &self.encoders()).extension)
            .unwrap_or_else(|| "mp4".into())
    }

    /// What the Export dialog needs to know about the project (REQ-118 to REQ-122).
    pub fn export_context(&self) -> export::choice::Context {
        let videos: Vec<bool> = self
            .project
            .timeline
            .clips
            .iter()
            .map(|c| {
                self.project
                    .media(c.media)
                    .is_some_and(|m| m.format.is_360())
            })
            .collect();
        let mut media_files: Vec<PathBuf> =
            self.project.media.iter().map(|m| m.path.clone()).collect();
        media_files.sort();
        media_files.dedup();
        let tl = &self.project.timeline;
        let mut frame_effects_360: Vec<&'static str> = Vec::new();
        if tl.frame_aspect.is_some() {
            frame_effects_360.push("Change aspect");
        }
        if tl
            .clips
            .iter()
            .any(|c| !tl.scale_keys(FrameTarget::Clip(c.id)).is_empty())
        {
            frame_effects_360.push("Scale");
        }
        if tl
            .clips
            .iter()
            .any(|c| !tl.move_keys(FrameTarget::Clip(c.id)).is_empty())
        {
            frame_effects_360.push("Move");
        }
        export::choice::Context {
            frame_effects_360,
            has_clips: !videos.is_empty(),
            has_subtitles: self.has_shown_subtitles(),
            subtitles_as_track: self.project.subtitles.preset == Preset::AsTrack,
            all_360: !videos.is_empty() && videos.iter().all(|v| *v),
            encoders: self.encoders(),
            auto_extension: self.export_extension(),
            project_file: self.project_path.clone(),
            media_files,
        }
    }

    /// The frame of Change aspect is larger than 4096 px on its longer side before the cap
    /// (REQ-221): the Export dialog warns that it is scaled down and takes longer. `preset`
    /// is the video format chosen in the dialog (its size is the base of the frame).
    pub fn frame_over_cap(&self, preset: Option<usize>) -> bool {
        let aspect = self.project.timeline.frame_aspect.as_ref();
        if aspect.is_none() {
            return false;
        }
        let base = match preset.and_then(|i| crate::format::VIDEO_PRESETS.get(i)) {
            Some(p) => (p.width, p.height),
            None => match render::base_geometry(&self.project, &self.encoders()) {
                Ok(g) => (g.width, g.height),
                Err(_) => return false,
            },
        };
        crate::frame::over_cap(base, aspect)
    }

    /// Opens the Export dialog with the choices of the session so far (REQ-118).
    pub(super) fn open_export_dialog(&mut self) {
        let choice = match &self.export_memory {
            Some(c) => c.clone(),
            None => export::choice::defaults(
                self.project_path.as_deref(),
                self.last_export_folder.as_deref(),
                directories::UserDirs::new()
                    .and_then(|d| d.document_dir().map(Path::to_path_buf))
                    .as_deref(),
            ),
        };
        self.modal = Some(Modal::Export(Box::new(choice)));
    }

    /// "Export" pressed in the dialog: checks the choices, asks before replacing a file.
    pub(super) fn start_export_choice(
        &mut self,
        choice: export::choice::ExportChoice,
        now: Instant,
    ) -> Result<(), String> {
        let ctx = self.export_context();
        if let Some(why) = export::choice::blocked(&choice, &ctx) {
            return Err(why);
        }
        if !self.missing_luts().is_empty() && !self.export_without_missing_luts {
            self.modal = Some(Modal::MissingLut(Box::new(choice)));
            return Ok(());
        }
        if export::choice::target_path(&choice, &ctx).exists() {
            self.modal = Some(Modal::ConfirmOverwrite(Box::new(choice)));
            return Ok(());
        }
        self.run_export_choice(choice, now)
    }

    pub(super) fn run_export_choice(
        &mut self,
        choice: export::choice::ExportChoice,
        now: Instant,
    ) -> Result<(), String> {
        let ctx = self.export_context();
        let path = export::choice::target_path(&choice, &ctx);
        let request = export::choice::request(&choice, &ctx);
        self.modal = None;
        self.last_export_folder = Some(choice.folder.clone());
        self.export_memory = Some(choice);
        match request {
            export::choice::Request::Srt => self.dispatch_inner(Command::ExportSrt(path), now),
            export::choice::Request::Audio(audio) => self.start_export_with(
                path,
                render::ExportMode::Automatic,
                crate::format::FormatChoice {
                    video: None,
                    audio,
                    subtitles: Default::default(),
                },
                true,
            ),
            export::choice::Request::Video { mode, format } => {
                self.start_export_with(path, mode, format, false)
            }
        }
    }

    pub(super) fn start_export(
        &mut self,
        path: PathBuf,
        mode: render::ExportMode,
    ) -> Result<(), String> {
        self.start_export_with(path, mode, Default::default(), false)
    }

    pub(super) fn start_export_with(
        &mut self,
        path: PathBuf,
        mode: render::ExportMode,
        format: crate::format::FormatChoice,
        audio_only: bool,
    ) -> Result<(), String> {
        if self.is_exporting() {
            return Err("an export is already running".into());
        }
        if let Some(name) = self.waiting_import() {
            return Err(format!("'{name}' is still being rendered from its project"));
        }
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        export::validate_output(&self.project, &path).map_err(|e| e.to_string())?;
        if mode == render::ExportMode::Sphere360 && self.has_flat_clip() {
            return Err(render::RenderError::MixedSphere.to_string());
        }
        let mut project = self.project.clone();
        if std::mem::take(&mut self.export_without_missing_luts) {
            // the LUT files that are not there are left out of this export only (REQ-292)
            project
                .timeline
                .clip_luts
                .retain(|l| Path::new(&l.path).exists());
            self.status = "Exporting without the LUT that is missing…".into();
        }
        let job = ExportJob {
            project,
            output: path,
            ffmpeg: info.ffmpeg.clone(),
            ffprobe: info.ffprobe.clone(),
            version: info.version,
            encoders: info.encoders.clone(),
            can_burn_subtitles: info.has_subtitles(),
            threads: self.budget.threads,
            work_root: self.work_root.clone(),
            hw: self.hw_offer(),
            stall_timeout: self.stall_timeout,
            check_timeout: export::CHECK_TIMEOUT,
            mode,
            format,
            audio_only,
        };
        let runner: Arc<dyn Runner> = Arc::new(self.limited(Priority::Export));
        self.export = Some(export::start(job, runner));
        self.export_status = ExportStatus {
            running: true,
            progress: 0.0,
            stream_copy: None,
        };
        if !self.status.starts_with("Exporting without") {
            self.status = "Exporting…".into();
        }
        Ok(())
    }
}
