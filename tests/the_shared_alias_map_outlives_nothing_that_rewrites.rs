//! Nothing rewrites the program between the alias map and the call-site pass.
//!
//! `inline::aliases_from` is a fixpoint over the whole program and
//! `wrapper_table` reads its answer. The checker and the call-site rewrite
//! both want that answer, and on the entry and module paths they run back to
//! back over a program neither of them changes, so both paths build it once
//! and hand it to each. That saves a fixpoint and two walks counting group
//! sizes — 429,165 instructions off `kanso check compile_corpus`, 1.17%.
//!
//! The saving is only sound while nothing between the two rewrites the
//! program. A pass slipped in there would leave `wrappers` describing a
//! program that no longer exists, and every golden in the tree would stay
//! green while the rewrite quietly acted on a stale map: the bodies it reads
//! are the ones the new pass wrote, and the names it looks for are the ones
//! the old program had.
//!
//! The borrow checker catches the easy half — `builtins` and `counts` borrow
//! `merged`, so a `&mut merged` before the last of them is used will not
//! compile. It does NOT catch a pass slipped in after `wrapper_table` and
//! before `apply_wrappers`, because by then only owned `String`s are left.
//! That gap is what this file reads the source for.

const SOURCE: &str = include_str!("../src/lib.rs");

/// The statements between `wrapper_table` and `apply_wrappers`, per path.
fn between_the_table_and_the_rewrite(source: &str) -> Vec<(usize, Vec<String>)> {
    let mut regions = Vec::new();
    let mut open: Option<(usize, Vec<String>)> = None;
    for (at, line) in source.lines().enumerate() {
        match &mut open {
            None => {
                if line.contains("inline::wrapper_table(") {
                    open = Some((at + 1, Vec::new()));
                }
            }
            Some((_, collected)) => {
                if line.contains("inline::apply_wrappers(") {
                    regions.push(open.take().expect("the region is open"));
                } else {
                    let statement = line.trim();
                    let noise = statement.is_empty()
                        || statement.starts_with("//")
                        || statement.starts_with("});");
                    if !noise {
                        collected.push(format!("{}: {statement}", at + 1));
                    }
                }
            }
        }
    }
    assert!(open.is_none(), "a wrapper_table with no apply_wrappers after it");
    regions
}

#[test]
fn nothing_touches_the_program_between_the_alias_table_and_the_call_site_pass() {
    let regions = between_the_table_and_the_rewrite(SOURCE);
    assert_eq!(
        regions.len(),
        2,
        "the entry path and the module path share the map; this found {} such region(s)",
        regions.len()
    );
    for (at, statements) in &regions {
        let rewrites: Vec<&String> = statements
            .iter()
            .filter(|s| s.contains("&mut merged") || s.contains("(&mut "))
            .collect();
        assert!(
            rewrites.is_empty(),
            "src/lib.rs:{at} hands a pass the program between the alias table and the \
             call-site rewrite, so the table describes a program that no longer exists:\n  {}",
            rewrites.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n  ")
        );
    }
}
