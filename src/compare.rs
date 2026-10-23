//! Byte comparison a word at a time, for the tab's `memcmp`.
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
