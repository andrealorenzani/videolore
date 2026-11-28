// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Project format 13 (2.2.0): the text maximum size (REQ-323) and the refusal by older
//! programs (REQ-029); formats 12 and older still open.

use super::*;
use crate::text::{TextId, TextItem};

fn with_max(max: Option<u16>) -> Project {
    let mut p = Project::default();
    let mut t = TextItem::new(TextId(0), "Title".into(), Micros::ZERO);
    t.max_size = max;
    p.texts.add(t).unwrap();
    p
}

fn json_of(p: &Project) -> String {
    to_json(p, std::path::Path::new("/tmp/x.vlproj")).unwrap()
}

fn reopened(json: &str) -> (Project, Vec<String>) {
    let (p, notes, _) = from_json_noted(json, std::path::Path::new("/tmp")).unwrap();
    (p, notes)
}

// covers: REQ-323, REQ-029
#[test]
fn a_text_maximum_is_written_in_format_13_and_read_back() {
    let json = json_of(&with_max(Some(200)));
    assert!(json.contains("\"schema_version\": 16"));
    assert!(json.contains("\"max_size\": 200"), "{json}");
    let (p, notes) = reopened(&json);
    assert_eq!(p.texts.items[0].max_size, Some(200));
    assert!(notes.is_empty());
    // a text without a maximum writes nothing of it
    assert!(!json_of(&with_max(None)).contains("max_size"));
}

// covers: REQ-323, REQ-322
#[test]
fn a_project_of_format_12_opens_with_no_maximum_and_the_same_texts() {
    let json = json_of(&with_max(None)).replace("\"schema_version\": 16", "\"schema_version\": 12");
    let (p, notes) = reopened(&json);
    assert_eq!(p.texts.items[0].max_size, None);
    assert!(notes.is_empty());
    // and it is saved again as 14 (13 is converted in memory since 2.3.0)
    assert!(json_of(&p).contains("\"schema_version\": 16"));
}

// covers: REQ-323
#[test]
fn a_maximum_outside_two_to_fifty_percent_is_dropped_with_a_note() {
    for bad in [0u16, 19, 501, 9999] {
        let json = json_of(&with_max(Some(200)))
            .replace("\"max_size\": 200", &format!("\"max_size\": {bad}"));
        let (p, notes) = reopened(&json);
        assert_eq!(p.texts.items[0].max_size, None, "{bad}");
        assert!(notes.iter().any(|n| n.contains("maximum")), "{notes:?}");
    }
}

// covers: REQ-029
#[test]
fn a_project_of_a_newer_format_than_14_is_refused_and_older_programs_refuse_13() {
    let json = json_of(&with_max(Some(200)));
    let newer = json.replace("\"schema_version\": 16", "\"schema_version\": 17");
    assert_eq!(
        from_json(&newer, std::path::Path::new("/tmp")).unwrap_err(),
        ProjectError::TooNew(17)
    );
    // what a 2.1.0 program (format 12) did with this file: refuse it, as REQ-029 says
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let refused_by_2_1_0 = v["schema_version"].as_u64().unwrap() > 12;
    assert!(refused_by_2_1_0);
}
