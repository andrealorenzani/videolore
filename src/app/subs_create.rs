// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Creating subtitles for a clip or for the whole video (REQ-085, REQ-087, REQ-095): the
//! steps before the background job (wait for the subtitle-track check of older media,
//! confirm replacing hand-edited lines, ask whether to use the video's own subtitles, ask
//! for the speech file), the job itself, and its result as one undo step.
//!
//! The request waits in [`AppState::pending_create`] while a dialog or a check is open;
//! every answer calls [`AppState::continue_create`] again, which goes as far as it can.

use super::*;
use crate::media::SubtitleTrack;
use crate::stt::{Mode, RangeJob};

/// Status while the subtitle tracks of older media are being checked.
pub const CHECKING_TRACKS: &str = "Checking the video for subtitles…";
/// Disabled reason while a creation runs (one at a time).
pub const BEING_CREATED: &str = "Subtitles are being created";

impl AppState {
    /// The parts of the sources a creation works on: the clip's source range, or every
    /// file of the timeline in full.
    pub fn create_ranges(&self, clip: Option<ClipId>) -> Vec<(MediaId, Option<(Micros, Micros)>)> {
        let tl = &self.project.timeline;
        match clip {
            Some(id) => tl
                .get(id)
                .map(|c| vec![(c.media, Some((c.src_in, c.src_out)))])
                .unwrap_or_default(),
            None => {
                let mut out: Vec<(MediaId, Option<(Micros, Micros)>)> = Vec::new();
                for c in &tl.clips {
                    if !out.iter().any(|(m, _)| *m == c.media) {
                        out.push((c.media, None));
                    }
                }
                out
            }
        }
    }

    /// The usable text subtitle tracks of a file (none when unknown or missing).
    fn tracks_of(&self, id: MediaId) -> &[SubtitleTrack] {
        self.project
            .media(id)
            .filter(|m| !m.missing)
            .and_then(|m| m.format.subtitle_tracks.as_deref())
            .unwrap_or(&[])
    }

    /// Whether this computer can match subtitle words to the speech.
    pub fn can_match_words(&self) -> bool {
        self.machine.speech_supported() && stt::available()
    }

    /// Starts creating subtitles for `clip` (`None`: the whole video).
    pub(super) fn create_subtitles(&mut self, clip: Option<ClipId>) -> Result<(), String> {
        if self.stt.is_some() || self.model_install.is_some() {
            return Err(BEING_CREATED.into());
        }
        if let Some(id) = clip
            && self.project.timeline.get(id).is_none()
        {
            return Err(EditError::NoSuchClip.to_string());
        }
        self.pending_create = Some(CreateRequest {
            clip,
            ..Default::default()
        });
        self.continue_create()
    }

