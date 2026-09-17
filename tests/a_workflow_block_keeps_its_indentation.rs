//! A shell continuation that leaves its block takes the whole workflow with it.
//!
//! On 2026-09-17 a step's shell was given a backslash continuation whose
//! second line started at column zero. Shell reads those two lines as one
//! command. YAML reads the second as a new key at the document's top level,
//! and the file stops parsing. GitHub's answer to a workflow it cannot parse
//! is a run with **zero jobs** and a conclusion of `failure` — no failing job
//! to open, no log to read, and no gate to catch it, because the thing that
//! would catch it is the workflow.
//!
//! So it is caught here, by the rule the line broke: a line inside a block
//! scalar that ends in a backslash continues inside that block. The next line
//! is indented past the key that opened it, or the continuation is not a
//! continuation any more.

use std::path::Path;

/// A line's indentation, or `None` when it is blank — a blank line belongs to
/// whatever block encloses it, at any width.
fn indent(line: &str) -> Option<usize> {
    if line.trim().is_empty() {
        return None;
    }
    Some(line.len() - line.trim_start().len())
}

/// Every `key: |` (or `>`, or either chomped) with the indent its body must
/// beat, and the line it sits on.
fn blocks(text: &str) -> Vec<(usize, usize)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_end();
            t.ends_with(": |") || t.ends_with(": >") || t.ends_with(": |-") || t.ends_with(": >-")
        })
        .filter_map(|(n, l)| indent(l).map(|w| (n, w)))
        .collect()
}

/// One file's verdict, so the spec can check a real workflow and a fixture
/// with the same code: the first offending line, if there is one.
fn escaping_continuation(text: &str) -> Option<(usize, String)> {
    let lines: Vec<&str> = text.lines().collect();
    for (at, key_indent) in blocks(text) {
        let mut line = at + 1;
        while line < lines.len() {
            match indent(lines[line]) {
                None => line += 1,
                // Still inside the block. A backslash at the end of it says
                // the NEXT line is part of the same command, so that line has
                // to be inside the block too.
                Some(w) if w > key_indent => {
                    if lines[line].trim_end().ends_with('\\') {
                        let next = lines.get(line + 1).copied().unwrap_or("");
                        if indent(next).is_some_and(|nw| nw <= key_indent) {
                            return Some((line + 2, next.to_string()));
                        }
                    }
                    line += 1;
                }
                // Out of the block, by a line that did not continue anything.
                Some(_) => break,
            }
        }
    }
    None
}

#[test]
fn no_shell_continuation_leaves_its_block() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows");
    let mut read = 0;
    for entry in std::fs::read_dir(&root).expect("the workflows directory reads") {
        let path = entry.expect("a directory entry reads").path();
        if path.extension().and_then(|e| e.to_str()) != Some("yml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a workflow reads");
        read += blocks(&text).len();
        if let Some((line, said)) = escaping_continuation(&text) {
            panic!(
                "{}:{line} continues a shell command from inside a block scalar out to a column \
                 the block does not cover, so YAML reads it as a top-level key and the whole \
                 workflow stops parsing — which GitHub reports as a run with no jobs at all:\n  \
                 {said}",
                path.display()
            );
        }
    }
    assert!(read > 20, "the workflows carry block scalars to check, and {read} were found");
}

/// The shape that broke it, kept here so the check above is known to be able
/// to fail — and the same shape indented properly, so it is known not to cry
/// wolf over an ordinary multi-line command.
#[test]
fn the_shape_that_broke_it_is_caught_and_its_fixed_twin_is_not() {
    let broken =
        "jobs:\n  a:\n    steps:\n      - run: |\n          echo \"x \\\nkernel=$(uname -r)\"\n";
    let fixed = "jobs:\n  a:\n    steps:\n      - run: |\n          echo \"x \\\n            kernel=$(uname -r)\"\n";
    assert!(
        escaping_continuation(broken).is_some(),
        "the continuation at column zero is the bug, and the check must see it"
    );
    assert!(
        escaping_continuation(fixed).is_none(),
        "a continuation indented inside the block is ordinary shell and must pass"
    );
}
