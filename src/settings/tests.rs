// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the settings decisions (pure) and the settings file.

use super::*;
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

// covers: REQ-057
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

// covers: REQ-057
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
    };
    let b = Choices::default();
    assert_eq!(
        summary(a, b),
        vec![
            "Preview quality: Original → Automatic".to_string(),
            "Subtitle quality: Fast → Balanced".to_string(),
            "How hard videolore works: Maximum speed → Balanced".to_string(),
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
        },
        Some(fp(8, 4, 16)),
    );
    let v = serde_json::to_value(&s).unwrap();
    assert_eq!(v["version"], 2);
    assert_eq!(v["preview_quality"], "smoother");
    assert_eq!(v["subtitle_quality"], "most_accurate");
    assert_eq!(v["effort"], "light");
    assert_eq!(v["machine"]["logical"], 8);
    assert_eq!(v["machine"]["ram_gib"], 16);
    assert!(v["ffmpeg"].is_null());
    for q in SubtitleQuality::ALL {
        assert_eq!(SubtitleQuality::ALL[q.index()], q);
    }
    assert_eq!(Settings::default().version, VERSION);
}
