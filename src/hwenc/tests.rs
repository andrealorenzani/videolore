// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the hardware candidates, their arguments and the probe (simulated ffmpeg).

use super::*;
use crate::ffmpeg::fake::{FakeRunner, fail, ok};
use crate::ffmpeg::{CmdOutput, Process};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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
        libva_version: Some((2, 21)),
    }
}

fn os(os: Os) -> Platform {
    Platform {
        os,
        render_nodes: Vec::new(),
        system_libva_new: None,
        libva_version: None,
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
        // AV1 and VP9 (REQ-212, REQ-213): no profile argument
        (
            Vendor::Nvenc,
            Codec::Av1,
            format!("-c:v av1_nvenc -preset p5 -tune hq -rc vbr -spatial_aq 1 {t} -pix_fmt yuv420p"),
        ),
        (
            Vendor::Qsv,
            Codec::Av1,
            format!("-c:v av1_qsv -preset medium {t} -pix_fmt nv12"),
        ),
        (
            Vendor::Qsv,
            Codec::Vp9,
            format!("-c:v vp9_qsv -preset medium {t} -pix_fmt nv12"),
        ),
        (
            Vendor::Amf,
            Codec::Av1,
            "-c:v av1_amf -usage transcoding -quality quality -rc vbr_peak -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -pix_fmt nv12".to_string(),
        ),
        (
            Vendor::Vaapi,
            Codec::Av1,
            "-c:v av1_vaapi -rc_mode VBR -b:v 4400000 -maxrate 5500000 -bufsize 8800000 -g 250".to_string(),
        ),
        (
            Vendor::Vaapi,
            Codec::Vp9,
            "-c:v vp9_vaapi -rc_mode VBR -b:v 4400000 -maxrate 5500000 -bufsize 8800000 -g 250".to_string(),
        ),
        (
            Vendor::VideoToolbox,
            Codec::Av1,
            "-c:v av1_videotoolbox -allow_sw 0 -realtime 0 -b:v 4800000 -maxrate 6000000 -bufsize 9600000 -g 250 -pix_fmt yuv420p".to_string(),
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
            // "-qp" and "-global_quality" belong to the constant quality of VA-API and Quick
            // Sync (see constant_quality_arguments)
            assert!(
                want.contains(o) || *o == "-qp" || *o == "-global_quality",
                "{o}"
            );
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
    assert_eq!(Codec::from_name("vp9"), Some(Codec::Vp9));
    assert_eq!(Codec::from_name("mpeg4"), None);
    assert_eq!(Codec::of_software_encoder("libx265"), Some(Codec::Hevc));
    assert_eq!(Codec::of_software_encoder("libsvtav1"), Some(Codec::Av1));
    assert_eq!(Codec::of_software_encoder("libvpx-vp9"), Some(Codec::Vp9));
    assert_eq!(Codec::of_software_encoder("mpeg4"), None);
    assert_eq!(Codec::H264.label(), "H.264");
    assert_eq!(Codec::Hevc.probed_profile(), Some("Main"));
    assert_eq!(Codec::Av1.probed_profile(), None);
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
        &BTreeSet::new(),
        &AtomicBool::new(false),
    )
}

// covers: REQ-061, REQ-216
#[test]
fn probe_lists_every_working_method_of_each_format_best_first() {
    let r = prober(|e| e == "h264_vaapi" || e == "hevc_qsv" || e == "hevc_vaapi");
    let st = probe_linux(&r, ProbeLimits::default());
    // Automatic uses the first of each format
    assert_eq!(st.get(Codec::H264).map(|c| c.vendor), Some(Vendor::Vaapi));
    assert_eq!(
        st.get(Codec::H264).unwrap().device,
        Some(PathBuf::from("/dev/dri/renderD128"))
    );
    assert_eq!(st.get(Codec::Hevc).map(|c| c.vendor), Some(Vendor::Qsv));
    assert_eq!(
        vendors(&st.working[&Codec::Hevc]),
        [Vendor::Qsv, Vendor::Vaapi]
    );
    assert!(st.get(Codec::Av1).is_none() && !st.none_found());
    // the devices are named once, in the order of the check
    assert_eq!(st.devices(), ["VA-API (renderD128)", "Intel Quick Sync"]);
    // the failed tries are noted, with the reason
    assert!(
        st.notes
            .contains(&(Codec::H264, Vendor::Nvenc, Reason::NotAvailable))
    );
    assert!(
        st.notes
            .contains(&(Codec::H264, Vendor::Qsv, Reason::NotAvailable))
    );
    assert!(
        !st.notes
            .iter()
            .any(|n| n.0 == Codec::Hevc && n.1 == Vendor::Qsv)
    );
    // four formats, three methods each: all are tried
    assert_eq!(r.calls.lock().unwrap().len(), 12);
    // every probe is a tiny synthetic encode written nowhere, with the real arguments; its
    // progress goes to stdout, where output shows that the try is alive
    for c in r.calls.lock().unwrap().iter() {
        let line = c.join(" ");
        assert!(
            line.contains("testsrc2=size=640x360:rate=30 -frames:v 8"),
            "{line}"
        );
        assert!(
            line.ends_with("-f null - -progress pipe:1 -nostats"),
            "{line}"
        );
        assert!(line.contains("-g 250"));
    }
    // the same working set always gives the same choice
    let again = probe_linux(&r, ProbeLimits::default());
    assert_eq!(again.working, st.working);
    // nothing works: none found
    let none = probe_linux(&prober(|_| false), ProbeLimits::default());
    assert!(none.none_found() && none.devices().is_empty());
    assert_eq!(none.notes.len(), 12);
}

// covers: REQ-216
#[test]
fn the_chosen_device_is_used_only_when_it_works_for_the_format() {
    let r = prober(|e| e == "h264_vaapi" || e == "hevc_qsv" || e == "hevc_vaapi");
    let st = probe_linux(&r, ProbeLimits::default());
    let vaapi = "VA-API (renderD128)";
    assert_eq!(
        st.chosen(Codec::Hevc, None).map(|c| c.vendor),
        Some(Vendor::Qsv)
    );
    assert_eq!(
        st.chosen(Codec::Hevc, Some(vaapi)).map(|c| c.vendor),
        Some(Vendor::Vaapi)
    );
    // Quick Sync cannot make H.264 here: the normal method, not another device
    assert!(st.chosen(Codec::H264, Some("Intel Quick Sync")).is_none());
    // a name that is not a working device (a stale or invented setting) is never used
    assert!(
        st.chosen(Codec::H264, Some("renderD999; rm -rf /"))
            .is_none()
    );
    // the offer of Automatic holds the best of each format
    let offer = HwOffer::automatic(&st);
    assert_eq!(offer.found.len(), 2);
    assert!(!offer.is_empty() && HwOffer::default().is_empty());
}

// covers: REQ-061
#[test]
fn probe_respects_its_time_limits_and_cancel() {
    // every candidate hangs: each is stopped after `per`, tried once more for `retry`,
    // and once `total` is used up the rest count as not answering without being started
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
        retry: Duration::from_millis(90),
        total: Duration::from_millis(400),
    };
    let t = Instant::now();
    let st = probe_linux(&hang, limits);
    let took = t.elapsed();
    assert!(st.none_found());
    assert!(st.inconclusive());
    assert_eq!(st.notes.len(), 12);
    assert!(st.notes.iter().all(|n| n.2 == Reason::TimedOut));
    assert_eq!(
        started.load(Ordering::SeqCst),
        6,
        "(60 + 90) twice, then 60 + the last 40 ms, then out of time"
    );
    assert!(took < Duration::from_millis(2000), "{took:?}");
    // a try cut short by the total budget is not tried again
    let started = Arc::new(AtomicUsize::new(0));
    let s2 = Arc::clone(&started);
    let hang = FakeRunner::default()
        .on(move |_, _| {
            s2.fetch_add(1, Ordering::SeqCst);
            ok("")
        })
        .holding_streams();
    let short = ProbeLimits {
        total: Duration::from_millis(50),
        ..limits
    };
    let st = probe_linux(&hang, short);
    assert_eq!(st.notes.len(), 12);
    assert_eq!(started.load(Ordering::SeqCst), 1);
    // the documented numbers: 1 s per try, 4 s for the retry, 10 s at most in total
    assert_eq!(ProbeLimits::default().per, Duration::from_secs(1));
    assert_eq!(ProbeLimits::default().retry, Duration::from_secs(4));
    assert_eq!(ProbeLimits::default().total, Duration::from_secs(10));
    // a cancelled probe stops before starting anything
    let r = prober(|_| true);
    let st = probe_all(
        &r,
        Path::new("ffmpeg"),
        &all_encoders(),
        &os(Os::Windows),
        ProbeLimits::default(),
        &BTreeSet::new(),
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
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert_eq!(
        st.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::VideoToolbox))
    );
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
    // only VA-API and Quick Sync with bitrate control have a constant-quality variant
    assert_eq!(cand.constant_quality(), None);
    assert_eq!(Candidate::new(Vendor::Nvenc).constant_quality(), None);
    assert_eq!(Candidate::new(Vendor::Amf).constant_quality(), None);
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
    let h = st.get(Codec::H264).cloned().unwrap();
    assert_eq!(
        (h.vendor, h.rc),
        (Vendor::Vaapi, RateControl::ConstantQuality)
    );
    assert_eq!(h.device, Some(PathBuf::from("/dev/dri/renderD128")));
    assert_eq!(
        st.get(Codec::Hevc).cloned().unwrap().rc,
        RateControl::ConstantQuality
    );
    // per format: nvenc, qsv, vaapi (VBR), vaapi (CQP); four formats
    assert_eq!(refuse_vbr.calls.lock().unwrap().len(), 16);
    // a missing device is not retried
    let missing = FakeRunner::default().on(|_, _| fail("Failed to initialise VAAPI connection"));
    let st = probe_linux(&missing, ProbeLimits::default());
    assert!(st.none_found());
    assert_eq!(missing.calls.lock().unwrap().len(), 12);
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
    assert_eq!(both.calls.lock().unwrap().len(), 16);
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
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert_eq!(
        st.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::Nvenc))
    );
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
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert!(st.none_found());
    assert!(none.calls.lock().unwrap().is_empty());
    assert!(Reason::LibvaTooOld.text().contains("libva 2.21"));
    // REQ-217: Advanced says in plain words which version is installed and which is needed
    old.libva_version = Some((2, 20));
    let st = probe_all(
        &prober(|_| true),
        &ff,
        &all_encoders(),
        &old,
        ProbeLimits::default(),
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    let note = st.libva_note.unwrap();
    assert!(
        note.contains("libva 2.20") && note.contains("libva 2.21 or newer"),
        "{note}"
    );
    assert!(note.contains("normal method"));
    // the version could not be read from the file name: older than the one needed
    assert!(libva_message(None).contains("older than 2.21"));
    // the same ffmpeg on a computer with a new libva: used normally
    old.system_libva_new = Some(true);
    let st = probe_all(
        &prober(|e| e == "h264_vaapi"),
        &ff,
        &enc,
        &old,
        ProbeLimits::default(),
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert_eq!(
        st.get(Codec::H264).cloned().map(|c| c.vendor),
        Some(Vendor::Vaapi)
    );
    assert!(find_system_libva().is_none_or(|p| p.is_file()));
}

/// A fake ffmpeg where every probe works, except that the first `slow` probe runs whose
/// arguments contain `encoder` do not answer in time (a busy computer).
fn slow_at_first(encoder: &'static str, slow: usize) -> (FakeRunner, Arc<AtomicUsize>) {
    let seen = Arc::new(AtomicUsize::new(0));
    let s2 = Arc::clone(&seen);
    let r = FakeRunner::default()
        .on(|_, _| ok(""))
        .hanging_if(move |a| {
            a.iter().any(|x| x == encoder) && s2.fetch_add(1, Ordering::SeqCst) < slow
        });
    (r, seen)
}

fn quick() -> ProbeLimits {
    ProbeLimits {
        per: Duration::from_millis(40),
        retry: Duration::from_millis(80),
        total: Duration::from_secs(5),
    }
}

/// A try that runs out of time because the computer is busy is not a "does not work":
/// the same candidate is tried again at once, so the choice is the one of a quiet computer.
// covers: REQ-061
#[test]
fn a_candidate_that_times_out_once_is_tried_again() {
    let (quiet, _) = slow_at_first("h264_nvenc", 0);
    let want = probe_linux(&quiet, quick());
    assert_eq!(want.get(Codec::H264), Some(&Candidate::new(Vendor::Nvenc)));
    let (busy, seen) = slow_at_first("h264_nvenc", 1);
    let st = probe_linux(&busy, quick());
    assert_eq!(st.working, want.working);
    assert_eq!(seen.load(Ordering::SeqCst), 2, "tried twice");
    assert!(st.notes.is_empty(), "{:?}", st.notes);
    assert!(!st.inconclusive());
    // the preference order is kept: Quick Sync is not tried before NVIDIA's retry
    let calls = busy.calls.lock().unwrap();
    assert!(calls[0].contains(&"h264_nvenc".to_string()));
    assert!(calls[1].contains(&"h264_nvenc".to_string()));
    drop(calls);
    // twice out of time: noted as not answering, the next candidate is used and the
    // answer is inconclusive only while a format has no hardware
    let (busier, seen) = slow_at_first("h264_nvenc", 2);
    let st = probe_linux(&busier, quick());
    assert_eq!(seen.load(Ordering::SeqCst), 2, "one retry only");
    assert_eq!(
        st.get(Codec::H264).cloned(),
        Some(Candidate::new(Vendor::Qsv))
    );
    assert_eq!(st.notes, [(Codec::H264, Vendor::Nvenc, Reason::TimedOut)]);
    assert!(!st.inconclusive());
    let only_nvenc: BTreeSet<String> = ["h264_nvenc".to_string()].into();
    let (busier, _) = slow_at_first("h264_nvenc", 2);
    let st = probe_all(
        &busier,
        Path::new("/usr/bin/ffmpeg"),
        &only_nvenc,
        &linux(&[]),
        quick(),
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert!(st.none_found() && st.inconclusive());
    // a definite failure is never inconclusive
    let st = probe_linux(&prober(|_| false), quick());
    assert!(st.none_found() && !st.inconclusive());
}

/// The VA-API constant-quality variant gets the same retry when the computer is busy.
// covers: REQ-061
#[test]
fn the_constant_quality_variant_is_also_tried_again() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c2 = Arc::clone(&calls);
    let r = FakeRunner::default()
        .on(|_, a| {
            let i = a.iter().position(|x| x == "-c:v")?;
            if a[i + 1] != "h264_vaapi" {
                return fail("Cannot load libcuda.so.1");
            }
            if a.iter().any(|x| x == "CQP") {
                ok("")
            } else {
                fail("Driver does not support VBR RC mode")
            }
        })
        .hanging_if(move |a| {
            // the first constant-quality try does not answer in time
            a.iter().any(|x| x == "CQP") && c2.fetch_add(1, Ordering::SeqCst) == 0
        });
    let st = probe_linux(&r, quick());
    let found = st.get(Codec::H264).cloned().expect("found on the retry");
    assert_eq!(found.vendor, Vendor::Vaapi);
    assert_eq!(found.rc, RateControl::ConstantQuality);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

// covers: REQ-215
#[test]
fn quick_sync_falls_back_to_constant_quality_when_the_driver_refuses_bitrate_control() {
    let qsv = Candidate::new(Vendor::Qsv);
    let cq = qsv.constant_quality().unwrap();
    assert_eq!(cq.rc, RateControl::ConstantQuality);
    assert_eq!(cq.label(), "Intel Quick Sync (constant quality)");
    let h = encode_args(&cq, Codec::H264, Target::new(4_000_000));
    assert_eq!(
        s(&h.vcodec).join(" "),
        "-c:v h264_qsv -preset medium -global_quality 23 -g 250 -profile:v high -pix_fmt nv12"
    );
    // no bitrate in constant quality; a second run uses the corrected quantiser
    assert!(!s(&h.vcodec).contains(&"-b:v".to_string()));
    let h = encode_args(&cq, Codec::Av1, Target { bps: 1, qp: 31 });
    assert_eq!(
        s(&h.vcodec).join(" "),
        "-c:v av1_qsv -preset medium -global_quality 31 -g 250 -pix_fmt nv12"
    );
    // the probe: a Quick Sync that refuses the bitrate is found at constant quality, and
    // that is not a failure (no note for the method)
    let refuse = FakeRunner::default().on(|_, a| {
        let line = a.join(" ");
        if !line.contains("_qsv") {
            return fail("Cannot load libcuda.so.1");
        }
        if line.contains("-global_quality") {
            ok("")
        } else {
            fail("Error initializing the encoder: unsupported rate control")
        }
    });
    let st = probe_linux(&refuse, ProbeLimits::default());
    let c = st.get(Codec::H264).unwrap();
    assert_eq!(
        (c.vendor, c.rc),
        (Vendor::Qsv, RateControl::ConstantQuality)
    );
    assert!(!st.notes.iter().any(|n| n.1 == Vendor::Qsv));
}

// covers: REQ-217
#[test]
fn the_libva_version_is_read_from_the_file_name() {
    assert_eq!(libva_version_from_name("libva.so.2.2000.0"), Some((2, 20)));
    assert_eq!(libva_version_from_name("libva.so.2.2100.0"), Some((2, 21)));
    assert_eq!(libva_version_from_name("libva.so.2.1900.0"), Some((2, 19)));
    assert_eq!(libva_version_from_name("libva.so.2"), None);
    assert_eq!(libva_version_from_name("libva.so.x.y"), None);
    assert_eq!(libva_version_from_name("libdrm.so.2.4.0"), None);
    // a real file whose name has the version (through a link, as on a system)
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("libva.so.2.2000.0");
    std::fs::write(&real, b"x").unwrap();
    assert_eq!(libva_version_of(&real), Some((2, 20)));
    assert_eq!(libva_version_of(&dir.path().join("missing")), None);
    #[cfg(unix)]
    {
        let link = dir.path().join("libva.so.2");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(libva_version_of(&link), Some((2, 20)));
    }
}

// covers: REQ-207
#[test]
fn integrated_and_dedicated_graphics_are_told_apart() {
    let none = |_: &Path| None;
    assert_eq!(
        Candidate::new(Vendor::Qsv).graphics(none),
        Graphics::Integrated
    );
    assert_eq!(
        Candidate::new(Vendor::VideoToolbox).graphics(none),
        Graphics::Integrated
    );
    assert_eq!(
        Candidate::new(Vendor::Nvenc).graphics(none),
        Graphics::Dedicated
    );
    assert_eq!(
        Candidate::new(Vendor::Amf).graphics(none),
        Graphics::Dedicated
    );
    let va = |n: &str| Candidate {
        vendor: Vendor::Vaapi,
        device: Some(PathBuf::from(format!("/dev/dri/{n}"))),
        rc: Default::default(),
    };
    // a VA-API device is told by the maker the system names for it
    let read = |p: &Path| match p.to_str()? {
        "/sys/class/drm/renderD128/device/vendor" => Some("0x8086\n".to_string()),
        "/sys/class/drm/renderD129/device/vendor" => Some("0x10de\n".to_string()),
        _ => None,
    };
    assert_eq!(va("renderD128").graphics(read), Graphics::Integrated);
    assert_eq!(va("renderD129").graphics(read), Graphics::Dedicated);
    // unknown: the safe answer is "dedicated" (no discouraging note)
    assert_eq!(va("renderD130").graphics(read), Graphics::Dedicated);
    assert_eq!(
        Candidate::new(Vendor::Vaapi).graphics(read),
        Graphics::Dedicated
    );
    assert_eq!(graphics_of_vendor_id(" 0x8086 "), Graphics::Integrated);
    // this computer's own answer is one of the two
    let _ = Candidate::new(Vendor::Vaapi).graphics_here();
}

/// A process that writes after `output_after` (nothing when `None`), ends after `ends_at`
/// (never when `None`), and has used `cpu` ms of processor time at each look.
struct Timed {
    output_after: Option<Duration>,
    ends_at: Option<Duration>,
    cpu: Arc<std::sync::Mutex<Vec<u64>>>,
    killed: Arc<AtomicBool>,
}

struct TimedOut {
    start: Instant,
    output_after: Option<Duration>,
    ends_at: Option<Duration>,
    killed: Arc<AtomicBool>,
    sent: bool,
}

impl Read for TimedOut {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        loop {
            if self.killed.load(Ordering::SeqCst) {
                return Ok(0);
            }
            let t = self.start.elapsed();
            if let Some(a) = self.output_after
                && !self.sent
                && t >= a
            {
                self.sent = true;
                buf[..9].copy_from_slice(b"progress=");
                return Ok(9);
            }
            if self.ends_at.is_some_and(|e| t >= e) {
                return Ok(0);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

struct TimedProcess {
    t: Timed,
    start: Instant,
    stdout_taken: bool,
}

impl Process for TimedProcess {
    fn take_stdout(&mut self) -> Option<Box<dyn Read + Send>> {
        self.stdout_taken = true;
        Some(Box::new(TimedOut {
            start: self.start,
            output_after: self.t.output_after,
            ends_at: self.t.ends_at,
            killed: Arc::clone(&self.t.killed),
            sent: false,
        }))
    }
    fn kill(&mut self) {
        self.t.killed.store(true, Ordering::SeqCst);
    }
    fn cpu_ms(&mut self) -> Option<u64> {
        let mut v = self.t.cpu.lock().unwrap();
        if v.len() > 1 {
            Some(v.remove(0))
        } else {
            v.first().copied()
        }
    }
    fn wait(&mut self) -> std::io::Result<CmdOutput> {
        Ok(CmdOutput {
            success: !self.t.killed.load(Ordering::SeqCst),
            stdout: Vec::new(),
            stderr: String::new(),
        })
    }
}

/// Spawns [`TimedProcess`]es: the n-th spawn behaves as `plan[n]` says (the last one repeats).
struct TimedRunner {
    plan: Vec<(Option<Duration>, Option<Duration>, Vec<u64>)>,
    spawns: AtomicUsize,
}

impl Runner for TimedRunner {
    fn run(&self, _: &Path, _: &[OsString]) -> std::io::Result<CmdOutput> {
        Ok(CmdOutput::default())
    }
    fn spawn_in(
        &self,
        _: &Path,
        _: &[OsString],
        _: Option<&Path>,
    ) -> std::io::Result<Box<dyn Process>> {
        let n = self.spawns.fetch_add(1, Ordering::SeqCst);
        let (output_after, ends_at, cpu) = self.plan[n.min(self.plan.len() - 1)].clone();
        Ok(Box::new(TimedProcess {
            t: Timed {
                output_after,
                ends_at,
                cpu: Arc::new(std::sync::Mutex::new(cpu)),
                killed: Arc::new(AtomicBool::new(false)),
            },
            start: Instant::now(),
            stdout_taken: false,
        }))
    }
}

fn one_candidate(r: &TimedRunner, limits: ProbeLimits) -> HwStatus {
    probe(
        r,
        Path::new("ffmpeg"),
        &[(Codec::H264, vec![Candidate::new(Vendor::Nvenc)])],
        limits,
        &BTreeSet::new(),
        &AtomicBool::new(false),
    )
}

fn scaled() -> ProbeLimits {
    // REQ-061's 1 s and 4 s, at a hundredth
    ProbeLimits {
        per: Duration::from_millis(60),
        retry: Duration::from_millis(240),
        total: Duration::from_secs(5),
    }
}

// covers: REQ-210
#[test]
fn a_busy_candidate_gets_the_retry_and_a_hanging_one_is_skipped_at_once() {
    let ms = Duration::from_millis;
    // no output and no processor use when the time is up (1 s, here 60 ms): hanging, skipped
    // at once, after its first try
    let r = TimedRunner {
        plan: vec![
            (Some(ms(900)), None, vec![0, 0]),
            (None, Some(ms(10)), vec![0]),
        ],
        spawns: AtomicUsize::new(0),
    };
    let st = one_candidate(&r, scaled());
    assert_eq!(r.spawns.load(Ordering::SeqCst), 1);
    assert_eq!(st.notes, [(Codec::H264, Vendor::Nvenc, Reason::Hanging)]);
    assert_eq!(st.slow, [(Codec::H264, Vendor::Nvenc)]);
    // output after 1.5 s (here 90 ms, the limit being 60) from a process that is using the
    // processor: busy, the retry finds it
    let r = TimedRunner {
        plan: vec![
            (Some(ms(90)), None, vec![5, 45]),
            (None, Some(ms(10)), vec![0]),
        ],
        spawns: AtomicUsize::new(0),
    };
    let st = one_candidate(&r, scaled());
    assert_eq!(r.spawns.load(Ordering::SeqCst), 2);
    assert_eq!(st.get(Codec::H264), Some(&Candidate::new(Vendor::Nvenc)));
    assert!(st.notes.is_empty() && st.slow.is_empty());
    // output seen before the time is up is activity too
    let r = TimedRunner {
        plan: vec![
            (Some(ms(5)), None, vec![0, 0]),
            (None, Some(ms(10)), vec![0]),
        ],
        spawns: AtomicUsize::new(0),
    };
    let st = one_candidate(&r, scaled());
    assert_eq!(r.spawns.load(Ordering::SeqCst), 2);
    assert!(st.get(Codec::H264).is_some());
    // a candidate busy on both tries is not counted against the session (REQ-209), and
    // is inconclusive
    let r = TimedRunner {
        plan: vec![(Some(ms(5)), None, vec![0, 0])],
        spawns: AtomicUsize::new(0),
    };
    let st = one_candidate(&r, scaled());
    assert_eq!(r.spawns.load(Ordering::SeqCst), 2);
    assert_eq!(st.notes, [(Codec::H264, Vendor::Nvenc, Reason::TimedOut)]);
    assert!(st.slow.is_empty() && st.inconclusive());
    // the classification itself
    assert_eq!(classify(true, None), Activity::Busy);
    assert_eq!(classify(false, Some((10, 40))), Activity::Busy);
    assert_eq!(classify(false, Some((10, 20))), Activity::Idle);
    assert_eq!(classify(false, None), Activity::Unknown);
}

// covers: REQ-209
#[test]
fn a_candidate_that_ran_out_of_time_twice_is_skipped_in_the_next_check() {
    let r = prober(|_| true);
    let slow: BTreeSet<(Codec, Vendor)> = [(Codec::H264, Vendor::Nvenc)].into();
    let st = probe_all(
        &r,
        Path::new("ffmpeg"),
        &all_encoders(),
        &linux(&["/dev/dri/renderD128"]),
        ProbeLimits::default(),
        &slow,
        &AtomicBool::new(false),
    );
    // the next candidate is tried at once; the skipped one is not started and says why
    assert_eq!(st.get(Codec::H264).map(|c| c.vendor), Some(Vendor::Qsv));
    assert!(
        st.notes
            .contains(&(Codec::H264, Vendor::Nvenc, Reason::SkippedSlow))
    );
    assert!(
        !r.calls
            .lock()
            .unwrap()
            .iter()
            .any(|c| c.iter().any(|x| x == "h264_nvenc"))
    );
    // other formats of the same method are not affected
    assert_eq!(st.get(Codec::Hevc).map(|c| c.vendor), Some(Vendor::Nvenc));
    assert!(Reason::SkippedSlow.text().contains("too slow twice"));
    assert!(!Reason::Hanging.text().is_empty());
    // the one a probe finds out of time twice (nothing says it is busy) is reported to be
    // remembered, once
    let hang = FakeRunner::default()
        .on(|_, _| ok(""))
        .hanging_if(|a| a.iter().any(|x| x == "h264_nvenc"));
    let st = probe(
        &hang,
        Path::new("ffmpeg"),
        &[(Codec::H264, vec![Candidate::new(Vendor::Nvenc)])],
        quick(),
        &BTreeSet::new(),
        &AtomicBool::new(false),
    );
    assert_eq!(st.slow, [(Codec::H264, Vendor::Nvenc)]);
}
