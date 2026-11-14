// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Speech-to-text subtitles, fully offline once the model is installed.
//!
//! For each file used on the timeline that has sound, ffmpeg extracts the audio as 16 kHz
//! mono float (one background ffmpeg slot, released when extraction ends). The audio is then
//! read in 120 s chunks (memory stays bounded) and given to a [`Transcriber`], which returns
//! timed words. The language is detected on the first chunk unless the user chose it.
//! Words are grouped into lines at pauses and length limits, in source time.
//!
//! The speech model (whisper.cpp `ggml-tiny-q5_1.bin`, MIT) is downloaded once, only after
//! the user's consent, from a pinned URL and checked by size and SHA-256 before use; a local
//! copy of the same file can be used instead. This download is the only network access of
//! the application. The real engine ([`whisper`]) and downloader ([`download`]) are compiled
//! only with the `stt` feature; everything else is tested with fakes.

use crate::ffmpeg::{Runner, argv, file_arg};
use crate::media::MediaId;
use crate::subtitles::{NewLine, Word};
use crate::time::Micros;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

pub const MODEL_FILE: &str = "ggml-tiny-q5_1.bin";
pub const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-tiny-q5_1.bin";
pub const MODEL_SIZE: u64 = 32_152_673;
pub const MODEL_SHA256: &str = "818710568da3ca15689e31a743197b520007872ff9576237bda97bd1b469c3d7";
pub const MODEL_LICENCE: &str = "MIT";
pub const MODEL_SOURCE: &str = "https://huggingface.co/ggerganov/whisper.cpp";

/// Sample rate the speech model expects.
pub const SAMPLE_RATE: u32 = 16_000;
/// Audio given to the model at once.
pub const CHUNK: Micros = Micros(120_000_000);
/// A pause longer than this starts a new line.
pub const LINE_PAUSE: Micros = Micros(600_000);
/// Longest line, in words and characters.
pub const MAX_LINE_WORDS: usize = 7;
pub const MAX_LINE_CHARS: usize = 42;

#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum SttError {
    #[error("subtitle generation is not available in this build of videolore")]
    NotAvailable,
    #[error("the speech model is not usable: {0}")]
    Model(String),
    #[error("the speech model could not be downloaded: {0}")]
    Download(String),
    #[error("could not read the audio of '{0}'")]
    Extraction(String),
    #[error("speech recognition failed: {0}")]
    Transcription(String),
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Io(String),
}

fn io_err(e: io::Error) -> SttError {
    SttError::Io(e.to_string())
}

/// What the model heard in a chunk. Word times include the chunk offset.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Transcript {
    pub language: Option<String>,
    pub words: Vec<Word>,
}

/// A speech recogniser.
pub trait Transcriber: Send {
    /// Recognises `pcm` (16 kHz mono) that starts at source time `offset`. `language` is
    /// `None` to detect it.
    fn transcribe(
        &mut self,
        pcm: &[f32],
        offset: Micros,
        language: Option<&str>,
        cancel: &Arc<AtomicBool>,
    ) -> Result<Transcript, SttError>;
}

/// Downloads a URL into `out`, reporting bytes received.
pub trait ModelFetcher: Send + Sync {
    fn fetch(
        &self,
        url: &str,
        out: &mut dyn Write,
        progress: &mut dyn FnMut(u64),
        cancel: &AtomicBool,
    ) -> Result<(), SttError>;
}

/// Loads a transcriber for a model file with `threads` threads.
pub type TranscriberFactory =
    Arc<dyn Fn(&Path, usize) -> Result<Box<dyn Transcriber>, SttError> + Send + Sync>;

/// True when this build can generate subtitles.
pub fn available() -> bool {
    cfg!(feature = "stt")
}

/// The real transcriber (whisper.cpp), or an error in builds without `stt`.
pub fn default_factory() -> TranscriberFactory {
    Arc::new(|_model: &Path, _threads: usize| {
        #[cfg(feature = "stt")]
        {
            whisper::WhisperTranscriber::load(_model, _threads)
                .map(|t| Box::new(t) as Box<dyn Transcriber>)
        }
        #[cfg(not(feature = "stt"))]
        {
            Err(SttError::NotAvailable)
        }
    })
}

