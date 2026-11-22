// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Real-ffmpeg tests of 2.2.0: the cost of the blurred fill in playback (REQ-314) and the
//! second-clip check of the speech-region timing (REQ-177).

use super::*;
use std::time::Instant;

/// Seconds ffmpeg takes to run `graph` over `frames` frames of a `size` test picture.
fn seconds(env: &Env, size: &str, frames: u32, graph: &str) -> f64 {
    let t = Instant::now();
    let ok = Command::new(&env.info.ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i"])
        .arg(format!(
            "testsrc2=s={size}:r=25:d={}",
            f64::from(frames) / 25.0
        ))
        .args(["-filter_complex", graph, "-map", "[o]", "-f", "null", "-"])
        .status()
        .unwrap()
        .success();
    assert!(ok, "the graph must run: {graph}");
    t.elapsed().as_secs_f64()
}

// covers: REQ-314
#[test]
fn the_blurred_fill_costs_less_than_one_frame_time_per_frame_at_1080p_and_4k() {
    let Some(env) = setup() else { return };
    // Fit empty with blurred video, 9:16 Change aspect, Medium blur (radius 10 at an eighth of
    // each side, REQ-222): the graph of render/graph.rs `compose_frame`
    let (w, h) = (1080, 1920);
    let (sw, sh) = (w / 8, h / 8);
    let plain = format!(
        "[0:v]scale={w}:-2,format=yuv420p[p];color=c=black:s={w}x{h}:r=25:d=10[b];\
         [b][p]overlay=x='(W-w)/2':y='(H-h)/2':shortest=1,format=yuv420p[o]"
    );
    let blur = format!(
        "[0:v]split=2[pa][pb];[pb]scale={sw}:{sh}:force_original_aspect_ratio=increase,\
         crop={sw}:{sh},boxblur=10:2,scale={w}:{h},setsar=1[b];[pa]scale={w}:-2,format=yuv420p[p];\
         [b][p]overlay=x='(W-w)/2':y='(H-h)/2':shortest=1,format=yuv420p[o]"
    );
    // the frame time of the reference clip at 25 fps is 40 ms (REQ-314 b)
    for (size, frames) in [("1920x1080", 100), ("3840x2160", 50)] {
        let best = |g: &str| {
            (0..3)
                .map(|_| seconds(&env, size, frames, g))
                .fold(f64::MAX, f64::min)
        };
        let (a, b) = (best(&plain), best(&blur));
        let step_ms = (b - a) * 1000.0 / f64::from(frames);
        eprintln!(
            "blur step at {size}: {step_ms:.2} ms per frame (plain {:.2}, blur {:.2})",
            a * 1000.0 / f64::from(frames),
            b * 1000.0 / f64::from(frames)
        );
        assert!(
            step_ms < 40.0,
            "{size}: the blur step takes {step_ms:.1} ms a frame"
        );
        // the whole chain keeps the frame rate (no dropped frames) with room to spare
        assert!(
            b * 1000.0 / f64::from(frames) < 40.0,
            "{size}: {b} s for {frames} frames"
        );
    }
}

/// The second clip of the speech-region check (REQ-177): about 30 s of real speech, with its
/// transcript. Andrea records it and commits it with its sha-256; until it exists the check
/// below says so and does not run.
const SECOND_CLIP: &str = "tests/fixtures/align/second-clip-30s.wav";

// covers: REQ-177
#[test]
fn the_frozen_speech_region_setting_is_checked_on_the_second_clip_when_it_exists() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SECOND_CLIP);
    if !path.exists() {
        eprintln!(
            "SKIPPED: the second clip {SECOND_CLIP} (about 30 s of real speech and its \
             transcript, committed with its sha-256) is not there yet; REQ-177 cannot be closed \
             until it is"
        );
        return;
    }
    // the check itself needs the recogniser and the transcript beside the clip: see
    // `speech_regions` and `align`; it is written when the clip arrives
    panic!("the second clip exists: write its check (docs/code-2.2.0.md, REQ-177)");
}
