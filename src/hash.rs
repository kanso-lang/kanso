//! The compiler's own maps, hashed for speed rather than against an attacker.
//!
//! `std`'s default hasher is SipHash-1-3 with a per-process random key, chosen
//! because a web server keying a map on a request header needs collisions to
//! be unpredictable. A compiler keying a map on the identifiers in a file it
//! was handed has no such adversary, and pays for the protection anyway:
//! callgrind on `kanso check lib/json` put `sip::Hasher::write` and
//! `BuildHasher::hash_one` together at 29.8% of every instruction the front
//! end retired.
//!
//! What replaces it is the multiply-rotate hash rustc has used for its own
//! interner since 2015. It is not collision-resistant and is not meant to be;
//! the keys are the program's own names.
//!
//! Iteration order changes, and nothing observable depends on it: `std`'s
//! random key already reseeds every process, so a compiler whose output moved
//! with map order would have had flaky goldens from the day it was written.

use std::hash::{BuildHasherDefault, Hasher};

pub type Map<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<Fx>>;
pub type Set<K> = std::collections::HashSet<K, BuildHasherDefault<Fx>>;

/// The odd 64-bit constant is the golden ratio scaled to the word size, which
/// is what spreads the multiply's entropy into the high bits.
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

#[derive(Default)]
pub struct Fx {
    hash: u64,
}

impl Fx {
    #[inline]
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for Fx {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while rest.len() >= 8 {
            let (word, tail) = rest.split_at(8);
            self.add(u64::from_le_bytes(word.try_into().unwrap()));
            rest = tail;
        }
        if rest.len() >= 4 {
            let (word, tail) = rest.split_at(4);
            self.add(u32::from_le_bytes(word.try_into().unwrap()) as u64);
            rest = tail;
        }
        if rest.len() >= 2 {
            let (word, tail) = rest.split_at(2);
            self.add(u16::from_le_bytes(word.try_into().unwrap()) as u64);
            rest = tail;
        }
        if let Some(byte) = rest.first() {
            self.add(*byte as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u16(&mut self, n: u16) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u32(&mut self, n: u32) {
        self.add(n as u64);
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.add(n);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.add(n as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}