/// The real downloader (HTTPS only), when built with `stt`.
pub fn default_fetcher() -> Option<Arc<dyn ModelFetcher>> {
    #[cfg(feature = "stt")]
    {
        Some(Arc::new(download::UreqFetcher))
    }
    #[cfg(not(feature = "stt"))]
    {
        None
    }
}

/// Lower-case hex SHA-256 of a file.
pub fn sha256_file(path: &Path) -> io::Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Where the speech model lives (`<user data>/models`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelStore {
    pub dir: PathBuf,
    /// Expected size and digest (the pinned model; changeable for tests).
    pub size: u64,
    pub sha256: String,
}

impl ModelStore {
    pub fn new(dir: PathBuf) -> ModelStore {
        ModelStore {
            dir,
            size: MODEL_SIZE,
            sha256: MODEL_SHA256.into(),
        }
    }

    pub fn default_dir() -> Option<PathBuf> {
        directories::ProjectDirs::from("", "", "videolore").map(|d| d.data_dir().join("models"))
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(MODEL_FILE)
    }

    /// The model file exists with the right size (the digest is checked before use).
    pub fn is_installed(&self) -> bool {
        std::fs::metadata(self.path()).is_ok_and(|m| m.is_file() && m.len() == self.size)
    }

    /// Checks size and SHA-256 of `file`.
    pub fn verify(&self, file: &Path) -> Result<(), SttError> {
        let len = std::fs::metadata(file).map_err(io_err)?.len();
        if len != self.size {
            return Err(SttError::Model(format!(
                "expected {} bytes, found {len}",
                self.size
            )));
        }
        if sha256_file(file).map_err(io_err)? != self.sha256 {
            return Err(SttError::Model("the file's checksum does not match".into()));
        }
        Ok(())
    }

    fn part(&self) -> PathBuf {
        self.dir.join(format!("{MODEL_FILE}.part"))
    }

    fn finish(&self, part: &Path) -> Result<PathBuf, SttError> {
        if let Err(e) = self.verify(part) {
            let _ = std::fs::remove_file(part);
            return Err(e);
        }
        std::fs::rename(part, self.path()).map_err(io_err)?;
        Ok(self.path())
    }

    /// Downloads the pinned model (to `.part`, checked, then renamed).
    pub fn install_download(
        &self,
        fetcher: &dyn ModelFetcher,
        cancel: &AtomicBool,
        progress: &mut dyn FnMut(f32),
    ) -> Result<PathBuf, SttError> {
        std::fs::create_dir_all(&self.dir).map_err(io_err)?;
        let part = self.part();
        let mut f = std::fs::File::create(&part).map_err(io_err)?;
        let size = self.size as f32;
        let r = fetcher.fetch(
            MODEL_URL,
            &mut f,
            &mut |n| progress((n as f32 / size).min(1.0)),
            cancel,
        );
        drop(f);
        if let Err(e) = r {
            let _ = std::fs::remove_file(&part);
            return Err(e);
        }
        self.finish(&part).map_err(|e| match e {
            SttError::Model(m) => SttError::Download(m),
            e => e,
        })
    }

    /// Copies a model file chosen by the user (it must be the pinned model).
    pub fn install_local(&self, file: &Path) -> Result<PathBuf, SttError> {
        self.verify(file)?;
        std::fs::create_dir_all(&self.dir).map_err(io_err)?;
        let part = self.part();
        std::fs::copy(file, &part).map_err(io_err)?;
        self.finish(&part)
    }
}

/// ffmpeg arguments extracting the first audio stream as 16 kHz mono f32 (video sound only:
/// the music track is never part of it).
pub fn extraction_args(source: &Path, out: &Path) -> Vec<OsString> {
    let mut a = argv([
        "-hide_banner",
        "-nostdin",
        "-loglevel",
        "error",
        "-y",
        "-protocol_whitelist",
        "file,pipe",
        "-i",
    ]);
    a.push(file_arg(source));
    a.extend(argv([
        "-map", "0:a:0", "-vn", "-ac", "1", "-ar", "16000", "-f", "f32le",
    ]));
    a.push(file_arg(out));
    a
}

/// Groups timed words into lines: a new line after a pause, or when the line is full.
pub fn words_to_lines(media: MediaId, words: &[Word]) -> Vec<NewLine> {
    let mut lines: Vec<NewLine> = Vec::new();
    for w in words.iter().filter(|w| !w.text.trim().is_empty()) {
        let text = w.text.trim();
        let start_new = match lines.last() {
            None => true,
            Some(l) => {
                w.start - l.end > LINE_PAUSE
                    || l.words.len() >= MAX_LINE_WORDS
                    || l.text.chars().count() + 1 + text.chars().count() > MAX_LINE_CHARS
            }
        };
        let word = Word {
            start: w.start,
            end: w.end.max(w.start + Micros(10_000)),
            text: text.to_string(),
        };
        if start_new {
            lines.push(NewLine {
                media,
                start: word.start,
                end: word.end,
                text: word.text.clone(),
                words: vec![word],
            });
        } else if let Some(l) = lines.last_mut() {
            l.end = l.end.max(word.end);
            l.text.push(' ');
            l.text.push_str(&word.text);
            l.words.push(word);
        }
    }
    lines
}

/// One subtitle generation.
#[derive(Debug, Clone)]
pub struct SttJob {
    /// Files used on the timeline that have sound.
    pub sources: Vec<(MediaId, PathBuf)>,
    /// Chosen language, or `None` to detect it.
    pub language: Option<String>,
    pub ffmpeg: PathBuf,
    pub model: PathBuf,
    pub threads: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SttEvent {
    Progress(f32),
    Done {
        language: Option<String>,
        lines: Vec<NewLine>,
    },
    Failed(String),
    Cancelled,
}

/// Result of a generation: the language and the lines.
pub type SttOutcome = (Option<String>, Vec<NewLine>);

/// Runs a generation (synchronous, testable).
pub fn run(
    job: &SttJob,
    runner: &dyn Runner,
    transcriber: &mut dyn Transcriber,
    cancel: &Arc<AtomicBool>,
    tx: &Sender<SttEvent>,
) -> Result<SttOutcome, SttError> {
    let tmp = tempfile::Builder::new()
        .prefix("videolore-stt-")
        .tempdir()
        .map_err(io_err)?;
    let audio = tmp.path().join("audio.f32");
    let mut language = job.language.clone();
    let mut lines = Vec::new();
    let n = job.sources.len().max(1) as f32;
    let chunk_bytes = (CHUNK.0 as u64 * u64::from(SAMPLE_RATE) / 1_000_000 * 4) as usize;
    for (i, (media, source)) in job.sources.iter().enumerate() {
        if cancel.load(Ordering::SeqCst) {
            return Err(SttError::Cancelled);
        }
        let name = crate::media::display_name(source);
        let out = runner
            .run(&job.ffmpeg, &extraction_args(source, &audio))
            .map_err(|e| match e.kind() {
                io::ErrorKind::Interrupted => SttError::Cancelled,
                _ => SttError::Extraction(name.clone()),
            })?;
        if !out.success {
            return Err(SttError::Extraction(name));
        }
        let total = std::fs::metadata(&audio).map_err(io_err)?.len() as usize;
        let chunks = total.div_ceil(chunk_bytes).max(1);
        let mut f = std::fs::File::open(&audio).map_err(io_err)?;
        let mut words = Vec::new();
        let mut buf = vec![0u8; chunk_bytes];
        for c in 0..chunks {
            if cancel.load(Ordering::SeqCst) {
                return Err(SttError::Cancelled);
            }
            let mut got = 0;
            while got < buf.len() {
                let k = f.read(&mut buf[got..]).map_err(io_err)?;
                if k == 0 {
                    break;
                }
                got += k;
            }
            let (samples, _) = buf[..got - got % 4].as_chunks::<4>();
            let pcm: Vec<f32> = samples.iter().map(|b| f32::from_le_bytes(*b)).collect();
            let offset = Micros(CHUNK.0 * c as i64);
            let t = transcriber.transcribe(&pcm, offset, language.as_deref(), cancel)?;
            if language.is_none() {
                language = t.language;
            }
            words.extend(t.words);
            let _ = tx.send(SttEvent::Progress(
                (i as f32 + (c + 1) as f32 / chunks as f32) / n,
            ));
        }
        let _ = std::fs::remove_file(&audio);
        lines.extend(words_to_lines(*media, &words));
    }
    Ok((language, lines))
}

/// A running generation.
pub struct SttHandle {
    pub rx: Receiver<SttEvent>,
    cancel: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl SttHandle {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// Waits for the end (tests).
    pub fn join(mut self) -> Vec<SttEvent> {
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        self.rx.try_iter().collect()
    }
}

impl Drop for SttHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

/// Starts a generation in a thread. Only one runs at a time (the caller enforces it).
pub fn start(job: SttJob, runner: crate::jobs::Limited, factory: TranscriberFactory) -> SttHandle {
    let (tx, rx) = channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&cancel);
    let thread = std::thread::spawn(move || {
        let runner = runner.cancellable(Arc::clone(&flag));
        let result = factory(&job.model, job.threads)
            .and_then(|mut t| run(&job, &runner, t.as_mut(), &flag, &tx));
        let ev = match result {
            Ok((language, lines)) => SttEvent::Done { language, lines },
            Err(SttError::Cancelled) => SttEvent::Cancelled,
            Err(e) => SttEvent::Failed(e.to_string()),
        };
        let _ = tx.send(ev);
    });
    SttHandle {
        rx,
        cancel,
        thread: Some(thread),
    }
}

/// Model installation running in a thread.
#[derive(Debug, Clone, PartialEq)]
pub enum ModelEvent {
    Progress(f32),
    Installed(PathBuf),
    Failed(String),
}

/// How the model is installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelSource {
    Download,
    LocalFile(PathBuf),
}

/// Installs the model in a thread; events arrive on the returned channel.
pub fn install_model(
    store: ModelStore,
    source: ModelSource,
    fetcher: Option<Arc<dyn ModelFetcher>>,
    cancel: Arc<AtomicBool>,
) -> Receiver<ModelEvent> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let r = match source {
            ModelSource::Download => match fetcher {
                Some(f) => {
                    let ptx = tx.clone();
                    store.install_download(f.as_ref(), &cancel, &mut |p| {
                        let _ = ptx.send(ModelEvent::Progress(p));
                    })
                }
                None => Err(SttError::NotAvailable),
            },
            ModelSource::LocalFile(p) => store.install_local(&p),
        };
        let _ = tx.send(match r {
            Ok(p) => ModelEvent::Installed(p),
            Err(e) => ModelEvent::Failed(e.to_string()),
        });
    });
    rx
}

