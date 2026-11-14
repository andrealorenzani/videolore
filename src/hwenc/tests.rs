// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the hardware candidates, their arguments and the probe (simulated ffmpeg).

use super::*;
use crate::ffmpeg::fake::{FakeRunner, fail, ok};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

fn all_encoders() -> BTreeSet<String> {
    let mut s = BTreeSet::new();
    for c in Codec::ALL {
        for v in Vendor::ALL {
            s.insert(Candidate::new(v).encoder(c));
        }
    }
    s
}

fn linux(nodes: &[&str]) -> Platform {
    Platform {
        os: Os::Linux,
        render_nodes: nodes.iter().map(PathBuf::from).collect(),
        system_libva_new: Some(true),
    }
}

fn os(os: Os) -> Platform {
    Platform {
        os,
        render_nodes: Vec::new(),
        system_libva_new: None,
    }
}

fn vendors(c: &[Candidate]) -> Vec<Vendor> {
    c.iter().map(|c| c.vendor).collect()
}

fn s(a: &[OsString]) -> Vec<String> {
    a.iter().map(|x| x.to_string_lossy().into_owned()).collect()
}

// covers: REQ-061
#[test]
fn candidates_follow_the_fixed_order_of_each_system() {
    let all = all_encoders();
    for codec in Codec::ALL {
        assert_eq!(
            vendors(&candidates(&os(Os::Windows), codec, &all)),
            [Vendor::Nvenc, Vendor::Qsv, Vendor::Amf]
        );
        let l = candidates(
            &linux(&["/dev/dri/renderD128", "/dev/dri/renderD129"]),
            codec,
            &all,
        );
        assert_eq!(
            vendors(&l),
            [Vendor::Nvenc, Vendor::Qsv, Vendor::Vaapi, Vendor::Vaapi]
        );
        assert_eq!(l[2].device, Some(PathBuf::from("/dev/dri/renderD128")));
        assert_eq!(l[3].label(), "VA-API (renderD129)");
        assert_eq!(
            vendors(&candidates(&os(Os::MacOs), codec, &all)),
            [Vendor::VideoToolbox]
        );
        assert!(candidates(&os(Os::Other), codec, &all).is_empty());
        assert!(candidates(&Platform::none(), codec, &all).is_empty());
    }
    // a method missing from this ffmpeg is skipped at no cost
    let few: BTreeSet<String> = ["h264_qsv", "hevc_nvenc", "h264_vaapi"]
        .iter()
        .map(|x| x.to_string())
        .collect();
    let p = linux(&["/dev/dri/renderD128"]);
    assert_eq!(
        vendors(&candidates(&p, Codec::H264, &few)),
        [Vendor::Qsv, Vendor::Vaapi]
    );
    assert_eq!(vendors(&candidates(&p, Codec::Hevc, &few)), [Vendor::Nvenc]);
    assert!(Os::current() != Os::Other || cfg!(not(any(unix, windows))));
    let d = Platform::detect();
    assert!(d.render_nodes.len() <= MAX_RENDER_NODES);
}

#[test]
fn render_devices_are_enumerated_safely() {
    let e = |n: &str, c: bool| (n.to_string(), c);
    let entries = vec![
        e("renderD129", true),
        e("card0", true),
        e("renderD128", true),
        e("renderD130", false), // not a character device
        e("renderD", true),
        e("renderD12a", true),
        e("renderD1;rm", true),
        e("renderD9999999", true),
        e("renderD131", true),
        e("renderD132", true),
        e("renderD133", true),
    ];
    assert_eq!(
        select_render_nodes(&entries),
        ["renderD128", "renderD129", "renderD131", "renderD132"]
    );
    // a real folder: plain files are never taken
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("renderD128"), b"").unwrap();
    assert!(render_nodes(dir.path()).is_empty());
    assert!(render_nodes(&dir.path().join("missing")).is_empty());
}

