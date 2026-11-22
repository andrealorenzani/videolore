// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The step builder of a clip whose effects were put in a free order (REQ-320, REQ-321, ADR-224).
//! A clip with the default order keeps the arguments of the older versions, and one that only
//! moved its Apply LUT, texts, pictures and lines keeps the 2.1.0 overlay stage ([`super::overlay`]);
//! every other order is built here, row by row, bottom to top, on a stream that is in one of
//! three states:
//!
//! - the **source**: the picture as the file has it;
//! - the **layer**: the picture fitted to the frame on a transparent surround (RGBA, the size of
//!   the frame): Apply LUT and Fade act on the colours and keep the transparency, Scale resizes the layer about the centre of the frame and Move offsets
//!   it, so Scale then Move and Move then Scale give two places; texts are drawn by a matte;
//! - the **frame**: the layer composed over the background (Change aspect), opaque: a Scale or
//!   Move now acts on the whole frame (the uncovered edge is black), a LUT grades the background
//!   too and a Fade fades it too.
//!
//! The picture is fitted when the first row needs it, and the background is
//! composed at the Change aspect row; a clip without that effect is composed over black before
//! its first text, picture or line (they need an opaque frame) or at the end. The clock of the
//! chain is the clip's own (its documents are written in it), restarted by the last filter.

use super::graph::{blur_chain, colour_chain, fades};
use super::overlay::OverlayStep;
use super::*;

/// One step of a clip built step by step, bottom to top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Change aspect: the background is composed under the picture here.
    Frame,
    Scale,
    Move,
    /// The clip's Fade IN/OUT.
    Fade,
    /// What the row drew: `overlay[n]` of the segment (the LUT, a text, a picture, the lines).
    Over(usize),
}

/// What the stream is at a place of the chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Source,
    Layer,
    Frame,
}

/// The things the chain of one clip needs from the graph builder.
pub(super) struct Ctx<'a> {
    pub i: usize,
    pub s: &'a Segment,
    /// The graph of the clip's own frame (a Change aspect of its own, REQ-286, or the output's).
    pub spec: &'a GraphSpec,
    /// The size of the output picture.
    pub outer: (u32, u32),
    /// The inputs of the picture steps, in order.
    pub pics: &'a [usize],
    pub sphere: bool,
}

/// The chain being written.
struct Build<'a> {
    ctx: &'a Ctx<'a>,
    parts: &'a mut Vec<String>,
    /// The open chain: starts with an input label, has no label at its end.
    c: String,
    n: usize,
    stage: Stage,
    /// The size of the picture as it is now (the frame, or the output once a clip's own frame is
    /// placed in it).
    cur: (u32, u32),
    aspect: Option<frame::FrameAspect>,
    blur: bool,
}

fn place(main: &str, over: &str, off: &str) -> String {
    if off == "0" {
        format!("({main}-{over})/2")
    } else {
        format!("({main}-{over})/2+{main}*({off})")
    }
}

