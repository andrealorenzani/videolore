// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Facts about the installers: the pinned ffmpeg builds (`packaging/ffmpeg.lock`, also
//! shown in the third-party notices), the pinned packaging tools (`packaging/tools.lock`)
//! and the names of the files attached to every release. `scripts/package.sh` and the CI
//! workflow use the same names; tests keep them in sync.

use std::collections::BTreeMap;

/// The pinned ffmpeg builds.
pub const FFMPEG_LOCK: &str = include_str!("../packaging/ffmpeg.lock");
/// The pinned packaging tools.
pub const TOOLS_LOCK: &str = include_str!("../packaging/tools.lock");

/// Installer targets.
pub const TARGETS: [&str; 3] = ["linux-x86_64", "windows-x86_64", "macos-aarch64"];

/// One `[section]` of a lock file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    pub fields: BTreeMap<String, String>,
}

impl LockEntry {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.fields.get(key).map(String::as_str)
    }
}

/// Parses a lock file: `[name]` headers followed by `key = value` lines; `#` comments.
pub fn parse_lock(text: &str) -> Result<Vec<LockEntry>, String> {
    let mut out: Vec<LockEntry> = Vec::new();
    for (n, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            out.push(LockEntry {
                name: name.trim().to_string(),
                fields: BTreeMap::new(),
            });
            continue;
        }
        let (k, v) = line
            .split_once('=')
            .ok_or_else(|| format!("line {}: expected key = value", n + 1))?;
        let entry = out
            .last_mut()
            .ok_or_else(|| format!("line {}: value before any [section]", n + 1))?;
        if entry
            .fields
            .insert(k.trim().to_string(), v.trim().to_string())
            .is_some()
        {
            return Err(format!("line {}: duplicate key {}", n + 1, k.trim()));
        }
    }
    Ok(out)
}

/// The pinned ffmpeg build for `target`.
pub fn ffmpeg_pin(target: &str) -> Option<LockEntry> {
    parse_lock(FFMPEG_LOCK)
        .ok()?
        .into_iter()
        .find(|e| e.name == target)
}

/// The installer target of the running build.
pub fn current_target() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos-aarch64"
    } else if cfg!(windows) {
        "windows-x86_64"
    } else {
        "linux-x86_64"
    }
}

/// The files attached to the release of `version`.
pub fn release_assets(version: &str) -> Vec<String> {
    vec![
        format!("videolore-{version}-windows-x86_64-setup.exe"),
        format!("videolore-{version}-macos-aarch64.dmg"),
        format!("videolore-{version}-linux-x86_64.AppImage"),
        format!("videolore-{version}-linux-x86_64.tar.gz"),
        format!("videolore-{version}-third-party-source.tar.xz"),
    ]
}