/// The arguments of every vendor and format (golden).
// covers: REQ-062
#[test]
fn encode_arguments_of_every_method() {
    let t = "-b:v 4000000 -maxrate 5000000 -bufsize 8000000 -g 250";
    let cases = [
        (
            Vendor::Nvenc,
            Codec::H264,
            format!("-c:v h264_nvenc -preset p5 -tune hq -rc vbr -spatial_aq 1 {t} -profile:v high -pix_fmt yuv420p"),
        ),
        (
            Vendor::Nvenc,
            Codec::Hevc,
            format!("-c:v hevc_nvenc -preset p5 -tune hq -rc vbr -spatial_aq 1 {t} -profile:v main -pix_fmt yuv420p"),
        ),
        (
            Vendor::Qsv,
            Codec::H264,
            format!("-c:v h264_qsv -preset medium {t} -profile:v high -pix_fmt nv12"),
        ),
        (
            Vendor::Qsv,
            Codec::Hevc,
            format!("-c:v hevc_qsv -preset medium {t} -profile:v main -pix_fmt nv12"),
        ),
        (
            Vendor::Amf,
            Codec::H264,
            "-c:v h264_amf -usage transcoding -quality quality -rc vbr_peak -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -profile:v high -pix_fmt nv12".to_string(),
        ),
        (
            Vendor::Amf,
            Codec::Hevc,
            "-c:v hevc_amf -usage transcoding -quality quality -rc vbr_peak -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -profile:v main -pix_fmt nv12".to_string(),
        ),
        (
            Vendor::Vaapi,
            Codec::H264,
            "-c:v h264_vaapi -rc_mode VBR -b:v 4400000 -maxrate 5500000 -bufsize 8800000 -g 250 -profile:v high".to_string(),
        ),
        (
            Vendor::Vaapi,
            Codec::Hevc,
            "-c:v hevc_vaapi -rc_mode VBR -b:v 4400000 -maxrate 5500000 -bufsize 8800000 -g 250 -profile:v main".to_string(),
        ),
        (
            Vendor::VideoToolbox,
            Codec::H264,
            "-c:v h264_videotoolbox -allow_sw 0 -realtime 0 -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -profile:v high -pix_fmt yuv420p".to_string(),
        ),
        (
            Vendor::VideoToolbox,
            Codec::Hevc,
            "-c:v hevc_videotoolbox -allow_sw 0 -realtime 0 -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -profile:v main -pix_fmt yuv420p".to_string(),
        ),
    ];
    for (v, c, want) in cases {
        let cand = Candidate {
            vendor: v,
            device: (v == Vendor::Vaapi).then(|| PathBuf::from("/dev/dri/renderD128")),
            rc: Default::default(),
        };
        let h = encode_args(&cand, c, Target::new(4_000_000));
        assert_eq!(s(&h.vcodec).join(" "), want, "{v:?} {c:?}");
        assert_eq!((h.codec, h.vendor), (c, v));
        // only the options listed for test doubles are vendor specific
        for o in v.options() {
            // "-qp" belongs to VA-API's constant quality (see constant_quality_arguments)
            assert!(want.contains(o) || *o == "-qp", "{o}");
        }
        if v == Vendor::Vaapi {
            assert_eq!(
                s(&h.global),
                [
                    "-init_hw_device",
                    "vaapi=va:/dev/dri/renderD128",
                    "-filter_hw_device",
                    "va"
                ]
            );
            assert_eq!(h.graph_tail, ";[outv]format=nv12,hwupload[outvhw]");
            assert_eq!(h.out_label, "[outvhw]");
        } else {
            assert!(h.global.is_empty() && h.graph_tail.is_empty());
            assert_eq!(h.out_label, "[outv]");
        }
    }
    // VA-API without a device uses the default one
    let h = encode_args(&Candidate::new(Vendor::Vaapi), Codec::H264, Target::new(1));
    assert_eq!(s(&h.global)[1], "vaapi=va");
    // a huge bitrate does not overflow
    let h = encode_args(
        &Candidate::new(Vendor::Amf),
        Codec::H264,
        Target::new(u64::MAX),
    );
    assert!(s(&h.vcodec).contains(&"-maxrate".to_string()));
}

#[test]
fn names_labels_and_profiles() {
    assert_eq!(Codec::from_name("h264"), Some(Codec::H264));
    assert_eq!(Codec::from_name("hevc"), Some(Codec::Hevc));
    assert_eq!(Codec::from_name("vp9"), None);
    assert_eq!(Codec::of_software_encoder("libx265"), Some(Codec::Hevc));
    assert_eq!(Codec::of_software_encoder("libsvtav1"), None);
    assert_eq!(Codec::H264.label(), "H.264");
    assert_eq!(Codec::Hevc.probed_profile(), "Main");
    for v in Vendor::ALL {
        assert!(!v.label().is_empty() && !v.maker().is_empty());
        assert!((100..=120).contains(&v.bitrate_percent()));
        let json = serde_json::to_string(&v).unwrap();
        assert_eq!(json, format!("\"{}\"", v.suffix()));
    }
    assert_eq!(serde_json::to_string(&Codec::Hevc).unwrap(), "\"hevc\"");
    assert_eq!(Candidate::new(Vendor::Nvenc).label(), "NVIDIA (NVENC)");
    for r in [
        Reason::DriverTooOld,
        Reason::NotAvailable,
        Reason::TimedOut,
        Reason::Failed,
    ] {
        assert!(!r.text().is_empty());
    }
}

