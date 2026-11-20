// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 Andrea Lorenzani

//! Structure check: no cycle between the top-level modules of `src/`, and the `base` module
//! (the shared helpers) uses no other module. Only code that ships is read: comments are not
//! in the syntax tree, and `#[cfg(test)]` items and test files are skipped.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::visit::Visit;

/// One edge of the graph: `from` uses `to`, seen in `file`.
type Edges = BTreeMap<(String, String), String>;

/// The source files as `(path relative to src, text)`.
type Sources<'a> = Vec<(String, &'a str)>;

fn is_test_file(rel: &str) -> bool {
    let stem = rel.rsplit('/').next().unwrap_or(rel);
    let stem = stem.strip_suffix(".rs").unwrap_or(stem);
    stem == "tests" || stem.ends_with("_tests")
}

/// The top-level module a file belongs to; `None` for the crate root files.
fn module_of(rel: &str) -> Option<String> {
    let first = rel.split('/').next()?;
    let name = first.strip_suffix(".rs").unwrap_or(first);
    (!matches!(rel, "lib.rs" | "main.rs")).then(|| name.to_string())
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "test")
    })
}

fn has_path_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("path"))
}

struct Collector<'a> {
    modules: &'a BTreeSet<String>,
    found: BTreeSet<String>,
    path_attr: bool,
}

impl Collector<'_> {
    fn note(&mut self, name: &syn::Ident) {
        let n = name.to_string();
        if self.modules.contains(&n) {
            self.found.insert(n);
        }
    }

    fn tree(&mut self, t: &syn::UseTree) {
        match t {
            syn::UseTree::Path(p) => self.note(&p.ident),
            syn::UseTree::Name(n) => self.note(&n.ident),
            syn::UseTree::Rename(r) => self.note(&r.ident),
            syn::UseTree::Glob(_) => {}
            syn::UseTree::Group(g) => g.items.iter().for_each(|i| self.tree(i)),
        }
    }
}

fn attrs_of(i: &syn::Item) -> &[syn::Attribute] {
    use syn::Item::*;
    match i {
        Const(x) => &x.attrs,
        Enum(x) => &x.attrs,
        Fn(x) => &x.attrs,
        Impl(x) => &x.attrs,
        Mod(x) => &x.attrs,
        Static(x) => &x.attrs,
        Struct(x) => &x.attrs,
        Trait(x) => &x.attrs,
        Type(x) => &x.attrs,
        Use(x) => &x.attrs,
        _ => &[],
    }
}

impl<'ast> Visit<'ast> for Collector<'_> {
    fn visit_item(&mut self, i: &'ast syn::Item) {
        let attrs = attrs_of(i);
        if has_path_attr(attrs) {
            self.path_attr = true;
        }
        if is_cfg_test(attrs) {
            return;
        }
        if let syn::Item::Use(u) = i
            && let syn::UseTree::Path(p) = &u.tree
            && p.ident == "crate"
        {
            self.tree(&p.tree);
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_impl_item(&mut self, i: &'ast syn::ImplItem) {
        let attrs = match i {
            syn::ImplItem::Fn(f) => &f.attrs,
            syn::ImplItem::Const(c) => &c.attrs,
            _ => &[][..],
        };
        if !is_cfg_test(attrs) {
            syn::visit::visit_impl_item(self, i);
        }
    }

    fn visit_path(&mut self, p: &'ast syn::Path) {
        let mut segs = p.segments.iter();
        if segs.next().is_some_and(|s| s.ident == "crate")
            && let Some(second) = segs.next()
        {
            self.note(&second.ident);
        }
        syn::visit::visit_path(self, p);
    }
}

/// The edges of the module graph, or an error text (a file that does not parse, or a
/// `#[path]` attribute, which the check cannot follow).
fn graph(sources: &Sources) -> Result<(BTreeSet<String>, Edges), String> {
    let modules: BTreeSet<String> = sources
        .iter()
        .filter_map(|(rel, _)| module_of(rel))
        .collect();
    let mut edges = Edges::new();
    for (rel, text) in sources {
        let (Some(from), false) = (module_of(rel), is_test_file(rel)) else {
            continue;
        };
        let file = syn::parse_file(text).map_err(|e| format!("{rel}: {e}"))?;
        let mut c = Collector {
            modules: &modules,
            found: BTreeSet::new(),
            path_attr: false,
        };
        c.visit_file(&file);
        if c.path_attr {
            return Err(format!("{rel}: #[path] is not followed by this check"));
        }
        for to in c.found.into_iter().filter(|t| *t != from) {
            edges
                .entry((from.clone(), to))
                .or_insert_with(|| rel.clone());
        }
    }
    Ok((modules, edges))
}

/// A cycle as `a -> b -> a` with the file of each edge, or `None`.
fn find_cycle(modules: &BTreeSet<String>, edges: &Edges) -> Option<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Open,
        Done,
    }
    fn walk(
        n: &str,
        edges: &Edges,
        marks: &mut BTreeMap<String, Mark>,
        stack: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        marks.insert(n.to_string(), Mark::Open);
        stack.push(n.to_string());
        for (_, to) in edges.keys().filter(|(from, _)| from == n) {
            match marks.get(to) {
                Some(Mark::Open) => {
                    let at = stack.iter().position(|s| s == to).unwrap_or(0);
                    let mut cyc = stack[at..].to_vec();
                    cyc.push(to.clone());
                    return Some(cyc);
                }
                Some(Mark::Done) => {}
                None => {
                    if let Some(c) = walk(to, edges, marks, stack) {
                        return Some(c);
                    }
                }
            }
        }
        stack.pop();
        marks.insert(n.to_string(), Mark::Done);
        None
    }
    let mut marks = BTreeMap::new();
    for m in modules {
        if marks.contains_key(m) {
            continue;
        }
        if let Some(c) = walk(m, edges, &mut marks, &mut Vec::new()) {
            let files: Vec<String> = c
                .windows(2)
                .map(|w| {
                    let f = &edges[&(w[0].clone(), w[1].clone())];
                    format!("{} -> {} in {f}", w[0], w[1])
                })
                .collect();
            return Some(format!("{} ({})", c.join(" -> "), files.join("; ")));
        }
    }
    None
}

