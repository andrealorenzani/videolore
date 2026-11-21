// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Opening a project of format 8 or older in format 9 (1.6.1, REQ-149, ADR-122..125).
//!
//! Until 1.6.0 a whole-video effect was one state of the timeline (Scale and Move keys in
//! timeline time, an Apply LUT, the fades at the video's edges, the video's sound level).
//! Since 1.6.1 it is a copy on each clip. This conversion is pure and runs in memory: no
//! undo step, no change mark. Each clip keeps the picture and the sound it had, with the
//! one exception of Q-F: a clip that had its own Scale or Move while the whole video had
//! one gets ONE combined path (exact at its key times, a straight line between them), and
//! a clip with its own Apply LUT keeps it (the whole video's is not applied to it).

use super::{ProjectError, invalid};
use crate::base::pos::{POS_MAX, Pos};
use crate::frame::{MAX_SCALE, MIN_SCALE, Scale};
use crate::keyframes::{self, Key, Lerp, MAX_KEYS};
use crate::music::Mix;
use crate::time::Micros;
use crate::timeline::{ClipFrames, ClipId, ClipLut, Timeline};

/// What a project of format 8 or older kept in the timeline for the whole video.
#[derive(Debug, Default)]
pub(super) struct Legacy {
    start_fade: bool,
    end_fade: bool,
    scale: Vec<Key<Scale>>,
    moved: Vec<Key<Pos>>,
    lut: Option<String>,
}

fn flag(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    name: &str,
) -> Result<bool, ProjectError> {
    match obj.remove(name) {
        None | Some(serde_json::Value::Null) => Ok(false),
        Some(serde_json::Value::Bool(b)) => Ok(b),
        Some(_) => Err(invalid(&format!("{name} is not true or false"))),
    }
}

impl Legacy {
    /// Takes the whole-video fields out of the timeline's JSON. A value of the wrong type
    /// is a damaged file.
    pub(super) fn take(value: &mut serde_json::Value) -> Result<Legacy, ProjectError> {
        let Some(obj) = value.get_mut("timeline").and_then(|t| t.as_object_mut()) else {
            return Ok(Legacy::default());
        };
        let mut take = |name: &str| obj.remove(name).unwrap_or(serde_json::Value::Null);
        let scale = take("whole_scale");
        let moved = take("whole_move");
        let lut = take("whole_lut");
        let start_fade = flag(obj, "start_fade")?;
        let end_fade = flag(obj, "end_fade")?;
        fn parse<T: serde::de::DeserializeOwned>(
            v: serde_json::Value,
        ) -> Result<Vec<Key<T>>, ProjectError> {
            if v.is_null() {
                return Ok(Vec::new());
            }
            serde_json::from_value(v).map_err(|e| invalid(&e.to_string()))
        }
        Ok(Legacy {
            start_fade,
            end_fade,
            scale: parse(scale)?,
            moved: parse(moved)?,
            lut: match lut {
                serde_json::Value::Null => None,
                serde_json::Value::String(s) => Some(s),
                _ => return Err(invalid("whole_lut is not text")),
            },
        })
    }
}

/// Product of two Scale values (permille product, offsets added), within the limits.
pub(super) fn combine_scale(a: Scale, b: Scale) -> Scale {
    let mut s = Scale {
        permille: (u32::from(a.permille) * u32::from(b.permille) + 500)
            .div_euclid(1000)
            .clamp(u32::from(MIN_SCALE), u32::from(MAX_SCALE)) as u16,
        dx: a.dx.saturating_add(b.dx),
        dy: a.dy.saturating_add(b.dy),
    };
    s.clamp();
    s
}

/// Two Move values as one: the offsets from the centre add, within the frame.
pub(super) fn combine_move(a: Pos, b: Pos) -> Pos {
    let one = |p: u16, q: u16| {
        (i32::from(p) + i32::from(q) - i32::from(POS_MAX / 2)).clamp(0, i32::from(POS_MAX)) as u16
    };
    Pos {
        x: one(a.x, b.x),
        y: one(a.y, b.y),
    }
}

