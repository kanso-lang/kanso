//! The clean flag a shared token gets when its slot fills, swept against the
//! byte walk it replaced.
//!
//! `k_token_miss` decides once whether a token of four to seven bytes holds a
//! quote, a backslash or a byte below a space, and the JSON writer skips its
//! scan for a clean one. The decision is made on one word: the token is
//! copied over eight bytes of 'A' and three bit tests are asked of the whole
//! word. A test off by one in its constant, or padding that one of the tests
//! flags, would mark a dirty token clean and the writer would put a raw quote
//! in its output, or mark a clean one dirty and cost a scan.
//!
//! THE TEXT IS LIFTED, NOT COPIED. The lines from the padded word to the flag
//! are cut out of `src/runtime.c` and compiled against a byte walk, over every
//! length from four to seven, every position, every byte value there with
//! every other byte clean, and a million random tokens.

use std::path::Path;
use std::process::Command;

#[test]
fn a_token_flag_agrees_with_the_byte_walk() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = std::fs::read_to_string(root.join("src/runtime.c")).expect("src/runtime.c reads");
    let open = "uint64_t w = 0x4141414141414141ull;";
    let shut = "unsigned char clean = (hits & highs) == 0;";
    let from = src.find(open).expect("src/runtime.c no longer pads the token's word");
    let to = src[from..].find(shut).expect("the token's flag no longer ends where it did");
    let body = &src[from..from + to + shut.len()];

    let harness = format!(
        r#"#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <stdlib.h>

static unsigned char flag(const char* data, long long len) {{
    {body}
    return clean;
}}

static unsigned char walk(const char* data, long long len) {{
    for (long long i = 0; i < len; i++) {{
        unsigned char c = (unsigned char)data[i];
        if (c == '"' || c == '\\' || c < 32) return 0;
    }}
    return 1;
}}

int main(void) {{
    long long cases = 0, wrong = 0;
    char t[8];
    for (long long len = 4; len <= 7; len++)
        for (long long at = 0; at < len; at++)
            for (int v = 0; v < 256; v++) {{
                memset(t, 'a', sizeof t);
                t[at] = (char)v;
                cases++;
                if (flag(t, len) != walk(t, len)) wrong++;
            }}
    uint64_t x = 88172645463325252ull;
    for (int n = 0; n < 1000000; n++) {{
        long long len = 4 + (long long)(n % 4);
        for (int i = 0; i < 8; i++) {{
            x ^= x << 13; x ^= x >> 7; x ^= x << 17;
            t[i] = (char)(x & 0xff);
        }}
        cases++;
        if (flag(t, len) != walk(t, len)) wrong++;
    }}
    printf("%lld cases, %lld wrong\n", cases, wrong);
    return wrong != 0;
}}
"#
    );
    let dir = std::env::temp_dir().join(format!("kanso_token_flag_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the scratch directory is made");
    let c = dir.join("flag.c");
    let bin = dir.join("flag");
    std::fs::write(&c, harness).expect("the harness writes");
    let built =
        Command::new("clang").arg("-O2").arg(&c).arg("-o").arg(&bin).output().expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted flag does not compile on its own: {}",
        String::from_utf8_lossy(&built.stderr)
    );
    let run = Command::new(&bin).output().expect("the harness runs");
    let said = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "the token flag disagrees with the byte walk: {said}");
    assert!(said.contains(" cases, 0 wrong"), "the harness said nothing: {said}");
    let _ = std::fs::remove_dir_all(&dir);
    println!("{said}");
}
