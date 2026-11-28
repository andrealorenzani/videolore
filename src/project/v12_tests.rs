// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Project format 12 (2.1.0): text shadows and the subtitle style settings (REQ-311).

use super::*;
use crate::base::shadow::Shadow;
use crate::subtitles::{Preset, StyleSettings};
use crate::text::TextItem;

fn with_settings() -> Project {
    let mut p = Project::default();
    let mut t = TextItem::new(crate::text::TextId(0), "Hello".into(), Micros::ZERO);
    t.shadow = Shadow {
        px: 8,
        colour: [255, 0, 0],
    };
    p.texts.add(t).unwrap();
    p.subtitles.preset = Preset::EnlargeWord;
    p.subtitles.style = StyleSettings {
        shadow: Shadow {
            px: 6,
            colour: [0, 0, 255],
        },
        highlighted: [255, 0, 0],
        not_yet: [0, 0, 255],
        enlarge: 20,
    };
    p
}

fn reopened(json: &str) -> Project {
    from_json(json, std::path::Path::new("/tmp")).unwrap().0
}

// covers: REQ-311
#[test]
fn shadows_and_style_settings_are_saved_and_read_back() {
    let json = to_json(&with_settings(), std::path::Path::new("/tmp/x.vlproj")).unwrap();
    assert!(json.contains("\"schema_version\": 16"));
    assert!(json.contains("enlarge_word"), "{json}");
    let p = reopened(&json);
    assert_eq!(p.texts.items[0].shadow.px, 8);
    assert_eq!(p.texts.items[0].shadow.colour, [255, 0, 0]);
    assert_eq!(p.subtitles.preset, Preset::EnlargeWord);
    assert_eq!(p.subtitles.style, with_settings().subtitles.style);
}

// covers: REQ-311
#[test]
fn a_project_of_format_11_opens_with_no_shadow_and_the_2_0_4_colours() {
    let mut p = Project::default();
    p.texts
        .add(TextItem::new(
            crate::text::TextId(0),
            "Old".into(),
            Micros::ZERO,
        ))
        .unwrap();
    let json = to_json(&p, std::path::Path::new("/tmp/x.vlproj")).unwrap();
    // nothing of 2.1.0 is written while it has its default value
    assert!(!json.contains("shadow") && !json.contains("not_yet") && !json.contains("enlarge"));
    let old = json.replace("\"schema_version\": 14", "\"schema_version\": 11");
    let q = reopened(&old);
    assert_eq!(q.texts.items[0].shadow, Shadow::default());
    assert_eq!(q.subtitles.style, StyleSettings::default());
    assert_eq!(q.subtitles.style.highlighted, [255, 255, 0]);
    assert_eq!(q.subtitles.style.not_yet, [255, 255, 255]);
    assert_eq!(q.subtitles.style.enlarge, 12);
}

// covers: REQ-311
#[test]
fn a_project_of_format_17_is_refused_and_out_of_range_values_are_brought_inside() {
    let json = to_json(&with_settings(), std::path::Path::new("/tmp/x.vlproj")).unwrap();
    let newer = json.replace("\"schema_version\": 16", "\"schema_version\": 17");
    assert_eq!(
        from_json(&newer, std::path::Path::new("/tmp")).unwrap_err(),
        ProjectError::TooNew(17)
    );
    let wild = json
        .replace("\"px\": 8", "\"px\": 200")
        .replace("\"enlarge\": 20", "\"enlarge\": 0");
    let p = reopened(&wild);
    assert_eq!(p.texts.items[0].shadow.px, crate::base::shadow::SHADOW_MAX);
    assert_eq!(p.subtitles.style.enlarge, crate::subtitles::ENLARGE_MIN);
}