/// True for a lower-case 64-digit hex SHA-256.
pub fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    // covers: REQ-040, REQ-030
    #[test]
    fn every_target_pins_an_ffmpeg_build_by_checksum() {
        let entries = parse_lock(FFMPEG_LOCK).unwrap();
        assert_eq!(entries.len(), 3);
        for t in TARGETS {
            let e = ffmpeg_pin(t).unwrap_or_else(|| panic!("no pin for {t}"));
            let pairs: Vec<(&str, &str)> = if t == "macos-aarch64" {
                vec![
                    ("ffmpeg_url", "ffmpeg_sha256"),
                    ("ffprobe_url", "ffprobe_sha256"),
                ]
            } else {
                vec![("url", "sha256")]
            };
            for (u, h) in pairs {
                assert!(e.get(u).unwrap().starts_with("https://"), "{t} {u}");
                assert!(is_sha256(e.get(h).unwrap()), "{t} {h}");
            }
            for k in [
                "version",
                "licence",
                "build_scripts",
                "build_scripts_commit",
                "source",
            ] {
                assert!(e.get(k).is_some_and(|v| !v.is_empty()), "{t} {k}");
            }
            assert!(e.get("licence").unwrap().starts_with("GPL-3.0"));
        }
        assert!(ffmpeg_pin(current_target()).is_some());
        assert!(ffmpeg_pin("amiga").is_none());
    }

    // covers: REQ-040
    #[test]
    fn packaging_tools_are_pinned() {
        let tools = parse_lock(TOOLS_LOCK).unwrap();
        let get = |n: &str| tools.iter().find(|e| e.name == n).unwrap();
        for n in ["appimagetool", "appimage-runtime"] {
            assert!(is_sha256(get(n).get("sha256").unwrap()));
            assert!(get(n).get("url").unwrap().starts_with("https://"));
        }
        assert_eq!(get("cargo-packager").get("version"), Some("0.11.8"));
    }

    // covers: REQ-042, REQ-040
    #[test]
    fn release_has_three_installers_plus_archive_and_sources() {
        let a = release_assets("1.2.3");
        assert_eq!(a.len(), 5);
        assert!(a.iter().all(|n| n.starts_with("videolore-1.2.3-")));
        assert_eq!(a.iter().filter(|n| n.ends_with("setup.exe")).count(), 1);
        assert_eq!(a.iter().filter(|n| n.ends_with(".dmg")).count(), 1);
        assert_eq!(a.iter().filter(|n| n.ends_with(".AppImage")).count(), 1);
        // the packaging script and the workflow produce/publish exactly these names
        let script = include_str!("../scripts/package.sh");
        for name in release_assets("${VERSION}") {
            assert!(script.contains(&name), "package.sh does not make {name}");
        }
        let ci = include_str!("../.github/workflows/build.yml");
        for glob in [
            "*-setup.exe",
            "*.dmg",
            "*.AppImage",
            "*.tar.gz",
            "*-third-party-source.tar.xz",
        ] {
            assert!(ci.contains(glob), "the workflow does not publish {glob}");
        }
        assert!(ci.contains("scripts/package.sh"));
        assert!(ci.contains("scripts/fetch-ffmpeg.sh"));
    }

    /// The speech recognition code is built for a fixed CPU baseline (never -march=native),
    /// and every CI cache uses the same prefix, so no job can restore an old native build.
    #[test]
    fn whisper_is_built_for_a_portable_cpu_baseline() {
        let config = include_str!("../.cargo/config.toml");
        for line in [
            "GGML_NATIVE = \"OFF\"",
            "GGML_AVX2 = \"ON\"",
            "GGML_FMA = \"ON\"",
            "GGML_F16C = \"ON\"",
        ] {
            assert!(config.lines().any(|l| l.trim() == line), "missing {line}");
        }
        let ci = include_str!("../.github/workflows/build.yml");
        let caches = ci.matches("Swatinem/rust-cache@").count();
        assert!(caches > 0);
        assert_eq!(
            ci.matches("prefix-key: v1-rust-ggml-portable").count(),
            caches
        );
    }

    /// No floating runner labels and no actions referenced by a movable tag: every
    /// action is pinned to a full commit, every runner to an explicit image.
    #[test]
    fn ci_pins_runners_and_actions() {
        let ci = include_str!("../.github/workflows/build.yml");
        assert!(!ci.contains("-latest"), "floating runner label");
        assert!(ci.contains("permissions:\n  contents: read"));
        for line in ci.lines().filter(|l| l.contains("uses:")) {
            let r = line.split("uses:").nth(1).unwrap().trim();
            let (_, rev) = r.split_once('@').expect("action without a version");
            let sha: String = rev.chars().take_while(|c| !c.is_whitespace()).collect();
            assert!(
                sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()),
                "not pinned to a commit: {line}"
            );
        }
    }

    #[test]
    fn lock_parser_rejects_malformed_files() {
        assert!(parse_lock("x = 1").is_err());
        assert!(parse_lock("[a]\njunk").is_err());
        assert!(parse_lock("[a]\nk = 1\nk = 2").is_err());
        let e = parse_lock("# c\n\n[a]\nk = v = w\n").unwrap();
        assert_eq!(e[0].get("k"), Some("v = w"));
        assert!(!is_sha256("ABC"));
    }

    /// Keys of a generated `Packager.toml`, by table: "" is the top level, "{}" collects
    /// the keys of every inline table. Only the subset of TOML the script writes.
    #[cfg(unix)]
    fn toml_keys(text: &str) -> BTreeMap<String, Vec<(String, String)>> {
        let mut out: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
        let mut section = String::new();
        let mut value = String::new();
        let mut key = String::new();
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            if value.is_empty() {
                if let Some(s) = t.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                    section = s.to_string();
                    continue;
                }
                let (k, v) = t.split_once(" = ").expect("key = value");
                key = k.to_string();
                value = v.to_string();
            } else {
                value.push(' ');
                value.push_str(t);
            }
            let open = value.matches('[').count() + value.matches('{').count();
            let close = value.matches(']').count() + value.matches('}').count();
            if open == close {
                let v = std::mem::take(&mut value);
                for inline in inline_tables(&v) {
                    out.entry("{}".into()).or_default().extend(inline);
                }
                out.entry(section.clone())
                    .or_default()
                    .push((key.clone(), v));
            }
        }
        out
    }

    /// The `key = value` pairs of every `{ ... }` inline table in a value.
    #[cfg(unix)]
    fn inline_tables(v: &str) -> Vec<Vec<(String, String)>> {
        let mut tables = Vec::new();
        let mut rest = v;
        while let Some(start) = rest.find('{') {
            let end = start + rest[start..].find('}').expect("closed inline table");
            let body = &rest[start + 1..end];
            let mut pairs = Vec::new();
            let mut depth = 0;
            let mut in_str = false;
            let mut item = String::new();
            for c in body.chars().chain(std::iter::once(',')) {
                match c {
                    '"' => in_str = !in_str,
                    '[' if !in_str => depth += 1,
                    ']' if !in_str => depth -= 1,
                    ',' if !in_str && depth == 0 => {
                        let (k, val) = item.trim().split_once(" = ").expect("inline pair");
                        pairs.push((k.to_string(), val.to_string()));
                        item.clear();
                        continue;
                    }
                    _ => {}
                }
                item.push(c);
            }
            tables.push(pairs);
            rest = &rest[end + 1..];
        }
        tables
    }

    #[cfg(unix)]
    fn unquote(v: &str) -> &str {
        v.trim().trim_matches('"')
    }

    /// The cargo-packager configuration written for Windows and macOS uses only keys and
    /// values that cargo-packager 0.11.8 accepts (it rejects unknown keys and enum values
    /// are case-sensitive), and only absolute paths that exist: cargo-packager reads
    /// relative paths from the folder of the configuration file, not the repository.
    // covers: REQ-040, REQ-041, REQ-042
    #[cfg(unix)]
    #[test]
    fn packager_config_is_valid_for_cargo_packager() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // accepted spellings (camelCase field names plus the kebab/snake aliases)
        let top = [
            "name",
            "product-name",
            "identifier",
            "version",
            "description",
            "authors",
            "publisher",
            "license-file",
            "out-dir",
            "binaries-dir",
            "binaries",
            "icons",
            "resources",
            "formats",
            "file-associations",
        ];
        let inline = [
            "path",
            "main",
            "src",
            "target",
            "extensions",
            "name",
            "description",
            "mime-type",
            "role",
        ];
        let nsis = ["installMode", "installer-mode", "installer_mode"];
        let macos = ["minimum-system-version"];
        for (target, format) in [("windows-x86_64", "nsis"), ("macos-aarch64", "app")] {
            let out = std::process::Command::new("bash")
                .arg(root.join("scripts/package.sh"))
                .args([target, "--config-only"])
                .current_dir(root)
                .output()
                .expect("bash runs");
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let file = root.join(format!("dist/stage-{target}/Packager.toml"));
            let text = std::fs::read_to_string(&file).unwrap();
            let keys = toml_keys(&text);
            let allowed: [(&str, &[&str]); 4] = [
                ("", &top),
                ("{}", &inline),
                ("nsis", &nsis),
                ("macos", &macos),
            ];
            assert_eq!(keys.len(), allowed.len(), "unexpected tables: {keys:?}");
            for (table, ok) in allowed {
                for (k, _) in &keys[table] {
                    assert!(ok.contains(&k.as_str()), "{target}: key {k} in [{table}]");
                }
            }
            let get = |table: &str, key: &str| -> Vec<String> {
                keys[table]
                    .iter()
                    .filter(|(k, _)| k == key)
                    .map(|(_, v)| v.clone())
                    .collect()
            };
            // enum values: BundleTypeRole and NSISInstallerMode are camelCase
            let roles = get("{}", "role");
            assert_eq!(roles.len(), 1);
            assert!(
                ["editor", "viewer", "shell", "qLGenerator", "none"].contains(&unquote(&roles[0])),
                "{target}: role {}",
                roles[0]
            );
            let mode = get("nsis", "installMode");
            assert_eq!(mode.len(), 1);
            assert_eq!(unquote(&mode[0]), "currentUser", "per-user install");
            // without a name cargo-packager looks for a Cargo.toml inside the config file
            assert_eq!(get("", "name"), ["\"videolore\""]);
            assert_eq!(get("", "formats"), [format!("[\"{format}\"]")]);
            assert_eq!(unquote(&get("", "version")[0]), env!("CARGO_PKG_VERSION"));
            // paths: absolute, and the ones from the repository exist
            let mut paths: Vec<String> = Vec::new();
            for k in ["license-file", "out-dir", "binaries-dir"] {
                paths.extend(get("", k).iter().map(|v| unquote(v).to_string()));
            }
            for k in ["icons", "resources"] {
                let v = &get("", k)[0];
                let v = v.trim_start_matches('[').trim_end_matches(']');
                paths.extend(
                    v.split(", ")
                        .filter(|p| p.starts_with('"'))
                        .map(|p| unquote(p).to_string()),
                );
            }
            paths.extend(get("{}", "src").iter().map(|v| unquote(v).to_string()));
            assert!(paths.len() >= 7, "{paths:?}");
            for p in &paths {
                let p = std::path::Path::new(p);
                assert!(p.is_absolute(), "{target}: relative path {}", p.display());
                if !p.starts_with(root.join("dist")) {
                    assert!(p.exists(), "{target}: missing {}", p.display());
                }
            }
            for icon in ["icon.png", "icon.ico", "icon.icns"] {
                assert!(paths.iter().any(|p| p.ends_with(icon)), "{target}: {icon}");
            }
            // the .vlproj association (REQ-041)
            assert_eq!(get("{}", "extensions"), ["[\"vlproj\"]"]);
        }
    }
}
