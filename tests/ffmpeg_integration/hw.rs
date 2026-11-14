// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Hardware export with the real ffmpeg. CI has no graphics hardware, so a test runner
//! simulates each vendor: it runs the real export command with the vendor's encoder
//! replaced by libx264/libx265 (vendor-only options and device setup removed, the upload
//! to the device replaced by `null`), then damages the file on request. Everything else
//! (the output check, the redo, the cleanup) is the real code.

use super::*;
use std::io::{Cursor, Read};
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use videolore::export::{ExportMethod, Problem};
use videolore::ffmpeg::{CmdOutput, Process, Runner};
use videolore::hwenc::{self, Candidate, Codec, HwOffer, Platform, ProbeLimits, Vendor};
use videolore::outcheck::CheckFailure;

/// What the simulated hardware does to its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Ok,
    Empty,
    Truncated,
    MissingFrames,
    WrongSize,
    Green,
    Garbage,
}

/// A finished process (the simulated hardware run is done synchronously).
struct Finished {
    stdout: Option<Vec<u8>>,
    out: Option<CmdOutput>,
}

impl Process for Finished {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stdout
            .take()
            .map(|s| Box::new(Cursor::new(s)) as Box<dyn Read + Send>)
    }
    fn kill(&mut self) {}
    fn wait(&mut self) -> std::io::Result<CmdOutput> {
        Ok(self.out.take().unwrap_or_default())
    }
}

/// Runs everything for real, except hardware encodes (see the module comment).
struct SimulatedHw {
    mode: Mode,
    /// Hardware runs seen.
    runs: AtomicUsize,
    /// The rewritten commands, for inspection.
    commands: Mutex<Vec<Vec<String>>>,
}

impl SimulatedHw {
    fn new(mode: Mode) -> SimulatedHw {
        SimulatedHw {
            mode,
            runs: AtomicUsize::new(0),
            commands: Mutex::new(Vec::new()),
        }
    }
}

/// The hardware encoder of an argv: its position and format.
fn hw_encoder(args: &[String]) -> Option<(Codec, Vendor)> {
    let i = args.iter().position(|a| a == "-c:v")?;
    let enc = args.get(i + 1)?;
    let (codec, suffix) = enc.split_once('_')?;
    let codec = Codec::from_name(codec)?;
    let vendor = Vendor::ALL.into_iter().find(|v| v.suffix() == suffix)?;
    Some((codec, vendor))
}

fn software_graph(g: &str) -> String {
    g.replace("hwupload", "null")
}