/// The path of a clip that had a whole-video path (timeline time) and maybe its own (source
/// time) as one path in source time. `None` when the clip keeps its own unchanged.
///
/// Times (clip time): 0 and the clip length always, then the clip's own key times, then the
/// whole-video key times shifted to clip time, those inside the clip, until [`MAX_KEYS`].
pub(super) fn merge_path<T: Lerp>(
    own: &[Key<T>],
    whole: &[Key<T>],
    start: Micros,
    src_in: Micros,
    len: Micros,
    combine: impl Fn(T, T) -> T,
) -> Option<Vec<Key<T>>> {
    if whole.is_empty() {
        return None;
    }
    let inside = |t: Micros| t >= Micros::ZERO && t <= len;
    let mut times = vec![Micros::ZERO, len];
    let own_times = own.iter().map(|k| k.t - src_in);
    let whole_times = whole.iter().map(|k| k.t - start);
    for t in own_times.chain(whole_times).filter(|t| inside(*t)) {
        if times.len() >= MAX_KEYS {
            break;
        }
        if !times.contains(&t) {
            times.push(t);
        }
    }
    times.sort();
    times.dedup();
    let keys = times
        .into_iter()
        .filter_map(|t| {
            let w = keyframes::at(whole, start + t)?;
            let v = match keyframes::at(own, src_in + t) {
                Some(o) => combine(o, w),
                None => w,
            };
            Some(Key {
                t: src_in + t,
                v,
                ease: Default::default(),
            })
        })
        .collect();
    Some(keys)
}

fn clamp_pos(p: &mut Pos) {
    p.x = p.x.min(POS_MAX);
    p.y = p.y.min(POS_MAX);
}

