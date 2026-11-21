// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Fonts of the interface for the scripts of subtitle lines (REQ-188): the bundled Arabic
//! and Devanagari faces and, for Chinese, Japanese and Korean, an installed font that
//! covers the script (read once, when a line first needs it). They are added as fallbacks
//! of the interface fonts, so a line in any of these scripts can be read and edited; when
//! no font covers a script the note of REQ-187 says so.

use crate::base::font::{Script, scripts_in};
use eframe::egui::epaint::text::{FontInsert, FontPriority, InsertFontFamily};
use eframe::egui::{Context, FontData, FontFamily};
use std::collections::BTreeSet;
use std::sync::Mutex;

/// The checked scripts the subtitle lines of the project use.
pub fn scripts_of_lines<'a>(texts: impl IntoIterator<Item = &'a str>) -> Vec<Script> {
    let mut found: Vec<Script> = Vec::new();
    for t in texts {
        for s in scripts_in(t) {
            if !found.contains(&s) {
                found.push(s);
            }
        }
        if found.len() == Script::ALL.len() {
            break;
        }
    }
    found
}

fn font_name(s: Script) -> String {
    format!("script-{}", s.name().to_ascii_lowercase())
}

fn add(ctx: &Context, name: String, data: FontData) {
    let family = |family| InsertFontFamily {
        family,
        priority: FontPriority::Lowest,
    };
    ctx.add_font(FontInsert::new(
        &name,
        data,
        vec![
            family(FontFamily::Proportional),
            family(FontFamily::Monospace),
        ],
    ));
}

/// Adds the fonts the scripts in `scripts` need, once each. The bundled faces are static
/// data; an installed face is read from its file the first time (a script no installed font
/// covers adds nothing).
pub fn install_for(ctx: &Context, scripts: &[Script]) {
    for s in scripts {
        let name = font_name(*s);
        if ctx.fonts(|f| f.definitions().font_data.contains_key(&name)) {
            continue;
        }
        match s {
            Script::Arabic => add(ctx, name, FontData::from_static(crate::ass::FONT_ARABIC)),
            Script::Devanagari => {
                add(
                    ctx,
                    name,
                    FontData::from_static(crate::ass::FONT_DEVANAGARI),
                );
            }
            _ => {
                // looked for once the scan of the fonts has finished; a miss is remembered
                static MISSING: Mutex<BTreeSet<Script>> = Mutex::new(BTreeSet::new());
                if !crate::fonts::scan_finished() || MISSING.lock().is_ok_and(|m| m.contains(s)) {
                    continue;
                }
                let Some(face) = crate::fonts::face_for(*s) else {
                    if let Ok(mut m) = MISSING.lock() {
                        m.insert(*s);
                    }
                    continue;
                };
                let mut data = FontData::from_owned(face.bytes.clone());
                data.index = face.index;
                add(ctx, name, data);
            }
        }
    }
}

/// True when the font of `script` is known to the interface fonts.
pub fn is_installed(ctx: &Context, script: Script) -> bool {
    let name = font_name(script);
    ctx.fonts(|f| f.definitions().font_data.contains_key(&name))
}
