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
        export::choice::Context {
            has_clips: !videos.is_empty(),
            has_subtitles: self.has_shown_subtitles(),
            all_360: !videos.is_empty() && videos.iter().all(|v| *v),
            encoders: self.encoders(),
            auto_extension: self.export_extension(),
            project_file: self.project_path.clone(),
            media_files,
        }
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
                crate::format::FormatChoice { video: None, audio },
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
        let info = self.ffmpeg.as_ref().map_err(|e| e.clone())?;
        export::validate_output(&self.project, &path).map_err(|e| e.to_string())?;
        if mode == render::ExportMode::Sphere360 && self.has_flat_clip() {
            return Err(render::RenderError::MixedSphere.to_string());
        }
        let job = ExportJob {
            project: self.project.clone(),
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
        self.status = "Exporting…".into();
        Ok(())
    }
}
