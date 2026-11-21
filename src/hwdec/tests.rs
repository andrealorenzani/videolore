// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the hardware decoders: the probe, the arguments and their use in a preview run.

use super::*;
use crate::ffmpeg::CmdOutput;
use crate::ffmpeg::fake::{FakeRunner, fail, ok};

fn linux(nodes: &[&str]) -> Platform {
    Platform {
        os: Os::Linux,
        render_nodes: nodes.iter().map(PathBuf::from).collect(),
        system_libva_new: Some(true),
        libva_version: Some((2, 21)),
    }
}

fn software() -> BTreeSet<String> {
    ["libx264", "libx265", "libsvtav1", "libvpx-vp9"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn s(a: &[OsString]) -> Vec<String> {
    a.iter().map(|x| x.to_string_lossy().into_owned()).collect()
}

/// A fake ffmpeg: it makes the probe clips, and decodes them with the methods in `works`
/// (the others fall back to the processor, with the message ffmpeg gives, or fail).
fn world(works: &'static [&'static str]) -> FakeRunner {
    FakeRunner::default().on(move |_, a| {
        if a.iter().any(|x| x.starts_with("testsrc2")) {
            return ok("");
        }
        let i = a.iter().position(|x| x == "-hwaccel")?;
        if works.contains(&a[i + 1].as_str()) {
            Some(CmdOutput {
                success: true,
                stdout: format!("frame={SAMPLE_FRAMES}\nprogress=end\n").into_bytes(),
                stderr: String::new(),
            })
        } else {
            Some(CmdOutput {
                success: true,
                stdout: format!("frame={SAMPLE_FRAMES}\nprogress=end\n").into_bytes(),
                stderr: "Failed setup for format vaapi: hwaccel initialisation returned error"
                    .into(),
            })
        }
    })
}

fn probe_with(r: &FakeRunner, p: &Platform) -> Working {
    probe(
        r,
        Path::new("ffmpeg"),
        &software(),
        p,
        false,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    )
}

// covers: REQ-214
#[test]
fn the_decoders_of_each_system_and_their_arguments() {
    let l = candidates(&linux(&["/dev/dri/renderD128"]));
    assert_eq!(
        l.iter().map(|d| d.method()).collect::<Vec<_>>(),
        ["cuda", "qsv", "vaapi"]
    );
    assert_eq!(l[2].label(), "VA-API (renderD128)");
    assert_eq!(l[1].label(), "Intel Quick Sync");
    assert_eq!(s(&l[0].input_args()), ["-hwaccel", "cuda"]);
    assert_eq!(
        s(&l[2].input_args()),
        [
            "-hwaccel",
            "vaapi",
            "-hwaccel_device",
            "/dev/dri/renderD128"
        ]
    );
    let win = Platform {
        os: Os::Windows,
        ..Platform::none()
    };
    assert_eq!(
        candidates(&win)
            .iter()
            .map(|d| d.method())
            .collect::<Vec<_>>(),
        ["cuda", "qsv", "d3d11va"]
    );
    let mac = Platform {
        os: Os::MacOs,
        ..Platform::none()
    };
    assert_eq!(candidates(&mac)[0].method(), "videotoolbox");
    assert!(candidates(&Platform::none()).is_empty());
}

// covers: REQ-214
#[test]
fn the_probe_keeps_the_decoders_that_give_every_frame_from_the_hardware() {
    let r = world(&["vaapi"]);
    let w = probe_with(&r, &linux(&["/dev/dri/renderD128"]));
    // every format the bundled encoders can make has the working decoder; the ones that
    // fell back to the processor (cuda, qsv: ffmpeg says so) are not kept
    for codec in Codec::ALL {
        assert_eq!(
            w[&codec].iter().map(|d| d.method()).collect::<Vec<_>>(),
            ["vaapi"],
            "{codec:?}"
        );
    }
    // the probe clip is made by the bundled software encoder of the format
    let calls = r.calls.lock().unwrap();
    for enc in ["libx264", "libx265", "libsvtav1", "libvpx-vp9"] {
        assert!(calls.iter().any(|c| c.iter().any(|x| x == enc)), "{enc}");
    }
    // 4 formats: 1 clip + 3 decoders each
    assert_eq!(calls.len(), 16);
    drop(calls);
    // the frame count is checked: a decode with missing frames is not a working decoder
    let short = FakeRunner::default().on(|_, a| {
        if a.iter().any(|x| x == "-hwaccel") {
            Some(CmdOutput {
                success: true,
                stdout: b"frame=3\nprogress=end\n".to_vec(),
                stderr: String::new(),
            })
        } else {
            ok("")
        }
    });
    assert!(probe_with(&short, &linux(&[])).is_empty());
    // a failing decode, and a clip that cannot be made, give nothing
    let broken = FakeRunner::default().on(|_, a| {
        if a.iter().any(|x| x == "-hwaccel") {
            fail("Cannot load libcuda.so.1")
        } else {
            ok("")
        }
    });
    assert!(probe_with(&broken, &linux(&[])).is_empty());
    let no_clip = FakeRunner::default().on(|_, _| fail("encoder missing"));
    assert!(probe_with(&no_clip, &linux(&[])).is_empty());
}

// covers: REQ-214
#[test]
fn the_probe_follows_the_limits_the_encoders_it_has_and_the_old_library() {
    let r = world(&["cuda", "qsv", "vaapi"]);
    // only the formats whose software encoder exists are tried
    let few: BTreeSet<String> = ["libx264".to_string()].into();
    let w = probe(
        &r,
        Path::new("ffmpeg"),
        &few,
        &linux(&["/dev/dri/renderD128"]),
        false,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert_eq!(w.keys().copied().collect::<Vec<_>>(), [Codec::H264]);
    assert_eq!(w[&Codec::H264].len(), 3);
    // an old system video library: VA-API and Quick Sync are left out
    let r = world(&["cuda", "qsv", "vaapi"]);
    let w = probe(
        &r,
        Path::new("ffmpeg"),
        &few,
        &linux(&["/dev/dri/renderD128"]),
        true,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert_eq!(
        w[&Codec::H264]
            .iter()
            .map(|d| d.method())
            .collect::<Vec<_>>(),
        ["cuda"]
    );
    // a cancelled probe tries nothing
    let r = world(&["cuda"]);
    let w = probe(
        &r,
        Path::new("ffmpeg"),
        &software(),
        &linux(&[]),
        false,
        ProbeLimits::default(),
        &AtomicBool::new(true),
    );
    assert!(w.is_empty() && r.calls.lock().unwrap().is_empty());
    // no hardware on this system (macOS without it, unknown systems): no process at all
    let r = world(&[]);
    assert!(probe_with(&r, &Platform::none()).is_empty() && r.calls.lock().unwrap().is_empty());
    // no time left: nothing more is started
    let r = world(&["cuda"]);
    let none = ProbeLimits {
        total: Duration::ZERO,
        ..ProbeLimits::default()
    };
    let w = probe(
        &r,
        Path::new("ffmpeg"),
        &software(),
        &linux(&[]),
        false,
        none,
        &AtomicBool::new(false),
    );
    assert!(w.is_empty());
    // the whole probe of this computer (no hardware in CI): bounded and quiet
    let w = probe_all(
        &FakeRunner::default(),
        Path::new("ffmpeg"),
        &software(),
        &Platform::none(),
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert!(w.is_empty());
}

// covers: REQ-214, REQ-216
#[test]
fn the_decoder_of_the_chosen_device_is_used() {
    let r = world(&["cuda", "vaapi"]);
    let w = probe_with(&r, &linux(&["/dev/dri/renderD128", "/dev/dri/renderD129"]));
    let auto = choose(&w, None);
    assert_eq!(auto[&Codec::H264].method(), "cuda");
    let second = choose(&w, Some("VA-API (renderD129)"));
    assert_eq!(
        second[&Codec::H264].device,
        Some(PathBuf::from("/dev/dri/renderD129"))
    );
    // a device that decodes nothing (or an invented name) gives no hardware decoding
    assert!(choose(&w, Some("Intel Quick Sync")).is_empty());
    assert!(choose(&w, Some("x\ny")).is_empty());
}

// covers: REQ-214
#[test]
fn a_preview_run_gets_the_decoder_of_each_source_before_its_input() {
    let dec: HashMap<Codec, Decoder> = HashMap::from([(
        Codec::H264,
        Decoder {
            vendor: Vendor::Vaapi,
            device: Some("/dev/dri/renderD128".into()),
        },
    )]);
    let args = crate::ffmpeg::argv([
        "-hide_banner",
        "-ss",
        "0",
        "-i",
        "file:/a.mp4",
        "-ss",
        "1",
        "-i",
        "file:/b.mkv",
        "-i",
        "file:/still.png",
        "-filter_complex",
        "[0:v]null[outv]",
    ]);
    let known = |f: &OsString| match f.to_str()? {
        "file:/a.mp4" => Some(Codec::H264),
        "file:/b.mkv" => Some(Codec::Vp9),
        _ => None,
    };
    let out = s(&apply(args.clone(), &dec, &known)).join(" ");
    // only the H.264 source is decoded by the hardware; VP9 has no decoder here; the
    // picture is left alone
    assert_eq!(
        out,
        "-hide_banner -ss 0 -hwaccel vaapi -hwaccel_device /dev/dri/renderD128 -i file:/a.mp4 \
         -ss 1 -i file:/b.mkv -i file:/still.png -filter_complex [0:v]null[outv]"
    );
    // no decoders (Best quality): the arguments are untouched
    assert_eq!(apply(args.clone(), &HashMap::new(), &known), args);
}

#[test]
fn the_probe_commands_and_the_judgement() {
    let a = s(&sample_args("libx265", Path::new("/t/s.mkv"))).join(" ");
    assert!(a.contains("-frames:v 15") && a.contains("-c:v libx265 -preset ultrafast"));
    assert!(a.ends_with("-f matroska file:/t/s.mkv"));
    for enc in ["libx264", "libsvtav1", "libaom-av1", "libvpx-vp9", "other"] {
        assert!(s(&sample_args(enc, Path::new("/t/s.mkv"))).contains(&enc.to_string()));
    }
    let d = Decoder {
        vendor: Vendor::Qsv,
        device: None,
    };
    let a = s(&decode_args(&d, Path::new("/t/s.mkv"))).join(" ");
    assert_eq!(
        a,
        "-hide_banner -nostdin -v warning -hwaccel qsv -protocol_whitelist file -i file:/t/s.mkv \
         -f null - -progress pipe:1 -nostats"
    );
    assert!(decoded_with_hardware("frame=15\nprogress=end\n", ""));
    assert!(!decoded_with_hardware("frame=14\n", ""));
    assert!(!decoded_with_hardware("", ""));
    assert!(!decoded_with_hardware(
        "frame=15\n",
        "Failed setup for format cuda"
    ));
    assert!(!decoded_with_hardware(
        "frame=15\n",
        "Device creation failed: -12."
    ));
}
