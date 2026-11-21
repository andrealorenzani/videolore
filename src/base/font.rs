// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Font family names, shared by the text items, the font list and the ASS writer.

/// Longest family name.
pub const MAX_FONT: usize = 100;
/// The font used when none is chosen (the bundled subtitle font).
pub const DEFAULT_FONT: &str = "Noto Sans";

/// True for a font family name that is safe to put in an ASS file.
pub fn valid_font(name: &str) -> bool {
    name.chars().count() <= MAX_FONT
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '.' | '_' | '(' | ')'))
}

/// The scripts the subtitle fonts are checked for (REQ-187): Arabic and Devanagari are
/// bundled, the three CJK scripts come from the fonts installed on the computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Script {
    Arabic,
    Devanagari,
    /// Chinese characters (Han).
    Han,
    /// Japanese Hiragana and Katakana.
    Kana,
    /// Korean Hangul.
    Hangul,
}

impl Script {
    pub const ALL: [Script; 5] = [
        Script::Arabic,
        Script::Devanagari,
        Script::Han,
        Script::Kana,
        Script::Hangul,
    ];

    /// The name shown in notes.
    pub fn name(self) -> &'static str {
        match self {
            Script::Arabic => "Arabic",
            Script::Devanagari => "Devanagari",
            Script::Han => "Chinese",
            Script::Kana => "Japanese",
            Script::Hangul => "Korean",
        }
    }

    /// True for a script whose font is bundled with the program.
    pub fn is_bundled(self) -> bool {
        matches!(self, Script::Arabic | Script::Devanagari)
    }

    /// Characters a font must have to show the script (a sample, not the whole script).
    pub fn sample(self) -> &'static [char] {
        match self {
            Script::Arabic => &['\u{0627}', '\u{0644}', '\u{0645}'],
            Script::Devanagari => &['\u{0928}', '\u{092E}', '\u{0938}'],
            Script::Han => &['\u{4E2D}', '\u{6587}', '\u{5B57}'],
            Script::Kana => &['\u{3042}', '\u{30A2}', '\u{3093}'],
            Script::Hangul => &['\u{D55C}', '\u{AE00}', '\u{C694}'],
        }
    }
}

/// The script of one character, when it belongs to one of the checked scripts.
pub fn script_of(c: char) -> Option<Script> {
    match u32::from(c) {
        0x0600..=0x06FF | 0x0750..=0x077F | 0x08A0..=0x08FF | 0xFB50..=0xFDFF | 0xFE70..=0xFEFF => {
            Some(Script::Arabic)
        }
        0x0900..=0x097F | 0xA8E0..=0xA8FF => Some(Script::Devanagari),
        0x3040..=0x30FF | 0x31F0..=0x31FF | 0xFF66..=0xFF9F => Some(Script::Kana),
        0xAC00..=0xD7AF | 0x1100..=0x11FF | 0x3130..=0x318F => Some(Script::Hangul),
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x20000..=0x2A6DF | 0xF900..=0xFAFF => {
            Some(Script::Han)
        }
        _ => None,
    }
}

/// The checked scripts a text uses, in the order of [`Script::ALL`].
pub fn scripts_in(text: &str) -> Vec<Script> {
    let mut found: Vec<Script> = Vec::new();
    for c in text.chars() {
        if let Some(s) = script_of(c)
            && !found.contains(&s)
        {
            found.push(s);
        }
    }
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-108
    #[test]
    fn only_plain_family_names_are_valid() {
        assert!(valid_font("Noto Sans") && valid_font("Fira-Code (Mono)"));
        assert!(!valid_font("a;b") && !valid_font("a\\b") && !valid_font("{x}"));
        assert!(!valid_font(&"x".repeat(MAX_FONT + 1)));
    }

    // covers: REQ-187
    #[test]
    fn characters_belong_to_their_script() {
        assert_eq!(script_of('م'), Some(Script::Arabic));
        assert_eq!(script_of('न'), Some(Script::Devanagari));
        assert_eq!(script_of('中'), Some(Script::Han));
        assert_eq!(script_of('あ'), Some(Script::Kana));
        assert_eq!(script_of('한'), Some(Script::Hangul));
        assert_eq!(script_of('a'), None);
        assert_eq!(script_of('й'), None);
        for s in Script::ALL {
            assert!(s.sample().iter().all(|c| script_of(*c) == Some(s)));
        }
        assert_eq!(
            scripts_in("hi 日本語 مرحبا 日本"),
            [Script::Arabic, Script::Han]
        );
        assert!(scripts_in("plain text").is_empty());
        assert!(Script::Arabic.is_bundled() && !Script::Han.is_bundled());
        assert_eq!(Script::Han.name(), "Chinese");
    }
}
