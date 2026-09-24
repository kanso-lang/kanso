//! The key that names a cached program binary changes whenever the IR does.
//!
//! `cached_program_binary` reuses `kanso_run_<key>` when the key matches, so
//! two IRs sharing a key run the first one's binary for the second: a program
//! silently replaced by a different one. The key is `hash::key_of` over the
//! IR, and this pins the two ways such a hash goes wrong on text.
//!
//! One changed bit anywhere must move the key. And two changed bits must not
//! cancel, in particular two in the top bits of words the same lane reads: a
//! lane that only xors and multiplies carries a difference upward and never
//! down, so a difference in bit 62 stays in bits 62 and 63 and a second one
//! words later can erase it. Each step's rotate and the multiply after it are
//! what prevent that.
//!
//! Watched red twice. Without the rotate, 3,866 of the 32,768 one-bit changes
//! meet another one's key and 4,920 of the 65,280 top-bit pairs cancel.
//! Without the multiply after it, which was the first draft's shape, 3,633
//! and 61.

use kanso::hash::key_of;

/// Something shaped like what the key is taken over: ASCII lines of IR.
fn text() -> Vec<u8> {
    let mut out = Vec::new();
    let mut n = 0u32;
    while out.len() < 4096 {
        out.extend_from_slice(
            format!("  %t{n} = call %KValue @k_b_append(%KValue %x{n})\n").as_bytes(),
        );
        n += 1;
    }
    out.truncate(4096);
    out
}

#[test]
fn every_single_bit_moves_the_key() {
    let base = text();
    let before = key_of(&base);
    let mut seen = std::collections::HashSet::new();
    seen.insert(before);
    let mut same = 0;
    for bit in 0..base.len() * 8 {
        let mut t = base.clone();
        t[bit / 8] ^= 1 << (bit % 8);
        if !seen.insert(key_of(&t)) {
            same += 1;
        }
    }
    assert_eq!(same, 0, "{same} of {} one-bit changes met another key", base.len() * 8);
}

#[test]
fn two_top_bits_in_one_lane_do_not_cancel() {
    // Lane a reads words 0, 2, 4, ... and lane b words 1, 3, 5, ...; bit 62 of
    // a word is bit 6 of its eighth byte.
    let base = text();
    let before = key_of(&base);
    let words = base.len() / 8;
    let mut pairs = 0;
    let mut same = 0;
    for i in 0..words {
        for j in (i + 2..words).step_by(2) {
            let mut t = base.clone();
            t[i * 8 + 7] ^= 0x40;
            t[j * 8 + 7] ^= 0x40;
            pairs += 1;
            if key_of(&t) == before {
                same += 1;
            }
        }
    }
    assert_eq!(pairs, 65_280);
    assert_eq!(same, 0, "{same} of {pairs} pairs of top-bit changes cancelled");
}

#[test]
fn a_short_input_is_not_one_ending_in_zeros() {
    assert_ne!(key_of(b"abc"), key_of(b"abc\0"));
    assert_ne!(key_of(b""), key_of(b"\0"));
}