/// The software command that stands in for the hardware one.
fn rewrite(args: &[String], codec: Codec, vendor: Vendor) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        let skip_pair = a == "-init_hw_device"
            || a == "-filter_hw_device"
            || vendor.options().contains(&a.as_str());
        if skip_pair {
            i += 2;
            continue;
        }
        out.push(a.clone());
        let next = args.get(i + 1).cloned();
        match (a.as_str(), next) {
            ("-c:v", Some(_)) => {
                out.push(
                    match codec {
                        Codec::H264 => "libx264",
                        Codec::Hevc => "libx265",
                    }
                    .into(),
                );
                i += 1;
            }
            ("-pix_fmt", Some(p)) if p == "nv12" => {
                out.push("yuv420p".into());
                i += 1;
            }
            ("-filter_complex", Some(g)) => {
                out.push(software_graph(&g));
                i += 1;
            }
            ("-/filter_complex" | "-filter_complex_script", Some(path)) => {
                let g = std::fs::read_to_string(&path).unwrap();
                std::fs::write(&path, software_graph(&g)).unwrap();
                out.push(path);
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    if codec == Codec::Hevc {
        let at = out.iter().position(|a| a == "-c:v").unwrap() + 2;
        out.splice(at..at, ["-x265-params".into(), "log-level=error".into()]);
    }
    out
}

/// Damages the finished file at `out` (a re-encode keeps everything but the damage).
fn corrupt(ffmpeg: &Path, args: &[String], mode: Mode, codec: Codec) {
    let out = PathBuf::from(args.last().unwrap().strip_prefix("file:").unwrap());
    let len = std::fs::metadata(&out).unwrap().len();
    let filter = match mode {
        Mode::Ok => return,
        Mode::Empty => {
            std::fs::File::create(&out).unwrap();
            return;
        }
        Mode::Truncated => {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&out)
                .unwrap()
                .set_len(len / 2)
                .unwrap();
            return;
        }
        Mode::MissingFrames => "select='not(eq(mod(n,7),3))',setpts=N/FRAME_RATE/TB",
        Mode::WrongSize => "scale=iw-16:ih",
        Mode::Green => "geq=lum=0:cb=0:cr=0",
        Mode::Garbage => "noise=alls=100:allf=t+u",
    };
    let muxer = args[args.iter().rposition(|a| a == "-f").unwrap() + 1].clone();
    let tmp = out.with_extension("corrupt");
    let (enc, profile) = match codec {
        Codec::H264 => ("libx264", "high"),
        Codec::Hevc => ("libx265", "main"),
    };
    let mut c = Command::new(ffmpeg);
    c.args(["-hide_banner", "-loglevel", "error", "-y", "-i"])
        .arg(&out)
        .args(["-map", "0", "-c:a", "copy", "-vf", filter, "-c:v", enc])
        .args(["-profile:v", profile, "-pix_fmt", "yuv420p"]);
    if codec == Codec::Hevc {
        c.args(["-x265-params", "log-level=error", "-tag:v", "hvc1"]);
    }
    c.args(["-f", &muxer]).arg(&tmp);
    assert!(c.status().unwrap().success(), "corrupting with {filter}");
    std::fs::rename(&tmp, &out).unwrap();
}

impl Runner for SimulatedHw {
    fn run(&self, program: &Path, args: &[std::ffi::OsString]) -> std::io::Result<CmdOutput> {
        SystemRunner.run(program, args)
    }

    fn spawn_in(
        &self,
        program: &Path,
        args: &[std::ffi::OsString],
        cwd: Option<&Path>,
    ) -> std::io::Result<Box<dyn Process>> {
        let a: Vec<String> = args
            .iter()
            .map(|s| s.to_string_lossy().into_owned())
            .collect();
        let Some((codec, vendor)) = hw_encoder(&a) else {
            return SystemRunner.spawn_in(program, args, cwd);
        };
        self.runs.fetch_add(1, Ordering::SeqCst);
        let sw = rewrite(&a, codec, vendor);
        self.commands.lock().unwrap().push(sw.clone());
        let mut c = Command::new(program);
        c.args(&sw).stdin(Stdio::null());
        if let Some(d) = cwd {
            c.current_dir(d);
        }
        let o = c.output()?;
        if o.status.success() {
            corrupt(program, &sw, self.mode, codec);
        }
        Ok(Box::new(Finished {
            stdout: Some(o.stdout),
            out: Some(CmdOutput {
                success: o.status.success(),
                stdout: Vec::new(),
                stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
            }),
        }))
    }
}

/// The candidate of `vendor` (VA-API on a made-up render device: the simulation ignores it).
fn cand(vendor: Vendor) -> Candidate {
    Candidate {
        vendor,
        device: (vendor == Vendor::Vaapi).then(|| PathBuf::from("/dev/dri/renderD128")),
        rc: Default::default(),
    }
}

fn offer(codec: Codec, vendor: Vendor) -> HwOffer {
    let c = Some(cand(vendor));
    match codec {
        Codec::H264 => HwOffer {
            h264: c,
            hevc: None,
        },
        Codec::Hevc => HwOffer {
            h264: None,
            hevc: c,
        },
    }
}

