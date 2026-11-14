// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The export argv of the normal method is frozen: with Export = Best quality (or no
//! working video hardware) videolore must run exactly the commands of 0.3.0 (QG-01).

use super::tests::{enc, project, strs};
use super::*;

/// The 0.3.0 argv, one export per block (`## <name>` then one argument per line).
const GOLDEN: &str = include_str!("../../tests/fixtures/export-argv-0.3.0.txt");

/// The plans of the golden file: stream copy, H.264, HEVC in MP4 and MOV, Light threads
/// and a graph long enough to go through a script file.
pub(crate) fn golden_cases() -> Vec<(&'static str, Plan, Option<u32>)> {
    let mut out = Vec::new();
    let p = project(&[(2, 1280, 720, 30), (3, 1280, 720, 30)]);
    out.push((
        "stream-copy",
        plan_export(&p, &enc(), |_, _| true).unwrap(),
        None,
    ));
    let mut p = project(&[(2, 1280, 720, 30), (3, 640, 360, 25)]);
    p.timeline.toggle_fade(0).unwrap();
    let h264 = plan_export(&p, &enc(), |_, _| true).unwrap();
    out.push(("h264", h264.clone(), None));
    out.push(("h264-light-threads", h264, Some(4)));
    for container in ["mp4", "mov"] {
        let mut p = project(&[(2, 1280, 720, 30), (2, 1280, 720, 30)]);
        for m in &mut p.media {
            m.format.vcodec = Some("hevc".into());
            m.format.container = container.into();
        }
        p.timeline.toggle_fade(0).unwrap();
        let name = if container == "mp4" {
            "hevc-mp4"
        } else {
            "hevc-mov"
        };
        out.push((name, plan_export(&p, &enc(), |_, _| true).unwrap(), None));
    }
    let mut p = project(&[(60, 640, 360, 25)]);
    let m = p.media[0].clone();
    p.timeline.clips.clear();
    for i in 0..200 {
        p.timeline.append(&m).unwrap();
        let id = p.timeline.clips[i].id;
        let a = Micros(i as i64 * 250_000);
        p.timeline
            .trim(id, a, a + Micros(80_000), m.duration, Micros(1))
            .unwrap();
    }
    out.push((
        "script-graph",
        plan_export(&p, &enc(), |_, _| false).unwrap(),
        Some(2),
    ));
    out
}

/// The argv of each golden case with the temporary folder replaced by `<dir>`.
pub(crate) fn render_golden(hw: impl Fn(&Plan) -> Option<crate::hwenc::HwEncode>) -> String {
    let mut text = String::new();
    for (name, plan, threads) in golden_cases() {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("o.mp4");
        let list = dir.path().join("l.txt");
        let script = dir.path().join("g.txt");
        let paths = ExportPaths {
            output: &out,
            concat_list: &list,
            graph_script: &script,
        };
        let hw = hw(&plan);
        let a = export_args(
            &plan,
            &paths,
            FfVersion::Release(6, 1, 0),
            threads,
            hw.as_ref(),
        )
        .unwrap();
        let prefix = dir.path().to_string_lossy().into_owned();
        text.push_str(&format!("## {name}\n"));
        for x in strs(&a) {
            text.push_str(&x.replace(&prefix, "<dir>"));
            text.push('\n');
        }
    }
    text
}

// covers: REQ-062
#[test]
fn best_quality_export_commands_are_those_of_0_3_0() {
    let now = render_golden(|_| None);
    if now != GOLDEN {
        for (a, b) in now.lines().zip(GOLDEN.lines()) {
            assert_eq!(a, b);
        }
        assert_eq!(now.lines().count(), GOLDEN.lines().count());
    }
}

/// The video arguments of the normal method in a golden argv (`-c:v` up to `-pix_fmt`).
fn without_software_video(a: &[String]) -> Vec<String> {
    let start = a.iter().position(|x| x == "-c:v").unwrap();
    let end = a.iter().position(|x| x == "-pix_fmt").unwrap() + 2;
    let mut out = a[..start].to_vec();
    out.extend(a[end..].iter().cloned());
    out.retain(|x| x != "hvc1" && x != "-tag:v");
    out
}

