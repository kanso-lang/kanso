//! `kanso::compare::bytes` is the tab's `memcmp`, and `byte_from` its search
//! for one byte; the second is held to `position` at the end of this file.
//!
//! `kanso::compare::bytes` is the tab's `memcmp`. This fuzzer holds it to an
//! independent reference, the standard library's ordering of two byte slices,
//! on runs of every length up to 48 at every pair of offsets into a buffer up
//! to seven bytes out of alignment. Each pair agrees on a prefix of random
//! length and then may differ in one byte, with the two bytes drawn so that
//! half the differences cross 0x80, where a signed comparison would order them
//! the other way. Only the sign of the answer is compared, which is all
//! `memcmp` promises.

use kanso::compare::{byte_from, bytes, either_from};
use std::cmp::Ordering;

/// xorshift64*, seeded, so a failure names a case that can be replayed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> usize {
        (self.next() % n) as usize
    }
}

const CASES: u64 = 4_000_000;

#[test]
fn the_word_compare_agrees_with_the_slice_order() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut a = [0u8; 64];
    let mut b = [0u8; 64];
    for case in 0..CASES {
        let n = rng.below(49);
        let (oa, ob) = (rng.below(8), rng.below(8));
        for i in 0..n {
            a[oa + i] = rng.next() as u8;
        }
        b[ob..ob + n].copy_from_slice(&a[oa..oa + n]);
        if n > 0 && rng.below(4) != 0 {
            let at = rng.below(n as u64);
            let (x, y) = if rng.below(2) == 0 {
                (rng.next() as u8 & 0x7f, rng.next() as u8 | 0x80)
            } else {
                (rng.next() as u8, rng.next() as u8)
            };
            a[oa + at] = x;
            b[ob + at] = y;
        }
        let want = a[oa..oa + n].cmp(&b[ob..ob + n]);
        let got = unsafe { bytes(a.as_ptr().add(oa), b.as_ptr().add(ob), n) }.cmp(&0);
        assert_eq!(
            got,
            want,
            "case {case}: n {n}, offsets {oa} and {ob}, {:?} against {:?}",
            &a[oa..oa + n],
            &b[ob..ob + n]
        );
    }
}

/// The edges the random cases reach only by chance: every length up to 24,
/// equal throughout, and differing in exactly one byte at every position, in
/// both directions, with the bytes 0x7f and 0x80.
#[test]
fn the_word_compare_orders_every_position_of_every_short_length() {
    for n in 0..=24usize {
        let a: Vec<u8> = (0..n as u8).map(|i| i.wrapping_mul(37)).collect();
        assert_eq!(unsafe { bytes(a.as_ptr(), a.clone().as_ptr(), n) }, 0, "n {n} equal");
        for at in 0..n {
            let (mut lo, mut hi) = (a.clone(), a.clone());
            lo[at] = 0x7f;
            hi[at] = 0x80;
            let below = unsafe { bytes(lo.as_ptr(), hi.as_ptr(), n) };
            let above = unsafe { bytes(hi.as_ptr(), lo.as_ptr(), n) };
            assert_eq!(below.cmp(&0), Ordering::Less, "n {n}, at {at}");
            assert_eq!(above.cmp(&0), Ordering::Greater, "n {n}, at {at}");
        }
    }
}

/// `byte_from` against `position` over the same bytes, for a newline, an `@`,
/// zero and two high bytes: texts up to 40 bytes drawn from bytes one bit away
/// from each target, so the has-zero test's borrows and its `!w` are tried, at
/// every starting point in each text.
#[test]
fn byte_from_agrees_with_position() {
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    for target in [b'\n', b'@', 0x00, 0x80, 0xff] {
        let pool = [target, target ^ 1, target ^ 2, target ^ 0x80, target ^ 0x7f, b'a', 0x01, 0xfe];
        for case in 0..100_000 {
            let len = rng.below(41);
            let text: Vec<u8> = (0..len).map(|_| pool[rng.below(pool.len() as u64)]).collect();
            for from in 0..=len {
                let want = text[from..].iter().position(|&b| b == target).map_or(len, |n| from + n);
                assert_eq!(
                    byte_from(&text, from, target),
                    want,
                    "target {target:#04x}, case {case}: {text:?} from {from}"
                );
            }
        }
    }
}

/// `either_from` against `position` for either of two bytes, over the pairs
/// the emitter asks about and two high bytes, the texts drawn from both
/// targets and their one-bit neighbours.
#[test]
fn either_from_agrees_with_position() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for (a, b) in [(b'@', b'\n'), (0x00, 0xff), (0x80, 0x7f)] {
        let pool = [a, b, a ^ 1, b ^ 1, a ^ 0x80, b ^ 0x80, b' ', 0x01];
        for case in 0..100_000 {
            let len = rng.below(41);
            let text: Vec<u8> = (0..len).map(|_| pool[rng.below(pool.len() as u64)]).collect();
            for from in 0..=len {
                let want =
                    text[from..].iter().position(|&c| c == a || c == b).map_or(len, |n| from + n);
                assert_eq!(
                    either_from(&text, from, a, b),
                    want,
                    "{a:#04x} or {b:#04x}, case {case}: {text:?} from {from}"
                );
            }
        }
    }
}