/// Problems found in `sources`: a cycle, or an edge out of `base`.
fn check(sources: &Sources) -> Vec<String> {
    let (modules, edges) = match graph(sources) {
        Ok(g) => g,
        Err(e) => return vec![e],
    };
    let mut problems = Vec::new();
    if let Some(c) = find_cycle(&modules, &edges) {
        problems.push(format!("module cycle: {c}"));
    }
    for ((from, to), file) in &edges {
        if from == "base" {
            problems.push(format!(
                "base uses {to} (in {file}); base must use no module"
            ));
        }
    }
    problems
}

fn read_src(dir: &Path, root: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            read_src(&p, root, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, std::fs::read_to_string(&p).unwrap()));
        }
    }
}

fn run(files: &[(&str, &str)]) -> Vec<String> {
    let sources: Sources = files.iter().map(|(p, t)| (p.to_string(), *t)).collect();
    check(&sources)
}

// covers: REQ-153
#[test]
fn the_real_tree_has_no_module_cycle_and_base_is_a_leaf() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut owned = Vec::new();
    read_src(&root, &root, &mut owned);
    assert!(owned.len() > 40, "the source tree was not found");
    let sources: Sources = owned.iter().map(|(p, t)| (p.clone(), t.as_str())).collect();
    let problems = check(&sources);
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

// covers: REQ-153
#[test]
fn two_modules_using_each_other_are_a_cycle() {
    let p = run(&[
        ("a.rs", "use crate::b::X;\nfn f() {}"),
        ("b.rs", "fn g() -> u8 { crate::a::f(); 1 }"),
    ]);
    assert_eq!(p.len(), 1);
    assert!(p[0].contains("a -> b -> a"), "{p:?}");
    assert!(p[0].contains("b.rs"), "{p:?}");
}

// covers: REQ-153
#[test]
fn comments_and_test_code_are_not_edges() {
    let p = run(&[
        (
            "a.rs",
            "use crate::b::X; // crate::b::Y\n/// see [`crate::b::Z`]\nfn f() {}",
        ),
        (
            "b.rs",
            "// crate::a::f()\n#[cfg(test)]\nmod tests { use crate::a::f; }\n\
             #[cfg(test)]\nfn t() { crate::a::f(); }\n\
             struct S; impl S { #[cfg(test)] fn t() { crate::a::f(); } }",
        ),
        ("b/tests.rs", "use crate::a::f;"),
        ("b/ux_tests.rs", "use crate::a::f;"),
    ]);
    assert!(p.is_empty(), "{p:?}");
}

// covers: REQ-153
#[test]
fn grouped_use_trees_are_followed() {
    let p = run(&[
        ("a.rs", "use crate::{b::X, c::Y};"),
        ("b.rs", "fn f() {}"),
        ("c.rs", "use crate::a::Z;"),
    ]);
    assert_eq!(p.len(), 1);
    assert!(p[0].contains("a -> c -> a"), "{p:?}");
}

// covers: REQ-153
#[test]
fn a_three_module_cycle_is_reported_with_its_path() {
    let p = run(&[
        ("a.rs", "use crate::b::X;"),
        ("b.rs", "use crate::c::X;"),
        ("c.rs", "fn f() { crate::a::g(); }"),
    ]);
    assert_eq!(p.len(), 1);
    assert!(p[0].contains("a -> b -> c -> a"), "{p:?}");
}

// covers: REQ-153
#[test]
fn base_must_not_use_any_module() {
    let p = run(&[("base/mod.rs", "use crate::a::X;"), ("a.rs", "fn f() {}")]);
    assert_eq!(p.len(), 1);
    assert!(p[0].contains("base uses a"), "{p:?}");
    assert!(p[0].contains("base/mod.rs"), "{p:?}");
}

// covers: REQ-153
#[test]
fn a_path_attribute_and_bad_syntax_fail_the_check() {
    let p = run(&[("a.rs", "#[path = \"x.rs\"]\nmod m;")]);
    assert!(p[0].contains("#[path]"), "{p:?}");
    let p = run(&[("a.rs", "fn (")]);
    assert!(p[0].contains("a.rs"), "{p:?}");
}
