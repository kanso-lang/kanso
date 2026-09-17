//! No tracked file carries a merge conflict marker.
//!
//! On 2026-09-17 `design/compiler-log.md` reached `claude/analysis-once` with
//! two nested conflict regions committed into it -- six marker lines, four
//! whole log entries wrapped around them -- and CI was green on that head.
//! Nothing in the tree reads the log for syntax, so nothing could see it. The
//! same hole covers every prose and data file the gates do not parse.
//!
//! The check is on the markers git writes, `<`x7 and `>`x7 followed by a
//! label. The bare `=`x7 separator is deliberately NOT checked: it is also how
//! Markdown underlines a heading, and a spec that fails on a legal document is
//! a spec people learn to ignore.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Built here rather than written, so this file is not its own counterexample.
fn markers() -> [String; 2] {
    ["<".repeat(7) + " ", ">".repeat(7) + " "]
}

/// Directories no committed file lives in.
const SKIP: [&str; 6] = [".git", "target", "node_modules", ".venv", "dist", "__pycache__"];

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name();
        let name = name.to_string_lossy();
        if SKIP.contains(&name.as_ref()) {
            continue;
        }
        match p.is_dir() {
            true => walk(&p, out),
            false => out.push(p),
        }
    }
}

#[test]
fn no_tracked_file_carries_a_conflict_marker() {
    let root = root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(files.len() > 500, "only {} files were walked", files.len());

    let m = markers();
    let mut caught = Vec::new();
    let mut read = 0usize;
    for f in &files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        read += 1;
        if f.file_name().is_some_and(|n| n == "no_file_carries_a_conflict_marker.rs") {
            continue;
        }
        for (n, line) in text.lines().enumerate() {
            if m.iter().any(|mark| line.starts_with(mark.as_str())) {
                let rel = f.strip_prefix(&root).unwrap_or(f);
                caught.push(format!("{}:{}: {}", rel.display(), n + 1, line));
            }
        }
    }
    assert!(read > 400, "only {read} of {} files were text", files.len());
    assert!(
        caught.is_empty(),
        "a merge conflict was committed rather than resolved:\n{}",
        caught.join("\n")
    );
}
