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
//! Iteration order changes, and nothing the compiler WRITES depends on it:
//! `std`'s random key already reseeds every process, so a compiler whose
//! output moved with map order would have had flaky goldens from the day it
//! was written.
//!
//! What the compiler COSTS is a different question, and the fixed seed is load
//! bearing for it. `bench/compile_instructions_golden.txt`,
//! `bench/entry_instructions_golden.txt` and
//! `bench/library_instructions_golden.txt` each hold one exact value, so two
//! runs of one binary over one input must retire the same instructions. Under
//! `RandomState` they do not: the probe sequence differs per process and the
//! work of building the same table with it. kanso#1449 declared one set with
//! std's default and the three rows returned three distinct values over three
//! CI rounds; eight local runs of one binary spread 313 instructions, and a
//! profile diff put the entire delta in `hashbrown`'s insert.
//!
//! So a container on the path `kanso check` walks belongs here rather than in
//! `std::collections`, and
//! `tests/the_compile_path_hashes_with_a_fixed_seed.rs` reads `src/` and says
//! so.

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

/// The digest of a byte string, taken at BUILD time.
///
/// `src/runtime.c` is 450,100 bytes and two cache keys must both change when
/// it does: the staged `kanso_runtime_*.o` and the linked `kanso_run_*`. Both
/// used to get that by handing the whole file to a `DefaultHasher`, so every
/// `kanso play`, `kanso run` and `kanso build` walked it twice before doing
/// any work of its own. Callgrind on `kanso play` over a one-line program put
/// `sip::Hasher::write` at 1,226,463 instructions of self time -- 25.35% of
/// everything under `kanso::main`, for a constant that cannot differ between
/// two processes of one binary.
///
/// A constant's digest is a constant, so `RUNTIME_DIGEST` is computed by the
/// compiler that builds this one and the running compiler folds in eight
/// bytes.
///
/// WHY TWO ACCUMULATORS. FNV-1a is a good enough mixer for content that has no
/// adversary, and its known weakness is the high bits of short inputs. Two
/// passes with different primes and different offsets are folded in together,
/// so a key carries 128 bits rather than 64. A collision here is not a slow
/// build; it is a runtime object reused against IR that was built for a
/// different one, which is a miscompile. The cost of the second pass is paid
/// once, by the build.
pub const fn digest_of(bytes: &[u8]) -> (u64, u64) {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut i = 0;
    // EIGHT BYTES A STEP, because this loop is interpreted. `const` evaluation
    // runs one step at a time under a budget rustc denies by default, and a
    // byte-at-a-time walk over 450,100 bytes exceeds it. A word at a time is
    // the same function of the same bytes at an eighth of the steps.
    while i + 8 <= bytes.len() {
        let word = u64::from_le_bytes([
            bytes[i],
            bytes[i + 1],
            bytes[i + 2],
            bytes[i + 3],
            bytes[i + 4],
            bytes[i + 5],
            bytes[i + 6],
            bytes[i + 7],
        ]);
        a = (a ^ word).wrapping_mul(0x0000_0100_0000_01b3);
        b = (b ^ word).wrapping_mul(0x0000_0000_0100_0193).rotate_left(29);
        i += 8;
    }
    while i < bytes.len() {
        let byte = bytes[i] as u64;
        a = (a ^ byte).wrapping_mul(0x0000_0100_0000_01b3);
        b = (b ^ byte).wrapping_mul(0x0000_0000_0100_0193).rotate_left(29);
        i += 1;
    }
    // The length goes in too: a tail shorter than a word is folded byte by
    // byte, so two contents differing only in trailing zero bytes would
    // otherwise be free to meet.
    a ^= bytes.len() as u64;
    b = b.wrapping_add(bytes.len() as u64);
    (a, b)
}

/// `src/runtime.c`, digested by the compiler that built this one.
///
/// `tests/the_runtime_source_is_not_hashed_at_run_time.rs` holds both halves
/// of what this is for: that nothing walks the source at run time, and that
/// this constant is the digest of the bytes it names.
pub const RUNTIME_DIGEST: (u64, u64) = digest_of(include_str!("runtime.c").as_bytes());

/// A 128-bit key for bytes read at RUN time, where `digest_of` is for bytes
/// known when the compiler is built.
///
/// `cached_program_binary` keys the linked program on its IR, and a collision
/// there runs a different program than the one asked for. `DefaultHasher`
/// gave it 64 bits of SipHash at about two and a half instructions a byte:
/// the one-line start-up corpus emits 40,413 bytes of IR, mostly runtime
/// declarations, and hashing them cost 101,260 instructions, about a tenth of
/// the start-up row.
///
/// Two lanes take alternate words, in xxHash64's round: the word is
/// multiplied, added, rotated, and multiplied again. A multiply carries a
/// difference only upward, so the rotate brings a high-bit difference back
/// down, and the multiply after it spreads that difference before the next
/// word arrives. Both halves matter. Without the rotate a difference
/// confined to the top bits stays there and a later one can cancel it, which
/// is the shape of the word-FNV in `digest_of`, and why that one carries a
/// second, rotating accumulator. Without the second multiply the top bit is
/// the exception: an odd multiply flips only the top bit when the top bit
/// flips, the rotate moves it to bit 30 whole, and a flip of bit 30 in the
/// lane's next word erases it. The first draft of this function had that
/// shape, and 1,010 of 32,768 single-bit changes met another one's key.
/// Each lane is then run through murmur3's finaliser so every output bit
/// depends on every input bit.
pub fn key_of(bytes: &[u8]) -> (u64, u64) {
    const K1: u64 = 0x9e37_79b9_7f4a_7c15;
    const K2: u64 = 0xc2b2_ae3d_27d4_eb4f;
    let mut a: u64 = 0x2545_f491_4f6c_dd1d;
    let mut b: u64 = 0x1656_67b1_9e37_79f9;
    let round = |acc: u64, word: &[u8]| {
        acc.wrapping_add(u64::from_le_bytes(word.try_into().unwrap()).wrapping_mul(K2))
            .rotate_left(31)
            .wrapping_mul(K1)
    };
    let mut pairs = bytes.chunks_exact(16);
    for pair in &mut pairs {
        let (x, y) = pair.split_at(8);
        a = round(a, x);
        b = round(b, y);
    }
    let mut tail = [0u8; 16];
    let rest = pairs.remainder();
    tail[..rest.len()].copy_from_slice(rest);
    let (x, y) = tail.split_at(8);
    a = round(a, x);
    b = round(b, y);
    // The zero-padded tail cannot tell a short input from one ending in
    // zeros, so the length goes in as well.
    a ^= bytes.len() as u64;
    b = b.wrapping_add(bytes.len() as u64);
    (fmix(a ^ b.rotate_left(32)), fmix(b ^ a))
}

fn fmix(mut h: u64) -> u64 {
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    h = h.wrapping_mul(0xc4ce_b9fe_1a85_ec53);
    h ^ (h >> 33)
}
