//! `text/replace` as the interpreter does it. The native half is
//! `k_replace_count` and `k_replace_into` in `src/runtime/text.c`, and
//! `tests/replace_agrees_with_its_reference.rs` holds both to one reference
//! written apart from either.

/// `text` with every occurrence of `from` written as `to`, found left to
/// right, the search taking up again after the end of each one. Nothing
/// when `text` holds no occurrence and is its own answer. `from` is never
/// empty: the builtin refuses that before it calls.
pub fn replaced(text: &str, from: &str, to: &str) -> Option<String> {
    let mut found = text.match_indices(from).peekable();
    found.peek()?;
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    for (hit, _) in found {
        out.push_str(&text[at..hit]);
        out.push_str(to);
        at = hit + from.len();
    }
    out.push_str(&text[at..]);
    Some(out)
}
