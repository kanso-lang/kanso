//! Byte comparison and byte search a word at a time, for the tab.
//!
//! The `memcmp` wasm32 links by default is compiler-builtins' loop of one byte
//! per pass, and comparing names and keys made it the costliest function in
//! the tab's front end: 9.5M of the 68.8M instructions it executed on
//! `bench/interp_corpus`, and 17.5M of the emitter's 119.2M. `src/main.rs`
//! exports `bytes` as `memcmp` and `bcmp` in the wasm build. Native builds
//! keep the C library's, which is already vectorised.
//!
//! It compares eight bytes per pass and ends with the eight that finish the
//! run, overlapping bytes already found equal, so no length needs a loop of
//! single bytes after the words. A run of four to seven bytes is read the same
//! way as two four-byte words. The first byte that differs is found from the
//! lowest set bit of the two words' difference, which is the first byte in
//! memory on a little-endian machine; wasm32 is one, and so is every host the
//! fuzzer in `tests/the_word_compare_agrees_with_bytes.rs` runs on.

/// `memcmp`: negative, zero or positive as the first `n` bytes at `a` order
/// before, equal to or after those at `b`, comparing bytes as unsigned.
///
/// # Safety
/// `a` and `b` must each be valid for reads of `n` bytes.
#[inline(always)]
pub unsafe fn bytes(a: *const u8, b: *const u8, n: usize) -> i32 {
    if n >= 8 {
        let mut i = 0;
        while i + 8 <= n {
            let d = unsafe { word(a, b, i) };
            if d != 0 {
                return unsafe { first(a, b, i, d) };
            }
            i += 8;
        }
        if i < n {
            let d = unsafe { word(a, b, n - 8) };
            if d != 0 {
                return unsafe { first(a, b, n - 8, d) };
            }
        }
        return 0;
    }
    if n >= 4 {
        let d = unsafe { half(a, b, 0) };
        if d != 0 {
            return unsafe { first(a, b, 0, d) };
        }
        let d = unsafe { half(a, b, n - 4) };
        if d != 0 {
            return unsafe { first(a, b, n - 4, d) };
        }
        return 0;
    }
    let mut i = 0;
    while i < n {
        let (x, y) = unsafe { (*a.add(i), *b.add(i)) };
        if x != y {
            return x as i32 - y as i32;
        }
        i += 1;
    }
    0
}

/// The difference of the eight bytes at `at`, zero where they agree.
#[inline]
unsafe fn word(a: *const u8, b: *const u8, at: usize) -> u64 {
    let x = unsafe { (a.add(at) as *const u64).read_unaligned() };
    x ^ unsafe { (b.add(at) as *const u64).read_unaligned() }
}

/// The difference of the four bytes at `at`.
#[inline]
unsafe fn half(a: *const u8, b: *const u8, at: usize) -> u64 {
    let x = unsafe { (a.add(at) as *const u32).read_unaligned() };
    (x ^ unsafe { (b.add(at) as *const u32).read_unaligned() }) as u64
}

/// The order of the first byte that differs in the word at `at`.
#[inline]
unsafe fn first(a: *const u8, b: *const u8, at: usize, diff: u64) -> i32 {
    let at = at + (diff.trailing_zeros() / 8) as usize;
    unsafe { *a.add(at) as i32 - *b.add(at) as i32 }
}

/// Where the first `byte` at or after `from` is in `s`, or `s.len()` if there
/// is none. Eight bytes are read per pass, and the lowest byte of the word
/// that equals `byte` is found from the has-zero test: subtracting one from
/// every byte borrows through a zero byte, and `!w` keeps a byte at 0x80 or
/// above from passing for one.
///
/// `str::find` on a char pays a searcher's setup, a call into memchr and a
/// one-byte compare before it reads a byte, which in the tab's compile costs
/// more than the scan between hits when they are close, as the module's
/// newlines and `@` signs are. Natively memchr is vectorised, and the callers
/// here are on the tab's path, where wasm32 has neither.
#[inline]
pub fn byte_from(s: &[u8], mut i: usize, byte: u8) -> usize {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    let pattern = LO * byte as u64;
    while i + 8 <= s.len() {
        let w = u64::from_le_bytes(s[i..i + 8].try_into().expect("eight bytes")) ^ pattern;
        let z = w.wrapping_sub(LO) & !w & HI;
        if z != 0 {
            return i + (z.trailing_zeros() / 8) as usize;
        }
        i += 8;
    }
    while i < s.len() && s[i] != byte {
        i += 1;
    }
    i
}

/// `byte_from` from the start, as `Option`, the shape `str::find` answers in.
#[inline]
pub fn find(s: &[u8], byte: u8) -> Option<usize> {
    let i = byte_from(s, 0, byte);
    (i < s.len()).then_some(i)
}

/// `byte_from` for whichever of two bytes comes first: where the first `a` or
/// `b` at or after `from` is in `s`, or `s.len()`. One word is tested for both,
/// which costs about twice a single byte's test and half of two searches.
#[inline]
pub fn either_from(s: &[u8], mut i: usize, a: u8, b: u8) -> usize {
    const LO: u64 = 0x0101_0101_0101_0101;
    const HI: u64 = 0x8080_8080_8080_8080;
    let (pa, pb) = (LO * a as u64, LO * b as u64);
    while i + 8 <= s.len() {
        let w = u64::from_le_bytes(s[i..i + 8].try_into().expect("eight bytes"));
        let (x, y) = (w ^ pa, w ^ pb);
        let z = (x.wrapping_sub(LO) & !x | y.wrapping_sub(LO) & !y) & HI;
        if z != 0 {
            return i + (z.trailing_zeros() / 8) as usize;
        }
        i += 8;
    }
    while i < s.len() && s[i] != a && s[i] != b {
        i += 1;
    }
    i
}
