//! The digit extraction at the end of ryū's core, swept against snprintf.
//!
//! `ryu_d2d` used to write its digits one at a time — a 64-bit division per
//! digit into a scratch buffer, then a reversal — and now writes them two at a
//! time out of a table, straight into place, after computing the length. That
//! is a precision kernel by the project's own definition: it decides how many
//! characters every float in every encoded document gets, and a length that is
//! one short truncates silently rather than crashing.
//!
//! `scripts/render_differential` covers 86 values and `bench/large.json` 2,123,
//! which is where the confidence for this change would otherwise stop. This
//! sweeps every value the extraction can be handed.
//!
//! THE TEXT IS LIFTED, NOT COPIED. The table, the length ladder and the
//! extraction block are cut out of `src/runtime.c` and compiled here, so a
//! change to the runtime that this spec would catch cannot pass by leaving a
//! stale duplicate behind. A copy would go green on code nobody ships.
//!
//! Watched red by moving one boundary of the ladder: with `v < 1000ULL`
//! returning 4 instead of 3, the sweep reports the first disagreement at 100
//! ("0100" against "100") and the count in the thousands.

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
fn every_length_the_extraction_can_be_handed_agrees_with_snprintf() {
    let src = runtime();
    let table = cut(&src, "static const char RYU_DIGITS[201]", "\";");
    let ladder = cut(&src, "static inline int ryu_declen(uint64_t v) {", "\n}");
    let body = cut(&src, "    int n = ryu_declen(output);", "    return n;");

    let harness = format!(
        r#"#include <stdint.h>
#include <stdio.h>
#include <string.h>

{table}
{ladder}

/* The shipped extraction, lifted verbatim. `output` and `dig` are the names it
   uses inside ryu_d2d, so the block compiles unchanged. */
static int extract(uint64_t output, char* dig) {{
{body}
}}

int main(void) {{
    char dig[24], want[24];
    long long bad = 0, seen = 0;
    uint64_t first_bad = 0;

    /* Every value up to eight digits, one at a time: this is where a json
       document's floats actually live, and it covers the 100-entry table's
       every pair in every position. */
    for (uint64_t v = 0; v <= 99999999ULL; v++) {{
        int n = extract(v, dig);
        int w = snprintf(want, sizeof want, "%llu", (unsigned long long)v);
        seen++;
        if (n != w || memcmp(dig, want, (size_t)w) != 0 || dig[n] != 0) {{
            if (!bad) first_bad = v;
            bad++;
        }}
    }}

    /* Every power of ten and its neighbours, to the top of the range a
       uint64 can hold — the ladder's every boundary, from both sides. A
       double's shortest form stops at seventeen digits and the rungs above
       that are unreachable from ryu_d2d, which is exactly why they are swept
       here: the extraction walks DOWN from the length it is given, so a rung
       that is one short writes at a negative index. */
    uint64_t p = 1;
    for (int i = 0; i < 19; i++) {{
        uint64_t around[3] = {{ p - 1, p, p + 1 }};
        for (int j = 0; j < 3; j++) {{
            uint64_t v = around[j];
            int n = extract(v, dig);
            int w = snprintf(want, sizeof want, "%llu", (unsigned long long)v);
            seen++;
            if (n != w || memcmp(dig, want, (size_t)w) != 0 || dig[n] != 0) {{
                if (!bad) first_bad = v;
                bad++;
            }}
        }}
        if (p > 1000000000000000000ULL) break;
        p *= 10;
    }}

    /* And a spread across the whole uint64 range, from a fixed seed so a
       failure is reproducible. */
    uint64_t x = 0x9E3779B97F4A7C15ULL;
    for (long long i = 0; i < 20000000; i++) {{
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        uint64_t v = x;
        int n = extract(v, dig);
        int w = snprintf(want, sizeof want, "%llu", (unsigned long long)v);
        seen++;
        if (n != w || memcmp(dig, want, (size_t)w) != 0 || dig[n] != 0) {{
            if (!bad) first_bad = v;
            bad++;
        }}
    }}

    printf("%lld swept, %lld disagree", seen, bad);
    if (bad) printf(", first at %llu", (unsigned long long)first_bad);
    printf("\n");
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join(format!("kanso_ryu_pairs_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let c = dir.join("pairs.c");
    let bin = dir.join("pairs");
    std::fs::write(&c, harness).expect("the harness writes");

    let built = Command::new("clang")
        .arg("-O2")
        .arg(&c)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted extraction does not compile on its own: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&bin).output().expect("the harness runs");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(
        run.status.success(),
        "the digit extraction disagrees with snprintf: {said}"
    );
    assert!(said.contains(" disagree"), "the harness said nothing: {said}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("{said}");
}