/// What must be equal between a Faster and a Best quality file.
#[derive(Debug, PartialEq)]
struct Format {
    video: Vec<String>,
    audio: Vec<String>,
}

impl Format {
    fn video(&self, key: &str) -> &str {
        self.video
            .iter()
            .find_map(|l| l.strip_prefix(&format!("{key}=")))
            .unwrap_or("")
    }
}

impl Env {
    fn format_of(&self, file: &Path) -> Format {
        let entries = |stream: &str, keys: &str| -> Vec<String> {
            let o = Command::new(&self.info.ffprobe)
                .args(["-v", "error", "-count_packets", "-select_streams", stream])
                .args([
                    "-show_entries",
                    &format!("stream={keys}"),
                    "-of",
                    "default=nw=1",
                ])
                .arg(file)
                .output()
                .unwrap();
            assert!(o.status.success());
            let mut v: Vec<String> = String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_string)
                .collect();
            v.sort();
            v
        };
        Format {
            video: entries(
                "v:0",
                "codec_name,profile,width,height,r_frame_rate,pix_fmt,codec_tag_string,nb_read_packets",
            ),
            audio: entries("a:0", "codec_name,sample_rate,channels"),
        }
    }

    /// Exports `p` with `runner` and the hardware `hw` (a fresh output name each time).
    fn export_with(
        &self,
        p: &Project,
        name: &str,
        runner: &dyn Runner,
        hw: Option<HwOffer>,
    ) -> (PathBuf, ExportMethod, Vec<ExportEvent>) {
        let out = self.dir.path().join(name);
        let mut job = self.job(p, &out);
        job.hw = hw;
        let (tx, rx) = channel();
        let (path, method) = export::run(&job, runner, &AtomicBool::new(false), &tx)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        (path, method, rx.try_iter().collect())
    }

    /// Two HEVC sources (so the export is HEVC).
    fn hevc_clip(&self, name: &str) -> PathBuf {
        self.fixture(
            name,
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=320x180:rate=25:duration=2",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000:duration=2",
                "-c:a",
                "aac",
                "-c:v",
                "libx265",
                "-preset",
                "ultrafast",
                "-x265-params",
                "log-level=error",
                "-pix_fmt",
                "yuv420p",
            ],
        )
    }

    /// A small project needing a re-encode to `codec`: letterbox (two sizes), a fade.
    fn hw_project(&self, codec: Codec) -> Project {
        let mut p = Project::default();
        let (a, b) = match codec {
            Codec::H264 => (
                self.clip("hw-a.mp4", 320, 180, 25, 2, true),
                self.clip("hw-b.mp4", 240, 180, 25, 2, true),
            ),
            Codec::Hevc => (self.hevc_clip("hw-ha.mp4"), self.hevc_clip("hw-hb.mp4")),
        };
        for f in [a, b] {
            let m = self.probe(&mut p, &f);
            p.timeline.append(&m).unwrap();
        }
        p.timeline.toggle_fade(0).unwrap();
        p
    }
}

/// Only the finished file is left in the export folder.
fn only_outputs(dir: &Path) {
    for e in std::fs::read_dir(dir).unwrap() {
        let name = e.unwrap().file_name().to_string_lossy().into_owned();
        assert!(!name.contains("videolore-partial"), "left behind: {name}");
        assert!(!name.ends_with(".corrupt"), "left behind: {name}");
    }
}

fn can_test_hevc(env: &Env) -> bool {
    if env.info.encoders.contains("libx265") {
        return true;
    }
    assert!(!required(), "the CI ffmpeg must have libx265");
    false
}

