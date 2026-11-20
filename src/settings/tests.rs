// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the settings decisions (pure) and the settings file.

use super::*;
use crate::base::io::read_capped;
use crate::hwenc::{Codec, Vendor};
use crate::machine::{CpuFeatures, GIB, Machine};

fn fp(logical: u32, physical: u32, ram_gib: u64) -> Fingerprint {
    Machine::simulated(logical, physical, ram_gib * GIB).fingerprint()
}

fn bytes(json: &str) -> Loaded {
    Loaded::Bytes(json.as_bytes().to_vec())
}

fn saved(s: &Settings) -> Loaded {
    Loaded::Bytes(serde_json::to_vec(s).unwrap())
}

// covers: REQ-045
#[test]
fn first_start_saves_recommended_settings_and_notifies_once() {
    let m = fp(8, 4, 16);
    let first = startup(&Loaded::Missing, &m);
    assert_eq!(first.notice, Some(StartNotice::Setup));
    assert!(first.write);
    assert_eq!(first.settings.choices(), recommend(&m));
    assert_eq!(first.settings.machine.as_ref(), Some(&m));
    assert_eq!(first.settings.version, VERSION);
    // second start with what was saved: no notice, nothing recomputed or written
    let second = startup(&saved(&first.settings), &m);
    assert_eq!(second.notice, None);
    assert!(!second.write);
    assert_eq!(second.settings, first.settings);
    // same machine characteristics, same decision
    assert_eq!(startup(&Loaded::Missing, &m), first);
    assert_eq!(recommend(&m), recommend(&m.clone()));
    assert_eq!(StartNotice::Setup.text(), SETUP_NOTICE);
}

// covers: REQ-045
#[test]
fn first_start_through_the_settings_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cfg").join("settings.json");
    let m = fp(4, 2, 8);
    assert_eq!(load_file(&path), Loaded::Missing);
    let st = startup(&load_file(&path), &m);
    save(&st.settings, &path).unwrap();
    let again = startup(&load_file(&path), &m);
    assert_eq!(again.notice, None);
    assert_eq!(again.settings, st.settings);
    assert!(Settings::default_path().is_some() || cfg!(not(unix)));
}

// covers: REQ-048
#[test]
fn a_different_machine_with_recommended_settings_is_set_up_again() {
    let old = fp(4, 2, 8);
    let new = fp(16, 8, 32);
    let s = Settings::with(recommend(&old), Some(old.clone()));
    assert_eq!(s.subtitle_quality, SubtitleQuality::Balanced);
    let st = startup(&saved(&s), &new);
    assert_eq!(st.notice, Some(StartNotice::Setup));
    assert!(st.write);
    assert_eq!(st.settings.choices(), recommend(&new));
    assert_eq!(st.settings.subtitle_quality, SubtitleQuality::MostAccurate);
    assert_eq!(st.settings.machine, Some(new));
}

// covers: REQ-048
#[test]
fn a_different_machine_keeps_custom_settings_and_offers_the_recommended_ones() {
    let old = fp(8, 4, 16);
    let new = fp(4, 4, 8);
    let mut s = Settings::with(recommend(&old), Some(old));
    s.effort = Effort::Maximum;
    s.ffmpeg = Some(PathBuf::from("/opt/ff/ffmpeg"));
    let st = startup(&saved(&s), &new);
    assert_eq!(st.notice, Some(StartNotice::MachineChanged));
    assert!(st.write);
    assert_eq!(st.settings.choices(), s.choices(), "custom choices win");
    assert_eq!(st.settings.ffmpeg, s.ffmpeg);
    assert_eq!(st.settings.machine, Some(new.clone()));
    // after that, the new machine is known: no more notices
    assert_eq!(startup(&saved(&st.settings), &new).notice, None);
    // a file without machine data counts as another machine
    let mut nomachine = s.clone();
    nomachine.machine = None;
    assert_eq!(
        startup(&saved(&nomachine), &new).notice,
        Some(StartNotice::MachineChanged)
    );
}