#[cfg(feature = "stt")]
pub mod whisper {
    //! whisper.cpp through whisper-rs (CPU only). Covered by the gated integration test
    //! `real_speech_is_transcribed` (needs `VIDEOLORE_STT_MODEL`).
    use super::*;
    use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

    pub struct WhisperTranscriber {
        ctx: WhisperContext,
        threads: usize,
    }

    impl WhisperTranscriber {
        pub fn load(model: &Path, threads: usize) -> Result<WhisperTranscriber, SttError> {
            whisper_rs::install_logging_hooks();
            let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default())
                .map_err(|e| SttError::Model(e.to_string()))?;
            Ok(WhisperTranscriber {
                ctx,
                threads: threads.max(1),
            })
        }
    }

    impl Transcriber for WhisperTranscriber {
        fn transcribe(
            &mut self,
            pcm: &[f32],
            offset: Micros,
            language: Option<&str>,
            cancel: &Arc<AtomicBool>,
        ) -> Result<Transcript, SttError> {
            let err = |e: whisper_rs::WhisperError| SttError::Transcription(e.to_string());
            let mut state = self.ctx.create_state().map_err(err)?;
            let lang = language.unwrap_or("auto").to_string();
            let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
            params.set_n_threads(self.threads as i32);
            params.set_language(Some(&lang));
            params.set_token_timestamps(true);
            params.set_split_on_word(true);
            params.set_max_len(1);
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);
            let flag = Arc::clone(cancel);
            params.set_abort_callback_safe(move || flag.load(Ordering::SeqCst));
            let r = state.full(params, pcm);
            if cancel.load(Ordering::SeqCst) {
                return Err(SttError::Cancelled);
            }
            r.map_err(err)?;
            let language =
                whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_string);
            let mut words = Vec::new();
            for seg in state.as_iter() {
                let text = seg.to_str_lossy().map_err(err)?.trim().to_string();
                if text.is_empty() || text.starts_with('[') || text.starts_with('(') {
                    continue;
                }
                // timestamps are in centiseconds
                words.push(Word {
                    start: offset + Micros(seg.start_timestamp() * 10_000),
                    end: offset + Micros(seg.end_timestamp() * 10_000),
                    text,
                });
            }
            Ok(Transcript { language, words })
        }
    }
}

