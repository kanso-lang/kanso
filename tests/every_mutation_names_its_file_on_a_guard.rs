//! A mutation that names a source file must name it on a GUARD line.
//!
//! The ratchet's `touched` pass selects the rows a branch could have made
//! blind, by intersecting the files the branch changed with the paths each
//! mutation names on a line its `guarding` predicate recognises. A mutation
//! that reaches its file only through a shell variable, or that carries no
//! grep at all, satisfies no guard — so it can never be selected, whatever a
//! branch touches, and its row is proved only by the nightly.
//!
//! That is not hypothetical. kanso#1337 opened `library_ir` and kanso#1338
//! patched the very file its mutation edits, and the pass selected neither it
//! nor its entry twin: both spelled `file=src/lib.rs` and then reached the
//! file through `"$file"`. kanso#1338 repaired those two and counted eight
//! more in the same position.
//!
//! THIS ASSERTS THE PROPERTY, NOT A LIST. kanso#1338 wrote the list-shaped
//! version of this spec, watched it red naming ten, and declined to ship it:
//! with the ten repaired it would have been a green list, and a list is the
//! shape that goes stale the next time somebody adds a mutation. Read off
//! disk, this one goes red on the mutation that arrives blind tomorrow.

use std::path::{Path, PathBuf};

fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Every `src/` file a mutation mentions anywhere in its text.
fn named(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in body.match_indices("src/") {
        let rest = &body[i..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || "._-/".contains(c)))
            .unwrap_or(rest.len());
        let path = &rest[..end];
        if (path.ends_with(".rs") || path.ends_with(".c") || path.ends_with(".h"))
            && !out.iter().any(|p| p == path)
        {
            out.push(path.to_string());
        }
    }
    out
}

/// The lines `scripts/ratchet/ratchet.kso`'s `guarding` keeps. Both spellings:
/// `grep -q` fails outright when the anchor moves, and `grep -c` feeds a count
/// assertion that catches the anchor multiplying as well as vanishing.
fn guards(body: &str) -> Vec<&str> {
    body.lines().filter(|l| l.contains("grep -q") || l.contains("grep -c")).collect()
}

#[test]
fn every_mutation_names_its_file_on_a_guard() {
    let dir: PathBuf = root().join("scripts/ratchet/mutations");
    let mut read = 0usize;
    let mut naming = 0usize;
    let mut blind: Vec<String> = Vec::new();

    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("the mutations directory reads")
        .map(|e| e.expect("an entry").path())
        .filter(|p| p.extension().is_some_and(|e| e == "sh"))
        .collect();
    entries.sort();

    for path in &entries {
        read += 1;
        let body = std::fs::read_to_string(path).expect("a mutation reads");
        let paths = named(&body);
        if paths.is_empty() {
            continue;
        }
        naming += 1;
        let lines = guards(&body);
        let seen = paths.iter().any(|p| lines.iter().any(|l| l.contains(p.as_str())));
        if !seen {
            let name = path.file_name().expect("a name").to_string_lossy().to_string();
            blind.push(format!("{name} names {paths:?} and guards none of them"));
        }
    }

    assert!(read > 100, "the mutations were found: {read} read");
    assert!(naming > 0, "some mutation names a source file: {naming}");
    assert!(
        blind.is_empty(),
        "a mutation the `touched` pass can never select, whatever a branch changes.\n\
         Spell the path on a line carrying `grep -q` or `grep -c`:\n  {}",
        blind.join("\n  ")
    );
}