// covers: REQ-049
#[test]
fn upgrade_from_0_2_keeps_proxies_off_and_the_ffmpeg_choice() {
    let m = fp(8, 4, 16);
    let old = r#"{"ffmpeg":"/opt/ff/ffmpeg","ffprobe":"/opt/ff/ffprobe","proxies_enabled":false}"#;
    let st = startup(&bytes(old), &m);
    assert_eq!(st.notice, Some(StartNotice::Setup));
    assert!(st.write);
    assert_eq!(st.settings.preview_quality, PreviewQuality::Original);
    assert_eq!(st.settings.ffmpeg, Some(PathBuf::from("/opt/ff/ffmpeg")));
    assert_eq!(st.settings.ffprobe, Some(PathBuf::from("/opt/ff/ffprobe")));
    assert_eq!(st.settings.subtitle_quality, recommend(&m).subtitles);
    assert_eq!(st.settings.effort, Effort::Balanced);
    // 0.2.x with proxies on (or 0.1.x without the field): a normal first start
    for old in [
        r#"{"ffmpeg":null,"ffprobe":null,"proxies_enabled":true}"#,
        r#"{"ffmpeg":null,"ffprobe":null}"#,
    ] {
        let st = startup(&bytes(old), &m);
        assert_eq!(st.settings.choices(), recommend(&m));
        assert_eq!(st.settings.ffmpeg, None);
    }
    // a relative override is dropped and reported
    let st = startup(&bytes(r#"{"ffmpeg":"ffmpeg","ffprobe":null}"#), &m);
    assert_eq!(st.settings.ffmpeg, None);
    assert_eq!(st.notice, Some(StartNotice::Malformed));
}

// covers: REQ-057, REQ-151, REQ-152
#[test]
fn malformed_settings_fall_back_to_recommended_with_a_notice() {
    let m = fp(8, 4, 16);
    let good = serde_json::to_value(Settings::with(recommend(&m), Some(m.clone()))).unwrap();
    let with = |key: &str, v: serde_json::Value| {
        let mut x = good.clone();
        x[key] = v;
        x.to_string()
    };
    let mut huge_machine = good.clone();
    huge_machine["machine"]["logical"] = serde_json::json!(u64::MAX);
    let mut zero_cores = good.clone();
    zero_cores["machine"]["physical"] = serde_json::json!(0);
    let mut much_ram = good.clone();
    much_ram["machine"]["ram_gib"] = serde_json::json!(1u64 << 40);
    let deep = "[".repeat(5000) + &"]".repeat(5000);
    let corpus = [
        "".to_string(),
        "{".to_string(),
        "null".to_string(),
        "[1,2]".to_string(),
        "\"text\"".to_string(),
        deep,
        with("preview_quality", serde_json::json!("ultra")),
        with("effort", serde_json::json!(7)),
        with("subtitle_quality", serde_json::json!(null)),
        with("version", serde_json::json!(-1)),
        with("version", serde_json::json!(1)),
        with("version", serde_json::json!("2")),
        with("ffmpeg", serde_json::json!(12)),
        huge_machine.to_string(),
        zero_cores.to_string(),
        much_ram.to_string(),
        r#"{"version":2}"#.to_string(),
        r#"{"proxies_enabled":"maybe"}"#.to_string(),
        "{\"version\":2,\"preview_quality\":\"automatic\u{0}\"}".to_string(),
    ];
    for c in corpus {
        let st = startup(&bytes(&c), &m);
        assert_eq!(st.notice, Some(StartNotice::Malformed), "{c:.80}");
        assert!(st.write && st.backup, "{c:.80}");
        assert_eq!(st.settings.choices(), recommend(&m));
        assert_eq!(st.settings.ffmpeg, None);
    }
    // an unusable file (too big, not a regular file): recommended, no copy
    let st = startup(&Loaded::Unusable, &m);
    assert_eq!(st.notice, Some(StartNotice::Malformed));
    assert!(!st.backup);
    // control characters in an override path: kept only if absolute, never run here
    let st = startup(
        &bytes(&with("ffmpeg", serde_json::json!("/opt/ff\n/ffmpeg"))),
        &m,
    );
    assert_eq!(st.notice, None);
    // unknown fields are ignored (forward compatible)
    let st = startup(&bytes(&with("future", serde_json::json!({"x": 1}))), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write);
    // a file from a newer videolore: recommended values in memory, file left alone
    let st = startup(&bytes(&with("version", serde_json::json!(3))), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write && !st.backup);
    assert_eq!(st.settings.choices(), recommend(&m));
    // relative override in a current file: dropped, reported, written
    let st = startup(&bytes(&with("ffprobe", serde_json::json!("ffprobe"))), &m);
    assert_eq!(st.notice, Some(StartNotice::Malformed));
    assert!(st.write && !st.backup);
    assert_eq!(st.settings.ffprobe, None);
}

// covers: REQ-057, REQ-151, REQ-152
#[test]
fn settings_files_are_read_safely_and_bad_ones_kept_aside() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    // too big
    std::fs::write(&path, vec![b' '; MAX_SETTINGS_BYTES as usize + 1]).unwrap();
    assert_eq!(load_file(&path), Loaded::Unusable);
    // a folder
    assert_eq!(load_file(dir.path()), Loaded::Unusable);
    // a bad file is copied aside before being replaced
    std::fs::write(&path, b"{oops").unwrap();
    let loaded = load_file(&path);
    assert_eq!(loaded, Loaded::Bytes(b"{oops".to_vec()));
    backup(&path, &loaded);
    assert_eq!(std::fs::read(backup_path(&path)).unwrap(), b"{oops");
    backup(&path, &Loaded::Missing);
    assert!(read_capped(&b"12345"[..], 4).is_err());
    assert_eq!(read_capped(&b"1234"[..], 4).unwrap(), b"1234");
    // a FIFO is refused at once instead of blocking
    #[cfg(unix)]
    {
        let fifo = dir.path().join("fifo.json");
        let ok = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            let t = std::time::Instant::now();
            assert_eq!(load_file(&fifo), Loaded::Unusable);
            assert!(t.elapsed() < std::time::Duration::from_secs(1));
        }
    }
    // saving into a folder that cannot be created fails cleanly
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, b"x").unwrap();
    assert!(save(&Settings::default(), &blocker.join("sub").join("s.json")).is_err());
}

