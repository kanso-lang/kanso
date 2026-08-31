//! The inline name holds a short identifier without touching the allocator.
//!
//! Names are 20% of what the front end allocates, and the ruling of
//! 2026-08-29 took the inline road over interning. This pins the properties
//! that road rests on, and it enters where the compiler will: through
//! `Name::new`, watching a counting allocator, rather than through the
//! representation.
//!
//! The threshold is a measurement. Across `lib/`, 89.8% of identifier
//! occurrences are seven bytes or fewer and 99.77% are twenty-two or fewer,
//! so the inline path is the one that runs and the heap path is the one that
//! must still be correct.

use kanso::name::{Name, INLINE};
use std::alloc::{GlobalAlloc, Layout, System};

/// Counts the allocations made by the CALLING THREAD, which is the whole of
/// why this is a thread-local and not a static.
///
/// It was a static first, and it was wrong: cargo runs these tests on
/// parallel threads, so another test's allocation lands between the two
/// readings and the delta reads 2 or 3 where the spec demands 1. It passed,
/// then failed, then passed. A counter shared by threads measures the
/// process, and what is under test is one call on one thread.
///
/// `Cell::new(0)` in a `const` block so the thread-local needs no lazy
/// initialisation — an initialiser that allocates would be re-entering the
/// allocator from inside `alloc`.
struct Counting;