#[cfg(feature = "stt")]
pub mod download {
    //! HTTPS download of the speech model with ureq (rustls). The only network code.
    use super::*;
    use std::time::Duration;

    pub struct UreqFetcher;

    impl ModelFetcher for UreqFetcher {
        fn fetch(
            &self,
            url: &str,
            out: &mut dyn Write,
            progress: &mut dyn FnMut(u64),
            cancel: &AtomicBool,
        ) -> Result<(), SttError> {
            let fail = |e: String| SttError::Download(e);
            if !url.starts_with("https://") {
                return Err(fail("only HTTPS downloads are allowed".into()));
            }
            let agent: ureq::Agent = ureq::Agent::config_builder()
                .https_only(true)
                .max_redirects(5)
                .timeout_connect(Some(Duration::from_secs(30)))
                .timeout_per_read(Some(Duration::from_secs(60)))
                .build()
                .into();
            let resp = agent.get(url).call().map_err(|e| fail(e.to_string()))?;
            let mut body = resp
                .into_body()
                .into_with_config()
                .limit(MODEL_SIZE + 1)
                .reader();
            let mut buf = vec![0u8; 1 << 16];
            let mut total = 0u64;
            loop {
                if cancel.load(Ordering::SeqCst) {
                    return Err(SttError::Cancelled);
                }
                let n = body.read(&mut buf).map_err(|e| fail(e.to_string()))?;
                if n == 0 {
                    return Ok(());
                }
                out.write_all(&buf[..n]).map_err(io_err)?;
                total += n as u64;
                progress(total);
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn refuses_plain_http_without_network() {
            let mut sink = Vec::new();
            let e = UreqFetcher
                .fetch(
                    "http://example.invalid/model.bin",
                    &mut sink,
                    &mut |_| {},
                    &AtomicBool::new(false),
                )
                .unwrap_err();
            assert!(matches!(e, SttError::Download(m) if m.contains("HTTPS")));
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ffmpeg::fake::{FakeRunner, fail};
    use crate::jobs::{Priority, Scheduler};

    /// What the fake transcriber was given: samples, offset, language.
    pub type Seen = Arc<std::sync::Mutex<Vec<(usize, Micros, Option<String>)>>>;

    /// Detects `it`, returns one word per second of audio, records the language it got.
    #[derive(Default)]
    pub struct FakeTranscriber {
        pub seen: Seen,
        pub cancel_after: Option<usize>,
    }

    impl Transcriber for FakeTranscriber {
        fn transcribe(
            &mut self,
            pcm: &[f32],
            offset: Micros,
            language: Option<&str>,
            cancel: &Arc<AtomicBool>,
        ) -> Result<Transcript, SttError> {
            let mut seen = self.seen.lock().unwrap();
            seen.push((pcm.len(), offset, language.map(str::to_string)));
            if self.cancel_after == Some(seen.len()) {
                cancel.store(true, Ordering::SeqCst);
            }
            let secs = pcm.len() as i64 / i64::from(SAMPLE_RATE);
            let words = (0..secs)
                .map(|i| Word {
                    start: offset + Micros(i * 1_000_000),
                    end: offset + Micros(i * 1_000_000 + 300_000),
                    text: format!("w{i}"),
                })
                .collect();
            Ok(Transcript {
                language: Some(language.unwrap_or("it").to_string()),
                words,
            })
        }
    }

    /// ffmpeg that "extracts" `secs` seconds of audio into its output file.
    pub fn extractor(secs: usize) -> FakeRunner {
        FakeRunner::default().on(move |_, a| {
            let out = a.last()?.strip_prefix("file:")?.to_string();
            std::fs::write(out, vec![0u8; secs * SAMPLE_RATE as usize * 4]).ok()?;
            crate::ffmpeg::fake::ok("")
        })
    }

    fn job(sources: usize) -> SttJob {
        SttJob {
            sources: (1..=sources as u64)
                .map(|i| (MediaId(i), PathBuf::from(format!("/m/{i}.mp4"))))
                .collect(),
            language: None,
            ffmpeg: "ffmpeg".into(),
            model: "/models/m.bin".into(),
            threads: 2,
        }
    }

    // covers: REQ-014, REQ-015
    #[test]
    fn chunks_progress_and_detected_language() {
        let runner = extractor(250);
        let mut t = FakeTranscriber::default();
        let seen = Arc::clone(&t.seen);
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let (lang, lines) = run(&job(1), &runner, &mut t, &cancel, &tx).unwrap();
        assert_eq!(lang.as_deref(), Some("it"));
        // 250 s in chunks of 120 s; the detected language is used after the first chunk
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            vec![
                (1_920_000, Micros::ZERO, None),
                (1_920_000, Micros(120_000_000), Some("it".into())),
                (160_000, Micros(240_000_000), Some("it".into())),
            ]
        );
        let progress: Vec<SttEvent> = rx.try_iter().collect();
        assert_eq!(progress.len(), 3);
        assert_eq!(progress.last(), Some(&SttEvent::Progress(1.0)));
        // words 1 s apart with 0.7 s pauses (> 0.6 s): one word per line, in source time
        assert_eq!(lines.len(), 250);
        assert_eq!(lines[121].start, Micros(121_000_000));
        assert!(lines.iter().all(|l| l.media == MediaId(1)));
        // the extraction reads only the video's own sound: one input, the source file
        let calls = runner.calls.lock().unwrap();
        let a = &calls[0];
        assert_eq!(a.iter().filter(|x| *x == "-i").count(), 1);
        assert!(a.contains(&"file:/m/1.mp4".to_string()));
        assert!(a.windows(2).any(|w| w == ["-map", "0:a:0"]));
        assert!(a.windows(2).any(|w| w == ["-ar", "16000"]));
    }

    // covers: REQ-015
    #[test]
    fn chosen_language_is_passed_to_the_model() {
        let runner = extractor(5);
        let mut t = FakeTranscriber::default();
        let seen = Arc::clone(&t.seen);
        let mut j = job(2);
        j.language = Some("en".into());
        let (tx, _rx) = channel();
        let (lang, lines) =
            run(&j, &runner, &mut t, &Arc::new(AtomicBool::new(false)), &tx).unwrap();
        assert_eq!(lang.as_deref(), Some("en"));
        assert!(
            seen.lock()
                .unwrap()
                .iter()
                .all(|s| s.2.as_deref() == Some("en"))
        );
        assert_eq!(lines.iter().filter(|l| l.media == MediaId(2)).count(), 5);
    }

    // covers: REQ-014
    #[test]
    fn cancel_and_errors() {
        let (tx, _rx) = channel();
        let mut t = FakeTranscriber {
            cancel_after: Some(1),
            ..Default::default()
        };
        let cancel = Arc::new(AtomicBool::new(false));
        assert_eq!(
            run(&job(2), &extractor(1), &mut t, &cancel, &tx),
            Err(SttError::Cancelled)
        );
        let broken = FakeRunner::default().on(|_, _| fail("no audio"));
        assert_eq!(
            run(
                &job(1),
                &broken,
                &mut t,
                &Arc::new(AtomicBool::new(false)),
                &tx
            ),
            Err(SttError::Extraction("1.mp4".into()))
        );
        // through the thread: cancel while waiting for a slot
        let sched = Scheduler::new(1);
        let hold = sched.acquire(Priority::Preview, 1, None, None).unwrap();
        let factory: TranscriberFactory =
            Arc::new(|_, _| Ok(Box::new(FakeTranscriber::default()) as Box<dyn Transcriber>));
        let h = start(
            job(1),
            sched.runner(Priority::Background, Arc::new(extractor(1))),
            Arc::clone(&factory),
        );
        std::thread::sleep(std::time::Duration::from_millis(30));
        h.cancel();
        assert_eq!(h.join().last(), Some(&SttEvent::Cancelled));
        drop(hold);
        let h = start(
            job(1),
            sched.runner(Priority::Background, Arc::new(extractor(3))),
            factory,
        );
        let ev = h.join();
        assert!(matches!(ev.last(), Some(SttEvent::Done { lines, .. }) if lines.len() == 3));
        let none: TranscriberFactory = Arc::new(|_, _| Err(SttError::NotAvailable));
        let h = start(
            job(1),
            sched.runner(Priority::Background, Arc::new(extractor(1))),
            none,
        );
        assert!(
            matches!(h.join().last(), Some(SttEvent::Failed(m)) if m.contains("not available"))
        );
    }

    #[test]
    fn words_are_grouped_into_lines() {
        let w = |a: i64, b: i64, t: &str| Word {
            start: Micros(a * 1000),
            end: Micros(b * 1000),
            text: t.into(),
        };
        let words = vec![
            w(0, 200, " Hello"),
            w(300, 500, "world,"),
            w(1500, 1700, "next"),
            w(1700, 1700, "  "),
            w(1800, 1800, "line"),
        ];
        let lines = words_to_lines(MediaId(4), &words);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].text, "Hello world,");
        assert_eq!((lines[0].start, lines[0].end), (Micros(0), Micros(500_000)));
        assert_eq!(lines[1].text, "next line");
        assert_eq!(lines[1].words[1].end, Micros(1_810_000));
        let many: Vec<Word> = (0..10).map(|i| w(i * 100, i * 100 + 50, "w")).collect();
        let lines = words_to_lines(MediaId(1), &many);
        assert_eq!(lines[0].words.len(), MAX_LINE_WORDS);
        let long: Vec<Word> = (0..3)
            .map(|i| w(i * 100, i * 100 + 50, &"x".repeat(25)))
            .collect();
        assert_eq!(words_to_lines(MediaId(1), &long).len(), 3);
    }