/// Worked values of the effort table (logical, physical, RAM GiB) -> (Light cap, Light
/// threads, Light speech, Balanced cap, Balanced speech, Maximum cap, Maximum speech).
/// (logical, physical, RAM GiB).
type Spec = (u32, u32, u64);
/// (Light cap, Light threads, Light speech, Balanced cap, Balanced speech, Maximum cap,
/// Maximum speech).
type Expected = (u8, u32, usize, u8, usize, u8, usize);

const TABLE: &[(Spec, Expected)] = &[
    ((1, 1, 16), (3, 1, 1, 3, 1, 3, 1)),
    ((2, 1, 16), (3, 1, 1, 3, 1, 3, 1)),
    ((4, 2, 16), (3, 2, 2, 3, 2, 3, 2)),
    ((8, 4, 16), (3, 4, 4, 3, 4, 4, 4)),
    ((8, 4, 4), (3, 4, 4, 3, 4, 3, 4)),
    ((8, 8, 16), (3, 4, 4, 3, 7, 4, 8)),
    ((16, 8, 16), (3, 8, 8, 3, 8, 6, 8)),
    ((32, 16, 16), (3, 16, 8, 3, 8, 6, 16)),
];

// covers: REQ-050
#[test]
fn effort_mapping_matches_the_documented_table() {
    for &((l, p, ram), (lc, lt, ls, bc, bs, mc, ms)) in TABLE {
        let f = fp(l, p, ram);
        let light = budget(Effort::Light, &f);
        let bal = budget(Effort::Balanced, &f);
        let max = budget(Effort::Maximum, &f);
        let got = (
            light.max_processes,
            light.threads.unwrap(),
            light.stt_threads,
            bal.max_processes,
            bal.stt_threads,
            max.max_processes,
            max.stt_threads,
        );
        assert_eq!(got, (lc, lt, ls, bc, bs, mc, ms), "L{l} P{p} {ram} GiB");
        assert_eq!(bal.threads, None);
        assert_eq!(max.threads, None);
    }
}

// covers: REQ-050
#[test]
fn effort_guarantees_hold_for_every_machine() {
    for l in [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 64, 128] {
        for p in [0, 1, l / 2, l] {
            for ram in [0, 4, 7, 8, 15, 16, 64] {
                let f = fp(l, p, ram);
                let half = (l / 2).max(1);
                let light = budget(Effort::Light, &f);
                let bal = budget(Effort::Balanced, &f);
                let max = budget(Effort::Maximum, &f);
                for b in [light, bal, max] {
                    assert!(b.stt_threads >= 1 && b.stt_threads <= l as usize);
                    assert!(b.threads.is_none_or(|t| t >= 1 && t <= l));
                    assert!((BASE_PROCESS_CAP..=MAX_PROCESS_CAP).contains(&b.max_processes));
                }
                assert!(light.threads.unwrap() <= half);
                assert!(light.stt_threads <= half as usize);
                // never fewer speech threads than 0.2.2 at the default level
                assert!(bal.stt_threads >= (l / 2).clamp(1, 4) as usize);
                assert!(light.stt_threads <= bal.stt_threads);
                assert!(bal.stt_threads <= max.stt_threads.max(bal.stt_threads));
                assert!(light.max_processes <= max.max_processes);
                if ram != 0 && ram < 7 {
                    assert_eq!(max.max_processes, BASE_PROCESS_CAP);
                }
            }
        }
    }
}