#[test]
fn probe_failures_are_explained() {
    assert_eq!(
        reason_of("Driver does not support the required nvenc API version. Required: 13.1"),
        Reason::DriverTooOld
    );
    assert_eq!(
        reason_of("The minimum required Nvidia driver for nvenc is 610.0 or newer"),
        Reason::DriverTooOld
    );
    assert_eq!(reason_of("Cannot load libcuda.so.1"), Reason::NotAvailable);
    assert_eq!(
        reason_of("[h264_nvenc] No capable devices found"),
        Reason::NotAvailable
    );
    assert_eq!(
        reason_of("Failed to initialise VAAPI connection: -1"),
        Reason::NotAvailable
    );
    assert_eq!(
        reason_of("Driver does not support VBR RC mode"),
        Reason::Failed
    );
}

/// A fake ffmpeg where `works(encoder)` decides the outcome of each probe.
fn prober(works: impl Fn(&str) -> bool + Send + Sync + 'static) -> FakeRunner {
    FakeRunner::default().on(move |_, a| {
        let i = a.iter().position(|x| x == "-c:v")?;
        if works(&a[i + 1]) {
            ok("")
        } else {
            fail("Cannot load nvcuda.dll")
        }
    })
}

fn probe_linux(r: &FakeRunner, limits: ProbeLimits) -> HwStatus {
    probe_all(
        r,
        Path::new("/usr/bin/ffmpeg"),
        &all_encoders(),
        &linux(&["/dev/dri/renderD128"]),
        limits,
        &AtomicBool::new(false),
    )
}

// covers: REQ-061
#[test]
fn probe_keeps_the_first_working_method_of_each_format() {
    let r = prober(|e| e == "h264_vaapi" || e == "hevc_qsv");
    let st = probe_linux(&r, ProbeLimits::default());
    assert_eq!(st.h264.as_ref().map(|c| c.vendor), Some(Vendor::Vaapi));
    assert_eq!(
        st.h264.as_ref().unwrap().device,
        Some(PathBuf::from("/dev/dri/renderD128"))
    );
    assert_eq!(st.hevc.as_ref().map(|c| c.vendor), Some(Vendor::Qsv));
    assert_eq!(st.get(Codec::Hevc), st.hevc.as_ref());
    assert!(!st.none_found());
    // the failed tries are noted, with the reason
    assert_eq!(
        st.notes,
        [
            (Codec::H264, Vendor::Nvenc, Reason::NotAvailable),
            (Codec::H264, Vendor::Qsv, Reason::NotAvailable),
            (Codec::Hevc, Vendor::Nvenc, Reason::NotAvailable),
        ]
    );
    // H.264: nvenc, qsv, vaapi; HEVC: nvenc, qsv (stops at the first that works)
    assert_eq!(r.calls.lock().unwrap().len(), 5);
    // every probe is a tiny synthetic encode written nowhere, with the real arguments
    for c in r.calls.lock().unwrap().iter() {
        let line = c.join(" ");
        assert!(
            line.contains("testsrc2=size=640x360:rate=30 -frames:v 8"),
            "{line}"
        );
        assert!(line.ends_with("-f null -"), "{line}");
        assert!(line.contains("-g 250"));
    }
    // the same working set always gives the same choice
    let again = probe_linux(&r, ProbeLimits::default());
    assert_eq!((again.h264, again.hevc), (st.h264, st.hevc));
    // nothing works: none found
    let none = probe_linux(&prober(|_| false), ProbeLimits::default());
    assert!(none.none_found());
    assert_eq!(none.notes.len(), 6);
}