/// Each vendor path, H.264 and HEVC: the file has the format, frame count, audio and
/// (HEVC in MP4) the hvc1 tag of the Best quality file, and passes the output check.
// covers: REQ-055, REQ-062, REQ-063
#[test]
fn every_hardware_path_makes_the_same_format_as_best_quality() {
    let Some(env) = setup() else { return };
    let mut codecs = vec![Codec::H264];
    if can_test_hevc(&env) {
        codecs.push(Codec::Hevc);
    }
    for codec in codecs {
        let p = env.hw_project(codec);
        let (best, m, _) = env.export_with(
            &p,
            &format!("best-{}.mp4", codec.name()),
            &SystemRunner,
            None,
        );
        assert_eq!(m, ExportMethod::Software);
        let want = env.format_of(&best);
        assert_eq!(want.video("codec_name"), codec.name());
        if codec == Codec::Hevc {
            assert_eq!(want.video("codec_tag_string"), "hvc1");
        }
        for vendor in Vendor::ALL {
            let sim = SimulatedHw::new(Mode::Ok);
            let name = format!("fast-{}-{}.mp4", codec.name(), vendor.suffix());
            let (file, method, ev) = env.export_with(&p, &name, &sim, Some(offer(codec, vendor)));
            assert_eq!(method, ExportMethod::Hardware(vendor), "{name}");
            assert_eq!(sim.runs.load(Ordering::SeqCst), 1);
            assert_eq!(env.format_of(&file), want, "{name}");
            // progress only goes forward, and reaches the end
            let progress: Vec<f32> = ev
                .iter()
                .filter_map(|e| match e {
                    ExportEvent::Progress(f) => Some(*f),
                    _ => None,
                })
                .collect();
            assert!(progress.windows(2).all(|w| w[0] <= w[1]), "{progress:?}");
            assert!(progress.last().is_some_and(|f| *f > 0.99), "{progress:?}");
            // the vendor's arguments were in the command (before the simulation)
            let cmd = sim.commands.lock().unwrap()[0].join(" ");
            assert!(
                cmd.contains("-g 250") && cmd.contains("-profile:v"),
                "{cmd}"
            );
        }
    }
    only_outputs(env.dir.path());
}

/// Stream copy is the same with or without video hardware: byte-identical files, and the
/// hardware is never started.
// covers: REQ-062
#[test]
fn stream_copy_is_byte_identical_with_either_choice() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    for name in ["sc-a.mp4", "sc-b.mp4"] {
        let f = env.clip(name, 320, 180, 25, 2, true);
        let m = env.probe(&mut p, &f);
        p.timeline.append(&m).unwrap();
    }
    let (best, m1, _) = env.export_with(&p, "copy-best.mp4", &SystemRunner, None);
    let sim = SimulatedHw::new(Mode::Ok);
    let both = HwOffer {
        h264: Some(cand(Vendor::Nvenc)),
        hevc: Some(cand(Vendor::Nvenc)),
    };
    let (fast, m2, _) = env.export_with(&p, "copy-fast.mp4", &sim, Some(both));
    assert_eq!(
        (m1, m2),
        (ExportMethod::StreamCopy, ExportMethod::StreamCopy)
    );
    assert_eq!(sim.runs.load(Ordering::SeqCst), 0);
    assert_eq!(sha256(&best), sha256(&fast));
}