// covers: REQ-052, REQ-054
#[test]
fn recommendation_depends_on_the_machine_and_never_picks_a_greyed_level() {
    use SubtitleQuality::*;
    let cases = [
        (fp(2, 1, 4), Fast),
        (fp(4, 2, 4), Fast),
        (fp(4, 2, 8), Balanced),
        (fp(8, 4, 16), Balanced),
        (fp(12, 6, 16), MostAccurate),
        (fp(16, 8, 8), Balanced),
        (fp(32, 16, 64), MostAccurate),
        (fp(16, 8, 0), Fast),
    ];
    for (f, want) in cases {
        let r = recommend(&f);
        assert_eq!(r.subtitles, want, "{f:?}");
        assert_eq!(r.preview, PreviewQuality::Automatic);
        assert_eq!(r.effort, Effort::Balanced);
        assert!(availability(&f)[r.subtitles.index()].is_none());
    }
    // memory greys out the bigger levels with a plain reason
    assert_eq!(
        availability(&fp(8, 4, 3)),
        [None, Some(MEMORY_REASON), Some(MEMORY_REASON)]
    );
    assert_eq!(
        availability(&fp(8, 4, 6)),
        [None, None, Some(MEMORY_REASON)]
    );
    assert_eq!(availability(&fp(8, 4, 16)), [None, None, None]);
    assert_eq!(availability(&fp(8, 4, 0)), [None, None, None]);
    // an old processor: everything greyed, Fast recommended (never run)
    let mut m = Machine::simulated(16, 8, 64 * GIB);
    m.arch = "x86_64";
    m.cpu = CpuFeatures {
        avx2: false,
        ..CpuFeatures::ALL
    };
    let old = m.fingerprint();
    assert_eq!(availability(&old), [Some(TOO_OLD_REASON); 3]);
    assert_eq!(recommend(&old).subtitles, Fast);
}

const BANNED: &[&str] = &[
    "thread",
    "threads",
    "core",
    "cores",
    "proxy",
    "proxies",
    "codec",
    "encoder",
    "ffmpeg",
    "avx",
    "avx2",
    "cpu",
    "gpu",
    "model",
    "models",
    "quantised",
    "quantized",
];

// covers: REQ-046
#[test]
fn main_settings_use_no_technical_words() {
    let texts = main_texts();
    assert!(texts.len() > 20);
    for t in &texts {
        for word in t
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
        {
            let w = word.to_lowercase();
            assert!(!BANNED.contains(&w.as_str()), "'{word}' in \"{t}\"");
        }
    }
    // every choice has a label and a one-line explanation
    for p in PreviewQuality::ALL {
        assert!(texts.contains(&p.label()) && !p.explanation().contains('\n'));
    }
    for s in SubtitleQuality::ALL {
        assert!(texts.contains(&s.label()) && !s.explanation().contains('\n'));
    }
    for e in Effort::ALL {
        assert!(texts.contains(&e.label()) && !e.explanation().contains('\n'));
    }
}

// covers: REQ-047
#[test]
fn summary_describes_what_changed() {
    let a = Choices {
        preview: PreviewQuality::Original,
        subtitles: SubtitleQuality::Fast,
        effort: Effort::Maximum,
        export: ExportChoice::Faster,
    };
    let b = Choices::default();
    assert_eq!(
        summary(a, b),
        vec![
            "Preview quality: Original → Automatic".to_string(),
            "Subtitle quality: Fast → Balanced".to_string(),
            "How hard videolore works: Maximum speed → Balanced".to_string(),
            "Export: Faster → Best quality".to_string(),
        ]
    );
    assert_eq!(summary(b, b), vec![ALREADY_RECOMMENDED.to_string()]);
    let mut s = Settings::default();
    s.set_choices(a);
    assert_eq!(s.choices(), a);
    assert!(!s.is_recommended(&fp(8, 4, 16)));
    s.set_choices(recommend(&fp(8, 4, 16)));
    assert!(s.is_recommended(&fp(8, 4, 16)));
}

