// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The language codes of the speech model and their English names.

/// The languages of the speech model: its code (ISO 639-1, or whisper's own for a few),
/// the ISO 639-2 codes (bibliographic and terminology) files use, the English name.
const LANGUAGE_TABLE: &[(&str, &[&str], &str)] = &[
    ("en", &["eng"], "English"),
    ("zh", &["chi", "zho"], "Chinese"),
    ("de", &["ger", "deu"], "German"),
    ("es", &["spa"], "Spanish"),
    ("ru", &["rus"], "Russian"),
    ("ko", &["kor"], "Korean"),
    ("fr", &["fre", "fra"], "French"),
    ("ja", &["jpn"], "Japanese"),
    ("pt", &["por"], "Portuguese"),
    ("tr", &["tur"], "Turkish"),
    ("pl", &["pol"], "Polish"),
    ("ca", &["cat"], "Catalan"),
    ("nl", &["dut", "nld"], "Dutch"),
    ("ar", &["ara"], "Arabic"),
    ("sv", &["swe"], "Swedish"),
    ("it", &["ita"], "Italian"),
    ("id", &["ind"], "Indonesian"),
    ("hi", &["hin"], "Hindi"),
    ("fi", &["fin"], "Finnish"),
    ("vi", &["vie"], "Vietnamese"),
    ("he", &["heb"], "Hebrew"),
    ("uk", &["ukr"], "Ukrainian"),
    ("el", &["gre", "ell"], "Greek"),
    ("ms", &["may", "msa"], "Malay"),
    ("cs", &["cze", "ces"], "Czech"),
    ("ro", &["rum", "ron"], "Romanian"),
    ("da", &["dan"], "Danish"),
    ("hu", &["hun"], "Hungarian"),
    ("ta", &["tam"], "Tamil"),
    ("no", &["nor", "nob"], "Norwegian"),
    ("th", &["tha"], "Thai"),
    ("ur", &["urd"], "Urdu"),
    ("hr", &["hrv"], "Croatian"),
    ("bg", &["bul"], "Bulgarian"),
    ("lt", &["lit"], "Lithuanian"),
    ("la", &["lat"], "Latin"),
    ("mi", &["mao", "mri"], "Maori"),
    ("ml", &["mal"], "Malayalam"),
    ("cy", &["wel", "cym"], "Welsh"),
    ("sk", &["slo", "slk"], "Slovak"),
    ("te", &["tel"], "Telugu"),
    ("fa", &["per", "fas"], "Persian"),
    ("lv", &["lav"], "Latvian"),
    ("bn", &["ben"], "Bengali"),
    ("sr", &["srp"], "Serbian"),
    ("az", &["aze"], "Azerbaijani"),
    ("sl", &["slv"], "Slovenian"),
    ("kn", &["kan"], "Kannada"),
    ("et", &["est"], "Estonian"),
    ("mk", &["mac", "mkd"], "Macedonian"),
    ("br", &["bre"], "Breton"),
    ("eu", &["baq", "eus"], "Basque"),
    ("is", &["ice", "isl"], "Icelandic"),
    ("hy", &["arm", "hye"], "Armenian"),
    ("ne", &["nep"], "Nepali"),
    ("mn", &["mon"], "Mongolian"),
    ("bs", &["bos"], "Bosnian"),
    ("kk", &["kaz"], "Kazakh"),
    ("sq", &["alb", "sqi"], "Albanian"),
    ("sw", &["swa"], "Swahili"),
    ("gl", &["glg"], "Galician"),
    ("mr", &["mar"], "Marathi"),
    ("pa", &["pan"], "Punjabi"),
    ("si", &["sin"], "Sinhala"),
    ("km", &["khm"], "Khmer"),
    ("sn", &["sna"], "Shona"),
    ("yo", &["yor"], "Yoruba"),
    ("so", &["som"], "Somali"),
    ("af", &["afr"], "Afrikaans"),
    ("oc", &["oci"], "Occitan"),
    ("ka", &["geo", "kat"], "Georgian"),
    ("be", &["bel"], "Belarusian"),
    ("tg", &["tgk"], "Tajik"),
    ("sd", &["snd"], "Sindhi"),
    ("gu", &["guj"], "Gujarati"),
    ("am", &["amh"], "Amharic"),
    ("yi", &["yid"], "Yiddish"),
    ("lo", &["lao"], "Lao"),
    ("uz", &["uzb"], "Uzbek"),
    ("fo", &["fao"], "Faroese"),
    ("ht", &["hat"], "Haitian Creole"),
    ("ps", &["pus"], "Pashto"),
    ("tk", &["tuk"], "Turkmen"),
    ("nn", &["nno"], "Norwegian Nynorsk"),
    ("mt", &["mlt"], "Maltese"),
    ("sa", &["san"], "Sanskrit"),
    ("lb", &["ltz"], "Luxembourgish"),
    ("my", &["bur", "mya"], "Burmese"),
    ("bo", &["tib", "bod"], "Tibetan"),
    ("tl", &["tgl", "fil"], "Tagalog"),
    ("mg", &["mlg"], "Malagasy"),
    ("as", &["asm"], "Assamese"),
    ("tt", &["tat"], "Tatar"),
    ("haw", &["haw"], "Hawaiian"),
    ("ln", &["lin"], "Lingala"),
    ("ha", &["hau"], "Hausa"),
    ("ba", &["bak"], "Bashkir"),
    ("jw", &["jav", "jv"], "Javanese"),
    ("su", &["sun"], "Sundanese"),
    ("yue", &["yue"], "Cantonese"),
];

fn language_entry(
    code: &str,
) -> Option<&'static (&'static str, &'static [&'static str], &'static str)> {
    let base = code.split(['-', '_']).next()?.to_ascii_lowercase();
    if base.is_empty() || base == "und" {
        return None;
    }
    LANGUAGE_TABLE
        .iter()
        .find(|(one, two, _)| *one == base || two.contains(&base.as_str()))
}

/// The speech model's code for a language code found in a file (`ita`, `ger`, `en-US`,
/// `it`); `None` when unknown or undetermined (the language is then detected).
pub fn iso639_2_to_1(code: &str) -> Option<&'static str> {
    language_entry(code).map(|e| e.0)
}

/// The ISO 639-2 code (the first, bibliographic one) of a language code of the speech model
/// or of a file; `None` when unknown.
pub fn iso639_2(code: &str) -> Option<&'static str> {
    language_entry(code).and_then(|e| e.1.first().copied())
}

/// English name of a language code (`it`, `ita`, …), when known.
pub fn language_name(code: &str) -> Option<&'static str> {
    language_entry(code).map(|e| e.2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_codes_of_files_map_to_the_model() {
        assert_eq!(iso639_2_to_1("ita"), Some("it"));
        assert_eq!(iso639_2_to_1("ger"), Some("de"));
        assert_eq!(iso639_2_to_1("deu"), Some("de"));
        assert_eq!(iso639_2_to_1("en-US"), Some("en"));
        assert_eq!(iso639_2_to_1("EN"), Some("en"));
        assert_eq!(iso639_2_to_1("und"), None);
        assert_eq!(iso639_2_to_1(""), None);
        assert_eq!(iso639_2_to_1("xx"), None);
        assert_eq!(iso639_2_to_1("yue"), Some("yue"));
        assert_eq!(language_name("it"), Some("Italian"));
        assert_eq!(language_name("fre"), Some("French"));
        assert!(LANGUAGE_TABLE.len() >= 99);
    }
}
