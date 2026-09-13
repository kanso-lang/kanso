//! Every capacity a `bytes` value carries is even, which is what lets bit 0
//! hold the storage regime.
//!
//! `KBytes.cap` answers two questions at once: how much room the buffer has,
//! and where that buffer came from. It used to answer the second in the SIGN,
//! so every read of the first was a `neg` and a `cmovs` — inlined at
//! ninety-three sites and worth 17,633,310 instructions on the run program.
//! The regime now rides in bit 0 and the room is one `and`.
//!
//! That trade is sound only while no capacity is odd, and there is exactly one
//! place a non-zero capacity is born: `k_b_append_grow` doubles `len + n` and
//! clamps the result up to 64. Both halves are even, so bit 0 is free. A change
//! to that formula — a `+ 1` for a sentinel byte, a round up to an odd class,
//! anything — would make a malloc-backed buffer read as arena-backed, and the
//! rewind would reclaim storage `free` still owns.
//!
//! The second half is narrower and was a real defect on the way in. There are
//! SIX places that read the room out of the field, and converting five left
//! `k_b_append_into`'s single-byte fast path reading an arena cap of C|1 as
//! C+1 — one byte more than the buffer holds, stored past the frontier. Every
//! spec in the suite passed with it in; what said so was the cost-golden sweep,
//! four veins' `alloc_bytes` moving with every allocation COUNT identical. So
//! the scan below is the guard: after this change exactly one sign-stripping
//! capacity expression may remain in the file, and it belongs to KBuf, a
//! different struct with a different convention.
//!
//! THE TEXT IS LIFTED, NOT COPIED. The growth formula and `k_bytes_malloced`
//! are cut out of `src/runtime.c` and compiled here, so the sweep runs the
//! bytes that ship. A copy would go green on code nobody builds.
//!
//! Watched red twice: with the formula written `2 * (a->len + n) + 1` the sweep
//! reports the first odd capacity at len 0, n 0; with `k_b_append_into`'s room
//! read restored to `acap0 < 0 ? -acap0 : acap0` the scan names the line.

use std::path::Path;
use std::process::Command;

fn runtime() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("src/runtime.c")).expect("src/runtime.c reads")
}

/// The text between `open` and the line `shut`, with `open` kept.
fn cut<'a>(src: &'a str, open: &str, shut: &str) -> &'a str {
    let from = src.find(open).unwrap_or_else(|| panic!("src/runtime.c no longer holds `{open}`"));
    let rest = &src[from..];
    let to = rest.find(shut).unwrap_or_else(|| panic!("`{open}` no longer ends with `{shut}`"));
    &rest[..to + shut.len()]
}

#[test]
fn one_struct_still_strips_a_sign_off_its_capacity_and_it_is_not_bytes() {
    // KBuf keeps the sign convention: a negative cap marks a permanent buffer,
    // and k_buf_cap is where that is stripped. KStr keeps its own, where a
    // negative cap is a cached character count. Neither is KBytes, and after
    // the regime bit no KBytes read may look like this.
    let src = runtime();
    let offenders: Vec<(usize, &str)> = src
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains("< 0 ? -"))
        .map(|(i, line)| (i + 1, line.trim()))
        .collect();
    assert_eq!(
        offenders.len(),
        1,
        "a capacity is still read by stripping its sign. KBytes.cap carries the \
         regime in bit 0 now, so the room is `cap & ~1LL`; only k_buf_cap may \
         strip a sign. Found:\n{offenders:#?}"
    );
    let (line, text) = offenders[0];
    assert!(
        text.starts_with("static long long k_buf_cap(const KBuf* b)"),
        "src/runtime.c:{line} strips a sign off a capacity and is not k_buf_cap: {text}"
    );
}