#[test]
fn settings_file_format_is_stable() {
    let s = Settings::with(
        Choices {
            preview: PreviewQuality::Smoother,
            subtitles: SubtitleQuality::MostAccurate,
            effort: Effort::Light,
            export: ExportChoice::Faster,
        },
        Some(fp(8, 4, 16)),
    );
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["version"], 2);
    assert_eq!(v["preview_quality"], "smoother");
    assert_eq!(v["subtitle_quality"], "most_accurate");
    assert_eq!(v["effort"], "light");
    assert_eq!(v["export"], "faster");
    assert_eq!(v["hardware_off"], serde_json::json!([]));
    assert_eq!(v["machine"]["logical"], 8);
    assert_eq!(v["machine"]["ram_gib"], 16);
    assert!(v["ffmpeg"].is_null());
    for q in SubtitleQuality::ALL {
        assert_eq!(SubtitleQuality::ALL[q.index()], q);
    }
    assert_eq!(Settings::default().version, VERSION);
}

/// The settings file of videolore 0.3.0 (no Export choice, no hardware memory).
fn file_0_3_0(m: &Fingerprint, effort: &str) -> String {
    let machine = serde_json::to_string(m).unwrap();
    let rec = recommend(m);
    let subs = serde_json::to_value(rec.subtitles).unwrap();
    format!(
        r#"{{"version":2,"preview_quality":"automatic","subtitle_quality":{subs},"effort":"{effort}","ffmpeg":null,"ffprobe":null,"machine":{machine}}}"#
    )
}

