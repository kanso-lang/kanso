//! Every `builtin_` name `lib/` calls is a pattern in `call_builtin`'s match.
//!
//! On 2026-09-22 that function's `match name` became `match name.as_bytes()`
//! and all 51 patterns gained a `b` prefix, so Rust compares the name inline
//! instead of calling `__memcmp_avx2_movbe`. The dispatch was 26.1% of every
//! memcmp the interpreted run made: 107,625 invocations, 564,790 comparisons,
//! 5.25 each, because a match on `&str` switches on the length and then walks
//! the candidates of that length — and the hot names sit in the two biggest
//! buckets, twelve wide. Twelve distinct builtins account for every call in the
//! corpus and `append` alone is 37% of them.
//!
//! The change is mechanical and that is the risk. A mangled literal —
//! `b"appned"`, a dropped underscore — compiles, passes every test that does
//! not exercise that particular builtin, and turns one name into `unknown
//! builtin` at run time. The corpus reaches twelve of the fifty-one.
//!
//! TWO DRAFTS OF THIS SPEC PROVED NOTHING, and both are worth recording
//! because each looked right.
//!
//! The first read the patterns off `src/eval.rs` and called every one. That is
//! self-referential: rename `b"append"` to `b"appned"` and the spec calls
//! `builtin_appned`, finds it dispatches, and passes. Watched doing exactly
//! that.
//!
//! The second took the names from `lib/` instead — a real oracle — and still
//! passed the same mutation, for a better reason: **the programs never reached
//! the interpreter at all**. The checker gates `builtin_` names to std-origin
//! files, so `builtin_append` in a scratch file is refused with `is internal to
//! the standard library` before anything dispatches. The control says it best:
//! `builtin_nosuchthing` gives that same refusal, not `unknown builtin`. Every
//! program in that spec, valid name or nonsense, produced one error message
//! that had nothing to do with what was being tested.
//!
//! So the question is asked where it can be answered: two files that must
//! agree. `lib/*.kso` names its builtins through the `builtin_` door and
//! `src/eval.rs` dispatches them, and neither can move without the other.

use std::path::Path;

const EVAL: &str = include_str!("../src/eval.rs");

/// The byte-string patterns of `call_builtin`'s own match, read off the file.
///
/// Anchored on the twelve-space indentation of that match's arms, which is
/// what separates it from the nested `match name` blocks inside its own arm
/// bodies.
fn patterns() -> Vec<String> {
    let start = EVAL
        .find("        match name.as_bytes() {")
        .expect("call_builtin matches on the name's bytes");
    let body = &EVAL[start..];
    let end = body.find("\n        }\n").expect("the match closes");
    let mut out = Vec::new();
    for line in body[..end].lines() {
        if !line.starts_with("            b\"") {
            continue;
        }
        for piece in line.split("b\"").skip(1) {
            if let Some(name) = piece.split('"').next() {
                if !name.is_empty() && !out.contains(&name.to_string()) {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

/// The `builtin_` names `lib/` calls. Independent of `src/eval.rs`.
fn names_lib_uses() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut stack = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("lib")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("lib reads") {
            let path = entry.expect("the entry reads").path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("kso") {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("the source reads");
            let bytes = text.as_bytes();
            let mut at = 0;
            while let Some(i) = text[at..].find("builtin_") {
                let start = at + i + "builtin_".len();
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_lowercase()
                        || bytes[end].is_ascii_digit()
                        || bytes[end] == b'_')
                {
                    end += 1;
                }
                let name = text[start..end].to_string();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
                at = start;
            }
        }
    }
    out
}

/// A spec whose two sides have both gone empty passes everything, so both are
/// pinned at what they held when the rewrite landed.
#[test]
fn neither_side_has_gone_empty() {
    let p = patterns();
    let u = names_lib_uses();
    assert!(
        p.len() >= 51,
        "only {} byte patterns were read off call_builtin's match, and there \
         were 51 when it was rewritten. Either the match moved or this spec \
         stopped finding it: {p:?}",
        p.len()
    );
    assert!(
        u.len() >= 46,
        "only {} builtin names were found in lib/, and there were 46. An oracle \
         that has gone empty agrees with anything: {u:?}",
        u.len()
    );
}

/// The one that catches a mangled literal.
#[test]
fn every_name_lib_calls_is_a_pattern() {
    let p = patterns();
    let missing: Vec<String> = names_lib_uses().into_iter().filter(|n| !p.contains(n)).collect();
    assert!(
        missing.is_empty(),
        "lib/ calls these through the `builtin_` door and call_builtin has no \
         pattern for them, so they would reach its `_` arm and answer `unknown \
         builtin` at run time. A mangled byte literal in that match does exactly \
         this: {missing:?}"
    );
}

/// Every pattern is plausible as a name, which a `b` landing inside the quotes
/// or a stray character would break.
#[test]
fn every_pattern_is_a_plausible_name() {
    for n in patterns() {
        assert!(
            !n.is_empty()
                && n.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
            "{n:?} is not a plausible builtin name"
        );
    }
}
