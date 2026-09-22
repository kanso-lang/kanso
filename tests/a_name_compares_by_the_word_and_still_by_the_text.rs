//! Inline names compare as two overlapping words, and the answer is the text.
//!
//! `PartialEq for Name` stopped going through `str == str` on 2026-09-22:
//! `eval::lookup` walks the environment comparing each bound name to the one
//! being looked up, and that was 909,375 calls to `__memcmp_avx2_movbe` for
//! 13,336,477 instructions on the interpreted corpus -- 14.7 each, on names of
//! twenty-two bytes or fewer. The comparing was never the cost. The bytes fit
//! in two registers and what those instructions bought was the AVX2 entry
//! sequence and the call around it.
//!
//! The replacement loads `buf[0..16]` as a `u128` and `buf[14..22]` as a
//! `u64`. Those cover all twenty-two bytes and OVERLAP BY TWO, which is what
//! makes the second load fixed-width instead of a tail loop, and it is also
//! what this spec is mostly about: bytes 14 and 15 are read twice, byte 16 is
//! the first byte only the second load sees, and byte 21 is the last byte
//! anything sees. A comparison that quietly stopped at byte 16 would pass a
//! spec built from short names alone, and 89.8% of real identifiers are seven
//! bytes or fewer, so the corpus would never have said.
//!
//! The other half is the zero-fill. `Name::new` zeroes the buffer past the
//! length, so `"ab"` and `"abc"` differ in byte 2 as well as in the length
//! byte. That holds only because `Repr` is private and `new` is the one door;
//! the spec asserts the consequence rather than the reasoning.

use kanso::name::{Name, INLINE};

fn n(s: &str) -> Name {
    Name::new(s)
}

/// Every pair here is EQUAL, and each names the property it would catch.
#[test]
fn equal_texts_are_equal_names() {
    for text in [
        "",                        // nothing at all
        "x",                       // one byte
        "andthen",                 // seven, the 89.8% case
        "sixteen_bytes___",        // exactly the first load
        "seventeen_bytes__",       // one byte into the second load
        "twenty_two_bytes_long!",  // exactly INLINE
        "twenty_three_bytes_long", // one over, so heap
        "a name far longer than twenty-two bytes, which takes the heap road",
    ] {
        assert_eq!(n(text), n(text), "two names of {text:?} must be equal");
        let owned = String::from(text);
        assert_eq!(n(text), n(&owned), "and however they were built");
    }
    assert_eq!(n("twenty_two_bytes_long!").as_str().len(), INLINE);
}

/// One byte apart, at each position the two loads treat differently.
///
/// Byte 16 is the one a comparison that only read the first `u128` would
/// miss. Bytes 14 and 15 are read by both loads. Byte 21 is the last byte of
/// the second. Each of these was a real way to get the ranges wrong.
#[test]
fn one_byte_apart_is_not_equal() {
    let base = "abcdefghijklmnopqrstu."; // 22 bytes
    assert_eq!(base.len(), INLINE);
    for at in 0..INLINE {
        let mut other = base.as_bytes().to_vec();
        other[at] ^= 0x20;
        let other = String::from_utf8(other).expect("still utf-8");
        assert_ne!(
            n(base),
            n(&other),
            "names differing only at byte {at} compared equal; the loads cover \
             0..16 and 14..22, so a wrong range shows up at 14, 15, 16 or 21"
        );
    }
}

/// A shorter name is not a prefix match, which is what the zero-fill buys.
#[test]
fn a_prefix_is_not_the_name() {
    for (short, long) in [
        ("ab", "abc"),
        ("", "a"),
        ("sixteen_bytes__", "sixteen_bytes___"),
        ("twenty_two_bytes_long", "twenty_two_bytes_long!"),
    ] {
        assert_ne!(n(short), n(long), "{short:?} must not equal {long:?}");
        assert_ne!(n(long), n(short), "and the other way round");
    }
}

/// Across the inline/heap boundary, where the fast path does not apply.
///
/// The fast path runs only when BOTH sides are inline. A name of twenty-three
/// bytes takes the heap road, so these pairs exercise the fallback in both
/// directions and prove it still answers about the text.
#[test]
fn the_two_roads_agree_about_the_text() {
    let inline = n("twenty_two_bytes_long!");
    let heap = n("twenty_three_bytes_long");
    assert_ne!(inline, heap);
    assert_ne!(heap, inline);
    assert_eq!(heap, n("twenty_three_bytes_long"));

    // A name built from a longer string that happens to be short is still
    // inline, and must equal the inline one.
    let owned = String::from("twenty_two_bytes_long!");
    let built = Name::new(&owned);
    assert_eq!(built, inline);
}

/// Equality and hashing still agree, which a word compare could break.
///
/// `Hash` reads the text and `eq` no longer does, so the two could drift
/// apart. Anything keyed by `Name` depends on them agreeing.
#[test]
fn equal_names_hash_alike() {
    use std::collections::HashMap;
    let mut m: HashMap<Name, u32> = HashMap::new();
    for (i, text) in [
        "",
        "x",
        "andthen",
        "sixteen_bytes___",
        "seventeen_bytes__",
        "twenty_two_bytes_long!",
        "twenty_three_bytes_long",
    ]
    .iter()
    .enumerate()
    {
        m.insert(n(text), i as u32);
    }
    for (i, text) in [
        "",
        "x",
        "andthen",
        "sixteen_bytes___",
        "seventeen_bytes__",
        "twenty_two_bytes_long!",
        "twenty_three_bytes_long",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(
            m.get(&n(text)),
            Some(&(i as u32)),
            "a name rebuilt from {text:?} did not find its own entry, so eq and \
             Hash have parted"
        );
    }
    assert_eq!(m.len(), 7, "seven distinct names, seven entries");
}