impl Build<'_> {
    fn size(&self) -> (u32, u32) {
        let g = &self.ctx.spec.geometry;
        (g.width, g.height)
    }

    fn fps(&self) -> String {
        self.ctx.spec.geometry.fps.ff()
    }

    fn label(&mut self, tag: &str) -> String {
        self.n += 1;
        format!("[{tag}{}x{}]", self.ctx.i, self.n)
    }

    /// Appends a filter (written with its leading comma) to the open chain: after a label the
    /// comma is left out.
    fn push(&mut self, filter: &str) {
        if self.c.ends_with(']') {
            self.c.push_str(filter.strip_prefix(',').unwrap_or(filter));
        } else {
            self.c.push_str(filter);
        }
    }

    /// Closes the open chain with a label and returns it.
    fn cut(&mut self) -> String {
        let l = self.label("s");
        self.parts.push(format!("{}{l}", self.c));
        self.c.clear();
        l
    }

    /// A canvas of the size of the picture that starts at the clip's first rendered instant.
    fn canvas(&mut self, colour: &str) -> String {
        let cv = self.label("k");
        let s = self.ctx.s;
        self.parts.push(format!(
            "color=c={colour}:s={}x{}:r={}:d={},format=rgba,setpts=PTS-STARTPTS+{}/TB{cv}",
            self.cur.0,
            self.cur.1,
            self.fps(),
            s.rendered().ff(),
            s.skip.ff(),
        ));
        cv
    }

    /// The source becomes the layer: fitted to the frame on a transparent surround.
    fn fit(&mut self) {
        if self.stage != Stage::Source {
            return;
        }
        let (w, h) = self.size();
        let i = self.ctx.i;
        if !self.ctx.sphere {
            let fit = frame::fit_filter((w, h), self.aspect.as_ref());
            self.push(&fit);
            self.push(&format!(",setsar=1,fps={}", self.fps()));
        }
        self.push(",format=rgba");
        let mut pic = self.cut();
        if self.blur {
            // the blurred background is made from the picture as it is here
            self.parts.push(format!("{pic}split=2[pa{i}][pb{i}]"));
            let level = self.aspect.map(|a| a.blur).unwrap_or_default();
            self.parts.push(blur_chain(i, (w, h), level));
            pic = format!("[pa{i}]");
        }
        self.cur = (w, h);
        let cv = self.canvas("0x00000000");
        self.c =
            format!("{cv}{pic}overlay=x='(W-w)/2':y='(H-h)/2':format=auto:shortest=1,format=rgba");
        self.stage = Stage::Layer;
    }

    /// The layer is composed over the background.
    fn compose(&mut self) {
        self.fit();
        if self.stage == Stage::Frame {
            return;
        }
        let (i, s, pf) = (self.ctx.i, self.ctx.s, self.ctx.spec.pix_fmt);
        let (w, h) = self.size();
        let l = self.cut();
        if !self.blur {
            let rgb = match self.aspect.map(|a| a.fill) {
                Some(frame::Fill::Colour(c)) => c,
                _ => frame::Rgb::BLACK,
            };
            let fps = self.ctx.spec.geometry.fps;
            self.parts.push(colour_chain(i, s, (w, h), fps, rgb));
        }
        self.c = format!("[b{i}]{l}overlay=x=0:y=0:format=auto:shortest=1,format={pf}");
        self.cur = (w, h);
        // a clip's own frame is placed in the output picture (REQ-286)
        if (w, h) != self.ctx.outer {
            let (ow, oh) = self.ctx.outer;
            self.push(&format!(
                ",scale={ow}:{oh}:force_original_aspect_ratio=decrease,\
                 pad={ow}:{oh}:(ow-iw)/2:(oh-ih)/2:color=black,setsar=1,format={pf}"
            ));
            self.cur = self.ctx.outer;
        }
        self.stage = Stage::Frame;
    }

    /// Scale (`scale`) or Move: a transparent canvas under a layer, black under a frame.
    fn place_picture(&mut self, scale: bool) {
        let Some(f) = self.ctx.s.frame.as_ref() else {
            return;
        };
        if (scale && f.stack.scale.is_empty()) || (!scale && f.stack.moved.is_empty()) {
            return;
        }
        self.fit();
        let on_frame = self.stage == Stage::Frame;
        let (ox, oy) = if scale {
            (f.stack.scale_offset_x(), f.stack.scale_offset_y())
        } else {
            (f.stack.move_offset_x(), f.stack.move_offset_y())
        };
        if on_frame {
            self.push(",format=rgba");
        }
        if scale {
            let k = f.stack.scale_factor();
            self.push(&format!(
                ",scale=w='max(2,trunc(iw*({k})/2)*2)':h='max(2,trunc(ih*({k})/2)*2)':eval=frame"
            ));
        }
        let l = self.cut();
        let cv = self.canvas(if on_frame { "black" } else { "0x00000000" });
        let end = if on_frame {
            self.ctx.spec.pix_fmt
        } else {
            "rgba"
        };
        self.c = format!(
            "{cv}{l}overlay=x='{x}':y='{y}':eval=frame:format=auto:shortest=1,format={end}",
            x = place("W", "w", &ox),
            y = place("H", "h", &oy),
        );
    }

    /// Fade IN/OUT.
    fn fade(&mut self) {
        let s = self.ctx.s;
        let early = if s.to_black {
            Micros(self.ctx.spec.geometry.fps.frame_duration().0 * 2)
        } else {
            Micros::ZERO
        };
        let mut f = String::new();
        fades(&mut f, "fade", s, early);
        if f.is_empty() {
            return;
        }
        self.fit();
        if self.stage == Stage::Layer {
            // the colours fade, the transparent surround stays (the background shows)
            self.push(&format!(",format=gbrap{f},format=rgba"));
        } else {
            self.push(&f);
        }
    }

    /// Draws the ASS document `name` on the transparent layer: the document is drawn on black
    /// and on white, so that what it covers (the matte) and its colours are known whatever the
    /// style is, and the text is laid over the layer with its own transparency (libass leaves
    /// the transparency of a frame as it finds it).
    fn matte(&mut self, name: &str) {
        let s = self.ctx.s;
        let (w, h) = self.size();
        let (d, o, fps) = (s.rendered().ff(), s.skip.ff(), self.fps());
        let l = self.cut();
        let [mb, mw, mb1, mb2, ma1, ma2, mg, col, tx] =
            ["mb", "mw", "mb", "mb", "ma", "ma", "mg", "mc", "tx"].map(|t| self.label(t));
        let draw = |colour: &str, out: &str| {
            format!(
                "color=c={colour}:s={w}x{h}:r={fps}:d={d},setpts=PTS-STARTPTS+{o}/TB,\
                 format=gbrp,subtitles=f={name}:fontsdir={}{out}",
                ass::FONTS_DIR
            )
        };
        self.parts.push(draw("black", &mb));
        self.parts.push(draw("white", &mw));
        self.parts.push(format!("{mb}split=2{mb1}{mb2}"));
        // what the text covers: 255 - (the white result - the black result)
        self.parts.push(format!(
            "{mw}{mb1}blend=all_expr='255-(A-B)',format=gbrp,split=2{ma1}{ma2}"
        ));
        self.parts.push(format!("{ma1}format=gray{mg}"));
        // its colours, as they are without the black behind them
        self.parts.push(format!(
            "{mb2}{ma2}blend=all_expr='if(lte(B,0.5),0,min(255,A*255/B))'{col}"
        ));
        self.parts
            .push(format!("{col}{mg}alphamerge,format=rgba{tx}"));
        self.c = format!("{l}{tx}overlay=format=auto:shortest=1,format=rgba");
    }
}

