//! An identifier that is usually short enough to keep in the AST node.
//!
//! Names are 20% of what the front end allocates: 6,983 of its blocks are a
//! `String` holding an identifier the source already contains, copied because
//! the AST outlives the buffer it was lexed from. Interning was measured and
//! declined in #1033 at 365 conversion sites for one field of twenty-nine; the
//! ruling on 2026-08-29 took the other road, which needs no table, no lifetime
//! and no id — the bytes live in the node.
//!
//! Twenty-two is the inline capacity, and it is a measurement rather than a
//! round number. Across `lib/`, 89.8% of identifier occurrences are seven
//! bytes or fewer and 99.77% are twenty-two or fewer; thirty distinct names
//! run longer and each appears exactly once, nearly all of them test function
//! names. So the heap path exists and is exercised, and it is not the path
//! anything hot takes.
//!
//! Twenty-two also makes the whole thing twenty-four bytes, which is what a
//! `String` already costs. The `Inline` variant is one length byte and
//! twenty-two of payload; the `Heap` variant is a `Box<str>` at sixteen. No
//! AST node grows by adopting this, and the spec pins that rather than
//! trusting it.
//!
//! Hand-written rather than taken from a crate, per Cargo.toml's own
//! precedent: the file carries two dependencies by policy, and `src/hash.rs`
//! is here for the same reason.

use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

/// The largest name that costs no allocation.
pub const INLINE: usize = 22;

/// The representation is private, and that is load-bearing rather than
/// tidiness. `new` is the only way in and it decides by length, so every name
/// holding a given text holds it the same way — which is what lets the
/// comparisons below be about the text and stay honest. With the variants
/// public a caller could box a short name by hand, and then two names reading
/// the same would differ in a way only the representation knows about. That
/// was not hypothetical: a `PartialEq` comparing representations passed this
/// file's whole spec until the variants were closed.
#[derive(Clone)]
pub struct Name(Repr);

#[derive(Clone)]
enum Repr {
    Inline { len: u8, buf: [u8; INLINE] },
    Heap(Box<str>),
}

impl Name {
    pub fn new(s: &str) -> Self {
        if s.len() <= INLINE {
            let mut buf = [0u8; INLINE];
            buf[..s.len()].copy_from_slice(s.as_bytes());
            Name(Repr::Inline { len: s.len() as u8, buf })
        } else {
            Name(Repr::Heap(s.into()))
        }
    }

    pub fn as_str(&self) -> &str {
        match &self.0 {
            Repr::Inline { len, buf } => {
                // SAFETY: `Repr` is private to this module and `Repr::Inline`
                // is built in exactly one place — `Name::new`, which copies
                // `s.as_bytes()` for a `&str` and records that slice's length.
                // So `buf[..len]` is a whole `&str` re-read, never a cut
                // through a multi-byte character.
                //
                // Checking it here instead cost 13,295,370 instructions
                // compiling lib/json — 23.5% of the whole front end, and more
                // than the change that introduced this type was ever going to
                // save. `a_name_holds_every_encoding` reads the range back
                // through the public door for one-, two-, three- and
                // four-byte characters, at the boundary and over it.
                unsafe { std::str::from_utf8_unchecked(&buf[..*len as usize]) }
            }
            Repr::Heap(s) => s,
        }
    }

    /// Whether this name is held without an allocation. The spec asserts on
    /// it; nothing in the compiler should need to ask.
    pub fn is_inline(&self) -> bool {
        matches!(self.0, Repr::Inline { .. })
    }
}

impl From<&str> for Name {
    fn from(s: &str) -> Self {
        Name::new(s)
    }
}

impl From<String> for Name {
    /// A `String` that is short enough gives its buffer back rather than
    /// keeping it: the whole point is that the allocation goes away.
    fn from(s: String) -> Self {
        Name::new(&s)
    }
}

impl std::ops::Deref for Name {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for Name {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

/// So a `Map<Name, _>` can be looked up by `&str` without building a `Name`.
/// `Borrow` requires that the borrowed form hash and compare identically,
/// which is why `Hash` below hashes the str and never the representation.
impl Borrow<str> for Name {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

/// Every one of these delegates to `str`. Two names that read the same are
/// the same name whichever side of twenty-two bytes they fell on, and a
/// derived impl would have made the representation observable.
impl PartialEq for Name {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Name {}

impl PartialEq<str> for Name {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Name {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl Ord for Name {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for Name {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// `{name}` prints the name, not the box around it.
impl std::fmt::Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// `{name:?}` prints what a `String` would, so a diagnostic that debug-prints
/// an AST node reads the same as it did before.
impl std::fmt::Debug for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Default for Name {
    fn default() -> Self {
        Name(Repr::Inline { len: 0, buf: [0u8; INLINE] })
    }
}

/// A name that has to become an owned `String` — a diagnostic building a
/// sentence, a backend writing a symbol — says so at the site rather than
/// through `Deref` and a `to_string` a reader has to notice.
impl From<Name> for String {
    fn from(n: Name) -> String {
        n.as_str().to_string()
    }
}

impl From<&Name> for String {
    fn from(n: &Name) -> String {
        n.as_str().to_string()
    }
}

/// Both directions, because the front end compares names against owned
/// strings held in tables as often as the other way round, and a comparison
/// that has to be spelled with an `.as_str()` on one side only is a comparison
/// somebody writes backwards.
impl PartialEq<String> for Name {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Name> for String {
    fn eq(&self, other: &Name) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Name> for &str {
    fn eq(&self, other: &Name) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Name> for str {
    fn eq(&self, other: &Name) -> bool {
        self == other.as_str()
    }
}
