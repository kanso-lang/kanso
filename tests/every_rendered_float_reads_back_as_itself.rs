//! The whole shipped float rendering, swept against strtod.
//!
//! `render_ryu` decides the text of every float a kanso program prints or
//! encodes. What it promises is two things at once: the text reads back as
//! the same 64 bits, and no shorter decimal does. Neither promise had a
//! harness. `scripts/render_differential` checks the interpreter and the C
//! runtime say the same thing, which is agreement rather than correctness —
//! two implementations wrong the same way pass it. The digit-extraction
//! sweep in `the_shortest_digits_come_out_in_pairs` checks the block that
//! writes digits into a buffer, not the decision of which digits.
//!
//! kanso#1423 found the other direction of this pair wrong: `k_b_to_float`
//! took eisel-lemire's answer as certain on a truncated significand and
//! diverged from the interpreter by one ULP, on 221 of 1,405,451 cases. That
//! bug lived in a function whose corpus was 86 values. This is the render
//! side of the same gap.
//!
//! THE TEXT IS LIFTED, NOT COPIED. `ryu_d2d`, `render_ryu`, their tables and
//! the two helpers they call are cut out of `src/runtime.c` and compiled
//! here, so a change to the runtime this spec would catch cannot pass by
//! leaving a stale duplicate behind.
//!
//! WATCHED RED THREE WAYS, one per property, each leaving the other two
//! clean:
//!
//!   `output = vr + (...)` -> `output = vr`   815,943 do not read back
//!   the removal loop breaks after one step   581,913 are not shortest
//!   `return (o - buf)` -> `+ 1`              2,809,321 lengths disagree
//!
//! And a fourth time on 2026-09-25, when the short search began starting from
//! the place count the last float took: with the walk down from a passing
//! guess removed, 975,871 of 5,809,326 were not shortest.

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
fn every_rendered_float_reads_back_as_itself_and_no_shorter_one_does() {
    let src = runtime();
    let digits = cut(&src, "static const char RYU_DIGITS[201]", "\";");
    let declen = cut(&src, "static inline int ryu_declen(uint64_t v) {", "\n}");
    // the short copy and the cold long copy it hands the rest to, one span
    let copy = cut(
        &src,
        "static __attribute__((noinline, cold, preserve_most)) void k_copy_cold(void* d, const void* s, size_t n) {",
        "\n    } else if (n == 1) {\n        d[0] = s[0];\n    }\n}",
    );
    // one contiguous span: the pow5 tables, the small helpers, ryu_d2d and
    // render_ryu, in the order the runtime declares them
    let core =
        cut(&src, "/* ryu d2s tables (adams, PLDI 2018)", "\n    return (long long)(o - buf);\n}");

    let harness = format!(
        r#"#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>
#include <math.h>

/* the runtime's counting switch and the one counter the core bumps */
#define K_COUNTING 0
static long long k_stat_ryu_short;

{digits}
{declen}
{copy}
{core}

/* what k_render_number writes: the sign, then render_ryu of the magnitude */
static long long render(double d, char* buf) {{
    if (d < 0) {{ buf[0] = '-'; return 1 + render_ryu(-d, buf + 1); }}
    return render_ryu(d, buf);
}}
static uint64_t bits_of(double d) {{ uint64_t b; memcpy(&b, &d, 8); return b; }}

static long long seen, bad_trip, bad_len, bad_short, bad_near;
static double first_bad; static int have_first;
static const char* first_why = "";

static void one(double d) {{
    char buf[64], shorter[64];
    if (!isfinite(d)) return;
    long long n = render(d, buf);
    seen++;
    if ((long long)strlen(buf) != n) {{
        bad_len++;
        if (!have_first) {{ first_bad = d; have_first = 1; first_why = "length"; }}
        return;
    }}
    double back = strtod(buf, NULL);
    if (bits_of(back) != bits_of(d)) {{
        bad_trip++;
        if (!have_first) {{ first_bad = d; have_first = 1; first_why = "round trip"; }}
        if (bad_trip <= 5) printf("  trip %.17g -> \"%s\" -> %.17g\n", d, buf, back);
        return;
    }}
    /* Shortest: one fewer significant digit must NOT read back. `k` comes
       from the digit core itself rather than from counting the text — the
       plain form pads with zeros to reach the point and those are not digits
       ryu chose. Counting them called "100" two digits and reported 2,863
       shortest failures that were the counter's, not the renderer's. */
    char dig[24]; int e10;
    int k = ryu_d2d(d < 0 ? -d : d, dig, &e10);
    /* Closest: of the k-digit decimals that read back as d, the one nearest
       it. glibc's `%.*e` is exact, so its digits are the nearest k-digit
       decimal, rounded half-even; when that one reads back, it is the one to
       choose. It need not read back. Below a power of two the doubles are
       twice as dense, so the interval that reads back as d is half as wide on
       that side, and the nearest decimal can sit outside it while a farther
       one above sits inside: 2^-1017 prints as 7.120236347223045e-307 though
       ...044 is nearer. Round trip and shortest both pass a renderer that
       picks a neighbour of the right length. */
    if (d != 0) {{
        char near[40], got[24]; int g = 0;
        snprintf(near, sizeof near, "%.*e", k - 1, fabs(d));
        for (const char* c = near; *c && *c != 'e'; c++) if (*c != '.') got[g++] = *c;
        got[g] = 0;
        if (bits_of(strtod(near, NULL)) == bits_of(fabs(d))
            && (g != k || memcmp(got, dig, (size_t)k) != 0)) {{
            bad_near++;
            if (!have_first) {{ first_bad = d; have_first = 1; first_why = "closest"; }}
            if (bad_near <= 5)
                printf("  near %.17g -> \"%s\" but the nearest %d digits are \"%s\"\n",
                       d, dig, k, got);
        }}
    }}
    if (k > 1) {{
        snprintf(shorter, sizeof shorter, "%.*e", k - 2, d);
        if (bits_of(strtod(shorter, NULL)) == bits_of(d)) {{
            bad_short++;
            if (!have_first) {{ first_bad = d; have_first = 1; first_why = "shortest"; }}
            if (bad_short <= 5)
                printf("  short %.17g -> \"%s\" (%d digits) but \"%s\" reads back\n",
                       d, buf, k, shorter);
        }}
    }}
}}

int main(void) {{
    /* 1. random bit patterns: every exponent, every mantissa */
    uint64_t x = 0x243F6A8885A308D3ULL;
    for (long long i = 0; i < 2000000; i++) {{
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        double d; memcpy(&d, &x, 8); one(d);
    }}
    /* 2. the ordinary range a json document holds. Random doubles almost
       never land here — their exponents are uniform over the whole field —
       so the values every real program prints need naming. */
    for (long long m = 0; m < 200000; m++) {{
        one((double)m);
        one((double)m / 10.0);
        one((double)m / 1000.0);
        one(-(double)m / 100.0);
    }}
    /* 2b. short decimals of every length a double holds, at every scale the
       renderer's short path searches, and the doubles either side of each */
    for (long long i = 0; i < 1000000; i++) {{
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        static const double p10[23] = {{1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8,
            1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20,
            1e21, 1e22}};
        int len = 1 + (int)(x % 17);
        double m = (double)((x >> 8) % (uint64_t)p10[len]);
        double v = m / p10[(x >> 40) % 23];
        one(v); one(nextafter(v, 0.0)); one(nextafter(v, INFINITY));
    }}
    /* 3. both sides of every binary exponent, subnormals included */
    for (int e = -1074; e <= 1023; e++) {{
        double p = ldexp(1.0, e);
        one(p); one(nextafter(p, 0.0)); one(nextafter(p, INFINITY)); one(-p);
    }}
    /* 4. the ends of the range, and both sides of every power of ten */
    one(5e-324); one(nextafter(5e-324, INFINITY));
    one(1.7976931348623157e308); one(2.2250738585072014e-308);
    one(nextafter(2.2250738585072014e-308, 0.0));
    for (int e = -323; e <= 308; e++) {{
        char b[32]; snprintf(b, sizeof b, "1e%d", e);
        double p = strtod(b, NULL);
        one(p); one(nextafter(p, 0.0)); one(nextafter(p, INFINITY));
    }}

    printf("%lld rendered, %lld do not read back, %lld length disagrees, "
           "%lld not shortest, %lld not closest\n", seen, bad_trip, bad_len, bad_short,
           bad_near);
    if (have_first) printf("first %s at %.17g (bits %016llx)\n",
                           first_why, first_bad, (unsigned long long)bits_of(first_bad));
    return (bad_trip || bad_len || bad_short || bad_near) != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join(format!("kanso_render_trip_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let c = dir.join("trip.c");
    let bin = dir.join("trip");
    std::fs::write(&c, harness).expect("the harness writes");

    let built = Command::new("clang")
        .arg("-O2")
        .arg(&c)
        .arg("-o")
        .arg(&bin)
        .arg("-lm")
        .output()
        .expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted rendering does not compile on its own: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&bin).output().expect("the harness runs");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "the rendering disagrees with strtod: {said}");
    assert!(said.contains(" rendered, "), "the harness said nothing: {said}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("{said}");
}