thread_local! {
    static ALLOCS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

fn allocs() -> usize {
    ALLOCS.with(|c| c.get())
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        // `try_with` rather than `with`: during thread teardown the
        // thread-local is gone and an allocation then must not panic.
        let _ = ALLOCS.try_with(|c| c.set(c.get() + 1));
        unsafe { System.alloc(l) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// The whole point of the type, stated as what a user of it can observe.
#[test]
fn a_name_of_twenty_two_bytes_allocates_nothing() {
    let short = "a".repeat(INLINE);
    let before = allocs();
    let n = Name::new(&short);
    let after = allocs();
    assert_eq!(after - before, 0, "a name at the inline limit reached the allocator");
    assert_eq!(n.as_str(), short, "and it did not survive the trip");
    assert!(n.is_inline());
}

/// One byte over, and it must still be a correct name. This is the path the
/// thirty long names in `lib/` take.
#[test]
fn a_name_of_twenty_three_bytes_spills_and_reads_back() {
    let long = "a".repeat(INLINE + 1);
    let before = allocs();
    let n = Name::new(&long);
    let after = allocs();
    assert_eq!(after - before, 1, "the spill did not take exactly one allocation");
    assert_eq!(n.as_str(), long);
    assert!(!n.is_inline());
}

/// A name is twenty-four bytes, which is what a `String` already costs, so no
/// AST node grows by adopting it. If this ever fails the trade has changed and
/// the change needs re-measuring rather than re-pinning.
#[test]
fn a_name_is_no_larger_than_the_string_it_replaces() {
    assert_eq!(std::mem::size_of::<Name>(), 24);
    assert_eq!(std::mem::size_of::<Name>(), std::mem::size_of::<String>());
}

/// A name reads and prints as its text on both sides of the threshold.
///
/// This used to try to prove more — that a hand-boxed short name compared
/// equal to an inline one — and it could, because the variants were public.
/// A `PartialEq` comparing representations passed the whole of this file. The
/// answer was to close the variants rather than to write a bigger assertion:
/// `new` is now the only way in, so a name's representation follows from its
/// text and the two cannot disagree. What is left to check here is that
/// nothing about which side it fell on reaches the reader.
#[test]
fn a_name_reads_as_its_text_on_both_sides() {
    let a = Name::new("a");
    assert!(a.is_inline());
    assert_eq!(a, Name::new("a"));
    assert_eq!(a.as_str(), "a");
    assert_eq!(format!("{a}"), "a");
    assert_eq!(format!("{a:?}"), "\"a\"");

    let text = "b".repeat(INLINE + 1);
    let heaped = Name::new(&text);
    assert!(!heaped.is_inline());
    assert_eq!(heaped, Name::new(&text));
    assert_eq!(format!("{heaped}"), text);
    assert_eq!(format!("{heaped:?}"), format!("{:?}", text));
}

/// `Borrow<str>` promises that the borrowed form hashes identically, and a
/// map keyed by `Name` is looked up by `&str` all over the front end. This
/// asks the map rather than the hasher, because the map is what would break.
#[test]
fn a_map_of_names_is_looked_up_by_str() {
    let mut m = std::collections::HashMap::new();
    m.insert(Name::new("value_for"), 1);
    m.insert(Name::new(&"c".repeat(INLINE + 1)), 2);
    assert_eq!(m.get("value_for"), Some(&1));
    assert_eq!(m.get("c".repeat(INLINE + 1).as_str()), Some(&2));
    assert_eq!(m.get("absent"), None);
}

/// Ordering delegates to `str`, so a sorted list of names reads the way a
/// sorted list of strings does regardless of which side each fell on.
#[test]
fn names_sort_the_way_their_text_sorts() {
    let text = ["b".repeat(INLINE + 1), "a".to_string(), "c".to_string()];
    let mut names: Vec<Name> = text.iter().map(|s| Name::new(s)).collect();
    names.sort();
    let mut strings: Vec<&str> = text.iter().map(|s| s.as_str()).collect();
    strings.sort();
    let got: Vec<&str> = names.iter().map(|n| n.as_str()).collect();
    assert_eq!(got, strings);
}

/// The empty name is inline and reads empty, which `Default` has to give.
#[test]
fn the_default_name_is_empty_and_inline() {
    let n = Name::default();
    assert_eq!(n.as_str(), "");
    assert!(n.is_inline());
}

/// Multi-byte utf-8 must not be cut by the byte threshold. Seven three-byte
/// characters are 21 bytes and inline; eight are 24 and spill, and both have
/// to read back as the same text they went in as.
#[test]
fn a_multibyte_name_is_not_cut_by_the_byte_threshold() {
    let inline = "日".repeat(7);
    assert_eq!(inline.len(), 21);
    let n = Name::new(&inline);
    assert!(n.is_inline());
    assert_eq!(n.as_str(), inline);

    let spilled = "日".repeat(8);
    assert_eq!(spilled.len(), 24);
    let n = Name::new(&spilled);
    assert!(!n.is_inline());
    assert_eq!(n.as_str(), spilled);
}

/// The read is unchecked, so the range it hands back is the whole invariant.
///
/// `as_str` calls `from_utf8_unchecked` because validating cost 13,295,370
/// instructions compiling lib/json — 23.5% of the front end. What makes that
/// sound is `Repr` being private and `Name::new` copying `s.as_bytes()`
/// whole, and this reads the range back through the public door for all four
/// utf-8 widths: at the longest length that still fits inline, and again one
/// character over, where the spill has to keep the same text.
///
/// A one-byte error either way is what would go wrong — a length recorded in
/// characters rather than bytes, or a threshold compared before the copy — and
/// either cuts a multi-byte character in half. Under the checked read that was
/// a panic; under the unchecked one it is a `&str` that is not utf-8, so the
/// corpus has to ask rather than trust the range.
#[test]
fn a_name_holds_every_encoding() {
    // one, two, three and four bytes per character
    for ch in ["a", "é", "日", "𝄞"] {
        let w = ch.len();
        let fits = kanso::name::INLINE / w;
        let inline = ch.repeat(fits);
        assert!(inline.len() <= kanso::name::INLINE);
        let n = Name::new(&inline);
        assert!(n.is_inline(), "{inline:?} is {} bytes and should be inline", inline.len());
        assert_eq!(n.as_str(), inline);
        assert_eq!(n.as_str().as_bytes(), inline.as_bytes());
        assert_eq!(n.as_str().chars().count(), fits);

        let over = ch.repeat(fits + 1);
        let n = Name::new(&over);
        assert_eq!(n.as_str(), over);
        assert_eq!(n.as_str().chars().count(), fits + 1);
    }
}
