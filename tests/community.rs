// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The GitHub community files (REQ-359 to REQ-366): they exist, say what they must, the
//! issue forms are valid YAML, and the public files carry no internal wording.

use sha2::{Digest, Sha256};
use std::path::PathBuf;
use yaml_rust2::{Yaml, YamlLoader};

const ADDRESS: &str = "community@andrealorenzani.name";
/// The clone address in the README: the shape of a mail address, but a git remote.
const GIT_REMOTE: &str = "git@github.com:andrealorenzani/videolore.git";
const ADVISORIES: &str = "https://github.com/andrealorenzani/videolore/security/advisories/new";

const CONDUCT: &str = "CODE_OF_CONDUCT.md";
const CONTRIBUTING: &str = "CONTRIBUTING.md";
const SECURITY: &str = "SECURITY.md";
const SUPPORT: &str = "SUPPORT.md";
const BUG_FORM: &str = ".github/ISSUE_TEMPLATE/bug_report.yml";
const FEATURE_FORM: &str = ".github/ISSUE_TEMPLATE/feature_request.yml";
const CHOOSER: &str = ".github/ISSUE_TEMPLATE/config.yml";
const PR_TEMPLATE: &str = ".github/pull_request_template.md";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(file: &str) -> String {
    std::fs::read_to_string(root().join(file)).unwrap_or_else(|e| panic!("{file}: {e}"))
}

/// Every public file of this scope: the community files, whatever is in the issue
/// template folder, and the README.
fn public_files() -> Vec<String> {
    let mut files: Vec<String> = [
        CONDUCT,
        CONTRIBUTING,
        SECURITY,
        SUPPORT,
        PR_TEMPLATE,
        "README.md",
    ]
    .map(String::from)
    .into();
    let folder = ".github/ISSUE_TEMPLATE";
    for entry in std::fs::read_dir(root().join(folder)).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        files.push(format!("{folder}/{name}"));
    }
    files.sort();
    files
}

/// The text of the `## <heading>` section, up to the next heading or rule.
fn section(text: &str, heading: &str) -> String {
    let title = format!("## {heading}");
    let mut lines = text.lines().skip_while(|l| l.trim_end() != title);
    assert!(lines.next().is_some(), "no section {title}");
    lines
        .take_while(|l| !l.starts_with("## ") && l.trim_end() != "---")
        .collect::<Vec<_>>()
        .join("\n")
}

/// The value of the first `key = value` line of `text`, without quotes.
fn value_of(text: &str, key: &str) -> String {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.trim_start().strip_prefix('='))
        .unwrap_or_else(|| panic!("no {key}"))
        .trim()
        .trim_matches('"')
        .to_string()
}

/// The words of `text` that have the shape of a mail address.
fn mail_addresses(text: &str) -> Vec<&str> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || "@._%+-".contains(c)))
        .map(|w| w.trim_matches(['.', '-']))
        .filter(|w| {
            w.split_once('@').is_some_and(|(local, domain)| {
                let top = domain.rsplit_once('.').map_or("", |(_, top)| top);
                !local.is_empty() && top.len() >= 2 && top.chars().all(|c| c.is_ascii_alphabetic())
            })
        })
        .collect()
}

/// The first term of the forbidden list found in `line`.
fn forbidden_term(line: &str) -> Option<&'static str> {
    const NAMES: [&str; 5] = ["AI", "Claude", "Anthropic", "ChatGPT", "Copilot"];
    const ANY_CASE: [&str; 2] = ["agent", "agents"];
    const PARTS: [&str; 7] = [
        "AGENTS",
        "ai-docs",
        ".claude",
        "docs/workflow",
        "workflow.md",
        "change-workflow",
        "add-feature",
    ];
    let mut words = line.split(|c: char| !(c.is_alphanumeric() || c == '_'));
    let word = words.find_map(|w| {
        NAMES
            .into_iter()
            .find(|n| w == *n)
            .or_else(|| ANY_CASE.into_iter().find(|n| w.eq_ignore_ascii_case(n)))
    });
    word.or_else(|| PARTS.into_iter().find(|p| line.contains(p)))
}

fn yaml(file: &str) -> Yaml {
    let mut docs = YamlLoader::load_from_str(&read(file))
        .unwrap_or_else(|e| panic!("{file} is not valid YAML: {e}"));
    assert_eq!(docs.len(), 1, "{file}: one document");
    docs.remove(0)
}