/// A hardware file that is empty, cut short, missing frames, the wrong size, green or
/// garbage is always caught; the export is redone with the normal method, silently.
// covers: REQ-063, REQ-055
#[test]
fn damaged_hardware_files_are_caught_and_redone() {
    let Some(env) = setup() else { return };
    let p = env.hw_project(Codec::H264);
    let (best, _, _) = env.export_with(&p, "dmg-best.mp4", &SystemRunner, None);
    let want = env.format_of(&best);
    let modes = [
        Mode::Empty,
        Mode::Truncated,
        Mode::MissingFrames,
        Mode::WrongSize,
        Mode::Green,
        Mode::Garbage,
    ];
    // NVENC: plain graph; VA-API: the graph ends with the upload to the device
    for vendor in [Vendor::Nvenc, Vendor::Vaapi] {
        for mode in modes {
            let sim = SimulatedHw::new(mode);
            let name = format!("dmg-{}-{mode:?}.mp4", vendor.suffix());
            let (file, method, ev) =
                env.export_with(&p, &name, &sim, Some(offer(Codec::H264, vendor)));
            let failure = match &method {
                ExportMethod::SoftwareAfterProblem {
                    codec: Codec::H264,
                    vendor: v,
                    problem: Problem::CheckFailed(f),
                } if *v == vendor => f.clone(),
                other => panic!("{name}: {other:?}"),
            };
            // each damage is caught by the step meant for it
            let caught = match mode {
                Mode::Empty | Mode::Truncated => {
                    matches!(
                        failure,
                        CheckFailure::Decode | CheckFailure::FrameCount { .. }
                    )
                }
                Mode::MissingFrames => matches!(failure, CheckFailure::FrameCount { .. }),
                Mode::WrongSize => failure == CheckFailure::Metadata("size"),
                Mode::Green | Mode::Garbage => matches!(failure, CheckFailure::Picture { .. }),
                Mode::Ok => false,
            };
            assert!(caught, "{name}: {failure:?}");
            assert_eq!(env.format_of(&file), want, "{name}");
            assert!(!ev.iter().any(|e| matches!(e, ExportEvent::Failed(_))));
            // the progress bar went back once for the redo, and stayed within 0..1
            let progress: Vec<f32> = ev
                .iter()
                .filter_map(|e| match e {
                    ExportEvent::Progress(f) => Some(*f),
                    _ => None,
                })
                .collect();
            assert_eq!(
                progress.windows(2).filter(|w| w[1] < w[0]).count(),
                1,
                "{name}"
            );
            assert!(progress.iter().all(|f| (0.0..=1.0).contains(f)));
        }
    }
    only_outputs(env.dir.path());
}

/// No false alarms: the check accepts a correct file of a rich project (60 cuts that are
/// not whole frames, two frame rates, fades, letterbox, music, burned-in subtitles).
// covers: REQ-063, REQ-062
#[test]
fn the_check_accepts_correct_files_of_rich_projects() {
    let Some(env) = setup() else { return };
    let mut p = Project::default();
    let long = env.clip("rich-long.mp4", 320, 180, 25, 12, true);
    let small = env.clip("rich-small.mp4", 240, 180, 30, 2, true);
    let song = env.fixture(
        "rich-song.m4a",
        &[
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=700:sample_rate=48000:duration=4",
            "-c:a",
            "aac",
        ],
    );
    let ml = env.probe(&mut p, &long);
    let msm = env.probe(&mut p, &small);
    let ms = env.probe(&mut p, &song);
    for i in 0..60 {
        p.timeline.append(&ml).unwrap();
        let id = p.timeline.clips[i].id;
        // uneven lengths: rounding to frames must not add up
        let a = Micros(i as i64 * 190_000);
        p.timeline
            .trim(
                id,
                a,
                a + Micros(130_000 + (i as i64 % 3) * 7_000),
                ml.duration,
                Micros(1),
            )
            .unwrap();
    }
    p.timeline.append(&msm).unwrap();
    p.timeline.toggle_fade(10).unwrap();
    p.timeline.toggle_fade(59).unwrap();
    p.music.place(&ms, Micros(500_000), Micros(40_000)).unwrap();
    if env.info.has_subtitles() {
        p.subtitles
            .add_at(&p.timeline, Micros(300_000), "CHECK ME", Micros(40_000))
            .unwrap();
    }
    let (best, _, _) = env.export_with(&p, "rich-best.mp4", &SystemRunner, None);
    let want = env.format_of(&best);
    for vendor in [Vendor::Vaapi, Vendor::VideoToolbox] {
        let sim = SimulatedHw::new(Mode::Ok);
        let name = format!("rich-{}.mp4", vendor.suffix());
        let (file, method, _) = env.export_with(&p, &name, &sim, Some(offer(Codec::H264, vendor)));
        assert_eq!(method, ExportMethod::Hardware(vendor), "{name}");
        assert_eq!(env.format_of(&file), want, "{name}");
    }
}

