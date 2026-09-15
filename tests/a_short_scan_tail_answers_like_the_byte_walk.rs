//! The two byte scanners' masked tail, swept against the byte walk.
//!
//! `k_b_find2_raw` and `k_b_find2_below_raw` scan sixteen bytes a step and
//! used to finish a string shorter than a vector one byte at a time. Now a
//! tail shorter than sixteen bytes is one vector load, masked down to the
//! bytes that are the string's, taken only when the sixteen bytes stay inside
//! the page the string ends in (`k_tail_window`). Two things can go wrong
//! and neither is visible from a program that happens to work: the mask can
//! be off by one, so a byte past the end answers, and the window test can be
//! wrong, so a string ending at a page edge reads the next page.
//!
//! THE TEXT IS LIFTED, NOT COPIED. Both scanners and the window test are cut
//! out of `src/runtime.c` and compiled here against a byte-at-a-time
//! reference, over every length from 0 to 40, every start position, four
//! byte pairs and five floors, with the string placed at every offset in the
//! last 64 bytes of a page whose NEXT page is mapped PROT_NONE. A load that
//! crosses the edge faults and the harness dies, which is the second failure
//! made visible.
//!
//! WATCHED RED TWO WAYS: the mask's `- 1` removed (`& (1 << n)` keeps only
//! the bit past the end) disagreed on 259,631 of 836,400 cases; `k_tail_window`
//! answering 1 unconditionally died on the guard page with signal 11.

use std::path::Path;
use std::process::Command;

fn runtime() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("src/runtime.c")).expect("src/runtime.c reads")
}

/// The text between `open` and the line `shut`, with both kept.
fn cut<'a>(src: &'a str, open: &str, shut: &str) -> &'a str {
    let from = src.find(open).unwrap_or_else(|| panic!("src/runtime.c no longer holds `{open}`"));
    let rest = &src[from..];
    let to = rest.find(shut).unwrap_or_else(|| panic!("`{open}` no longer ends with `{shut}`"));
    &rest[..to + shut.len()]
}

#[test]
fn a_short_scan_tail_answers_like_the_byte_walk() {
    let src = runtime();
    let window = cut(&src, "static inline int k_tail_window(const unsigned char* p) {", "\n}");
    let find2 = cut(
        &src,
        "__attribute__((always_inline)) long long k_b_find2_raw(",
        "\n    return len + 1;\n}",
    );
    let below = cut(
        &src,
        "__attribute__((always_inline)) long long k_b_find2_below_raw(",
        "\n    return (len + 1);\n}",
    );

    let harness = format!(
        r#"#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <sys/mman.h>
#include <unistd.h>
#if defined(__aarch64__)
#include <arm_neon.h>
#elif defined(__x86_64__)
#include <tmmintrin.h>
#endif

static long long k_stat_find2_calls = 0;

{window}
{find2}
{below}

/* the byte walk both scanners promise to agree with */
static long long ref_find2(const unsigned char* d, long long len, long long from,
                           long long a, long long b) {{
    long long i = from < 1 ? 0 : from - 1;
    unsigned char ca = (unsigned char)(a & 0xff), cb = (unsigned char)(b & 0xff);
    for (; i < len; i++) if (d[i] == ca || d[i] == cb) return i + 1;
    return len + 1;
}}
static long long ref_below(const unsigned char* d, long long len, long long from,
                           long long a, long long b, long long floor_v) {{
    long long i = from < 1 ? 0 : from - 1;
    unsigned char ca = (unsigned char)(a & 0xff), cb = (unsigned char)(b & 0xff);
    for (; i < len; i++)
        if (d[i] == ca || d[i] == cb || (long long)d[i] < floor_v) return i + 1;
    return len + 1;
}}

int main(void) {{
    /* two pages: the strings end in the first, the second is a guard. The
       page is the host's, not 4096: Apple silicon maps 16 KiB pages, and
       an mprotect at +4096 there is refused as unaligned. */
    long page = sysconf(_SC_PAGESIZE);
    if (page <= 0) {{ printf("sysconf failed\n"); return 2; }}
    unsigned char* base = mmap(NULL, 2 * page, PROT_READ | PROT_WRITE,
                               MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    if (base == MAP_FAILED) {{ printf("mmap failed\n"); return 2; }}
    if (mprotect(base + page, page, PROT_NONE) != 0) {{ printf("mprotect failed\n"); return 2; }}
    /* bytes that exercise every arm: quotes, backslashes, controls, ascii,
       and bytes above 127, which the below-floor compare must treat unsigned */
    static const unsigned char alphabet[] = {{
        'a', 'b', '"', 'c', '\\', 'd', 1, 'e', 31, 32, 255, 128, 127, 'f', 'g', ',', ';', 'h'
    }};
    uint64_t x = 0x9E3779B97F4A7C15ULL;
    for (long i = 0; i < page; i++) {{
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        base[i] = alphabet[x % sizeof alphabet];
    }}
    static const long long pairs[][2] = {{{{34, 92}}, {{44, 59}}, {{0, 255}}, {{0x7f, 0x80}}}};
    static const long long floors[] = {{32, 0, 1, 256, 128}};
    long long cases = 0, bad = 0;
    for (long off = page - 64; off <= page; off++) {{
        for (long long len = 0; len <= 40 && off + len <= page; len++) {{
            const unsigned char* d = base + off;
            for (long long from = 0; from <= len + 1; from++) {{
                for (size_t p = 0; p < 4; p++) {{
                    long long a = pairs[p][0], b = pairs[p][1];
                    long long got = k_b_find2_raw(d, len, from, a, b);
                    long long want = ref_find2(d, len, from, a, b);
                    cases++;
                    if (got != want) {{
                        if (bad < 5) printf("  find2 off=%ld len=%lld from=%lld pair=%lld,%lld: %lld, walk says %lld\n",
                                            off, len, from, a, b, got, want);
                        bad++;
                    }}
                    for (size_t f = 0; f < 5; f++) {{
                        long long got2 = k_b_find2_below_raw(d, len, from, a, b, floors[f]);
                        long long want2 = ref_below(d, len, from, a, b, floors[f]);
                        cases++;
                        if (got2 != want2) {{
                            if (bad < 5) printf("  find2_below off=%ld len=%lld from=%lld pair=%lld,%lld floor=%lld: %lld, walk says %lld\n",
                                                off, len, from, a, b, floors[f], got2, want2);
                            bad++;
                        }}
                    }}
                }}
            }}
        }}
    }}
    printf("%lld cases, %lld disagree\n", cases, bad);
    return bad != 0;
}}
"#
    );
    let dir = std::env::temp_dir().join(format!("kanso_scan_tail_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let c = dir.join("tail.c");
    let bin = dir.join("tail");
    std::fs::write(&c, harness).expect("the harness writes");
    let built = Command::new("clang")
        .arg("-O2")
        .arg("-Wno-ignored-attributes")
        .arg(&c)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted scanners do not compile on their own: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(&bin).output().expect("the harness runs");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success(),
        "a scanner disagrees with the byte walk, or read past a page: {said} (status {})",
        run.status
    );
    assert!(said.contains(" cases, "), "the harness said nothing: {said}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("{said}");
}
