//! A blank line in a golden fails a gate on a row that agrees.
//!
//! Nine gates read their golden with `grep -v '^#'` and hand the result to
//! `diff`, so every line that survives the strip has to be a line the gate's
//! own output carries. A comment block appended AFTER the value leaves a blank
//! line between them, that blank line survives `grep -v '^#'`, and the diff
//! reports a difference on a file whose numbers are right.
//!
//! That happened on kanso#1465's round two: `bench/compile_allocs_golden.txt`
//! read `compile_allocs=27397`, CI measured `compile_allocs=27397`, and the
//! job said `compile allocations disagrees with its golden`. The reading was
//! in the job log, agreeing, three lines above the error.
//!
//! So: find every golden read that way, strip it the way the gate does, and
//! assert nothing blank comes through. The list is read off the gates rather
//! than written here, because a gate added with this shape is exactly the one
//! a hand-written list would miss.

use std::path::{Path, PathBuf};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Every `grep -v '^#' <file>` a gate script runs, as the path it names.
///
/// Both spellings appear: the file straight through, and a `$golden` the
/// script set a few lines above.
fn goldens_stripped_by_a_gate() -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root().join("scripts/gates")).expect("the gates are readable") {
        let path = entry.expect("a gate").path();
        if path.extension().is_none_or(|e| e != "sh") {
            continue;
        }
        let script = std::fs::read_to_string(&path).expect("a gate reads");
        let named = script
            .lines()
            .find_map(|l| l.trim().strip_prefix("golden=").map(str::to_string))
            .map(|g| g.trim_matches(['"', '\'']).to_string());
        for line in script.lines() {
            let Some(rest) = line.split_once("grep -v '^#' ").map(|(_, r)| r) else { continue };
            let arg = rest.split_whitespace().next().unwrap_or("");
            let arg = arg.trim_matches(['"', '\'']);
            let file = match arg {
                "$golden" => match &named {
                    Some(g) => g.clone(),
                    // A gate that strips a golden it never names is one this
                    // spec cannot follow, and a silent skip is how a list goes
                    // stale. The loop below fails on the empty result instead.
                    None => continue,
                },
                other if other.starts_with("bench/") => other.to_string(),
                _ => continue,
            };
            let whole = root().join(&file);
            if whole.is_file() {
                found.push((file, whole));
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

#[test]
fn nothing_blank_survives_the_strip() {
    let goldens = goldens_stripped_by_a_gate();
    assert!(
        goldens.len() >= 8,
        "only {} goldens were found to be diffed line by line, and there were \
         nine when this was written. The reader above has gone stale, or the \
         gates stopped naming their goldens where it looks.",
        goldens.len()
    );
    for (name, path) in goldens {
        let text = std::fs::read_to_string(&path).expect("the golden reads");
        for (at, line) in text.lines().filter(|l| !l.starts_with('#')).enumerate() {
            assert!(
                !line.trim().is_empty(),
                "{name} carries a blank line that `grep -v '^#'` lets through, \
                 so its gate diffs one more line than it measures and fails on \
                 a row that agrees. Put the note ABOVE the value, or inside the \
                 comment block, rather than after it. (line {} of the stripped \
                 file)",
                at + 1
            );
        }
    }
}