/// The real check of this computer's video hardware finishes within its time budget,
/// picks the same methods every time and, on CI (no graphics hardware), finds nothing.
/// It runs next to the other tests, so the computer is busy: a candidate that does not
/// answer in time is retried, and both checks must still agree.
// covers: REQ-061
#[test]
fn the_real_hardware_check_is_quick_and_stable() {
    let Some(env) = setup() else { return };
    let platform = Platform::detect();
    let limits = ProbeLimits::default();
    let run = || {
        let t = Instant::now();
        let s = hwenc::probe_all(
            &limited(Priority::Probe),
            &env.info.ffmpeg,
            &env.info.encoders,
            &platform,
            limits,
            &AtomicBool::new(false),
        );
        (s, t.elapsed())
    };
    let (first, took) = run();
    let (second, took2) = run();
    // the total running time at most, plus process start-up and slot bookkeeping
    let bound = limits.total + Duration::from_millis(1500);
    assert!(took < bound && took2 < bound, "{took:?} {took2:?}");
    assert_eq!(first.h264, second.h264, "{first:?}\n{second:?}");
    assert_eq!(first.hevc, second.hevc, "{first:?}\n{second:?}");
    if std::env::var("CI").as_deref() == Ok("true") && platform.render_nodes.is_empty() {
        assert!(first.none_found(), "{first:?}");
    }
    eprintln!("hardware found: {first:?} in {took:?} and {took2:?}");
}

