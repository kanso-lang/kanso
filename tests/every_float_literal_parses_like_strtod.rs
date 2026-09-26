//! `k_b_to_float`'s fast path, swept against strtod.
//!
//! The function scans a plain decimal into a `(w, q)` pair and hands it to
//! eisel-lemire, keeping strtod behind as the semantic authority for
//! anything the scan cannot be certain about. Correctness rests entirely on
//! the fast path deferring in exactly the cases where it would be wrong,
//! and until kanso#1423 nothing in the corpus watched that.
//!
//! IT HAS BEEN WRONG. kanso#1423 found the scan calling a TRUNCATED
//! significand certain: past nineteen significant digits an integer digit
//! is traded for a `q++` and a fraction digit is dropped, so the pair no
//! longer describes the number that was written, and a truncated
//! significand can sit on the far side of a rounding boundary from the
//! true value. `text/to_float "4409065699.4409065699e-2"` came back one ULP
//! below the correctly-rounded double while the interpreter — the oracle —
//! returned the right one, which the differential law forbids outright.
//! 221 of 1,405,451 cases were wrong.
//!
//! The harness that found those 221 was built, used once and thrown away,
//! and the fix shipped with a six-line micro fixture. That is the gap this
//! file closes: a bug with no home in the corpus is a hole in the corpus,
//! and CLAUDE.md asks for the fuzzer to be the thing that ships, not the
//! anecdote it produced. Float rendering has had one since kanso#1424 and
//! integer parsing since the same PR; parsing a float, the kernel with the
//! actual defect, had none.
//!
//! THE TEXT IS LIFTED, NOT COPIED, in two spans: the pow5 tables plus
//! `k_el_parse`, and the scan out of `k_b_to_float` itself. A copy would go
//! green on code nobody ships, which is the failure this whole family of
//! specs exists to avoid.
//!
//! WATCHED RED by dropping `!cut` from the fast path's condition — the
//! exact edit kanso#1423 made in reverse, and the mutation
//! `a_truncated_significand_taken_as_certain.sh` already applies.
//!
//! A THIRD SPAN, the word shape. `[-]I.F` with up to eight digits of I and
//! seven of F is read as two words that end at the dot and at the last byte,
//! when the buffer holds sixteen bytes there. It is lifted with its helpers
//! and asked about every string twice, at the end of a buffer whose sixteen
//! bytes in front are random, so what precedes the number has to be ignored.

use std::path::Path;
use std::process::Command;

fn runtime() -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join("src/runtime.c")).expect("src/runtime.c reads")
}

fn cut<'a>(src: &'a str, open: &str, shut: &str) -> &'a str {
    let from = src.find(open).unwrap_or_else(|| panic!("src/runtime.c no longer holds `{open}`"));
    let rest = &src[from..];
    let to = rest.find(shut).unwrap_or_else(|| panic!("`{open}` no longer ends with `{shut}`"));
    &rest[..to + shut.len()]
}

