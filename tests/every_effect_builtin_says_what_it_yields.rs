//! Every builtin that answers a description says what running it hands over.
//!
//! `desc_yield` decides what a bound continuation receives. For a std wrapper
//! whose body pipes through a declaration — `builtin_listen at . held` — the
//! per-declaration yield answers, because `held` is a group the fixpoint has
//! walked. For a wrapper whose body is a bare builtin call — `net/read c` is
//! `builtin_net_read c.handle`, and nothing follows it — there is no
//! declaration to ask, and the answer comes from a table of builtin names.
//!
//! A name missing from that table falls to the top set. `beat.rs` reads the
//! set to decide whether a loop's carried slot may cross a rewind, so a
//! missing name means the loop keeps the grow-only arena: for a document-sized
//! value that was 248 arena blocks against 2, and a 260 MB peak against 2 MB.
//! Five names were missing — `kill`, `listen`, `accept`, `net_port` and
//! `net_read` — and nothing in the tree could see it, because a table is only
//! as good as whoever last remembered to add to it.
//!
//! So this reads both tables out of the source rather than restating either.
//! Add a builtin that answers a description and forget its yield, and this
//! goes red naming it.
//!
//! Watched red by deleting `net_read`'s arm: `these builtins answer a
//! description and desc_yield does not say what they yield: ["net_read"]`.

/// The names whose call answers `DESC`, read off `builtin_returns`.
fn answer_a_description(source: &str) -> Vec<String> {
    let lines: Vec<&str> = source.lines().collect();
    let end = lines
        .iter()
        .position(|l| l.contains("=> DESC | fails,"))
        .expect("builtin_returns has an arm answering DESC");
    // The arm's patterns run back over as many lines as they need; each
    // earlier arm ends in its own `=>`, which is where this stops.
    let mut start = end;
    while start > 0 && !lines[start - 1].contains("=>") {
        start -= 1;
    }
    let mut names = literals(&lines[start..=end].join("\n"));
    // `print` is typed on its own, beside `err`, because its answer carries
    // its argument's failure bit.
    assert!(
        source.contains(r#"if name == "print" {"#),
        "print is no longer typed where this expects it"
    );
    names.push("print".to_string());
    names.sort();
    names
}

/// Every string literal in a stretch of source.
fn literals(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('"') {
        rest = &rest[open + 1..];
        let Some(close) = rest.find('"') else { break };
        out.push(rest[..close].to_string());
        rest = &rest[close + 1..];
    }
    out
}

/// The body of `fn desc_yield`, up to the next item.
fn the_yield_table(source: &str) -> &str {
    let at = source.find("fn desc_yield<'a>").expect("desc_yield is there");
    let rest = &source[at..];
    let end = rest.find("\n}\n").map(|e| e + 2).unwrap_or(rest.len());
    &rest[..end]
}

#[test]
fn a_builtin_that_answers_a_description_says_what_it_yields() {
    let source = include_str!("../src/infer.rs");
    let table = the_yield_table(source);
    let named: Vec<String> = literals(table);
    let missing: Vec<String> = answer_a_description(source)
        .into_iter()
        .filter(|n| !named.iter().any(|m| m == n))
        .collect();
    assert!(
        missing.is_empty(),
        "these builtins answer a description and desc_yield does not say what \
         they yield: {missing:?}"
    );
}

/// And the reading is not vacuous: the twenty-two names are actually found.
#[test]
fn the_reading_finds_every_effect_builtin() {
    let source = include_str!("../src/infer.rs");
    let found = answer_a_description(source);
    assert_eq!(
        found,
        [
            "accept",
            "env",
            "exists",
            "is_dir",
            "kill",
            "list_dir",
            "listen",
            "make_dir",
            "net_close",
            "net_port",
            "net_read",
            "net_write",
            "now",
            "print",
            "random",
            "read_file",
            "run",
            "sleep",
            "start",
            "write",
            "write_err",
            "write_file",
        ],
        "the effect builtins moved"
    );
}
