//! Base64, RFC 4648, as `std/base64` speaks it: the interpreter's half. The
//! native half is `k_base64_spell` and `k_base64_read` in
//! `src/runtime/text.c`, and `tests/base64_agrees_with_its_reference.rs`
//! holds both to one reference written apart from either.
//!
//! Reading is as lenient as the library was while it was written in kanso,
//! and no more. Text comes in groups of four characters. The first two of a
//! group are letters of the alphabet. The third and fourth are letters or
//! `=`: an `=` in the third place ends the group after one byte, whatever
//! the fourth holds, and an `=` in the fourth ends it after two. A group
//! with padding may be followed by more groups, and the bits a padded group
//! leaves over are not asked to be zero.

const LETTERS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as base64, padded with `=` to whole groups.
pub fn spell(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let a = group[0] as u32;
        let b = group.get(1).copied().unwrap_or(0) as u32;
        let c = group.get(2).copied().unwrap_or(0) as u32;
        let n = a << 16 | b << 8 | c;
        out.push(LETTERS[(n >> 18) as usize]);
        out.push(LETTERS[(n >> 12 & 63) as usize]);
        out.push(if group.len() > 1 { LETTERS[(n >> 6 & 63) as usize] } else { b'=' });
        out.push(if group.len() > 2 { LETTERS[(n & 63) as usize] } else { b'=' });
    }
    String::from_utf8(out).expect("the alphabet is ascii")
}

/// A character's six bits, 64 for the `=` that pads, 255 for anything else.
fn place(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        b'=' => 64,
        _ => 255,
    }
}

/// The bytes `text` spells, or `None` when it is not base64.
pub fn read(text: &[u8]) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for group in text.chunks_exact(4) {
        let (a, b, c, d) = (place(group[0]), place(group[1]), place(group[2]), place(group[3]));
        if a > 63 || b > 63 || c > 64 || d > 64 {
            return None;
        }
        out.push(a << 2 | b >> 4);
        if c == 64 {
            continue;
        }
        out.push(b << 4 | c >> 2);
        if d == 64 {
            continue;
        }
        out.push(c << 6 | d);
    }
    Some(out)
}