/// A hardware export changes only the video encoding: same inputs, graph (plus the
/// method's tail), audio, container and progress; HEVC in MP4/MOV keeps the hvc1 tag;
/// stream copy is untouched.
// covers: REQ-062
#[test]
fn hardware_commands_change_only_the_video_encoding() {
    use crate::hwenc::{Candidate, Codec, Target, Vendor, encode_args};
    for vendor in Vendor::ALL {
        let cand = Candidate {
            vendor,
            device: (vendor == Vendor::Vaapi).then(|| "/dev/dri/renderD128".into()),
            rc: Default::default(),
        };
        let hw_of = |plan: &Plan| match plan {
            Plan::Reencode { settings, .. } => {
                let codec = Codec::of_software_encoder(&settings.vencoder).unwrap();
                Some(encode_args(
                    &cand,
                    codec,
                    Target::new(settings.video_bitrate.unwrap()),
                ))
            }
            Plan::StreamCopy { .. } => Some(encode_args(&cand, Codec::H264, Target::new(1))),
        };
        let soft = render_golden(|_| None);
        let hard = render_golden(hw_of);
        let blocks = |t: &str| -> Vec<(String, Vec<String>)> {
            t.split("## ")
                .filter(|b| !b.is_empty())
                .map(|b| {
                    let mut l = b.lines().map(str::to_string);
                    (l.next().unwrap(), l.collect())
                })
                .collect()
        };
        for ((name, s), (_, h)) in blocks(&soft).into_iter().zip(blocks(&hard)) {
            if name == "stream-copy" {
                assert_eq!(s, h, "{vendor:?}: stream copy never changes");
                continue;
            }
            let hevc = name.starts_with("hevc");
            let codec = if hevc { Codec::Hevc } else { Codec::H264 };
            let enc = encode_args(&cand, codec, Target::new(1_000_000));
            // the hardware argv, with the method's additions taken out again
            let mut rest = h.clone();
            for g in &enc.global {
                let g = g.to_string_lossy();
                let i = rest.iter().position(|x| *x == g).unwrap();
                assert!(
                    i < rest.iter().position(|x| x == "-i").unwrap(),
                    "global first"
                );
                rest.remove(i);
            }
            let vstart = rest.iter().position(|x| x == "-c:v").unwrap();
            let vlen = enc.vcodec.len();
            let video: Vec<String> = rest.drain(vstart..vstart + vlen).collect();
            assert_eq!(video[1], enc.vcodec[1].to_string_lossy(), "{name}");
            assert!(video.windows(2).any(|w| w == ["-g", "250"]));
            if hevc {
                assert_eq!(
                    &rest[vstart..vstart + 2],
                    ["-tag:v", "hvc1"],
                    "{name} {vendor:?}"
                );
            }
            let label = rest.iter().position(|x| *x == enc.out_label).unwrap();
            assert_eq!(rest[label - 1], "-map");
            rest[label] = "[outv]".into();
            if let Some(g) = rest.iter_mut().find(|x| x.contains("concat=")) {
                assert!(g.ends_with(&enc.graph_tail), "{name}");
                let cut = g.len() - enc.graph_tail.len();
                g.truncate(cut);
            }
            let mut want = without_software_video(&s);
            want.retain(|x| x != "-tag:v" && x != "hvc1");
            rest.retain(|x| x != "-tag:v" && x != "hvc1");
            assert_eq!(rest, want, "{name} {vendor:?}");
        }
    }
}

/// A long graph goes through the script file with the method's tail.
// covers: REQ-062
#[test]
fn hardware_tail_reaches_the_graph_script() {
    use crate::hwenc::{Candidate, Codec, Target, Vendor, encode_args};
    let (_, plan, threads) = golden_cases()
        .into_iter()
        .find(|(n, _, _)| *n == "script-graph")
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let (out, list, script) = (
        dir.path().join("o.mp4"),
        dir.path().join("l.txt"),
        dir.path().join("g.txt"),
    );
    let paths = ExportPaths {
        output: &out,
        concat_list: &list,
        graph_script: &script,
    };
    let cand = Candidate {
        vendor: Vendor::Vaapi,
        device: Some("/dev/dri/renderD129".into()),
        rc: Default::default(),
    };
    let enc = encode_args(&cand, Codec::H264, Target::new(1_000_000));
    let a = strs(
        &export_args(
            &plan,
            &paths,
            FfVersion::Release(7, 1, 0),
            threads,
            Some(&enc),
        )
        .unwrap(),
    );
    assert!(a.windows(2).any(|w| w[0] == "-/filter_complex"));
    let g = std::fs::read_to_string(&script).unwrap();
    assert!(g.ends_with(";[outv]format=nv12,hwupload[outvhw]"));
    assert!(a.windows(2).any(|w| w == ["-map", "[outvhw]"]));
    assert!(a.windows(2).any(|w| w == ["-threads", "2"]));
}
