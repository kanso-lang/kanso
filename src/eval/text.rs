// The interpreter's string: the text a value holds, shared by count so that
// reading a variable copies a pointer, with the traits the rest of eval
// compares, hashes and prints it by. eval.rs reads this file with include!, so
// the items stay in the eval module, as the other files under src/eval/ do
// (kanso#1867). cargo fmt does not reach an included file, so CI's format job
// checks the files under src/'s subdirectories itself.

/// A string value. Reading a variable copies its value, and the strings an
/// interpreted program holds are mostly a few bytes long, so copying a
/// `String` was mostly the allocator's work. A string of up to
/// `TEXT_INLINE` bytes is held in place and copies as plain bytes; a longer
/// one is a `String` as before.
pub struct Text(Held);

enum Held {
    Inline(u8, [u8; TEXT_INLINE]),
    Heap(String),
}

impl Text {
    pub fn as_str(&self) -> &str {
        match &self.0 {
            // Only `From<&str>` writes these bytes, copied whole from a
            // `str`, so the first `len` of them are valid UTF-8.
            Held::Inline(len, bytes) => unsafe {
                std::str::from_utf8_unchecked(&bytes[..*len as usize])
            },
            Held::Heap(s) => s,
        }
    }

    pub fn into_string(self) -> String {
        match self.0 {
            Held::Inline(..) => self.as_str().to_string(),
            Held::Heap(s) => s,
        }
    }
}

// Written by hand: the derived `Clone` on the wrapper cost 5.7 million
// instructions on the interpreted corpus inside `Value::clone`, and this one
// costs what a bare enum's does.
impl Clone for Text {
    #[inline]
    fn clone(&self) -> Text {
        match &self.0 {
            Held::Inline(len, bytes) => Text(Held::Inline(*len, *bytes)),
            Held::Heap(s) => Text(Held::Heap(s.clone())),
        }
    }
}

impl From<&str> for Text {
    fn from(s: &str) -> Text {
        match s.len() <= TEXT_INLINE {
            true => {
                let mut bytes = [0; TEXT_INLINE];
                bytes[..s.len()].copy_from_slice(s.as_bytes());
                Text(Held::Inline(s.len() as u8, bytes))
            }
            false => Text(Held::Heap(s.to_string())),
        }
    }
}

impl From<String> for Text {
    fn from(s: String) -> Text {
        match s.len() <= TEXT_INLINE {
            true => Text::from(s.as_str()),
            false => Text(Held::Heap(s)),
        }
    }
}

impl std::ops::Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Text) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Text {}

impl PartialOrd for Text {
    fn partial_cmp(&self, other: &Text) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Text {
    fn cmp(&self, other: &Text) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl std::hash::Hash for Text {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.as_str().hash(h)
    }
}

impl std::fmt::Debug for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