    /// Goes on with the waiting creation as far as possible (until a dialog, a check or
    /// the background job).
    pub(super) fn continue_create(&mut self) -> Result<(), String> {
        let Some(mut req) = self.pending_create.clone() else {
            return Ok(());
        };
        let ranges = self.create_ranges(req.clip);
        if ranges.is_empty() {
            self.pending_create = None;
            return Err("the timeline has no sound to transcribe: add clips first".into());
        }
        // 1. older media: their subtitle tracks are checked first (M17)
        if ranges.iter().any(|(m, _)| self.tracks_checking.contains(m)) {
            self.status = CHECKING_TRACKS.into();
            return Ok(());
        }
        let with_tracks = ranges.iter().any(|(m, _)| !self.tracks_of(*m).is_empty());
        // 2. without subtitle tracks only speech recognition is possible
        if !with_tracks && let Some(stop) = self.recognition_blocked() {
            self.pending_create = None;
            return stop;
        }
        // 3. hand-edited lines would be replaced (REQ-044)
        if !req.confirmed && self.edits_in(&ranges) {
            self.modal = Some(Modal::ConfirmRegenerate);
            return Ok(());
        }
        // 4. the video already contains subtitles: use them or recognise the speech?
        if with_tracks && req.source.is_none() {
            let mut labels: Vec<String> = Vec::new();
            for (m, _) in &ranges {
                for t in self.tracks_of(*m) {
                    let l = t.label();
                    if !labels.contains(&l) {
                        labels.push(l);
                    }
                }
            }
            req.labels = labels.clone();
            self.pending_create = Some(req);
            self.modal = Some(Modal::SubtitleSource {
                labels,
                can_match: self.can_match_words(),
            });
            return Ok(());
        }
        let use_label = match req.source {
            Some(SourceChoice::Use(i)) => req.labels.get(i).cloned(),
            _ => None,
        };
        let spread = req.spread || !self.can_match_words();
        let mut jobs = Vec::new();
        for (media, range) in &ranges {
            let Some(m) = self.project.media(*media) else {
                continue;
            };
            if m.missing {
                continue;
            }
            let track = use_label
                .as_ref()
                .and_then(|l| pick_track(self.tracks_of(*media), l));
            let mode = match track {
                Some(t) if spread || !m.format.has_audio() => Mode::Spread { track: t.ordinal },
                Some(t) => Mode::Align { track: t.ordinal },
                None if m.format.has_audio() => Mode::Recognise,
                None => continue,
            };
            jobs.push(RangeJob {
                media: *media,
                path: m.path.clone(),
                range: *range,
                mode,
                track_language: track.and_then(|t| t.language.clone()),
            });
        }
        if jobs.is_empty() {
            self.pending_create = None;
            return Err(if req.clip.is_some() {
                "this clip has no sound to transcribe".into()
            } else {
                "the clips on the timeline have no sound to transcribe".into()
            });
        }
        let recognise = jobs.iter().any(|j| j.mode == Mode::Recognise);
        if recognise && let Some(stop) = self.recognition_blocked() {
            self.pending_create = None;
            return stop;
        }
        let ffmpeg = self.ffmpeg.as_ref().map_err(|e| e.clone())?.ffmpeg.clone();
        let needs_model = jobs.iter().any(|j| !matches!(j.mode, Mode::Spread { .. }));
        let mut model = PathBuf::new();
        let mut model_check = None;
        if needs_model {
            let Some(store) = self.model_store.clone() else {
                self.pending_create = None;
                return Err("no folder for speech recognition files".into());
            };
            let mut level = req.level.unwrap_or(self.settings.subtitle_quality);
            if settings::availability(&self.fingerprint())[level.index()].is_some() {
                level = SubtitleQuality::Fast;
            }
            if !store.is_installed(level) {
                self.pending_create = Some(req);
                self.modal = Some(Modal::ModelConsent(level));
                return Ok(());
            }
            model = store.path(level);
            model_check = Some(store.spec(level).clone());
        }
        let subs = &self.project.subtitles;
        let job = SttJob {
            ranges: jobs,
            language: subs.language.clone().filter(|_| subs.language_overridden),
            ffmpeg,
            model,
            threads: self.budget.stt_threads,
            work_root: self.work_root.clone(),
            model_check,
        };
        self.stt_matching = job.is_matching();
        self.creating_for = Some(req.clip);
        self.pending_create = None;
        self.stt = Some(stt::start(
            job,
            self.limited(Priority::Background),
            Arc::clone(&self.transcriber),
        ));
        self.stt_progress = Some(0.0);
        self.status = if self.stt_matching {
            "Matching subtitles to the speech…".into()
        } else {
            "Generating subtitles…".into()
        };
        Ok(())
    }

    /// Why speech recognition cannot run here: an error (build without it), or a notice
    /// for processors the engine cannot use (`Some(Ok)`: stop quietly).
    fn recognition_blocked(&mut self) -> Option<Result<(), String>> {
        if !stt::available() {
            return Some(Err(
                "subtitle generation is not available in this build".into()
            ));
        }
        // the speech engine is never reached on a processor it cannot run on
        if !self.machine.speech_supported() {
            self.notice = Some(TOO_OLD_FOR_SUBTITLES.into());
            return Some(Ok(()));
        }
        None
    }

    /// True when hand-edited lines would be replaced by a creation on `ranges`.
    fn edits_in(&self, ranges: &[(MediaId, Option<(Micros, Micros)>)]) -> bool {
        let subs = &self.project.subtitles;
        if ranges.iter().all(|(_, r)| r.is_none()) {
            return subs.has_edits();
        }
        ranges.iter().any(|(m, r)| match r {
            Some((a, b)) => subs.has_edits_in(*m, *a, *b),
            None => subs.lines.iter().any(|l| l.edited && l.media == *m),
        })
    }

    /// The answer to the subtitle source dialog.
    pub(super) fn choose_subtitle_source(&mut self, choice: SourceChoice) -> Result<(), String> {
        if !matches!(self.modal, Some(Modal::SubtitleSource { .. })) {
            return Ok(());
        }
        self.modal = None;
        if choice == SourceChoice::Cancel {
            self.pending_create = None;
            return Ok(());
        }
        if let Some(r) = &mut self.pending_create {
            r.source = Some(choice);
        }
        self.continue_create()
    }