#[test]
fn every_capacity_the_growth_can_produce_is_even_and_reads_back_whole() {
    let src = runtime();

    // The regime test, whole. It is the only place in the tree that asks which
    // allocator a bytes buffer came from.
    let malloced = cut(&src, "static inline int k_bytes_malloced(const KBytes* b) {", "\n}");
    assert!(malloced.contains("b->cap & 1"), "the regime no longer rides in bit 0:\n{malloced}");

    // The capacity's one origin, and the two spellings that mark it.
    let sizing = cut(&src, "    long long cap = 2 * (a->len + n);", "if (cap < 64) cap = 64;");
    // From the growth's first statement rather than its head: the head also
    // appears as a forward declaration, and a cut from there ends at the wrong
    // brace and quietly asserts nothing.
    let grow = cut(&src, "    k_stat_append_grow++;", "\n}");
    assert!(
        grow.contains("        marked = cap | 1;"),
        "the arena regime is no longer bit 0 set in k_b_append_grow"
    );
    assert!(
        grow.contains("        marked = cap;"),
        "the malloc regime no longer leaves bit 0 clear in k_b_append_grow"
    );

    let program = format!(
        r#"
#include <stdio.h>
typedef struct {{ long long len; const unsigned char* data; long long cap; }} KBytes;
{malloced}

static long long grow(const KBytes* a, long long n) {{
{sizing}
    return cap;
}}

int main(void) {{
    long long lens[] = {{0, 1, 2, 3, 7, 8, 31, 32, 33, 63, 64, 65, 1023, 1024,
                         1025, 65535, 65536, 1048575, 1048576, 16777215}};
    long long ns[] = {{0, 1, 2, 3, 4, 7, 8, 15, 16, 24, 31, 32, 64, 4096, 65536}};
    long long bad = 0, checked = 0;
    for (unsigned i = 0; i < sizeof lens / sizeof *lens; i++) {{
        for (unsigned j = 0; j < sizeof ns / sizeof *ns; j++) {{
            KBytes seed = {{lens[i], 0, 0}};
            long long cap = grow(&seed, ns[j]);
            checked++;
            if (cap & 1) {{
                if (!bad) printf("odd capacity at len %lld n %lld: %lld\n", lens[i], ns[j], cap);
                bad++;
                continue;
            }}
            /* the two regimes, as k_b_append_grow spells them */
            KBytes malloc_backed = {{lens[i] + ns[j], 0, cap}};
            KBytes arena_backed = {{lens[i] + ns[j], 0, cap | 1}};
            /* the room reads back whole from either */
            if ((malloc_backed.cap & ~1LL) != cap || (arena_backed.cap & ~1LL) != cap) {{
                if (!bad) printf("room lost at len %lld n %lld\n", lens[i], ns[j]);
                bad++;
                continue;
            }}
            /* and neither regime answers the other's question */
            if (!k_bytes_malloced(&malloc_backed) || k_bytes_malloced(&arena_backed)) {{
                if (!bad) printf("regime confused at len %lld n %lld: %lld\n", lens[i], ns[j], cap);
                bad++;
            }}
        }}
    }}
    /* a borrowed view owns nothing, and always did */
    KBytes view = {{4, 0, 0}};
    if (k_bytes_malloced(&view)) {{ printf("a borrowed view claimed a malloc'd buffer\n"); bad++; }}
    if ((view.cap & ~1LL) != 0) {{ printf("a borrowed view grew room\n"); bad++; }}
    printf("checked %lld, bad %lld\n", checked, bad);
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join("kanso-cap-bit-spec");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the staging directory is made");
    let source = dir.join("cap.c");
    std::fs::write(&source, program).expect("the lifted program writes");
    let binary = dir.join("cap");
    let built =
        Command::new("cc").args(["-O2", "-o"]).arg(&binary).arg(&source).output().expect("cc runs");
    assert!(
        built.status.success(),
        "the lifted capacity program does not compile:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&binary).output().expect("the sweep runs");
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(ran.status.success(), "the capacity sweep found a case:\n{said}");
    assert!(said.contains(", bad 0"), "the sweep did not finish clean:\n{said}");
    let _ = std::fs::remove_dir_all(&dir);
}
