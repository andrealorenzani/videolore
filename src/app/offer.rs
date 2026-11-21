// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The offer to give a new clip the whole-video effects (REQ-201): when a clip is appended,
//! pasted or split off while every other clip has an effect (Volume, Clean audio, Scale,
//! Move or Apply LUT), the status bar says "The new clip has no <effect>" with the button
//! "Apply to new clips". Nothing changes by itself (REQ-147): only the button copies the
//! effect, as one undo step.

use super::*;

/// The effects an offer can be about.
const OFFERED: [EffectKind; 5] = [
    EffectKind::Volume,
    EffectKind::CleanAudio,
    EffectKind::Scale,
    EffectKind::Move,
    EffectKind::ApplyLut,
];

/// Clips added and the whole-video effects they lack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewClipOffer {
    pub clips: Vec<ClipId>,
    pub missing: Vec<EffectKind>,
}

/// Whether clip `id` has `effect` of its own.
fn has(tl: &crate::timeline::Timeline, id: ClipId, effect: EffectKind) -> bool {
    let Some(c) = tl.get(id) else { return false };
    let target = FrameTarget::Clip(id);
    match effect {
        EffectKind::Volume => c.has_volume(),
        EffectKind::CleanAudio => c.clean_audio,
        EffectKind::Scale => !tl.scale_keys(target).is_empty(),
        EffectKind::Move => !tl.move_keys(target).is_empty(),
        EffectKind::ApplyLut => tl.lut_of(target).is_some(),
        _ => false,
    }
}

impl AppState {
    /// Called after a clip was appended, pasted or split off: records the offer when
    /// the other clips share an effect the new ones lack.
    pub(super) fn note_new_clips(&mut self, before: &[ClipId]) {
        let tl = &self.project.timeline;
        let new: Vec<ClipId> = tl
            .clips
            .iter()
            .map(|c| c.id)
            .filter(|id| !before.contains(id))
            .collect();
        let others: Vec<ClipId> = tl
            .clips
            .iter()
            .map(|c| c.id)
            .filter(|id| before.contains(id))
            .collect();
        self.new_clip_offer = None;
        if new.is_empty() || others.is_empty() {
            return;
        }
        let missing: Vec<EffectKind> = OFFERED
            .into_iter()
            .filter(|e| {
                others.iter().all(|o| has(tl, *o, *e)) && new.iter().any(|n| !has(tl, *n, *e))
            })
            .collect();
        if !missing.is_empty() {
            self.new_clip_offer = Some(NewClipOffer {
                clips: new,
                missing,
            });
        }
    }

    /// The text of the offer, while it still applies: "The new clip has no Volume".
    pub fn new_clip_offer_text(&self) -> Option<String> {
        let o = self.new_clip_offer.as_ref()?;
        let tl = &self.project.timeline;
        let still: Vec<&str> = o
            .missing
            .iter()
            .filter(|e| {
                o.clips
                    .iter()
                    .any(|c| tl.get(*c).is_some() && !has(tl, *c, **e))
            })
            .map(|e| e.name())
            .collect();
        (!still.is_empty()).then(|| format!("The new clip has no {}", still.join(" and ")))
    }

    /// "Apply to new clips": gives the new clips the effects of the other clips, in one
    /// undo step (REQ-201).
    pub(super) fn apply_whole_to_new(&mut self, now: Instant) -> Result<(), String> {
        let Some(o) = self.new_clip_offer.take() else {
            return Ok(());
        };
        let tl = &self.project.timeline;
        // the first other clip that has the effect is the pattern
        let pattern = |e: EffectKind| {
            tl.clips
                .iter()
                .map(|c| c.id)
                .find(|id| !o.clips.contains(id) && has(tl, *id, e))
        };
        let plan: Vec<(EffectKind, ClipId)> = o
            .missing
            .iter()
            .filter_map(|e| pattern(*e).map(|p| (*e, p)))
            .collect();
        if plan.is_empty() {
            return Ok(());
        }
        let clips: Vec<ClipId> = o
            .clips
            .iter()
            .copied()
            .filter(|c| tl.get(*c).is_some())
            .collect();
        self.edit(now, "Apply to new clips", |e| {
            for (effect, from) in &plan {
                let src = *e.timeline.get(*from).ok_or("no such clip")?;
                // only the new clips that lack it (a split-off half keeps its own)
                let lacking: Vec<ClipId> = clips
                    .iter()
                    .copied()
                    .filter(|c| !has(&e.timeline, *c, *effect))
                    .collect();
                for c in &lacking {
                    match effect {
                        EffectKind::Volume => e
                            .timeline
                            .set_clip_volume(*c, src.volume, src.muted)
                            .map_err(err)?,
                        EffectKind::CleanAudio => {
                            e.timeline.set_clean_audio(&[*c], true).map_err(err)?
                        }
                        EffectKind::Scale | EffectKind::Move => {
                            let keys = if *effect == EffectKind::Scale {
                                e.timeline.copy_scale_keys(*from)
                            } else {
                                e.timeline.copy_move_keys(*from)
                            };
                            if let Some(k) = keys {
                                e.timeline.paste_keys(*c, &k).map_err(err)?;
                            }
                        }
                        EffectKind::ApplyLut => {
                            let path = e
                                .timeline
                                .lut_of(FrameTarget::Clip(*from))
                                .map(str::to_string);
                            e.timeline
                                .set_lut(FrameTarget::Clip(*c), path)
                                .map_err(err)?;
                        }
                        _ => {}
                    }
                }
            }
            Ok(())
        })?;
        self.status = "Applied to the new clips".into();
        Ok(())
    }
}