// covers: REQ-061
#[test]
fn probe_respects_its_time_limits_and_cancel() {
    // every candidate hangs: each is stopped after `per`, and once `total` is used up
    // the rest count as not working without being started
    let started = Arc::new(AtomicUsize::new(0));
    let s2 = Arc::clone(&started);
    let hang = FakeRunner::default()
        .on(move |_, _| {
            s2.fetch_add(1, Ordering::SeqCst);
            ok("")
        })
        .holding_streams();
    let limits = ProbeLimits {
        per: Duration::from_millis(60),
        total: Duration::from_millis(150),
    };
    let t = Instant::now();
    let st = probe_linux(&hang, limits);
    let took = t.elapsed();
    assert!(st.none_found());
    assert_eq!(st.notes.len(), 6);
    assert!(st.notes.iter().all(|n| n.2 == Reason::TimedOut));
    assert_eq!(
        started.load(Ordering::SeqCst),
        3,
        "60 + 60 + 30 ms, then out of time"
    );
    assert!(took < Duration::from_millis(1500), "{took:?}");
    assert_eq!(ProbeLimits::default().per, Duration::from_secs(1));
    assert_eq!(ProbeLimits::default().total, Duration::from_secs(3));
    // a cancelled probe stops before starting anything
    let r = prober(|_| true);
    let st = probe_all(
        &r,
        Path::new("ffmpeg"),
        &all_encoders(),
        &os(Os::Windows),
        ProbeLimits::default(),
        &AtomicBool::new(true),
    );
    assert!(st.none_found() && r.calls.lock().unwrap().is_empty());
    // a program that cannot start is a failure, not a hang
    let none = FakeRunner::default();
    let st = probe_linux(&none, ProbeLimits::default());
    assert!(st.notes.iter().all(|n| n.2 == Reason::Failed));
}

/// The probe uses the ffmpeg it is given (the user's own choice included).
// covers: REQ-061
#[test]
fn probe_uses_the_chosen_ffmpeg() {
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let s2 = Arc::clone(&seen);
    let r = FakeRunner::default().on(move |p, _| {
        s2.lock().unwrap().push(p.to_path_buf());
        ok("")
    });
    let st = probe_all(
        &r,
        Path::new("/home/me/ffmpeg-custom/ffmpeg"),
        &all_encoders(),
        &os(Os::MacOs),
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert_eq!(st.h264, Some(Candidate::new(Vendor::VideoToolbox)));
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|p| p == Path::new("/home/me/ffmpeg-custom/ffmpeg"))
    );
    let args = s(&probe_args(
        &Candidate::new(Vendor::VideoToolbox),
        Codec::Hevc,
    ));
    assert!(args.contains(&"hevc_videotoolbox".to_string()));
}

/// Constant quality (VA-API drivers that refuse bitrate control): the arguments, the
/// quantiser of a second run, and the name shown.
// covers: REQ-062
#[test]
fn constant_quality_arguments() {
    let cand = Candidate {
        vendor: Vendor::Vaapi,
        device: Some(PathBuf::from("/dev/dri/renderD128")),
        rc: RateControl::ConstantQuality,
    };
    for (codec, p) in [(Codec::H264, "high"), (Codec::Hevc, "main")] {
        let h = encode_args(&cand, codec, Target::new(4_000_000));
        assert_eq!(
            s(&h.vcodec).join(" "),
            format!(
                "-c:v {}_vaapi -rc_mode CQP -qp 23 -g 250 -profile:v {p}",
                codec.name()
            )
        );
        assert_eq!(h.out_label, "[outvhw]");
        assert_eq!(s(&h.global)[1], "vaapi=va:/dev/dri/renderD128");
        let h = encode_args(&cand, codec, Target { bps: 1, qp: 31 });
        assert!(s(&h.vcodec).join(" ").contains("-qp 31"));
    }
    assert_eq!(cand.label(), "VA-API (renderD128, constant quality)");
    assert_eq!(
        Candidate {
            device: None,
            ..cand.clone()
        }
        .label(),
        "VA-API (constant quality)"
    );
    // only VA-API with bitrate control has a constant-quality variant
    assert_eq!(cand.constant_quality(), None);
    assert_eq!(Candidate::new(Vendor::Nvenc).constant_quality(), None);
    assert_eq!(
        Candidate::new(Vendor::Vaapi)
            .constant_quality()
            .map(|c| c.rc),
        Some(RateControl::ConstantQuality)
    );
    // second-run quantiser: about 6 steps halve the size, aiming at 1.3x
    assert_eq!(cq_retry_qp(23, 2.73), 30);
    assert_eq!(cq_retry_qp(23, 1.87), 27);
    assert_eq!(cq_retry_qp(23, 1.0), 23);
    assert_eq!(cq_retry_qp(23, f64::NAN), 23);
    assert_eq!(cq_retry_qp(38, 50.0), CQ_MAX_QP);
    assert_eq!(cq_retry_qp(23, 1.45), 24);
}