/// The inputs of an issue form as (label, type, required). Checks what GitHub asks of
/// every form: a name, a description, a body and unique ids.
fn form_fields(file: &str, name: &str, label: &str) -> Vec<(String, String, bool)> {
    let form = yaml(file);
    assert_eq!(form["name"].as_str(), Some(name), "{file}");
    assert!(form["description"].as_str().is_some(), "{file}");
    let labels: Vec<_> = form["labels"]
        .as_vec()
        .expect("labels")
        .iter()
        .filter_map(Yaml::as_str)
        .collect();
    assert_eq!(labels, [label], "{file}");
    let mut ids = Vec::new();
    let mut fields = Vec::new();
    for item in form["body"].as_vec().expect("a body") {
        let kind = item["type"].as_str().expect("a type");
        if kind == "markdown" {
            assert!(item["attributes"]["value"].as_str().is_some(), "{file}");
            continue;
        }
        let id = item["id"].as_str().expect("an id");
        assert!(!ids.contains(&id), "{file}: id {id} twice");
        ids.push(id);
        fields.push((
            item["attributes"]["label"]
                .as_str()
                .expect("a label")
                .to_string(),
            kind.to_string(),
            item["validations"]["required"].as_bool() == Some(true),
        ));
    }
    fields
}

fn field(label: &str, kind: &str, required: bool) -> (String, String, bool) {
    (label.to_string(), kind.to_string(), required)
}

// covers: REQ-359
#[test]
fn conduct_is_covenant_2_1() {
    let text = read(CONDUCT);
    for part in [
        "Contributor Covenant",
        "version 2.1",
        "https://www.contributor-covenant.org/version/2/1/code_of_conduct.html",
    ] {
        assert!(text.contains(part), "{part}");
    }
    assert!(!text.contains("[INSERT"), "the contact placeholder is left");
    assert_eq!(mail_addresses(&text), [ADDRESS]);
    // no other address in any public file
    for file in public_files() {
        for address in mail_addresses(&read(&file).replace(GIT_REMOTE, "")) {
            assert_eq!(address, ADDRESS, "{file}");
        }
    }
    assert_eq!(
        mail_addresses("Write to a.b@example.org. Not cargo-about@0.9.3 or @name."),
        ["a.b@example.org"]
    );
}

// covers: REQ-360
#[test]
fn contributing_names_checks_and_licence() {
    let text = read(CONTRIBUTING);
    for part in [
        "(SUPPORT.md)",
        "(SECURITY.md)",
        "(CODE_OF_CONDUCT.md)",
        "pull request",
    ] {
        assert!(text.contains(part), "{part}");
    }
    // the checks cover the whole workspace, as the continuous integration does
    let checks = section(&text, "Local checks");
    for command in [
        "cargo fmt --all --check",
        "cargo clippy --workspace --all-targets -- -D warnings",
        "cargo test --workspace",
        "scripts/coverage.sh --fail-under 85",
        "scripts/feature-coverage.sh",
    ] {
        assert!(checks.contains(&format!("`{command}`")), "{command}");
    }
    assert!(checks.contains("85%"));
    let needs = section(&text, "Build prerequisites");
    // the versions are those of the repository, so the guide cannot go stale unseen
    let rust = value_of(&read("Cargo.toml"), "rust-version");
    let lock = read("packaging/ffmpeg.lock");
    let linux = lock.split("[linux-x86_64]").nth(1).expect("a Linux pin");
    let ffmpeg = value_of(linux, "version");
    let ffmpeg = ffmpeg.split('-').next().unwrap();
    for part in [
        format!("{rust} or newer").as_str(),
        format!("ffmpeg {ffmpeg}").as_str(),
        "rustfmt",
        "clippy",
        "llvm-tools-preview",
        "cargo install cargo-llvm-cov",
        "libasound2-dev",
        "pkg-config",
        "cmake",
        "libclang-dev",
        "libxcb-randr0-dev",
        "C++ compiler",
        "packaging/ffmpeg.lock",
        "scripts/fetch-ffmpeg.sh",
        "VIDEOLORE_FFMPEG",
        "VIDEOLORE_FFPROBE",
        "VIDEOLORE_REQUIRE_FFMPEG=1",
    ] {
        assert!(needs.contains(part), "{part}");
    }
    assert!(section(&text, "Licence of contributions").contains("GPL-3.0"));
}

// covers: REQ-361
#[test]
fn security_policy_names_private_channel() {
    let text = read(SECURITY);
    let lower = text.to_lowercase();
    for part in [
        "private vulnerability reporting",
        "latest release",
        "do not open a public issue",
    ] {
        assert!(lower.contains(part), "{part}");
    }
    assert!(text.contains(ADVISORIES));
    // security reports have one channel: no mail address at all
    assert_eq!(mail_addresses(&text), Vec::<&str>::new());
}

