//! A span `from..to` is inside a container exactly when the two unsigned
//! compares say so.
//!
//! `1 <= from <= to <= len` was three signed compares, and the runtime and the
//! emitter both ask it at the head of every slice the decoder takes. As
//! `from - 1 <u to` and `to <=u len` it is two, and it is the same test only
//! because a length is never negative: a `from` below one wraps past every
//! `to`, and a negative `to` wraps past every length. A slice that thinks a
//! bad span is good reads outside its container.
//!
//! THE TEXT IS LIFTED, NOT COPIED. `k_span_in` is cut out of `src/runtime.c`
//! and swept here against the signed definition, and the emitter's two
//! compares are read out of `src/codegen.rs` by the lines that spell them.
//!
//! Watched red: with `k_span_in` testing `from < to` instead of
//! `from - 1 < to`, the sweep names the first span it gets wrong.

use std::path::Path;
use std::process::Command;

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("{rel} reads"))
}

#[test]
fn the_emitted_span_test_is_the_unsigned_one() {
    let codegen = read("src/codegen.rs");
    for line in [
        "  %qoff = add i64 %qfrom, -1",
        "  %qorder = icmp ult i64 %qoff, %qto",
        "  %qhi = icmp ule i64 %qto, %qclen",
    ] {
        assert_eq!(
            codegen.lines().filter(|l| *l == line).count(),
            1,
            "the emitted span test no longer reads `{line}` exactly once; the sweep \
             below proves only that spelling"
        );
    }
}

#[test]
fn every_span_is_judged_the_way_the_signed_test_judges_it() {
    let src = read("src/runtime.c");
    let open = "static inline __attribute__((always_inline)) int k_span_in(";
    let from = src.find(open).expect("src/runtime.c no longer defines k_span_in");
    let rest = &src[from..];
    let helper = &rest[..rest.find("\n}\n").expect("k_span_in ends") + 3];

    let program = format!(
        r#"
#include <stdio.h>
#include <limits.h>
{helper}

int main(void) {{
    long long edge[] = {{LLONG_MIN, LLONG_MIN + 1, -3, -2, -1, 0, 1, 2, 3, 4, 5, 7, 8,
                         1000, LLONG_MAX - 1, LLONG_MAX}};
    long long lens[] = {{0, 1, 2, 3, 4, 7, 1000, LLONG_MAX}};
    int n = sizeof edge / sizeof *edge, m = sizeof lens / sizeof *lens;
    long long bad = 0, checked = 0;
    for (int k = 0; k < m; k++)
        for (int i = 0; i < n; i++)
            for (int j = 0; j < n; j++) {{
                long long f = edge[i], t = edge[j], len = lens[k];
                int want = f >= 1 && f <= t && t <= len;
                checked++;
                if (k_span_in(f, t, len) != want) {{
                    if (!bad) printf("from %lld to %lld len %lld: want %d\n", f, t, len, want);
                    bad++;
                }}
            }}
    printf("checked %lld, bad %lld\n", checked, bad);
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join("kanso-span-spec");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("the staging directory is made");
    let source = dir.join("span.c");
    std::fs::write(&source, program).expect("the lifted program writes");
    let binary = dir.join("span");
    let built =
        Command::new("cc").args(["-O2", "-o"]).arg(&binary).arg(&source).output().expect("cc runs");
    assert!(
        built.status.success(),
        "the lifted span program does not compile:\n{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let ran = Command::new(&binary).output().expect("the sweep runs");
    let said = String::from_utf8_lossy(&ran.stdout);
    assert!(
        ran.status.success(),
        "the span sweep found a case:\n{said}{}",
        String::from_utf8_lossy(&ran.stderr)
    );
    assert!(said.contains(", bad 0"), "the sweep did not finish clean:\n{said}");
    let _ = std::fs::remove_dir_all(&dir);
}
