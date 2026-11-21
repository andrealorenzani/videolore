// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Tests of the output check's judgement (metadata, samples, PSNR log) and commands.

use super::*;
use std::path::PathBuf;

fn expected(codec: Codec, hvc1: bool) -> Expected {
    Expected {
        codec,
        width: 1280,
        height: 720,
        fps: Fps::new(30, 1).unwrap(),
        hvc1,
        audio: Some(ExpectedAudio {
            codec: "aac".into(),
            sample_rate: 48_000,
            channels: 2,
        }),
    }
}

/// ffprobe JSON of a good H.264 export, with `edit` applied to the video stream.
fn probe_json(edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut v = serde_json::json!({
        "streams": [
            {
                "codec_type": "video",
                "codec_name": "h264",
                "profile": "High",
                "pix_fmt": "yuv420p",
                "width": 1280,
                "height": 720,
                "r_frame_rate": "30/1",
                "avg_frame_rate": "30/1",
                "sample_aspect_ratio": "1:1",
                "codec_tag_string": "avc1",
                "nb_read_packets": "300",
                "duration": "10.000000"
            },
            {
                "codec_type": "audio",
                "codec_name": "aac",
                "sample_rate": "48000",
                "channels": 2
            }
        ],
        "format": { "duration": "10.010000" }
    });
    edit(&mut v["streams"][0]);
    serde_json::to_vec(&v).unwrap()
}