/// Converts the timeline `tl` (clip effects already checked) and the video level of `mix`;
/// notes for the user are added to `notes`.
pub(super) fn apply(tl: &mut Timeline, mix: &mut Mix, mut old: Legacy, notes: &mut Vec<String>) {
    if tl.clips.is_empty() {
        return;
    }
    let total = tl.duration();
    for k in &mut old.scale {
        k.v.clamp();
    }
    for k in &mut old.moved {
        clamp_pos(&mut k.v);
    }
    keyframes::sanitise(&mut old.scale, total);
    keyframes::sanitise(&mut old.moved, total);
    let lut = old.lut.filter(|p| {
        let ok = crate::lut::path_ok(p);
        if !ok {
            notes.push("A damaged colour (LUT) entry was removed".to_string());
        }
        ok
    });
    let (mut combined, mut kept_lut) = (0usize, 0usize);
    let mut start = Micros::ZERO;
    let ids: Vec<ClipId> = tl.clips.iter().map(|c| c.id).collect();
    for (i, id) in ids.into_iter().enumerate() {
        let c = tl.clips[i];
        let mut e = tl.frames_of(id).cloned().unwrap_or(ClipFrames {
            clip: id,
            scale: vec![],
            moved: vec![],
        });
        let (had_scale, had_move) = (!e.scale.is_empty(), !e.moved.is_empty());
        let mut both = false;
        if let Some(k) = merge_path(
            &e.scale,
            &old.scale,
            start,
            c.src_in,
            c.len(),
            combine_scale,
        ) {
            both |= had_scale;
            e.scale = k;
        }
        if let Some(k) = merge_path(&e.moved, &old.moved, start, c.src_in, c.len(), combine_move) {
            both |= had_move;
            e.moved = k;
        }
        combined += usize::from(both);
        tl.put_frames(e);
        if let Some(p) = &lut {
            if tl.clip_luts.iter().any(|l| l.clip == id) {
                kept_lut += 1;
            } else {
                tl.clip_luts.push(ClipLut {
                    strength: 100,
                    clip: id,
                    path: p.clone(),
                });
            }
        }
        // the video's sound level was multiplied into every clip's own level
        let clip = &mut tl.clips[i];
        clip.volume = ((u32::from(mix.video_volume.min(100)) * u32::from(clip.volume) + 50) / 100)
            .min(100) as u8;
        clip.muted |= mix.video_muted;
        start = start + c.len();
    }
    tl.clip_luts.sort_by_key(|l| l.clip);
    mix.video_volume = 100;
    mix.video_muted = false;
    if let Some(c) = tl.clips.first_mut() {
        c.fade_in |= old.start_fade;
    }
    if let Some(c) = tl.clips.last_mut() {
        c.fade_out |= old.end_fade;
    }
    if combined > 0 {
        notes.push(format!(
            "{combined} clip(s) had their own Scale or Move and the whole video's: they are now one path"
        ));
    }
    if kept_lut > 0 {
        notes.push(format!(
            "{kept_lut} clip(s) kept their own colour (LUT); the whole video's LUT is not applied to them"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(p: u16) -> Scale {
        Scale {
            permille: p,
            dx: 0,
            dy: 0,
        }
    }

    fn k(t: i64, p: u16) -> Key<Scale> {
        Key {
            t: Micros(t),
            v: s(p),
            ease: Default::default(),
        }
    }

    // covers: REQ-149
    #[test]
    fn scales_multiply_moves_add_and_both_stay_inside_their_limits() {
        assert_eq!(combine_scale(s(2000), s(1500)).permille, 3000);
        assert_eq!(combine_scale(s(4000), s(4000)).permille, MAX_SCALE);
        assert_eq!(combine_scale(s(100), s(100)).permille, MIN_SCALE);
        let a = Scale {
            permille: 1000,
            dx: i16::MAX,
            dy: -5,
        };
        assert_eq!(combine_scale(a, a).dx, crate::frame::MAX_OFFSET);
        let c = crate::frame::CENTRE;
        assert_eq!(combine_move(c, c), c);
        let a = Pos { x: 6000, y: 2000 };
        let b = Pos { x: 7000, y: 0 };
        assert_eq!(combine_move(a, b), Pos { x: 8000, y: 0 });
        assert_eq!(
            combine_move(Pos { x: 10000, y: 10000 }, Pos { x: 10000, y: 10000 }).x,
            10000
        );
    }

    // covers: REQ-149
    #[test]
    fn the_conflict_case_is_exact_at_its_times_and_a_line_between() {
        // clip 0..4 s at the start of the video: own 100% -> 200%, whole 100% at 0, 200% at 2 s
        let own = vec![k(0, 1000), k(4_000_000, 2000)];
        let whole = vec![k(0, 1000), k(2_000_000, 2000)];
        let out = merge_path(
            &own,
            &whole,
            Micros::ZERO,
            Micros::ZERO,
            Micros(4_000_000),
            combine_scale,
        )
        .unwrap();
        let at = |t: i64| keyframes::at(&out, Micros(t)).unwrap().permille;
        assert_eq!(at(0), 1000);
        assert_eq!(at(2_000_000), 3000);
        assert_eq!(at(4_000_000), 4000); // 200% x 200% = 400%
        assert_eq!(at(1_000_000), 2000);
        // a clip without an own path gets the whole path unchanged
        let out = merge_path(
            &[],
            &whole,
            Micros::ZERO,
            Micros::ZERO,
            Micros(4_000_000),
            combine_scale,
        )
        .unwrap();
        assert_eq!(
            keyframes::at(&out, Micros(1_000_000)).unwrap().permille,
            1500
        );
        assert_eq!(
            keyframes::at(&out, Micros(4_000_000)).unwrap().permille,
            2000
        );
        // no whole path: nothing to do
        assert!(
            merge_path(
                &own,
                &[],
                Micros::ZERO,
                Micros::ZERO,
                Micros(4_000_000),
                combine_scale
            )
            .is_none()
        );
    }

    // covers: REQ-149
    #[test]
    fn a_later_clip_is_shifted_to_clip_time_and_keys_outside_are_dropped() {
        // the clip starts at 10 s in the video and at 3 s in its source, 4 s long
        let whole = vec![k(0, 1000), k(12_000_000, 2000), k(30_000_000, 4000)];
        let out = merge_path(
            &[],
            &whole,
            Micros(10_000_000),
            Micros(3_000_000),
            Micros(4_000_000),
            combine_scale,
        )
        .unwrap();
        let times: Vec<i64> = out.iter().map(|k| k.t.0).collect();
        assert_eq!(times, vec![3_000_000, 5_000_000, 7_000_000]);
        // the picture at every clip time is what the whole path gave
        for t in [0, 1_000_000, 2_000_000, 3_500_000, 4_000_000] {
            assert_eq!(
                keyframes::at(&out, Micros(3_000_000 + t)),
                keyframes::at(&whole, Micros(10_000_000 + t))
            );
        }
    }

    // covers: REQ-149
    #[test]
    fn at_most_200_keys_and_own_keys_come_first() {
        let own: Vec<_> = (0..150).map(|i| k(i * 10_000, 1000)).collect();
        let whole: Vec<_> = (0..150).map(|i| k(i * 10_000 + 5_000, 1000)).collect();
        let out = merge_path(
            &own,
            &whole,
            Micros::ZERO,
            Micros::ZERO,
            Micros(2_000_000),
            combine_scale,
        )
        .unwrap();
        assert_eq!(out.len(), MAX_KEYS);
        // 0 and the clip length stay, every own key stays
        assert!(out.iter().any(|x| x.t == Micros(2_000_000)));
        for o in &own {
            assert!(out.iter().any(|x| x.t == o.t), "own key {:?} lost", o.t);
        }
    }
}