/// The chain of clip `ctx.i`, from `base` (the source, trimmed and on the clip's clock, or the
/// camera view of a 360 clip) to the picture in the output format, without a label. The LUTs
/// and the documents it needs go to `luts` and `files`.
pub(super) fn chain(
    parts: &mut Vec<String>,
    ctx: &Ctx,
    base: String,
    (luts, files): (
        &mut Vec<std::sync::Arc<crate::lut::Lut>>,
        &mut Vec<(String, String)>,
    ),
) -> String {
    let s = ctx.s;
    let aspect = s.frame.as_ref().and_then(|f| f.aspect);
    let frame_row = s.steps.contains(&Step::Frame);
    // without a Change aspect row, the frame is composed over black before the first row that
    // needs an opaque frame (a text, a picture or the lines), or at the end
    let implicit_at = if frame_row {
        None
    } else {
        s.steps.iter().position(|st| {
            matches!(st, Step::Over(k) if !matches!(s.overlay.get(*k), Some(OverlayStep::Lut(_))))
        })
    };
    let g = &ctx.spec.geometry;
    let mut b = Build {
        ctx,
        parts,
        c: base,
        n: 0,
        stage: Stage::Source,
        cur: (g.width, g.height),
        aspect,
        blur: frame_row && matches!(aspect.map(|a| a.fill), Some(frame::Fill::Blur)),
    };
    let pf = ctx.spec.pix_fmt;
    let mut pics = ctx.pics.iter();
    for (idx, step) in s.steps.iter().enumerate() {
        if implicit_at == Some(idx) {
            b.compose();
        }
        match *step {
            Step::Frame => b.compose(),
            Step::Scale => b.place_picture(true),
            Step::Move => b.place_picture(false),
            Step::Fade => b.fade(),
            Step::Over(k) => {
                let Some(over) = s.overlay.get(k) else {
                    continue;
                };
                b.fit();
                match over {
                    OverlayStep::Lut(l) => {
                        if !luts.iter().any(|x| x.hash() == l.hash()) {
                            luts.push(l.clone());
                        }
                        let name = crate::lut::store::copy_name(l);
                        let (before, after) = if b.stage == Stage::Layer {
                            ("gbrap", "rgba")
                        } else {
                            ("gbrp", pf)
                        };
                        b.push(&format!(
                            ",format={before},lut3d=file={name}:interp=tetrahedral,format={after}"
                        ));
                    }
                    OverlayStep::Ass { name, doc } => {
                        files.push((name.clone(), doc.clone()));
                        if b.stage == Stage::Frame {
                            b.push(&format!(",subtitles=f={name}:fontsdir={}", ass::FONTS_DIR));
                        } else {
                            b.matte(name);
                        }
                    }
                    OverlayStep::Pic(p) => {
                        let Some(input) = pics.next() else { continue };
                        let l = b.cut();
                        let (pi, q) = (b.label("pi"), b.label("q"));
                        let end = if b.stage == Stage::Layer { "rgba" } else { pf };
                        b.parts.push(format!(
                            "{prep};{l}{pi}overlay={laid}:format=auto,format={end}{q}",
                            prep = p.prepared(*input, &pi),
                            laid = p.laid(),
                        ));
                        b.c = q;
                    }
                }
            }
        }
    }
    // what is left: the picture is fitted and composed (over black without a Change aspect)
    b.compose();
    // the picture read from the frame before the first instant ends with the clip
    let cut = if s.read_back() > Micros::ZERO {
        format!(",trim=duration={}", s.rendered().ff())
    } else {
        String::new()
    };
    b.push(&format!(",setpts=PTS-STARTPTS{cut}"));
    b.c
}
