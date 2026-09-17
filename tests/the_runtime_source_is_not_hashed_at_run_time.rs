//! The 450,100 bytes of `src/runtime.c` are digested at build time, not on
//! every start-up.
//!
//! Two cache keys decide whether a staged object or a linked binary may be
//! reused: `cached_runtime_object` and `cached_program_binary`. Both must
//! change when `runtime.c` changes, and both used to get that by feeding the
//! whole file to a `DefaultHasher` -- so every `kanso play`, `kanso run` and
//! `kanso build` walked 450,100 bytes twice before it had done any work.
//!
//! Callgrind on `kanso play` over a program holding one `print`, on 2026-09-17:
//! `sip::Hasher::write` cost 1,226,463 instructions of self time, 25.35% of
//! everything under `kanso::main`. 900,200 bytes at roughly 1.36 instructions
//! a byte is the whole of it; the one-line program's own IR is rounding.
//!
//! A constant's digest is a constant, so `hash::RUNTIME_DIGEST` is computed
//! once by the compiler that builds this one. The two tests below hold the two
//! halves of that: the cost (nothing walks the source at run time) and the
//! correctness (the constant is the digest of the bytes it claims).

/// The cost half. Reading the source is how this one is written because the
/// property is about what the binary DOES NOT DO, and a run-time assertion
/// that something did not happen has nothing to observe.
#[test]
fn neither_cache_key_feeds_the_runtime_source_to_a_hasher() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(root.join("src/main.rs")).expect("src/main.rs reads");

    let mut offenders = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        if !code.contains(".hash(&mut hasher)") {
            continue;
        }
        if code.contains("include_str!") || code.contains("source.hash") {
            offenders.push(format!("  src/main.rs:{}: {}", n + 1, line.trim()));
        }
    }

    assert!(
        offenders.is_empty(),
        "a cache key hashes the runtime source itself, which costs every \
         start-up a walk over {} bytes. Hash `kanso::hash::RUNTIME_DIGEST` \
         instead -- it is the same function of the same bytes, taken at build \
         time:\n{}",
        include_str!("../src/runtime.c").len(),
        offenders.join("\n"),
    );
}

/// The correctness half. A constant that has drifted from the file it digests
/// would let a changed runtime reuse an object built from the old one, which
/// is a miscompile rather than a slow build.
#[test]
fn the_constant_is_the_digest_of_the_bytes_it_names() {
    let source = include_str!("../src/runtime.c");
    assert_eq!(
        kanso::hash::RUNTIME_DIGEST,
        kanso::hash::digest_of(source.as_bytes()),
        "hash::RUNTIME_DIGEST is not the digest of src/runtime.c",
    );
}

/// And the digest has to separate the contents it is asked to separate. One
/// byte changed anywhere must move it, or the key it feeds cannot do its job.
#[test]
fn one_changed_byte_moves_the_digest() {
    let source = include_str!("../src/runtime.c").as_bytes();
    let mut seen = std::collections::HashSet::new();
    seen.insert(kanso::hash::digest_of(source));

    // The first byte, a byte in the middle, and the last -- the three places a
    // rolling digest is most likely to be blind to.
    for at in [0, source.len() / 2, source.len() - 1] {
        let mut altered = source.to_vec();
        altered[at] ^= 0x01;
        assert!(
            seen.insert(kanso::hash::digest_of(&altered)),
            "flipping a bit at byte {at} left the digest where it was",
        );
    }
    assert_eq!(seen.len(), 4, "four contents, four digests");
}