/// A driver that refuses bitrate control is tried again at constant quality, within the
/// time budget; other failures are not.
// covers: REQ-061
#[test]
fn probe_falls_back_to_constant_quality() {
    let refuse_vbr = FakeRunner::default().on(|_, a| {
        let line = a.join(" ");
        if !line.contains("_vaapi") {
            return fail("No capable devices found");
        }
        if line.contains("-rc_mode VBR") {
            fail("Driver does not support VBR RC mode (supported modes: CQP).")
        } else {
            ok("")
        }
    });
    let st = probe_linux(&refuse_vbr, ProbeLimits::default());
    let h = st.h264.unwrap();
    assert_eq!(
        (h.vendor, h.rc),
        (Vendor::Vaapi, RateControl::ConstantQuality)
    );
    assert_eq!(h.device, Some(PathBuf::from("/dev/dri/renderD128")));
    assert_eq!(st.hevc.unwrap().rc, RateControl::ConstantQuality);
    // per format: nvenc, qsv, vaapi (VBR), vaapi (CQP)
    assert_eq!(refuse_vbr.calls.lock().unwrap().len(), 8);
    // a missing device is not retried
    let missing = FakeRunner::default().on(|_, _| fail("Failed to initialise VAAPI connection"));
    let st = probe_linux(&missing, ProbeLimits::default());
    assert!(st.none_found());
    assert_eq!(missing.calls.lock().unwrap().len(), 6);
    // both variants refused: the second reason is noted
    let both = FakeRunner::default().on(|_, a| {
        if a.join(" ").contains("_vaapi") {
            fail("internal encoding error")
        } else {
            fail("Cannot load libcuda.so.1")
        }
    });
    let st = probe_linux(&both, ProbeLimits::default());
    assert!(st.none_found());
    assert!(
        st.notes
            .contains(&(Codec::Hevc, Vendor::Vaapi, Reason::Failed))
    );
    assert_eq!(both.calls.lock().unwrap().len(), 8);
}

/// An ffmpeg that needs a newer system libva than this computer has never starts VA-API
/// or Quick Sync (they would abort); the reason is noted.
// covers: REQ-061
#[test]
fn old_system_libva_skips_va_api_and_quick_sync() {
    assert!(libva_too_old(true, Some(false)));
    assert!(libva_too_old(true, None));
    assert!(!libva_too_old(true, Some(true)));
    assert!(!libva_too_old(false, Some(false)));
    assert!(!libva_too_old(false, None));
    // an ffmpeg file naming the libva 2.21 function (across a 1 MiB read boundary)
    let dir = tempfile::tempdir().unwrap();
    let ff = dir.path().join("ffmpeg");
    let mut bytes = vec![0u8; (1 << 20) - 5];
    bytes.extend_from_slice(LIBVA_NEW_SYMBOL);
    bytes.extend_from_slice(b"\0tail");
    std::fs::write(&ff, &bytes).unwrap();
    assert!(file_mentions(&ff, LIBVA_NEW_SYMBOL));
    assert!(ffmpeg_needs_new_libva(&ff));
    assert!(ffmpeg_needs_new_libva(&ff), "remembered");
    let plain = dir.path().join("ffmpeg-system");
    std::fs::write(&plain, vec![7u8; 3 << 20]).unwrap();
    assert!(!file_mentions(&plain, LIBVA_NEW_SYMBOL));
    assert!(!ffmpeg_needs_new_libva(&dir.path().join("missing")));
    let works = prober(|_| true);
    let mut old = linux(&["/dev/dri/renderD128"]);
    old.system_libva_new = Some(false);
    let st = probe_all(
        &works,
        &ff,
        &all_encoders(),
        &old,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert_eq!(st.h264, Some(Candidate::new(Vendor::Nvenc)));
    for codec in Codec::ALL {
        for v in [Vendor::Qsv, Vendor::Vaapi] {
            assert!(
                st.notes.contains(&(codec, v, Reason::LibvaTooOld)),
                "{codec:?} {v:?}"
            );
        }
    }
    let ran = works.calls.lock().unwrap().join(&[][..]).join(" ");
    assert!(!ran.contains("_vaapi") && !ran.contains("_qsv"));
    // NVIDIA missing too: nothing is started at all
    let none = prober(|e| e.ends_with("_qsv"));
    let mut enc = all_encoders();
    enc.retain(|e| !e.ends_with("_nvenc"));
    let st = probe_all(
        &none,
        &ff,
        &enc,
        &old,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert!(st.none_found());
    assert!(none.calls.lock().unwrap().is_empty());
    assert!(Reason::LibvaTooOld.text().contains("libva 2.21"));
    // the same ffmpeg on a computer with a new libva: used normally
    old.system_libva_new = Some(true);
    let st = probe_all(
        &prober(|e| e == "h264_vaapi"),
        &ff,
        &enc,
        &old,
        ProbeLimits::default(),
        &AtomicBool::new(false),
    );
    assert_eq!(st.h264.map(|c| c.vendor), Some(Vendor::Vaapi));
    assert!(find_system_libva().is_none_or(|p| p.is_file()));
}