    /// Starts checking the subtitle tracks of media analysed before 1.2.0.
    pub(super) fn start_track_check(&mut self) {
        let Ok(ffprobe) = self.ffmpeg.as_ref().map(|i| i.ffprobe.clone()) else {
            return;
        };
        let todo: Vec<(MediaId, PathBuf)> = self
            .project
            .media
            .iter()
            .filter(|m| {
                m.is_ready()
                    && !m.missing
                    && m.has_video()
                    && (m.format.subtitle_tracks.is_none() || m.format.spherical.is_none())
            })
            .map(|m| (m.id, m.path.clone()))
            .collect();
        if todo.is_empty() {
            return;
        }
        self.stop_track_check();
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let runner = self
            .limited(Priority::Probe)
            .cancellable(Arc::clone(&cancel));
        let flag = Arc::clone(&cancel);
        self.tracks_checking = todo.iter().map(|(id, _)| *id).collect();
        std::thread::spawn(move || {
            for (id, path) in todo {
                if flag.load(Ordering::SeqCst) {
                    return;
                }
                // a file that cannot be read now simply has no usable track
                let probed = media::probe(&runner, &ffprobe, &path, id).ok();
                let spherical = probed
                    .as_ref()
                    .is_some_and(|m| m.format.spherical == Some(true));
                let tracks = probed
                    .and_then(|m| m.format.subtitle_tracks)
                    .unwrap_or_default();
                if tx.send((id, tracks, spherical)).is_err() {
                    return;
                }
            }
        });
        self.track_check = Some((rx, cancel));
    }

    pub(super) fn stop_track_check(&mut self) {
        if let Some((_, c)) = self.track_check.take() {
            c.store(true, Ordering::SeqCst);
        }
        self.tracks_checking.clear();
    }

    /// Collects the track checks; the project is updated without becoming "unsaved".
    pub(super) fn tick_tracks(&mut self) {
        let Some((rx, _)) = &self.track_check else {
            return;
        };
        let results: Vec<TrackCheck> = rx.try_iter().collect();
        let finished = results.is_empty()
            && matches!(
                rx.try_recv(),
                Err(std::sync::mpsc::TryRecvError::Disconnected)
            );
        let mut newly_360 = Vec::new();
        for (id, tracks, spherical) in results {
            self.tracks_checking.remove(&id);
            if let Some(m) = self.project.media.iter_mut().find(|m| m.id == id) {
                if m.format.subtitle_tracks.is_none() {
                    m.format.subtitle_tracks = Some(tracks.clone());
                    self.saved.set_subtitle_tracks(id, tracks);
                }
                if m.format.spherical.is_none() {
                    m.format.spherical = Some(spherical);
                    self.saved.set_spherical(id, spherical);
                    if m.format.is_360() {
                        newly_360.push(id);
                    }
                }
            }
        }
        for id in newly_360 {
            // the camera view needs its own preview copy and picture
            self.redo_proxy(id);
            let now = Instant::now();
            self.refresh_still(now);
        }
        if finished {
            self.track_check = None;
            self.tracks_checking.clear();
        }
        if self.tracks_checking.is_empty() {
            self.track_check = None;
            if self.pending_create.is_some()
                && self.modal.is_none()
                && let Err(e) = self.continue_create()
            {
                self.status = e;
            }
        }
        self.refresh_dirty();
    }

    /// The end of a creation: one undo step with every new line.
    pub(super) fn finish_create(&mut self, r: stt::SttResult, now: Instant) {
        let clip = self.creating_for.take().flatten();
        let label = if clip.is_some() {
            "Create subtitles"
        } else {
            "Generate subtitles"
        };
        let n: usize = r.ranges.iter().map(|x| x.lines.len()).sum();
        let language = r
            .language
            .clone()
            .filter(|l| crate::media::valid_language(l));
        let all = r.lines();
        let parts = r.ranges.clone();
        let _ = self.edit(now, label, |e| {
            if clip.is_none() {
                e.subtitles.replace_all(all, language);
            } else {
                for part in parts {
                    if let Some((a, b)) = part.range {
                        e.subtitles
                            .replace_range(part.media, a, b, part.lines, language.clone());
                    }
                }
            }
            Ok(())
        });
        self.refresh_dirty();
        self.status = if r.unmatched.0 > 0 {
            stt::unmatched_message(r.unmatched.0, r.unmatched.1)
        } else {
            format!("Generated {n} subtitle line(s)")
        };
        if let Some(m) = r.fallback {
            self.notice = Some(m);
        }
    }
}

/// The track of a file for the label chosen in the dialog: the same label, else the same
/// language, else the first track.
pub fn pick_track<'a>(tracks: &'a [SubtitleTrack], label: &str) -> Option<&'a SubtitleTrack> {
    if tracks.is_empty() {
        return None;
    }
    if let Some(t) = tracks.iter().find(|t| t.label() == label) {
        return Some(t);
    }
    let language = |t: &SubtitleTrack| {
        t.language
            .as_deref()
            .and_then(stt::iso639_2_to_1)
            .map(str::to_string)
    };
    // a label starts with the language name (or code) of its track
    tracks
        .iter()
        .find(|t| {
            language(t)
                .and_then(|l| stt::language_name(&l))
                .is_some_and(|n| label.starts_with(n))
        })
        .or_else(|| tracks.first())
}