// covers: REQ-362
#[test]
fn support_links_forms_and_readme() {
    let text = read(SUPPORT);
    for part in [
        "https://github.com/andrealorenzani/videolore/issues/new/choose",
        "(README.md)",
        "(SECURITY.md)",
        "videolore version",
        "operating system",
        "steps",
    ] {
        assert!(text.contains(part), "{part}");
    }
    assert!(text.to_lowercase().contains("no response time is promised"));
}

// covers: REQ-363
#[test]
fn issue_forms_parse_and_require_fields() {
    assert_eq!(
        form_fields(BUG_FORM, "Bug report", "bug"),
        [
            field("videolore version", "input", true),
            field("Operating system", "input", true),
            field("Steps to reproduce", "textarea", true),
            field("Expected result", "textarea", true),
            field("Actual result", "textarea", true),
            field("Logs or screenshots", "textarea", false),
        ]
    );
    assert_eq!(
        form_fields(FEATURE_FORM, "Feature request", "enhancement"),
        [
            field("Problem", "textarea", true),
            field("Proposal", "textarea", true),
        ]
    );
    let chooser = yaml(CHOOSER);
    assert_eq!(chooser["blank_issues_enabled"].as_bool(), Some(false));
    let links = chooser["contact_links"].as_vec().expect("contact links");
    for link in links {
        for key in ["name", "url", "about"] {
            assert!(link[key].as_str().is_some(), "a contact link needs {key}");
        }
    }
    let urls: Vec<_> = links.iter().filter_map(|l| l["url"].as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://github.com/andrealorenzani/videolore/blob/main/SUPPORT.md",
            ADVISORIES,
        ]
    );
}

// covers: REQ-364
#[test]
fn pull_request_template_lists_four_checks() {
    let text = read(PR_TEMPLATE);
    let items: Vec<_> = text.lines().filter(|l| l.starts_with("- [ ] ")).collect();
    assert_eq!(items.len(), 4, "{items:?}");
    for (item, keyword) in
        items
            .iter()
            .zip(["cargo", "Tests", "User-facing", "SPDX-License-Identifier"])
    {
        assert!(item.contains(keyword), "{keyword}: {item}");
    }
}

// covers: REQ-365
#[test]
fn community_files_exist_and_readme_links_them() {
    for file in [
        CONDUCT,
        CONTRIBUTING,
        SECURITY,
        SUPPORT,
        BUG_FORM,
        FEATURE_FORM,
        CHOOSER,
        PR_TEMPLATE,
        "LICENSE",
    ] {
        assert!(!read(file).trim().is_empty(), "{file} is empty");
    }
    let community = section(&read("README.md"), "Community");
    for file in [CONDUCT, CONTRIBUTING, SECURITY, SUPPORT] {
        assert!(community.contains(&format!("]({file})")), "{file}");
    }
    assert!(community.contains("https://github.com/andrealorenzani/videolore/issues/new/choose"));
}

// covers: REQ-366
#[test]
fn public_files_have_no_internal_wording() {
    let mut found = Vec::new();
    for file in public_files() {
        for (number, line) in read(&file).lines().enumerate() {
            if let Some(term) = forbidden_term(line) {
                found.push(format!("{file}:{}: {term}", number + 1));
            }
        }
    }
    assert!(found.is_empty(), "internal wording:\n{}", found.join("\n"));
    // the scanner finds the terms, and leaves the ordinary words alone
    for line in [
        "made with AI.",
        "an Agent does it",
        "see AGENTS.md",
        "in `ai-docs/`",
        "docs/workflow-x.md",
    ] {
        assert!(forbidden_term(line).is_some(), "{line}");
    }
    for line in [
        "AIFF audio, said the maintainer",
        "the command line",
        "actions/workflows/build.yml",
    ] {
        assert_eq!(forbidden_term(line), None, "{line}");
    }
}

// covers: REQ-366
#[test]
fn licence_is_unmodified_gpl3() {
    // a checkout may turn the line ends into CRLF
    let text = read("LICENSE").replace("\r\n", "\n");
    let hash: String = Sha256::digest(text.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    // https://www.gnu.org/licenses/gpl-3.0.txt
    assert_eq!(
        hash,
        "3972dc9744f6499f0f9b2dbf76696f2ae7ad8af9b23dde66d6af86c9dfb36986"
    );
    assert!(section(&read("README.md"), "License").contains("Andrea Lorenzani"));
}
