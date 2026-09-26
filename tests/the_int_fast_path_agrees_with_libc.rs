//! `k_b_to_int`'s fast path, swept against strtoll.
//!
//! The function parses `[-]?digits` in a bare loop when the digit run is
//! eighteen or fewer, on the ground that eighteen digits cannot overflow an
//! i64, and hands everything else to strtoll so behaviour stays exactly
//! libc's. That bound is the whole safety argument and nothing checked it.
//!
//! The number it sits next to has already been wrong once: kanso#1423 found
//! `k_b_to_float` treating a truncated significand as certain, on 221 of
//! 1,405,451 cases, in a function whose corpus was 86 values.
//!
//! THE TEXT IS LIFTED, NOT COPIED. The decision is cut out of
//! `src/runtime.c` between the comment that states the bound and the line
//! that falls through, so a change to the bound cannot pass by leaving a
//! stale duplicate behind.
//!
//! WATCHED RED by widening the bound to nineteen, which is the edit someone
//! optimising this would reach for: 50,249 of 23,600,018 fast-path takes
//! disagree with strtoll, the first at "9223372036854775808" — 2^63 exactly,
//! where the loop wraps to the negative and libc saturates.
//!
//! A run of up to eight digits is read as one word, loaded from the eight
//! bytes that end at its last digit, when the buffer holds eight bytes there.
//! The word's three helpers are lifted with the decision, and every string is
//! asked twice: once with junk in front of it, so the word is read and what
//! precedes the number has to be ignored, and once alone, so it is not.

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
fn the_int_fast_path_agrees_with_libc() {
    let src = runtime();
    // from the `long long start = ...` that opens the fast path to the
    // `if (j == len) return ...` that closes it
    let decision = cut(
        &src,
        "    long long start = (len > 0 && data[0] == '-') ? 1 : 0;",
        "        if (j == len) return k_int(start ? -acc : acc);",
    )
    .replace("return k_int(start ? -acc : acc);", "{ *out = start ? -acc : acc; return 1; }")
    .replace("return k_int(start ? -v : v);", "{ *out = start ? -v : v; return 1; }");
    let word = format!(
        "{}\n\n{}\n\n{}",
        cut(&src, "static inline uint64_t k_nondigits8(uint64_t x) {", "\n}"),
        cut(&src, "static inline long long k_digits8_value(uint64_t w, long long n) {", "\n}"),
        cut(
            &src,
            "static inline int k_digits_back8(const char* end, long long n, long long* out) {",
            "\n}"
        ),
    );

    let harness = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <stdint.h>

{word}

/* the shipped decision, lifted: 1 and *out when the fast path takes it,
   0 when it falls through to strtoll. `room` is how many bytes of the buffer
   end at data + len. */
static int fast(const char* data, long long len, long long room, long long* out) {{
{decision}
    }}
    return 0;
}}

static long long seen, took, bad;
static char first[64]; static int have_first;
static void check(const char* s, const char* data, long long len, long long room);

static unsigned long long junk = 0x2545F4914F6CDD1DULL;

/* Asked twice: alone, where the room is the string, and at the end of a
   buffer whose sixteen bytes in front are random, where the word is read. */
static void one(const char* s) {{
    long long len = (long long)strlen(s);
    char buf[96];
    for (int i = 0; i < 16; i++) {{
        junk ^= junk << 13; junk ^= junk >> 7; junk ^= junk << 17;
        buf[i] = (char)junk;
    }}
    memcpy(buf + 16, s, (size_t)len);
    check(s, s, len, len);
    check(s, buf + 16, len, 16 + len);
}}

static void check(const char* s, const char* data, long long len, long long room) {{
    long long got;
    seen++;
    if (!fast(data, len, room, &got)) return;
    took++;
    char* end = NULL; errno = 0;
    long long want = strtoll(s, &end, 10);
    int libc_ok = (errno != ERANGE) && (len != 0) && (end == s + len);
    if (!libc_ok || want != got) {{
        bad++;
        if (!have_first) {{ snprintf(first, sizeof first, "%s", s); have_first = 1; }}
        if (bad <= 8) printf("  \"%s\" fast=%lld strtoll=%lld libc_accepts=%d\n",
                             s, got, want, libc_ok);
    }}
}}

int main(void) {{
    char b[64];
    /* the boundaries by hand: i64's ends, the 18/19 digit step, the shapes
       that must fall through */
    const char* edge[] = {{
        "0","-0","1","-1","9","-9",
        "999999999999999999","-999999999999999999",
        "1000000000000000000","-1000000000000000000",
        "9223372036854775807","-9223372036854775808",
        "9223372036854775808","-9223372036854775809",
        "000000000000000000","-000000000000000000",
        "0000000000000000001","-000000000000000001",
        "","-","+1"," 1","1 ","1a","a1","--1","1-",
        "18446744073709551615","99999999999999999999",
    }};
    for (unsigned i = 0; i < sizeof edge / sizeof *edge; i++) one(edge[i]);

    /* every byte value at every position of a run of one to eight, where the
       word decides: a digit, or a byte either side of the digits, or a sign */
    for (int n = 1; n <= 8; n++) {{
        for (int at = 0; at < n; at++) {{
            for (int c = 1; c < 256; c++) {{
                for (int k = 0; k < n; k++) b[k] = (char)('0' + (k * 7 + n) % 10);
                b[at] = (char)c;
                b[n] = 0;
                one(b);
            }}
        }}
    }}

    /* every digit width from one to twenty, plain, negative and zero-padded */
    unsigned long long x = 0x9E3779B97F4A7C15ULL;
    for (long long i = 0; i < 400000; i++) {{
        x ^= x << 13; x ^= x >> 7; x ^= x << 17;
        for (int d = 1; d <= 20; d++) {{
            unsigned long long m = 1; for (int k = 0; k < d; k++) m *= 10ULL;
            unsigned long long v = m ? (x % m) : x;
            snprintf(b, sizeof b, "%llu", v); one(b);
            snprintf(b, sizeof b, "-%llu", v); one(b);
            snprintf(b, sizeof b, "%0*llu", d, v); one(b);
        }}
    }}
    printf("%lld strings, %lld took the fast path, %lld disagree with strtoll\n",
           seen, took, bad);
    if (have_first) printf("first at \"%s\"\n", first);
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join(format!("kanso_to_int_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let c = dir.join("toint.c");
    let bin = dir.join("toint");
    std::fs::write(&c, harness).expect("the harness writes");

    let built =
        Command::new("clang").arg("-O2").arg(&c).arg("-o").arg(&bin).output().expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted decision does not compile on its own: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let run = Command::new(&bin).output().expect("the harness runs");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "the fast path disagrees with strtoll: {said}");
    assert!(said.contains("took the fast path"), "the harness said nothing: {said}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("{said}");
}
