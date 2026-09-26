//! A list or map buffer's capacity reads back whole in either regime, and the
//! emitted fast paths ask for room the way the runtime does.
//!
//! `KBuf.capw` holds the capacity doubled, plus one when the storage came
//! from malloc. The regime used to be the sign, and every inlined push paid a
//! `neg` and a `cmovs` for the magnitude before it could compare: 4,032,318
//! times on the run program. Doubled, "is there room for `need` slots" is
//! `2 * need <= capw` whatever the regime, because the regime bit can never
//! lift an even number past the next even one.
//!
//! Two things have to hold for that to be sound. The helpers in
//! `src/runtime.c` must round-trip a capacity and a regime without confusing
//! either, and the two tests the emitter inlines — the list push's
//! `2 * len + 2 <= capw` and the map insert's `2 * need <= capw` — must answer
//! exactly what `len < cap` and `need <= cap` answer. A push that thinks it has
//! one slot more than it does writes past the buffer, and nothing downstream
//! is guaranteed to notice.
//!
//! THE TEXT IS LIFTED, NOT COPIED. The helpers are cut out of `src/runtime.c`
//! and compiled here, and the emitter's two comparisons are read out of
//! `src/codegen.rs` by the lines that spell them.
//!
//! Watched red: with `k_buf_set_cap` writing `2 * cap - 1` for a malloc'd
//! buffer the sweep reports the first capacity it reads back short, and with
//! the push's `%lneed` written `add i64 %llen2, 1` the scan names the line.

use std::path::Path;
use std::process::Command;

fn read(rel: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(root.join(rel)).unwrap_or_else(|_| panic!("{rel} reads"))
}

/// The text between `open` and the line `shut`, with `open` kept.
fn cut<'a>(src: &'a str, open: &str, shut: &str) -> &'a str {
    let from = src.find(open).unwrap_or_else(|| panic!("the source no longer holds `{open}`"));
    let rest = &src[from..];
    let to = rest.find(shut).unwrap_or_else(|| panic!("`{open}` no longer ends with `{shut}`"));
    &rest[..to + shut.len()]
}

#[test]
fn the_emitted_room_tests_are_the_doubled_ones() {
    let codegen = read("src/codegen.rs");
    for line in [
        "  %llen2 = shl i64 %llen, 1",
        "  %lneed = add i64 %llen2, 2",
        "  %lfits = icmp sle i64 %lneed, %lcap",
        "  %pneed2 = shl i64 %pneed, 1",
        "  %pfits = icmp sle i64 %pneed2, %pcap",
    ] {
        assert_eq!(
            codegen.lines().filter(|l| *l == line).count(),
            1,
            "the inlined room test no longer reads `{line}` exactly once; the \
             sweep below proves only that spelling"
        );
    }
}

#[test]
fn every_capacity_reads_back_whole_and_asks_for_room_exactly() {
    let src = read("src/runtime.c");
    let helpers = cut(
        &src,
        "typedef struct { long long capw; long long used; } KBuf;",
        "\n}",
    );
    assert!(helpers.contains("k_buf_set_cap"), "the cut ended before the setter:\n{helpers}");

    let program = format!(
        r#"
#include <stdio.h>
{helpers}

int main(void) {{
    long long caps[] = {{0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 17, 31, 32, 33, 1023, 1024,
                         1025, 65535, 65536, 1048575, 1048576, 1LL << 40}};
    long long bad = 0, checked = 0;
    for (unsigned i = 0; i < sizeof caps / sizeof *caps; i++) {{
        for (int m = 0; m < 2; m++) {{
            KBuf b;
            k_buf_set_cap(&b, caps[i], m);
            checked++;
            if (k_buf_cap(&b) != caps[i] || k_buf_malloced(&b) != m) {{
                if (!bad) printf("cap %lld regime %d read back as %lld regime %d\n",
                                 caps[i], m, k_buf_cap(&b), k_buf_malloced(&b));
                bad++;
                continue;
            }}
            /* every length and need either side of the edge */
            for (long long d = -2; d <= 2; d++) {{
                long long len = caps[i] + d;
                if (len < 0) continue;
                /* the list push: room for one more */
                if ((2 * len + 2 <= b.capw) != (len < caps[i])) {{
                    if (!bad) printf("push at len %lld cap %lld regime %d\n", len, caps[i], m);
                    bad++;
                }}
                /* the map insert: room for `need` slots in all */
                if ((2 * len <= b.capw) != (len <= caps[i])) {{
                    if (!bad) printf("insert of need %lld cap %lld regime %d\n", len, caps[i], m);
                    bad++;
                }}
            }}
        }}
    }}
    printf("checked %lld, bad %lld\n", checked, bad);
    return bad != 0;
}}
"#
    );

    let dir = std::env::temp_dir().join("kanso-buf-cap-spec");
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