/// With real video hardware (only when VIDEOLORE_REAL_HW=1): each format the check finds
/// exports on the hardware and passes the output check.
#[test]
fn real_hardware_exports_pass_the_check() {
    if std::env::var("VIDEOLORE_REAL_HW").as_deref() != Ok("1") {
        return;
    }
    let Some(env) = setup() else { return };
    let status = hwenc::probe_all(
        &SystemRunner,
        &env.info.ffmpeg,
        &env.info.encoders,
        &Platform::detect(),
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    eprintln!("hardware: {status:?}");
    for codec in Codec::ALL {
        let Some(c) = status.get(codec) else { continue };
        if codec == Codec::Hevc && !can_test_hevc(&env) {
            continue;
        }
        let p = env.hw_project(codec);
        let hw = HwOffer {
            h264: status.h264.clone(),
            hevc: status.hevc.clone(),
        };
        let name = format!("real-{}.mp4", codec.name());
        let (file, method, _) = env.export_with(&p, &name, &SystemRunner, Some(hw));
        assert_eq!(method, ExportMethod::Hardware(c.vendor), "{name}");
        let (best, _, _) = env.export_with(
            &p,
            &format!("real-best-{}.mp4", codec.name()),
            &SystemRunner,
            None,
        );
        assert_eq!(env.format_of(&file), env.format_of(&best));
    }
}

/// QG-05 measurement (manual, real hardware): exports the project named by
/// `VIDEOLORE_QG05_PROJECT` with Best quality and with Faster, renders a lossless FFV1
/// reference of the same plan, and prints for each file the method, time, size and (when
/// the ffmpeg has libvmaf, e.g. the bundled one) the VMAF score against the reference.
/// Files go to `VIDEOLORE_QG05_OUT` (default: a temporary folder); VMAF is computed with
/// `VIDEOLORE_VMAF_FFMPEG` when set (an ffmpeg with libvmaf). Run with
/// `cargo test --release --test ffmpeg_integration qg05 -- --ignored --nocapture`.
#[test]
#[ignore]
fn qg05_reference_render() {
    let Ok(project) = std::env::var("VIDEOLORE_QG05_PROJECT") else {
        eprintln!("set VIDEOLORE_QG05_PROJECT to a .vlproj file");
        return;
    };
    let Some(env) = setup() else { return };
    let (p, _) = project::load(Path::new(&project)).unwrap();
    let out_dir = std::env::var_os("VIDEOLORE_QG05_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| env.dir.path().to_path_buf());
    std::fs::create_dir_all(&out_dir).unwrap();
    // the lossless reference: the export graph, video only, FFV1
    let plan = export::plan(&env.job(&p, &out_dir.join("x.mp4")), &SystemRunner).unwrap();
    let Plan::Reencode { settings, .. } = &plan else {
        panic!("this project is exported by stream copy: nothing to compare")
    };
    let graph = render::check_graph(&p, &render::Geometry::of(settings)).unwrap();
    let ext = settings.extension.clone();
    let work = tempfile::tempdir().unwrap();
    if let Some(doc) = plan.subtitles() {
        export::write_subtitles(work.path(), doc).unwrap();
    }
    let reference = out_dir.join("qg05-reference.mkv");
    let mut c = Command::new(&env.info.ffmpeg);
    c.current_dir(work.path())
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(render::input_args(&graph.inputs))
        .args(
            ffmpeg::filter_graph_args(&graph.graph, env.info.version, &work.path().join("g.txt"))
                .unwrap(),
        )
        .args([
            "-map", "[outv]", "-c:v", "ffv1", "-level", "3", "-pix_fmt", "yuv420p",
        ])
        .arg(&reference);
    assert!(c.status().unwrap().success(), "reference render");
    let status = hwenc::probe_all(
        &SystemRunner,
        &env.info.ffmpeg,
        &env.info.encoders,
        &Platform::detect(),
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    eprintln!("QG-05 hardware: {status:?}");
    let hw = HwOffer {
        h264: status.h264.clone(),
        hevc: status.hevc.clone(),
    };
    // VMAF needs libvmaf: the ffmpeg in use, or the one in VIDEOLORE_VMAF_FFMPEG (e.g. the
    // bundled build, when the system ffmpeg is the one that reaches the hardware)
    let vmaf_ffmpeg = std::env::var_os("VIDEOLORE_VMAF_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| env.info.ffmpeg.clone());
    let has_vmaf = vmaf_ffmpeg != env.info.ffmpeg || env.info.filters.contains("libvmaf");
    for (name, offer) in [("best", None), ("faster", Some(hw))] {
        let file = out_dir.join(format!("qg05-{name}.{ext}"));
        let _ = std::fs::remove_file(&file);
        let mut job = env.job(&p, &file);
        job.threads = None;
        job.hw = offer;
        let (tx, _rx) = channel();
        let t = Instant::now();
        let (path, method) =
            export::run(&job, &SystemRunner, &AtomicBool::new(false), &tx).unwrap();
        let took = t.elapsed();
        let size = std::fs::metadata(&path).unwrap().len();
        let vmaf = if has_vmaf {
            let o = Command::new(&vmaf_ffmpeg)
                .args(["-hide_banner", "-nostdin", "-i"])
                .arg(&path)
                .arg("-i")
                .arg(&reference)
                .args([
                    "-lavfi",
                    "[0:v]setpts=PTS-STARTPTS[a];[1:v]setpts=PTS-STARTPTS[b];[a][b]libvmaf=n_threads=8",
                    "-f",
                    "null",
                    "-",
                ])
                .output()
                .unwrap();
            String::from_utf8_lossy(&o.stderr)
                .lines()
                .find_map(|l| l.split("VMAF score:").nth(1).map(|s| s.trim().to_string()))
                .unwrap_or_else(|| "?".into())
        } else {
            "no libvmaf".into()
        };
        eprintln!(
            "QG-05 {name}: {method:?}, {:.2} s, {size} bytes, VMAF {vmaf}, format {:?}",
            took.as_secs_f64(),
            env.format_of(&path)
        );
    }
}
