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

/// The files attached to the release of `version`: the editor's installers, the third-party
/// source archive, and the installers of videolore player (1.6.0) and of videolore screen
/// spy (3.1.2).
pub fn release_assets(version: &str) -> Vec<String> {
    let mut v = vec![
        format!("videolore-{version}-windows-x86_64-setup.exe"),
        format!("videolore-{version}-macos-aarch64.dmg"),
        format!("videolore-{version}-linux-x86_64.AppImage"),
        format!("videolore-{version}-linux-x86_64.tar.gz"),
        format!("videolore-{version}-third-party-source.tar.xz"),
    ];
    v.extend([
        format!("videolore-player-{version}-windows-x86_64-setup.exe"),
        format!("videolore-player-{version}-macos-aarch64.dmg"),
        format!("videolore-player-{version}-linux-x86_64.AppImage"),
        format!("videolore-player-{version}-linux-x86_64.tar.gz"),
    ]);
    v.extend([
        format!("videolore-screen-spy-{version}-windows-x86_64-setup.exe"),
        format!("videolore-screen-spy-{version}-macos-aarch64.dmg"),
        format!("videolore-screen-spy-{version}-linux-x86_64.AppImage"),
        format!("videolore-screen-spy-{version}-linux-x86_64.tar.gz"),
    ]);
    v
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

    // covers: REQ-042, REQ-040, REQ-138, REQ-341
    #[test]
    fn release_has_three_installers_per_program_plus_archive_and_sources() {
        let a = release_assets("1.2.3");
        assert_eq!(a.len(), 13);
        let editor: Vec<_> = a
            .iter()
            .filter(|n| n.starts_with("videolore-1.2.3-"))
            .collect();
        let player: Vec<_> = a
            .iter()
            .filter(|n| n.starts_with("videolore-player-1.2.3-"))
            .collect();
        let spy: Vec<_> = a
            .iter()
            .filter(|n| n.starts_with("videolore-screen-spy-1.2.3-"))
            .collect();
        assert_eq!((editor.len(), player.len(), spy.len()), (5, 4, 4));
        for set in [&editor, &player, &spy] {
            assert_eq!(set.iter().filter(|n| n.ends_with("setup.exe")).count(), 1);
            assert_eq!(set.iter().filter(|n| n.ends_with(".dmg")).count(), 1);
            assert_eq!(set.iter().filter(|n| n.ends_with(".AppImage")).count(), 1);
        }
        // the packaging script and the workflow produce/publish exactly these names (the
        // script writes the product's name as ${pname})
        let script = include_str!("../scripts/package.sh");
        for name in release_assets("${VERSION}") {
            let in_script = if name.contains("third-party-source") {
                name.clone()
            } else {
                name.replacen("videolore-player", "${pname}", 1)
                    .replacen("videolore-screen-spy", "${pname}", 1)
                    .replacen("videolore-${VERSION}", "${pname}-${VERSION}", 1)
            };
            assert!(
                script.contains(&in_script),
                "package.sh does not make {name}"
            );
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
            ci.matches("prefix-key: v2-rust-ggml-portable").count(),
            caches
        );
    }

    /// No floating runner labels and no actions referenced by a movable tag: every
    /// action is pinned to a full commit, every runner to an explicit image.
    #[test]
    fn ci_pins_runners_and_actions() {
        for ci in [BUILD_YML, INSTALLER_YML, ADVISORIES_YML] {
            assert!(!ci.contains("-latest"), "floating runner label");
            assert!(ci.contains("permissions:\n  contents: read"));
            for line in ci.lines().filter(|l| l.contains("uses:")) {
                let r = line.split("uses:").nth(1).unwrap().trim();
                // a workflow of this repository is not an action from outside
                if r.starts_with("./") {
                    continue;
                }
                let (_, rev) = r.split_once('@').expect("action without a version");
                let sha: String = rev.chars().take_while(|c| !c.is_whitespace()).collect();
                assert!(
                    sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()),
                    "not pinned to a commit: {line}"
                );
            }
        }
    }

    /// The tests that run `package.sh --config-only` write the same stage folders in `dist/`:
    /// one at a time.
    #[cfg(unix)]
    static STAGE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    const BUILD_YML: &str = include_str!("../.github/workflows/build.yml");
    const INSTALLER_YML: &str = include_str!("../.github/workflows/installer-config.yml");
    const ADVISORIES_YML: &str = include_str!("../.github/workflows/advisories.yml");

    // covers: REQ-225
    #[test]
    fn the_installer_configuration_runs_on_packaging_pull_requests_and_gates_the_executables() {
        // the triggers of the installer configuration list the packaging paths
        let on = INSTALLER_YML.split("\njobs:").next().unwrap();
        assert!(on.contains("pull_request:\n    paths:"));
        for path in ["scripts/package.sh", "packaging/**", "assets/icon/**"] {
            assert!(
                on.contains(&format!("- \"{path}\"")),
                "no trigger for {path}"
            );
        }
        assert!(on.contains("workflow_call:"));
        // the Build workflow calls it, and neither the executables nor the installers start
        // unless it passed
        assert!(BUILD_YML.contains("uses: ./.github/workflows/installer-config.yml"));
        let needs: Vec<&str> = BUILD_YML
            .lines()
            .filter(|l| l.trim_start().starts_with("needs:"))
            .collect();
        assert_eq!(needs.len(), 2, "{needs:?}");
        assert!(
            needs.iter().all(|n| n.contains("package-config")),
            "{needs:?}"
        );
        // a broken configuration fails the job: every command of the job stops at its first
        // failure (bash -e is the default of `run`)
        assert!(INSTALLER_YML.contains("scripts/package.sh \"$t\" \"$p\" --config-only"));
    }

    // covers: REQ-226
    #[test]
    fn the_advisory_check_runs_every_week_and_cannot_write() {
        assert!(ADVISORIES_YML.contains("schedule:\n    - cron: \""));
        assert!(ADVISORIES_YML.contains("cargo deny --locked check advisories"));
        assert!(ADVISORIES_YML.contains("permissions:\n  contents: read"));
        assert!(!ADVISORIES_YML.contains("contents: write"));
        assert!(!ADVISORIES_YML.contains("${{ secrets"));
        // the weekly run is not a build: it does not build or publish anything
        assert!(!ADVISORIES_YML.contains("cargo build") && !ADVISORIES_YML.contains("upload"));
        // the check also stays in the build of every push
        assert!(BUILD_YML.contains("cargo deny --locked check"));
    }

    // covers: REQ-227
    #[test]
    fn the_cpu_message_test_runs_on_every_push_and_fails_when_it_is_missing() {
        assert!(BUILD_YML.contains("cargo test --locked -p videolore --lib cpu_message"));
        assert!(BUILD_YML.contains("test result: ok\\. [1-9][0-9]* passed"));
        assert!(BUILD_YML.contains("no cpu_message test ran"));
        // the step belongs to the check job that runs on pushes and pull requests
        let step = BUILD_YML.find("cpu_message").unwrap();
        let build_job = BUILD_YML.find("\n  build:\n").unwrap();
        assert!(step < build_job);
        // and there is such a test in the library (the name of its module path matches)
        let tests = include_str!("app/settings_tests.rs");
        assert!(tests.contains("fn cpu_message_"));
    }

    // covers: REQ-228
    #[test]
    fn the_nsis_toolchain_is_pinned_by_checksum() {
        let tools = parse_lock(TOOLS_LOCK).unwrap();
        let nsis = tools
            .iter()
            .find(|e| e.name == "nsis")
            .expect("an nsis entry");
        assert_eq!(nsis.get("version"), Some("3.09"));
        assert!(nsis.get("url").unwrap().starts_with("https://"));
        assert!(is_sha256(nsis.get("sha256").unwrap()));
        // the Windows installer is not built before the pinned file is checked
        let script = include_str!("../scripts/package.sh");
        let check = script.find("fetch_tool nsis").expect("the nsis check");
        let packager = script.find("cargo packager --release").unwrap();
        assert!(check < packager);
    }

    /// A pinned tool whose file changed fails (a wrong hash in a lock file that points at a
    /// local file), and the right hash passes: the same function checks the NSIS toolchain.
    // covers: REQ-228
    #[cfg(unix)]
    #[test]
    fn a_wrong_hash_fails_the_tool_check() {
        use sha2::{Digest, Sha256};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("nsis.zip");
        std::fs::write(&file, b"not the real toolchain").unwrap();
        let good: String = Sha256::digest(b"not the real toolchain")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let lock = |sha: &str| {
            let p = dir.path().join("tools.lock");
            std::fs::write(
                &p,
                format!(
                    "[nsis]\nversion = 3.09\nurl = file://{}\nsha256 = {sha}\n",
                    file.display()
                ),
            )
            .unwrap();
            std::process::Command::new("bash")
                .arg(root.join("scripts/package.sh"))
                .args(["tool", "nsis"])
                .env("VIDEOLORE_TOOLS_LOCK", &p)
                .current_dir(root)
                .output()
                .expect("bash runs")
        };
        let ok = lock(&good);
        assert!(
            ok.status.success(),
            "{}",
            String::from_utf8_lossy(&ok.stderr)
        );
        let bad = lock(&"0".repeat(64));
        assert!(!bad.status.success());
        assert!(String::from_utf8_lossy(&bad.stderr).contains("checksum mismatch for nsis"));
        // a tool that is not in the lock file stops the build too
        let missing = std::process::Command::new("bash")
            .arg(root.join("scripts/package.sh"))
            .args(["tool", "no-such-tool"])
            .current_dir(root)
            .output()
            .unwrap();
        assert!(!missing.status.success());
    }

    /// The licence file of the Windows installer is the notices with a UTF-8 byte-order mark
    /// (the notices have non-ASCII text); the copy in the repository stays plain.
    // covers: REQ-229
    #[cfg(unix)]
    #[test]
    fn the_licence_file_of_the_windows_installer_starts_with_a_byte_order_mark() {
        let _one = STAGE.lock().unwrap_or_else(|e| e.into_inner());
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for product in ["editor", "player", "screen-spy"] {
            let out = std::process::Command::new("bash")
                .arg(root.join("scripts/package.sh"))
                .args(["windows-x86_64", product, "--config-only"])
                .current_dir(root)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let suffix = match product {
                "editor" => String::new(),
                other => format!("-{other}"),
            };
            let dir = root.join(format!("dist/stage-windows-x86_64{suffix}"));
            let made = std::fs::read(dir.join("third-party-licence.txt")).unwrap();
            assert_eq!(&made[..3], b"\xEF\xBB\xBF");
            let plain: &[u8] = match product {
                "player" => include_bytes!("../packaging/notices/third-party-player.txt"),
                "screen-spy" => include_bytes!("../packaging/notices/third-party-screen-spy.txt"),
                _ => include_bytes!("../packaging/notices/third-party.txt"),
            };
            assert_eq!(&made[3..], plain, "the text is the notices, unchanged");
            let config = std::fs::read_to_string(dir.join("Packager.toml")).unwrap();
            assert!(config.contains("third-party-licence.txt"));
        }
        // the program shows the notices without the mark
        assert!(!crate::notices::THIRD_PARTY.starts_with('\u{feff}'));
        assert!(
            !include_bytes!("../packaging/notices/third-party.txt").starts_with(b"\xEF\xBB\xBF")
        );
    }

    /// The archive has a launcher and a desktop entry next to the program, for file managers
    /// that do not start an extracted program on double-click.
    // covers: REQ-231
    #[cfg(unix)]
    #[test]
    fn the_linux_archive_has_a_launcher_and_a_desktop_entry() {
        use std::os::unix::fs::PermissionsExt;
        let script = include_str!("../scripts/package.sh");
        for name in ["$pname.sh", "$pname.desktop", "$pname.png"] {
            assert!(script.contains(&format!("\"$folder/{name}\"")), "{name}");
        }
        assert!(script.contains("chmod 0755 \"$folder/$pname.sh\""));
        let launcher = include_str!("../packaging/linux/launcher.sh");
        let entry = include_str!("../packaging/linux/launcher.desktop");
        assert!(launcher.starts_with("#!/bin/sh\n"));
        for key in [
            "Type=Application",
            "Icon=@NAME@",
            "Exec=",
            "Name=@DISPLAY@",
            "Terminal=false",
        ] {
            assert!(entry.lines().any(|l| l.starts_with(key)), "{key}");
        }
        assert!(entry.contains("%k"), "the entry finds the folder it is in");
        // the launcher starts the program next to it, through a link too, with the arguments
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("videolore-1.0.0-linux-x86_64");
        std::fs::create_dir(&folder).unwrap();
        let program = folder.join("videolore");
        std::fs::write(&program, "#!/bin/sh\necho \"started: $*\"\n").unwrap();
        let sh = folder.join("videolore.sh");
        std::fs::write(&sh, launcher.replace("@NAME@", "videolore")).unwrap();
        for f in [&program, &sh] {
            std::fs::set_permissions(f, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let run = |launch: &std::path::Path| {
            let o = std::process::Command::new(launch)
                .args(["a b.vlproj"])
                .current_dir("/")
                .output()
                .unwrap();
            assert!(o.status.success());
            String::from_utf8_lossy(&o.stdout).trim().to_string()
        };
        assert_eq!(run(&sh), "started: a b.vlproj");
        let link = dir.path().join("link.sh");
        std::os::unix::fs::symlink(&sh, &link).unwrap();
        assert_eq!(run(&link), "started: a b.vlproj");
        // the entry runs the launcher from the folder of the entry (%k is its path)
        let exec = entry
            .lines()
            .find_map(|l| l.strip_prefix("Exec="))
            .unwrap()
            .replace("@NAME@", "videolore");
        let command = exec.replace(
            "%k",
            &format!("'{}'", folder.join("videolore.desktop").display()),
        );
        let o = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .current_dir("/")
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "started:");
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
    // covers: REQ-040, REQ-041, REQ-042, REQ-138, REQ-248
    #[cfg(unix)]
    #[test]
    fn packager_config_is_valid_for_cargo_packager() {
        let _one = STAGE.lock().unwrap_or_else(|e| e.into_inner());
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
        let nsis = [
            "installMode",
            "installer-mode",
            "installer_mode",
            "template",
        ];
        let macos = ["minimum-system-version", "info-plist-path"];
        // (product, package name, identifier, stage folder suffix, number of associations)
        let products = [
            (
                "editor",
                "videolore",
                "io.github.andrealorenzani.videolore",
                "",
                1,
            ),
            (
                "player",
                "videolore-player",
                "io.github.andrealorenzani.videolore-player",
                "-player",
                5,
            ),
            (
                "screen-spy",
                "videolore-screen-spy",
                "io.github.andrealorenzani.videolore-screen-spy",
                "-screen-spy",
                0,
            ),
        ];
        let mut identifiers = Vec::new();
        for ((target, format), (product, name, ident, suffix, assoc)) in
            [("windows-x86_64", "nsis"), ("macos-aarch64", "app")]
                .into_iter()
                .flat_map(|t| products.map(|p| (t, p)))
        {
            let out = std::process::Command::new("bash")
                .arg(root.join("scripts/package.sh"))
                .args([target, product, "--config-only"])
                .current_dir(root)
                .output()
                .expect("bash runs");
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
            let file = root.join(format!("dist/stage-{target}{suffix}/Packager.toml"));
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
            assert_eq!(roles.len(), assoc, "{product} on {target}");
            for r in &roles {
                assert!(
                    ["editor", "viewer", "shell", "qLGenerator", "none"].contains(&unquote(r)),
                    "{target}: role {r}"
                );
            }
            let mode = get("nsis", "installMode");
            assert_eq!(mode.len(), 1);
            assert_eq!(unquote(&mode[0]), "currentUser", "per-user install");
            // without a name cargo-packager looks for a Cargo.toml inside the config file
            assert_eq!(get("", "name"), [format!("\"{name}\"")]);
            assert_eq!(unquote(&get("", "identifier")[0]), ident);
            identifiers.push(ident);
            // the executable of the product is the main binary
            let bins = get("{}", "path");
            assert_eq!(bins[0], format!("\"{name}\""));
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
            // the .vlproj association (REQ-041) belongs to the editor (REQ-138); the player is
            // offered for the six video types it plays, as a viewer only (REQ-248)
            let exts = get("{}", "extensions").join(" ");
            if product == "editor" {
                assert_eq!(get("{}", "extensions"), ["[\"vlproj\"]"]);
                assert!(text.contains("file-associations"));
            } else if product == "screen-spy" {
                // REQ-341: it opens no file; it tells macOS why it asks for the sound
                assert!(!text.contains("file-associations"));
                assert!(get("nsis", "template").is_empty());
                assert_eq!(get("macos", "info-plist-path").len(), 1);
                assert!(!text.contains("fonts\""), "no subtitle font");
            } else {
                for ext in ["mp4", "mov", "mkv", "webm", "m4v", "avi"] {
                    assert!(exts.contains(&format!("\"{ext}\"")), "{target}: {ext}");
                }
                assert!(!text.contains("vlproj") && !roles.iter().any(|r| unquote(r) != "viewer"));
                // Windows: its own installer template registers "Open with" entries only
                if target == "windows-x86_64" {
                    let template = get("nsis", "template");
                    assert_eq!(template.len(), 1, "the player has an installer template");
                    let path = unquote(&template[0]);
                    assert!(
                        path.ends_with("packaging/windows-player.nsi")
                            && std::path::Path::new(path).is_file()
                    );
                    let nsi = std::fs::read_to_string(path).unwrap();
                    assert!(nsi.contains("OpenWithProgids") && nsi.contains("APP_OPENWITH"));
                    assert!(
                        !nsi.contains("!insertmacro APP_ASSOCIATE"),
                        "never the default"
                    );
                } else {
                    assert!(get("nsis", "template").is_empty());
                }
            }
            // the icons of both programs are the editor's
            for icon in ["icon.png", "icon.ico", "icon.icns"] {
                assert!(text.contains(&format!("assets/icon/{icon}")));
            }
        }
        // the two installers have different identifiers (they can be installed together)
        identifiers.sort();
        identifiers.dedup();
        assert_eq!(identifiers.len(), 3);
    }

    // covers: REQ-248
    #[test]
    fn the_player_is_offered_for_the_six_video_types_on_linux() {
        let script = include_str!("../scripts/package.sh");
        let player = script
            .split("Categories=AudioVideo;Player;Video;")
            .nth(1)
            .and_then(|t| t.lines().nth(1))
            .expect("the player's desktop entry");
        for mime in [
            "video/mp4",
            "video/quicktime",
            "video/x-matroska",
            "video/webm",
            "video/x-m4v",
            "video/x-msvideo",
        ] {
            assert!(player.contains(mime), "{mime} in {player}");
        }
        // an "Open with" entry: nothing sets it as the default
        assert!(!script.contains("xdg-mime default"));
    }
}