// covers: REQ-065
#[test]
fn upgrade_from_0_3_keeps_the_status_and_loads_best_quality() {
    let m = fp(8, 4, 16);
    // recommended settings stay recommended, silently, and the file is not rewritten
    let st = startup(&bytes(&file_0_3_0(&m, "balanced")), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write && !st.backup);
    assert_eq!(st.settings.export, ExportChoice::BestQuality);
    assert!(st.settings.hardware_off.is_empty());
    assert!(st.settings.is_recommended(&m));
    // custom settings stay custom
    let st = startup(&bytes(&file_0_3_0(&m, "maximum")), &m);
    assert_eq!(st.notice, None);
    assert_eq!(st.settings.export, ExportChoice::BestQuality);
    assert!(!st.settings.is_recommended(&m));
    // 0.2.x upgrades and damaged files get Best quality too
    let st = startup(&bytes(r#"{"ffmpeg":null,"proxies_enabled":false}"#), &m);
    assert_eq!(st.settings.export, ExportChoice::BestQuality);
    let st = startup(&bytes("{"), &m);
    assert_eq!(st.notice, Some(StartNotice::Malformed));
    assert_eq!(st.settings.export, ExportChoice::BestQuality);
    // an unknown Export value is a damaged file
    let st = startup(
        &bytes(
            &file_0_3_0(&m, "balanced")
                .replace("\"version\":2", "\"export\":\"turbo\",\"version\":2"),
        ),
        &m,
    );
    assert_eq!(st.notice, Some(StartNotice::Malformed));
    assert_eq!(st.settings.export, ExportChoice::BestQuality);
}

// covers: REQ-060
#[test]
fn best_quality_is_the_recommendation_everywhere_and_faster_is_custom() {
    for m in [fp(2, 1, 2), fp(4, 2, 8), fp(16, 8, 64), fp(64, 32, 256)] {
        assert_eq!(recommend(&m).export, ExportChoice::BestQuality);
        // first start saves Best quality
        let first = startup(&Loaded::Missing, &m);
        assert_eq!(first.settings.export, ExportChoice::BestQuality);
        let saved_json = serde_json::to_value(&first.settings).unwrap();
        assert_eq!(saved_json["export"], "best_quality");
        // choosing Faster makes the settings custom
        let mut s = first.settings.clone();
        s.export = ExportChoice::Faster;
        assert!(!s.is_recommended(&m));
        assert_eq!(s.choices().export, ExportChoice::Faster);
        // "Use recommended settings" goes back to Best quality and says so
        let new = recommend(&m);
        assert_eq!(
            summary(s.choices(), new),
            vec!["Export: Faster → Best quality".to_string()]
        );
        s.set_choices(new);
        assert_eq!(s.export, ExportChoice::BestQuality);
        assert!(s.is_recommended(&m));
    }
    // the choice is saved and read back
    let m = fp(8, 4, 16);
    let mut s = Settings::with(recommend(&m), Some(m.clone()));
    s.export = ExportChoice::Faster;
    let st = startup(&saved(&s), &m);
    assert_eq!(st.settings.export, ExportChoice::Faster);
    assert_eq!(st.notice, None);
    // the main section shows both choices with their one-line explanations
    let texts = main_texts();
    for e in ExportChoice::ALL {
        assert!(texts.contains(&e.label()) && texts.contains(&e.explanation()));
        assert!(!e.explanation().contains('\n'));
    }
    assert!(texts.contains(&EXPORT_TITLE) && texts.contains(&NO_HARDWARE_NOTE));
    assert!(
        ExportChoice::Faster
            .explanation()
            .contains("video hardware")
    );
}

fn off(format: &str, method: &str, version: &str) -> serde_json::Value {
    serde_json::json!({"format": format, "method": method, "version": version})
}

// covers: REQ-064
#[test]
fn hardware_memory_is_validated_entry_by_entry() {
    let m = fp(8, 4, 16);
    let v = "1.2.3";
    let mut file = serde_json::to_value(Settings::with(recommend(&m), Some(m.clone()))).unwrap();
    file["hardware_off"] = serde_json::json!([
        off("h264", "nvenc", v),
        off("hevc", "vaapi", v),
        off("av1", "nvenc", v),             // unknown format
        off("h264", "quantum", v),          // unknown method
        off("h264", "qsv", &"9".repeat(40)), // version too long
        off("h264", "qsv", ""),             // no version
        off("h264", "nvenc", v),            // repeated
        42,
        {"format": "hevc"},
    ]);
    let st = startup_at(&bytes(&file.to_string()), &m, v);
    assert_eq!(st.notice, None, "bad entries do not make the file damaged");
    assert_eq!(
        st.settings.hardware_off,
        vec![
            HardwareOff {
                format: Codec::H264,
                method: Vendor::Nvenc,
                version: v.into()
            },
            HardwareOff {
                format: Codec::Hevc,
                method: Vendor::Vaapi,
                version: v.into()
            },
        ]
    );
    assert!(st.settings.hardware_is_off(Codec::H264, Vendor::Nvenc));
    assert!(!st.settings.hardware_is_off(Codec::Hevc, Vendor::Nvenc));
    // not a list at all: nothing remembered, file still fine
    file["hardware_off"] = serde_json::json!("nvenc");
    let st = startup_at(&bytes(&file.to_string()), &m, v);
    assert_eq!(st.notice, None);
    assert!(st.settings.hardware_off.is_empty());
    // at most 8 entries are kept
    file["hardware_off"] = serde_json::Value::Array(
        (0..20)
            .map(|i| {
                off(
                    if i % 2 == 0 { "h264" } else { "hevc" },
                    "nvenc",
                    &format!("{v}-{i}"),
                )
            })
            .collect(),
    );
    let st = startup_at(&bytes(&file.to_string()), &m, v);
    assert!(
        st.settings.hardware_off.is_empty(),
        "other versions are forgotten"
    );
    let mut s = Settings::default();
    for i in 0..20 {
        s.turn_hardware_off(
            if i % 2 == 0 { Codec::H264 } else { Codec::Hevc },
            Vendor::ALL[i % 5],
            v,
        );
    }
    assert_eq!(s.hardware_off.len(), MAX_HARDWARE_OFF);
    let n = s.hardware_off.len();
    s.turn_hardware_off(s.hardware_off[0].format, s.hardware_off[0].method, v);
    assert_eq!(s.hardware_off.len(), n, "no duplicates");
}

/// A remembered problem is forgotten when the program version or the computer changes.
// covers: REQ-064
#[test]
fn hardware_memory_resets_with_a_new_version_or_computer() {
    let m = fp(8, 4, 16);
    let mut s = Settings::with(recommend(&m), Some(m.clone()));
    s.export = ExportChoice::Faster;
    s.turn_hardware_off(Codec::H264, Vendor::Qsv, "0.4.0");
    // same version, same computer: kept, nothing written
    let st = startup_at(&saved(&s), &m, "0.4.0");
    assert!(st.settings.hardware_is_off(Codec::H264, Vendor::Qsv));
    assert!(!st.write);
    // a new version: forgotten (and the file updated); the choice stays
    let st = startup_at(&saved(&s), &m, "0.4.1");
    assert!(st.settings.hardware_off.is_empty());
    assert!(st.write);
    assert_eq!(st.settings.export, ExportChoice::Faster);
    // another computer: forgotten
    let st = startup_at(&saved(&s), &fp(16, 8, 32), "0.4.0");
    assert!(st.settings.hardware_off.is_empty());
    assert_eq!(st.notice, Some(StartNotice::MachineChanged));
    // the running version is the default
    s.hardware_off.clear();
    s.turn_hardware_off(Codec::Hevc, Vendor::Amf, env!("CARGO_PKG_VERSION"));
    assert!(
        startup(&saved(&s), &m)
            .settings
            .hardware_is_off(Codec::Hevc, Vendor::Amf)
    );
}

/// The settings file of videolore 1.0 (no layout, no recent projects).
fn file_1_0(m: &Fingerprint) -> serde_json::Value {
    let mut v = serde_json::to_value(Settings::with(recommend(m), Some(m.clone()))).unwrap();
    let o = v.as_object_mut().unwrap();
    o.remove("layout");
    o.remove("recent");
    v
}

// covers: REQ-067, REQ-080
#[test]
fn files_of_1_0_load_silently_and_1_1_values_round_trip() {
    let m = fp(8, 4, 16);
    let st = startup(&bytes(&file_1_0(&m).to_string()), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write && !st.backup);
    assert_eq!(st.settings.layout, None);
    assert!(st.settings.recent.is_empty());
    // 1.1 values are kept as they are
    let mut s = st.settings.clone();
    s.layout = Some(SavedLayout {
        top: 0.5,
        media: 0.25,
        timeline: 0.6,
    });
    let root = if cfg!(windows) { "C:\\p" } else { "/p" };
    for i in 0..3 {
        push_recent(&mut s.recent, &PathBuf::from(format!("{root}/{i}.vlproj")));
    }
    let st = startup(&saved(&s), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write);
    assert_eq!(st.settings, s);
    // no layout saved: the field is left out (1.0 files look the same)
    let v = serde_json::to_value(Settings::with(recommend(&m), None)).unwrap();
    assert!(v.get("layout").is_none());
    assert_eq!(v["recent"], serde_json::json!([]));
    assert!(SavedLayout::DEFAULT.is_valid());
    assert_eq!(SavedLayout::default(), SavedLayout::DEFAULT);
}

// covers: REQ-057, REQ-067, REQ-080
#[test]
fn damaged_layout_and_recent_values_give_defaults_and_keep_the_rest() {
    let m = fp(8, 4, 16);
    let root = if cfg!(windows) { "C:\\p\\" } else { "/p/" };
    let base = || {
        let mut f = file_1_0(&m);
        f["effort"] = serde_json::json!("maximum");
        f
    };
    let bad_layouts = [
        serde_json::json!({"top": -0.5, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": 0.0, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": 1.0, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": 2.5, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": 1e300, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": null, "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": "half", "media": 0.3, "timeline": 0.7}),
        serde_json::json!({"top": 0.5}),
        serde_json::json!([[0.5, 0.3, 0.7]]),
        serde_json::json!("wide"),
        serde_json::json!(42),
        serde_json::json!(null),
    ];
    for layout in bad_layouts {
        let mut f = base();
        f["layout"] = layout.clone();
        let st = startup(&bytes(&f.to_string()), &m);
        assert_eq!(st.notice, None, "{layout}");
        assert!(!st.write && !st.backup, "{layout}");
        assert_eq!(st.settings.layout, None, "{layout}");
        assert_eq!(st.settings.effort, Effort::Maximum, "other settings kept");
    }
    // the recent list: entry by entry
    let long = format!("{root}{}", "x".repeat(5000));
    let mut f = base();
    f["recent"] = serde_json::json!([
        format!("{root}a.vlproj"),
        "relative/b.vlproj",
        format!("{root}c\u{1}.vlproj"),
        format!("{root}d\n.vlproj"),
        long,
        42,
        null,
        ["nested"],
        format!("{root}a.vlproj"),
        format!("{root}e.vlproj"),
    ]);
    let st = startup(&bytes(&f.to_string()), &m);
    assert_eq!(st.notice, None);
    assert!(!st.write);
    assert_eq!(
        st.settings.recent,
        vec![
            PathBuf::from(format!("{root}a.vlproj")),
            PathBuf::from(format!("{root}e.vlproj"))
        ]
    );
    assert_eq!(st.settings.effort, Effort::Maximum);
    // not a list, or a huge one
    for recent in [
        serde_json::json!("x"),
        serde_json::json!({"a": 1}),
        serde_json::Value::Array(
            (0..10_000)
                .map(|i| serde_json::json!(format!("{root}{i}.vlproj")))
                .collect(),
        ),
    ] {
        let mut f = base();
        f["recent"] = recent;
        let st = startup_at(&bytes(&f.to_string()), &m, "1");
        assert_eq!(st.notice, None);
        assert!(st.settings.recent.len() <= MAX_RECENT);
    }
}

// covers: REQ-080
#[test]
fn recent_projects_are_most_recent_first_without_duplicates() {
    let root = if cfg!(windows) { "C:\\p\\" } else { "/p/" };
    let p = |i: usize| PathBuf::from(format!("{root}{i}.vlproj"));
    let mut list = Vec::new();
    for i in 0..12 {
        assert!(push_recent(&mut list, &p(i)));
    }
    assert_eq!(list, (2..12).rev().map(p).collect::<Vec<_>>());
    // opening one again moves it first
    assert!(push_recent(&mut list, &p(5)));
    assert_eq!(list[0], p(5));
    assert_eq!(list.len(), MAX_RECENT);
    assert_eq!(list.iter().filter(|x| **x == p(5)).count(), 1);
    assert!(!push_recent(&mut list, &p(5)), "already first: no change");
    // never remembered: relative, control characters, too long
    for bad in [
        PathBuf::from("rel.vlproj"),
        PathBuf::from(format!("{root}a\tb.vlproj")),
        PathBuf::from(format!("{root}{}", "y".repeat(MAX_RECENT_PATH))),
    ] {
        assert!(!push_recent(&mut list, &bad));
    }
    assert_eq!(list.len(), MAX_RECENT);
}

#[cfg(unix)]
// covers: REQ-080
#[test]
fn non_utf8_paths_are_never_remembered() {
    use std::os::unix::ffi::OsStrExt;
    let p = PathBuf::from(std::ffi::OsStr::from_bytes(b"/p/\xff\xfe.vlproj"));
    let mut list = Vec::new();
    assert!(!push_recent(&mut list, &p));
    assert!(list.is_empty());
}

// covers: REQ-080, REQ-057
#[test]
fn the_largest_settings_file_fits_the_size_limit() {
    let m = fp(4096, 4096, 1 << 20);
    let mut s = Settings::with(recommend(&m), Some(m.clone()));
    s.ffmpeg = Some(PathBuf::from(format!("/{}", "f".repeat(4000))));
    s.ffprobe = s.ffmpeg.clone();
    s.layout = Some(SavedLayout::DEFAULT);
    let root = if cfg!(windows) { "C:\\" } else { "/" };
    for i in 0..MAX_RECENT {
        let name = format!("{root}{i}{}", "\u{10348}".repeat((MAX_RECENT_PATH - 8) / 4));
        assert!(push_recent(&mut s.recent, Path::new(&name)), "{i}");
    }
    for i in 0..MAX_HARDWARE_OFF {
        s.turn_hardware_off(
            Codec::H264,
            Vendor::ALL[i % Vendor::ALL.len()],
            &"9".repeat(32),
        );
        s.turn_hardware_off(
            Codec::Hevc,
            Vendor::ALL[i % Vendor::ALL.len()],
            &"9".repeat(32),
        );
    }
    let json = serde_json::to_string_pretty(&s).unwrap();
    assert!(
        (json.len() as u64) < MAX_SETTINGS_BYTES,
        "{} bytes",
        json.len()
    );
}

// covers: REQ-096, REQ-057
#[test]
fn show_boxes_defaults_on_survives_save_and_ignores_hostile_values() {
    let m = fp(8, 4, 16);
    // files of 1.2.5 have no such field: on, and nothing is rewritten
    let mut old = file_1_0(&m);
    old.as_object_mut().unwrap().remove("show_boxes");
    let st = startup(&bytes(&old.to_string()), &m);
    assert!(st.settings.show_boxes);
    assert!(!st.write);
    // turned off: written and read back
    let mut s = st.settings.clone();
    s.show_boxes = false;
    let st = startup(&saved(&s), &m);
    assert!(!st.settings.show_boxes);
    assert_eq!(st.notice, None);
    assert_eq!(st.settings, s);
    // anything that is not a yes/no is the default, the rest of the file stays valid
    for bad in [
        serde_json::json!("no"),
        serde_json::json!(0),
        serde_json::json!(null),
        serde_json::json!([false]),
    ] {
        let mut f = file_1_0(&m);
        f["effort"] = serde_json::json!("maximum");
        f["show_boxes"] = bad;
        let st = startup(&bytes(&f.to_string()), &m);
        assert!(st.settings.show_boxes);
        assert_eq!(st.settings.effort, Effort::Maximum);
    }
}