#[test]
fn every_float_literal_parses_like_strtod() {
    let src = runtime();

    // the pow5 tables, the two bounds and k_el_parse, in the order the
    // runtime declares them
    let el = cut(
        &src,
        "static const struct { unsigned long long hi, lo; int e2; } k_el_pow10[] = {",
        "    __builtin_memcpy(&d, &bits, 8);\n    *out = d;\n    return 1;\n}",
    )
    .replace("k_stat_el_parses++;", "");

    // The word shape and the three helpers it reads with. The helpers are
    // declared before the pow5 tables in the runtime and the shape after
    // k_el_parse, so they are lifted in that order.
    let words = format!(
        "{}\n\n{}",
        cut(&src, "static inline uint64_t k_nondigits8(uint64_t x) {", "\n}"),
        cut(&src, "static inline long long k_digits8_value(uint64_t w, long long n) {", "\n}"),
    );
    let shape = cut(
        &src,
        "static const unsigned long long k_pow10_small[8] = {",
        "    *out = s ? -d : d;\n    return 1;\n}",
    );

    // The scan: from the `if (len > 0)` that opens it to the strtod
    // fallthrough that follows the block, which is then trimmed back off.
    //
    // THE CLOSING ANCHOR DELIBERATELY DOES NOT NAME `!cut`. It did at
    // first, and that made the spec useless in exactly the case it exists
    // for: the mutation drops `!cut` from that line, so the anchor stopped
    // matching and the spec died with "no longer ends with" instead of
    // reporting a single disagreement. It went red, which is why the
    // mistake was survivable, but it proved the anchor rather than the
    // parser. An anchor may not mention the thing under test.
    let scan =
        cut(&src, "    if (len > 0) {\n        const char* p = data;", "    char* end = NULL;")
            .strip_suffix("    char* end = NULL;")
            .expect("the cut ends where it was told to")
            .replace("return k_float(neg ? -out : out);", "{ *res = neg ? -out : out; return 1; }");

    let harness = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>
/* the lifted text guards its counters; a harness counts nothing */
#define K_COUNTING 0

{words}

{el}

{shape}

/* the shipped scan, lifted: 1 and *res when the fast path is certain,
   0 when it defers to strtod */
static int fast(const char* data, long long len, double* res) {{
{scan}
    return 0;
}}

static unsigned long long bits_of(double d) {{
    unsigned long long u;
    memcpy(&u, &d, 8);
    return u;
}}

static long long seen = 0, took = 0, bad = 0, shaped = 0;
static unsigned long long junk = 0x2545F4914F6CDD1DULL;

static void one(const char* s) {{
    seen++;
    long long len = (long long)strlen(s);
    double want = strtod(s, NULL);
    double got;
    if (fast(s, len, &got)) {{
        took++;
        if (bits_of(got) != bits_of(want)) {{
            if (bad < 8) fprintf(stderr, "  %s -> %.17g want %.17g\n", s, got, want);
            bad++;
        }}
    }}
    /* the word shape, at the end of a buffer with sixteen random bytes in
       front: when it answers, it answers what strtod does for the whole
       string, which rules out a shape taken by a string it does not fit */
    char buf[600];
    if (len > 500) return;
    for (int i = 0; i < 16; i++) {{
        junk ^= junk << 13; junk ^= junk >> 7; junk ^= junk << 17;
        buf[i] = (char)junk;
    }}
    memcpy(buf + 16, s, (size_t)len);
    char* end = NULL;
    strtod(s, &end);
    if (k_float_shape8(buf + 16, len, 16 + len, &got)) {{
        shaped++;
        if (end != s + len || bits_of(got) != bits_of(want)) {{
            if (bad < 8) fprintf(stderr, "  shape %s -> %.17g want %.17g\n", s, got, want);
            bad++;
        }}
    }}
}}

int main(void) {{
    char buf[512];

    /* 1. the shape that was wrong: significands past nineteen digits, with
       the twentieth and beyond nonzero, at every exponent that matters */
    for (int extra = 1; extra <= 12; extra++) {{
        for (unsigned long long seed = 1; seed < 40000; seed += 7) {{
            int n = snprintf(buf, sizeof buf, "%llu", seed * 1000000000000000ULL + 1);
            for (int k = 0; k < extra && n < 400; k++) buf[n++] = (char)('1' + (k % 9));
            buf[n] = 0;
            one(buf);
            int base = n;
            for (int e = -30; e <= 30; e += 10) {{
                snprintf(buf + base, sizeof buf - base, "e%d", e);
                one(buf);
            }}
            buf[base] = 0;
        }}
    }}

    /* 2. the same, with the point inside the run — the fraction branch
       drops its overlong digits rather than trading them for a q++ */
    for (int lead = 1; lead <= 18; lead++) {{
        for (unsigned long long seed = 1; seed < 6000; seed += 3) {{
            int n = 0;
            for (int k = 0; k < lead; k++) buf[n++] = (char)('1' + ((seed + k) % 9));
            buf[n++] = '.';
            n += snprintf(buf + n, sizeof buf - n, "%llu", seed * 7919ULL + 13);
            for (int k = 0; k < 6; k++) buf[n++] = (char)('0' + ((seed + k) % 10));
            buf[n] = 0;
            one(buf);
        }}
    }}

    /* 3. trailing zeros past nineteen digits, which are NOT a truncation
       and must stay on the fast path — the other side of the flag */
    for (int zeros = 1; zeros <= 40; zeros++) {{
        for (unsigned long long seed = 1; seed < 400; seed++) {{
            int n = snprintf(buf, sizeof buf, "%llu", seed);
            for (int k = 0; k < zeros && n < 400; k++) buf[n++] = '0';
            buf[n] = 0;
            one(buf);
        }}
    }}

    /* 4. the exponent extremes, where eisel-lemire's own table runs out and
       where doubles go subnormal and overflow */
    for (int e = -400; e <= 400; e++) {{
        for (unsigned long long seed = 1; seed < 200; seed += 3) {{
            snprintf(buf, sizeof buf, "%llu.%llue%d", seed, seed * 31ULL, e);
            one(buf);
            snprintf(buf, sizeof buf, "%llue%d", seed, e);
            one(buf);
        }}
    }}

    /* 5. round-to-even boundaries: values whose binary expansion lands
       exactly halfway, which is where a one-ULP error shows up first */
    for (unsigned long long m = (1ULL << 52); m < (1ULL << 52) + 60000; m += 7) {{
        for (int e2 = -12; e2 <= 12; e2 += 6) {{
            double d = ldexp((double)m, e2);
            snprintf(buf, sizeof buf, "%.17g", d);
            one(buf);
            snprintf(buf, sizeof buf, "%.20g", d);
            one(buf);
        }}
    }}

    /* 6. the plain short decimals the benchmarks actually parse, so a
       regression that only hits the common path cannot hide either */
    for (unsigned long long seed = 0; seed < 200000; seed++) {{
        snprintf(buf, sizeof buf, "%llu.%llu", seed, seed % 9973ULL);
        one(buf);
        snprintf(buf, sizeof buf, "-%llu.%03llu", seed % 1000ULL, seed % 1000ULL);
        one(buf);
    }}

    /* 7. the word shape's own edges: every byte value at every position of
       a short decimal, so a sign, a second dot, a letter or a space where a
       digit should be is refused rather than read */
    for (int ilen = 1; ilen <= 9; ilen++) {{
        for (int flen = 0; flen <= 8; flen++) {{
            int n = ilen + 1 + flen;
            for (int at = 0; at < n; at++) {{
                for (int c = 1; c < 256; c++) {{
                    for (int k = 0; k < ilen; k++) buf[k] = (char)('1' + (k * 3 + flen) % 9);
                    buf[ilen] = '.';
                    for (int k = 0; k < flen; k++) buf[ilen + 1 + k] = (char)('0' + (k * 7 + ilen) % 10);
                    buf[at] = (char)c;
                    buf[n] = 0;
                    one(buf);

                }}
            }}
            for (int k = 0; k < ilen; k++) buf[k + 1] = (char)('1' + (k * 3 + flen) % 9);
            buf[0] = '-';
            buf[ilen + 1] = '.';
            for (int k = 0; k < flen; k++) buf[ilen + 2 + k] = (char)('0' + (k * 7 + ilen) % 10);
            buf[n + 1] = 0;
            one(buf);
        }}
    }}

    printf("%lld parsed, %lld took the fast path, %lld took the word shape, %lld disagree with strtod\n",
           seen, took, shaped, bad);
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join(format!("kanso-float-parse-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let c = dir.join("h.c");
    let bin = dir.join("h");
    std::fs::write(&c, harness).expect("harness writes");

    let built = Command::new("clang")
        .args(["-O2", "-w", "-o"])
        .arg(&bin)
        .arg(&c)
        .arg("-lm")
        .output()
        .expect("clang runs");
    assert!(
        built.status.success(),
        "harness did not build:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&bin).output().expect("harness runs");
    let out = String::from_utf8_lossy(&run.stdout);
    let err = String::from_utf8_lossy(&run.stderr);
    assert!(run.status.success(), "the fast path disagrees with strtod:\n{err}{out}");
    print!("{out}");

    let _ = std::fs::remove_dir_all(&dir);
}
