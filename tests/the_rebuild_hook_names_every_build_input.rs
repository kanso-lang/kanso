//! The rebuild hook names every root the release binary is built from.
//!
//! `hooks/post-merge` rebuilds target/release/kanso after a pull, but only when
//! the pull touched something the binary is built from. That list is the whole
//! of the hook: a root missing from it is a pull that leaves the binary stale
//! and says nothing, which is the failure the hook exists to end.
//!
//! Hand-writing the list gets it wrong. `src`, `Cargo.toml` and `Cargo.lock`
//! are obvious and `lib` is remembered, but the compiler also reads `hako`,
//! and a list written from memory omitted it. So the list is pinned to a
//! property of the tree instead: every `include_str!("../X/...")` under src/
//! names a root X that cargo will rebuild for, and every such X must be in the
//! hook. A new embedded tree turns this red on the commit that adds it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

const HOOK: &str = include_str!("../hooks/post-merge");

/// The names on the hook's `roots="..."` line.
fn watched() -> BTreeSet<String> {
    HOOK.split_once("roots=\"")
        .expect("the hook declares a roots list")
        .1
        .split_once('"')
        .expect("the roots list is closed")
        .0
        .split_whitespace()
        .map(str::to_string)
        .collect()
}

fn rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("src/ is readable") {
        let path = entry.expect("the entry reads").path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            found.push(path);
        }
    }
}

/// Every X in an `include_str!("../X/...")` under src/.
fn embedded_roots() -> BTreeSet<String> {
    let mut sources = Vec::new();
    rust_sources(&root().join("src"), &mut sources);

    let mut roots = BTreeSet::new();
    for path in sources {
        let text = fs::read_to_string(&path).expect("a source file reads");
        for (_, rest) in text.match_indices("include_str!(\"../").map(|(i, m)| (i, &text[i + m.len()..])) {
            let Some((arg, _)) = rest.split_once('"') else {
                continue;
            };
            let Some((first, _)) = arg.split_once('/') else {
                continue;
            };
            roots.insert(first.to_string());
        }
    }
    roots
}

#[test]
fn the_hook_watches_every_embedded_root() {
    let embedded = embedded_roots();
    assert!(!embedded.is_empty(), "src/ embeds at least one tree");

    let watched = watched();
    let missed: Vec<&String> = embedded.difference(&watched).collect();

    assert!(
        missed.is_empty(),
        "hooks/post-merge does not watch {missed:?}, so a pull touching only \
         those leaves target/release/kanso stale and silent"
    );
}

#[test]
fn the_hook_watches_the_crate_itself() {
    let watched = watched();

    for required in ["src", "Cargo.toml", "Cargo.lock"] {
        assert!(
            watched.contains(required),
            "hooks/post-merge does not watch {required}"
        );
    }
}