    /// Serves a fixed body.
    pub struct FakeFetcher(pub Vec<u8>);

    impl ModelFetcher for FakeFetcher {
        fn fetch(
            &self,
            url: &str,
            out: &mut dyn Write,
            progress: &mut dyn FnMut(u64),
            cancel: &AtomicBool,
        ) -> Result<(), SttError> {
            assert_eq!(url, MODEL_URL);
            if cancel.load(Ordering::SeqCst) {
                return Err(SttError::Cancelled);
            }
            for c in self.0.chunks(4) {
                out.write_all(c).map_err(io_err)?;
                progress(c.len() as u64);
            }
            Ok(())
        }
    }

    /// A store expecting `body` as the model.
    pub fn store_for(dir: &Path, body: &[u8]) -> ModelStore {
        let mut s = ModelStore::new(dir.join("models"));
        s.size = body.len() as u64;
        s.sha256 = Sha256::digest(body)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        s
    }

    // covers: REQ-014
    #[test]
    fn model_is_checked_by_size_and_checksum() {
        let dir = tempfile::tempdir().unwrap();
        let body = b"fake model bytes".to_vec();
        let store = store_for(dir.path(), &body);
        assert!(!store.is_installed());
        // a download with the wrong content is rejected and removed
        let mut wrong = body.clone();
        wrong[0] ^= 1;
        let e = store
            .install_download(&FakeFetcher(wrong), &AtomicBool::new(false), &mut |_| {})
            .unwrap_err();
        assert!(matches!(e, SttError::Download(m) if m.contains("checksum")));
        assert!(!store.path().exists());
        assert!(!store.dir.join(format!("{MODEL_FILE}.part")).exists());
        let e = store
            .install_download(
                &FakeFetcher(vec![1, 2]),
                &AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap_err();
        assert!(matches!(e, SttError::Download(m) if m.contains("bytes")));
        assert_eq!(
            store.install_download(
                &FakeFetcher(body.clone()),
                &AtomicBool::new(true),
                &mut |_| {}
            ),
            Err(SttError::Cancelled)
        );
        // the right one is installed
        let mut seen = Vec::new();
        let p = store
            .install_download(
                &FakeFetcher(body.clone()),
                &AtomicBool::new(false),
                &mut |f| seen.push(f),
            )
            .unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), body);
        assert!(store.is_installed());
        assert!(!seen.is_empty());
        // a local file must be the same model
        let other = tempfile::tempdir().unwrap();
        let store2 = store_for(other.path(), &body);
        let bad = other.path().join("bad.bin");
        std::fs::write(&bad, b"something else entirely").unwrap();
        assert!(matches!(
            store2.install_local(&bad),
            Err(SttError::Model(_))
        ));
        let good = other.path().join("good.bin");
        std::fs::write(&good, &body).unwrap();
        assert_eq!(store2.install_local(&good).unwrap(), store2.path());
        assert!(store2.is_installed());
        assert!(matches!(
            store2.verify(&other.path().join("nope")),
            Err(SttError::Io(_))
        ));
        // through the installer thread
        let third = tempfile::tempdir().unwrap();
        let rx = install_model(
            store_for(third.path(), &body),
            ModelSource::Download,
            Some(Arc::new(FakeFetcher(body.clone()))),
            Arc::new(AtomicBool::new(false)),
        );
        let events: Vec<ModelEvent> = rx.iter().collect();
        assert!(matches!(events.last(), Some(ModelEvent::Installed(_))));
        let rx = install_model(
            store_for(third.path(), &body),
            ModelSource::LocalFile(bad),
            None,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(matches!(rx.iter().last(), Some(ModelEvent::Failed(_))));
        let rx = install_model(
            store_for(third.path(), &body),
            ModelSource::Download,
            None,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(
            matches!(rx.iter().last(), Some(ModelEvent::Failed(m)) if m.contains("not available"))
        );
        // the pinned model
        let real = ModelStore::new(PathBuf::from("/x"));
        assert_eq!(real.size, MODEL_SIZE);
        assert_eq!(real.sha256.len(), 64);
        assert!(MODEL_URL.starts_with("https://"));
        assert!(ModelStore::default_dir().is_some() || cfg!(not(unix)));
        assert_eq!(available(), cfg!(feature = "stt"));
        assert_eq!(default_fetcher().is_some(), cfg!(feature = "stt"));
    }

    #[test]
    fn default_factory_rejects_a_bad_model() {
        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.bin");
        std::fs::write(&bad, b"not a model").unwrap();
        assert!(default_factory()(&bad, 1).is_err());
    }
}