// covers: REQ-063
#[test]
fn metadata_must_match_the_plan() {
    let exp = expected(Codec::H264, false);
    let m = check_metadata(&probe_json(|_| {}), &exp).unwrap();
    assert_eq!(
        m,
        Measured {
            packets: 300,
            duration: Some(10.0)
        }
    );
    // no sample aspect ratio, or the unset form, is fine; a close average rate too
    let ok = probe_json(|v| {
        v.as_object_mut().unwrap().remove("sample_aspect_ratio");
        v["r_frame_rate"] = "60/1".into();
        v["avg_frame_rate"] = "2997/100".into();
    });
    assert_eq!(check_metadata(&ok, &exp).map(|m| m.packets), Ok(300));
    let bad = |edit: fn(&mut Value)| check_metadata(&probe_json(edit), &exp);
    assert_eq!(
        bad(|v| v["codec_name"] = "hevc".into()),
        Err(CheckFailure::Metadata("codec"))
    );
    assert_eq!(
        bad(|v| v["profile"] = "Main".into()),
        Err(CheckFailure::Metadata("profile"))
    );
    assert_eq!(
        bad(|v| v["pix_fmt"] = "yuv420p10le".into()),
        Err(CheckFailure::Metadata("picture format"))
    );
    assert_eq!(
        bad(|v| v["width"] = 1264.into()),
        Err(CheckFailure::Metadata("size"))
    );
    assert_eq!(
        bad(|v| {
            v["r_frame_rate"] = "25/1".into();
            v["avg_frame_rate"] = "25/1".into();
        }),
        Err(CheckFailure::Metadata("frame rate"))
    );
    assert_eq!(
        bad(|v| v["sample_aspect_ratio"] = "4:3".into()),
        Err(CheckFailure::Metadata("pixel shape"))
    );
    assert_eq!(
        bad(|v| v["nb_read_packets"] = "0".into()),
        Err(CheckFailure::FrameCount {
            expected: 1,
            got: 0
        })
    );
    assert_eq!(
        bad(|v| {
            v.as_object_mut().unwrap().remove("nb_read_packets");
        }),
        Err(CheckFailure::Decode)
    );
    // HEVC in MP4 must carry hvc1
    let hevc = |tag: &str| {
        let t = tag.to_string();
        probe_json(move |v| {
            v["codec_name"] = "hevc".into();
            v["profile"] = "Main".into();
            v["codec_tag_string"] = t.into();
        })
    };
    let exp_hevc = expected(Codec::Hevc, true);
    assert_eq!(
        check_metadata(&hevc("hvc1"), &exp_hevc).map(|m| m.packets),
        Ok(300)
    );
    assert_eq!(
        check_metadata(&hevc("hev1"), &exp_hevc),
        Err(CheckFailure::Metadata("hvc1 tag"))
    );
    // audio as planned
    let mut mono = expected(Codec::H264, false);
    mono.audio.as_mut().unwrap().channels = 1;
    assert_eq!(
        check_metadata(&probe_json(|_| {}), &mono),
        Err(CheckFailure::Metadata("audio"))
    );
    let no_audio = br#"{"streams":[{"codec_type":"video","codec_name":"h264","profile":"High","pix_fmt":"yuv420p","width":1280,"height":720,"r_frame_rate":"30/1","nb_read_packets":"3"}]}"#;
    assert_eq!(
        check_metadata(no_audio, &exp),
        Err(CheckFailure::Metadata("audio stream"))
    );
    assert_eq!(
        check_metadata(br#"{"streams":[]}"#, &exp),
        Err(CheckFailure::Metadata("video stream"))
    );
    assert_eq!(check_metadata(b"", &exp), Err(CheckFailure::Decode));
    assert_eq!(check_metadata(b"{}", &exp), Err(CheckFailure::Decode));
    // the duration: the stream's, else the container's
    let d = check_metadata(
        &probe_json(|v| {
            v.as_object_mut().unwrap().remove("duration");
        }),
        &exp,
    )
    .unwrap();
    assert_eq!(d.duration, Some(10.01));
    assert_eq!(audio_codec_of("libopus"), "opus");
    assert_eq!(audio_codec_of("libmp3lame"), "mp3");
    assert_eq!(audio_codec_of("libvorbis"), "vorbis");
    assert_eq!(audio_codec_of("aac"), "aac");
}

// covers: REQ-063
#[test]
fn the_duration_must_match_the_normal_method() {
    let fps = Fps::new(30, 1).unwrap();
    let m = |d| Measured {
        packets: 300,
        duration: d,
    };
    // the graph reports the last frame's time (ffmpeg 6) or its end (ffmpeg 7+)
    assert_eq!(check_duration(&m(Some(10.0)), 9_966_667, fps), Ok(()));
    assert_eq!(check_duration(&m(Some(10.0)), 10_000_000, fps), Ok(()));
    assert_eq!(
        check_duration(&m(Some(9.5)), 10_000_000, fps),
        Err(CheckFailure::Metadata("duration"))
    );
    assert_eq!(
        check_duration(&m(Some(10.2)), 10_000_000, fps),
        Err(CheckFailure::Metadata("duration"))
    );
    assert_eq!(check_duration(&m(None), 10_000_000, fps), Ok(()));
}

// covers: REQ-063
#[test]
fn samples_are_start_middle_and_end_with_neighbours() {
    let s = sample_indices(300);
    assert_eq!(s, [30, 150, 270]);
    let g = neighbours(300, &s);
    assert_eq!(g[0], vec![29, 30, 31]);
    assert_eq!(
        extracted(&g),
        vec![29, 30, 31, 149, 150, 151, 269, 270, 271]
    );
    // very short videos: indices stay inside and are not repeated
    let s = sample_indices(2);
    assert_eq!(s, [0, 1, 1]);
    let g = neighbours(2, &s);
    assert_eq!(g[0], vec![0, 1]);
    assert_eq!(extracted(&g), vec![0, 1]);
    assert_eq!(sample_indices(0), [0, 0, 0]);
}

#[test]
fn psnr_log_is_read() {
    let log = "n:1 mse_avg:6.27 mse_y:5.68 mse_u:7.04 mse_v:7.82 psnr_avg:40.16 psnr_y:40.58 psnr_u:39.66 psnr_v:39.20 \n\
               n:2 mse_avg:0.00 mse_y:0.00 mse_u:0.00 mse_v:0.00 psnr_avg:inf psnr_y:inf psnr_u:inf psnr_v:inf \n\
               garbage line\n";
    assert_eq!(
        parse_psnr_log(log),
        vec![[40.58, 39.66, 39.20], [100.0, 100.0, 100.0]]
    );
    assert!(parse_psnr_log("").is_empty());
    assert_eq!(last_frame(&["frame=3".into(), "fps=1".into()]), Some(3));
    assert_eq!(last_frame(&["fps=1".into()]), None);
}

// covers: REQ-063
#[test]
fn every_sample_needs_one_good_neighbour_on_every_plane() {
    let good = [38.0, 41.0, 40.0];
    assert_eq!(judge_sample(0, &[good; 3]), Ok(()));
    // one bad neighbour is absorbed by the others (alignment off by one)
    assert_eq!(
        judge_sample(1, &[[12.0, 30.0, 30.0], good, [12.0, 30.0, 30.0]]),
        Ok(())
    );
    // green frames: the chroma planes fail
    assert_eq!(
        judge_sample(2, &[[31.0, 9.5, 8.0], [31.0, 9.0, 8.0]]),
        Err(CheckFailure::Picture {
            index: 2,
            psnr: 8.0
        })
    );
    // missing scores (a short file) fail
    assert_eq!(
        judge_sample(1, &[]),
        Err(CheckFailure::Picture {
            index: 1,
            psnr: 0.0
        })
    );
    // the threshold itself
    assert_eq!(judge_sample(0, &[[MIN_PSNR; 3]]), Ok(()));
    assert!(
        CheckFailure::Picture {
            index: 0,
            psnr: 3.0
        }
        .to_string()
        .contains("3.0 dB")
    );
    assert!(
        CheckFailure::FrameCount {
            expected: 3,
            got: 2
        }
        .to_string()
        .contains("2 frames instead of 3")
    );
}

#[test]
fn check_commands() {
    let file = PathBuf::from("/w/.out.mp4.hw.videolore-partial");
    let d = decode_args(&file, Some(2)).join(std::ffi::OsStr::new(" "));
    let d = d.to_string_lossy();
    assert!(d.contains("-xerror"));
    assert!(
        d.contains("-threads 2 -protocol_whitelist file -i file:/w/.out.mp4.hw.videolore-partial")
    );
    assert!(d.contains("-map 0:v:0 -f null -"));
    let dir = tempfile::tempdir().unwrap();
    let a = compare_args(
        argv(["-i", "file:/a.mp4"]),
        1,
        "[0:v:0]null[outv]",
        &file,
        &[1, 2],
        3840,
        2160,
        FfVersion::Release(6, 1, 0),
        Some(4),
        &dir.path().join("g.txt"),
    )
    .unwrap();
    let a: Vec<String> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
    let line = a.join(" ");
    assert!(line.starts_with(
        "-hide_banner -nostdin -v error -filter_complex_threads 4 -i file:/a.mp4 -threads 4"
    ));
    let g = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
    assert_eq!(
        g,
        "[0:v:0]null[outv];[outv]split[all][r];\
         [r]select='eq(n,1)+eq(n,2)',settb=1/1000000,setpts=N,format=yuv420p,scale=1920:1080:flags=area[rs];\
         [1:v:0]select='eq(n,1)+eq(n,2)',settb=1/1000000,setpts=N,format=yuv420p,scale=1920:1080:flags=area[hs];\
         [hs][rs]psnr=stats_file=psnr.log[cmp]"
    );
    assert!(line.contains("-map [all] -f null - -map [cmp] -f null -"));
    assert!(line.ends_with("-progress pipe:1 -nostats"));
    // small pictures are compared as they are
    let a = compare_args(
        Vec::new(),
        0,
        "[0:v]null[outv]",
        &file,
        &[0],
        640,
        360,
        FfVersion::Release(4, 4, 0),
        None,
        &dir.path().join("g.txt"),
    )
    .unwrap();
    assert!(!a.iter().any(|x| x.to_string_lossy().contains("scale=")));
    let m: Vec<String> = metadata_args(&file)
        .iter()
        .map(|x| x.to_string_lossy().into_owned())
        .collect();
    assert!(m.contains(&"-count_packets".to_string()));
    assert_eq!(m.last().unwrap(), "file:/w/.out.mp4.hw.videolore-partial");
}

/// Constant quality must keep the file within 1.5 times the normal method's size.
// covers: REQ-063
#[test]
fn constant_quality_files_keep_the_usual_size() {
    // 2 Mb/s for 10 s: 2.5 MB is usual
    assert_eq!(check_size(2_500_000, 2_000_000, 10_000_000), Ok(()));
    assert_eq!(check_size(3_700_000, 2_000_000, 10_000_000), Ok(()));
    assert!(matches!(
        check_size(4_000_000, 2_000_000, 10_000_000),
        Err(CheckFailure::TooLarge(r)) if r > 1.5
    ));
    assert!(check_size(1, 0, 0).is_ok());
    assert!(
        CheckFailure::TooLarge(2.0)
            .to_string()
            .contains("2.0 times")
    );
}

// covers: REQ-206
#[test]
fn the_check_renders_three_short_ranges_only() {
    let fps = Fps::new(25, 1).unwrap();
    let groups = neighbours(300, &sample_indices(300));
    let ranges = sample_ranges(&groups, fps);
    assert_eq!(ranges.len(), 3);
    assert_eq!(ranges[0].first, 29);
    assert_eq!(ranges[0].from, Micros(1_160_000));
    assert_eq!(ranges[0].local, vec![0, 1, 2]);
    assert_eq!(ranges[0].seek(), Micros(1_159_000));
    // each range is one second long and holds its three frames
    assert_eq!(RANGE_LEN, Micros::SECOND);
    assert!(ranges.iter().all(|r| r.local.iter().all(|k| *k < 25)));
    // a video of one frame: still inside, never more than three
    let tiny = sample_ranges(&neighbours(1, &sample_indices(1)), fps);
    assert!(tiny.len() <= 3 && tiny.iter().all(|r| r.local == vec![0]));
    // the hardware file is read from the start of the video, not before it
    assert_eq!(tiny[0].seek(), Micros::ZERO);
    // NTSC rates round to the nearest microsecond
    let ntsc = sample_ranges(
        &[vec![30], vec![30], vec![30]],
        Fps::new(30_000, 1001).unwrap(),
    );
    assert_eq!(ntsc[0].from, Micros(1_001_000));
}

// covers: REQ-206
#[test]
fn the_short_check_needs_the_frame_count_of_the_plan() {
    assert!(counts_match(250, 250) && counts_match(251, 250) && counts_match(249, 250));
    // a clip that gained a frame from the rounding shifts every later frame
    assert!(!counts_match(300, 250) && !counts_match(243, 250));
}

// covers: REQ-206
#[test]
fn the_range_comparison_reads_one_second_of_each_side() {
    let file = PathBuf::from("/w/.out.mp4.hw.videolore-partial");
    let dir = tempfile::tempdir().unwrap();
    let range = SampleRange {
        first: 10,
        from: Micros(400_000),
        local: vec![1, 2],
    };
    let a = compare_range_args(
        argv(["-i", "file:/a.mp4"]),
        1,
        "[0:v:0]null[outv]",
        &file,
        &range,
        3840,
        2160,
        FfVersion::Release(6, 1, 0),
        Some(4),
        &dir.path().join("g.txt"),
    )
    .unwrap();
    let a: Vec<String> = a.iter().map(|x| x.to_string_lossy().into_owned()).collect();
    let line = a.join(" ");
    assert!(line.starts_with(
        "-hide_banner -nostdin -v error -filter_complex_threads 4 -i file:/a.mp4 -threads 4 \
         -ss 0.399000 -t 2.000000 -protocol_whitelist file -i file:/w/.out.mp4.hw.videolore-partial"
    ));
    let g = &a[a.iter().position(|x| x == "-filter_complex").unwrap() + 1];
    assert_eq!(
        g,
        "[0:v:0]null[outv];[outv]trim=duration=1.000000[r];\
         [r]select='eq(n,1)+eq(n,2)',settb=1/1000000,setpts=N,format=yuv420p,scale=1920:1080:flags=area[rs];\
         [1:v:0]select='eq(n,1)+eq(n,2)',settb=1/1000000,setpts=N,format=yuv420p,scale=1920:1080:flags=area[hs];\
         [hs][rs]psnr=stats_file=psnr.log[cmp]"
    );
    assert!(line.contains("-map [cmp] -f null -") && !line.contains("[all]"));
}
