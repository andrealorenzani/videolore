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
}
