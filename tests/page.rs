// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! The project page (`github_page/index.html`): the picture shown when it is shared
//! (REQ-298) and the Content-Security-Policy (REQ-299).

use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("github_page")
}

fn page() -> String {
    std::fs::read_to_string(root().join("index.html")).unwrap()
}

/// The value of the attribute `name` of the first tag that contains `marker`.
fn attr_of(html: &str, marker: &str, name: &str) -> Option<String> {
    let tag = html.split('<').find(|t| t.contains(marker))?;
    let rest = tag.split(&format!("{name}=\"")).nth(1)?;
    Some(rest.split('"').next()?.to_string())
}

// covers: REQ-298
#[test]
fn the_head_names_an_image_that_is_published_and_small() {
    let html = page();
    let url = attr_of(&html, "property=\"og:image\"", "content").expect("og:image");
    let file = url.rsplit('/').next().unwrap();
    assert_eq!(file, "og-card.png");
    let path = root().join("images").join(file);
    let bytes = std::fs::read(&path).expect("the file is in the published folder");
    assert!(bytes.len() <= 200_000, "{} bytes", bytes.len());
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "a still PNG");
    let twitter = attr_of(&html, "name=\"twitter:image\"", "content").unwrap();
    assert_eq!(twitter, url);
    // the workflow publishes the whole folder
    let flow = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/pages.yml"),
    )
    .unwrap();
    assert!(flow.contains("path: github_page"));
}

fn sources(policy: &str, directive: &str) -> Vec<String> {
    let fallback = policy
        .split(';')
        .map(str::trim)
        .find(|d| d.starts_with("default-src "));
    policy
        .split(';')
        .map(str::trim)
        .find(|d| d.starts_with(&format!("{directive} ")))
        .or(fallback)
        .map(|d| d.split_whitespace().skip(1).map(str::to_string).collect())
        .unwrap_or_default()
}

fn allowed(list: &[String], url: &str) -> bool {
    list.iter().any(|s| match s.as_str() {
        "'self'" => !url.contains("://") && !url.starts_with("data:"),
        "data:" => url.starts_with("data:"),
        host => url.starts_with(host) && host.starts_with("https://"),
    })
}

// covers: REQ-299
#[test]
fn the_policy_allows_every_resource_the_page_loads_and_no_script() {
    use sha2::{Digest, Sha256};
    let html = page();
    let policy = attr_of(&html, "Content-Security-Policy", "content").expect("a policy");
    for d in [
        "default-src 'self'",
        "connect-src 'none'",
        "script-src 'none'",
        "object-src 'none'",
    ] {
        assert!(policy.contains(d), "{d}");
    }
    // images: the page's own files, the data icon and the badge host
    let img = sources(&policy, "img-src");
    let mut loaded = 0;
    for chunk in html.split("<img ").skip(1) {
        let tag = chunk.split('>').next().unwrap();
        let src = tag
            .split("src=\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(allowed(&img, src), "image {src} is not allowed");
        loaded += 1;
    }
    assert!(loaded > 0);
    let icon = attr_of(&html, "rel=\"icon\"", "href").unwrap();
    assert!(allowed(&img, &icon));
    // no script is run (the data block of the search engines is not one), no outside style
    for chunk in html.split("<script").skip(1) {
        assert!(
            chunk
                .trim_start()
                .starts_with("type=\"application/ld+json\""),
            "{chunk:.60}"
        );
    }
    assert!(!html.contains("rel=\"stylesheet\""));
    assert!(
        !html.contains(" style=\""),
        "an inline style attribute needs 'unsafe-inline'"
    );
    assert!(!html.contains("@import") && !html.contains("<iframe"));
    // the one inline style block is named by its hash
    let css = html
        .split("<style>")
        .nth(1)
        .unwrap()
        .split("</style>")
        .next()
        .unwrap();
    let hash = base64_of(&Sha256::digest(css.as_bytes()));
    assert!(
        policy.contains(&format!("'sha256-{hash}'")),
        "style hash {hash}"
    );
    assert!(
        sources(&policy, "style-src")
            .iter()
            .any(|s| s.starts_with("'sha256-"))
    );
    // url() in the style stays on the page or on data
    for part in css.split("url(").skip(1) {
        let u = part.split(')').next().unwrap().trim_matches(['"', '\'']);
        assert!(allowed(&img, u), "css url {u}");
    }
}

fn base64_of(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in bytes.chunks(3) {
        let n = u32::from(c[0]) << 16
            | u32::from(*c.get(1).unwrap_or(&0)) << 8
            | u32::from(*c.get(2).unwrap_or(&0));
        for i in 0..4 {
            if i <= c.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}
