//! `text/replace` is two kernels: `kanso::replace::replaced` for the
//! interpreter, and `k_replace_count` with `k_replace_into` in
//! `src/runtime/text.c` for a built program. This holds both to one
//! reference, written here apart from them. The kernels count the matches
//! with a search that jumps to the next first byte, size the answer, and
//! copy; the reference walks one byte at a time and asks at each whether the
//! text to find starts there.
//!
//! THE C IS LIFTED, NOT COPIED. The search and the two functions are cut out
//! of the runtime's text and fed each case in buffers that end at a page
//! nothing may touch, in exactly the room the runtime gives them, so a read
//! past the end of the text or a write past the answer stops the harness. It
//! needs no sanitizer, which CI's clang cannot link.
//!
//! The cases draw from a few characters so that matches are common: text to
//! find that overlaps itself (`aa` in `aaaa`), that is longer than the text
//! it is sought in, that is the whole text, a replacement that is empty,
//! longer, shorter, or holds the text to find again, and an empty text.

use std::io::Write;
use std::process::Command;

/// xorshift64*, seeded, so a failure names a case that can be replayed.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    fn below(&mut self, n: u64) -> usize {
        (self.next() % n) as usize
    }
}

/// `text` with each occurrence of `from` written as `to`, found left to
/// right, the search taking up again after the end of each one.
fn reference(text: &[u8], from: &[u8], to: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with(from) {
            out.extend_from_slice(to);
            at += from.len();
        } else {
            out.push(text[at]);
            at += 1;
        }
    }
    out
}

/// How many occurrences that walk writes over.
fn reference_count(text: &[u8], from: &[u8]) -> u32 {
    let mut count = 0;
    let mut at = 0;
    while at < text.len() {
        if text[at..].starts_with(from) {
            count += 1;
            at += from.len();
        } else {
            at += 1;
        }
    }
    count
}

/// Few enough characters that a text of any length holds the one sought,
/// with two that take more than a byte.
const POOL: &[char] = &['a', 'a', 'b', '&', '<', '\'', ' ', 'é', '語'];

fn drawn(rng: &mut Rng, longest: u64) -> String {
    let n = rng.below(longest + 1);
    (0..n).map(|_| POOL[rng.below(POOL.len() as u64)]).collect()
}

/// A text, something to find in it that is never empty, and what to write
/// in its place.
fn case(rng: &mut Rng) -> (String, String, String) {
    let text = drawn(rng, 40);
    let mut from = match rng.below(8) {
        0 => text.clone(),
        1 => drawn(rng, 60),
        _ => drawn(rng, 3),
    };
    if from.is_empty() {
        from.push(POOL[rng.below(POOL.len() as u64)]);
    }
    let to = match rng.below(6) {
        0 => String::new(),
        1 => from.clone(),
        2 => format!("{from}{from}"),
        _ => drawn(rng, 6),
    };
    (text, from, to)
}

const CASES: u64 = 4_000_000;

#[test]
fn the_interpreter_s_replace_agrees_with_the_reference() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for n in 0..CASES {
        let (text, from, to) = case(&mut rng);
        let want = reference(text.as_bytes(), from.as_bytes(), to.as_bytes());
        let got = kanso::replace::replaced(&text, &from, &to);
        assert_eq!(
            got.as_deref().unwrap_or(&text).as_bytes(),
            want,
            "case {n}: {text:?} with {from:?} written as {to:?}"
        );
        assert_eq!(
            got.is_none(),
            !text.contains(from.as_str()),
            "case {n}: a text is handed back as it came exactly when it holds no {from:?}: {text:?}"
        );
    }
}

/// The text between `open` and the line `shut`, with both kept.
fn cut<'a>(src: &'a str, open: &str, shut: &str) -> &'a str {
    let from = src.find(open).unwrap_or_else(|| panic!("src/runtime.c no longer holds `{open}`"));
    let rest = &src[from..];
    let to = rest.find(shut).unwrap_or_else(|| panic!("`{open}` no longer ends with `{shut}`"));
    &rest[..to + shut.len()]
}

const NATIVE_CASES: u64 = 1_000_000;

#[test]
fn the_runtime_s_replace_agrees_with_the_reference() {
    let src = kanso::RUNTIME_C;
    let find = cut(src, "static long k_split_find(", "\n    return -1;\n}");
    let count = cut(src, "static long long k_replace_count(", "\n    return count;\n}");
    let into = cut(src, "static long long k_replace_into(", "\n    return o;\n}");
    let harness = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

{find}
{count}
{into}

static void word(unsigned n) {{
    unsigned char w[4] = {{ n & 255, n >> 8 & 255, n >> 16 & 255, n >> 24 & 255 }};
    fwrite(w, 1, 4, stdout);
}}

/* A run of pages with one after it that nothing may touch. A buffer of `n`
   bytes is the last `n` before that page, so a kernel that reads or writes
   one byte past what it was given stops the process. */
#define ROOM (1 << 16)
static char* fenced(void) {{
    long page = sysconf(_SC_PAGESIZE);
    char* at = mmap(0, ROOM + page, PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANON, -1, 0);
    if (at == MAP_FAILED || mprotect(at + ROOM, page, PROT_NONE)) exit(4);
    return at + ROOM;
}}

static unsigned length_read(FILE* in) {{
    unsigned char w[4];
    if (fread(w, 1, 4, in) != 4) exit(3);
    return w[0] | w[1] << 8 | w[2] << 16 | (unsigned)w[3] << 24;
}}

/* Each case is three lengths and that many bytes each: the text, what to
   find, what to write. Each ends at its own fence, and so does the answer,
   in the room the count says it needs. */
int main(int argc, char** argv) {{
    FILE* in = fopen(argv[1], "rb");
    if (argc < 2 || !in) return 2;
    char* text_end = fenced();
    char* from_end = fenced();
    char* to_end = fenced();
    char* out_end = fenced();
    unsigned cases = length_read(in);
    for (unsigned c = 0; c < cases; c++) {{
        unsigned n = length_read(in), fn = length_read(in), tn = length_read(in);
        if (n > 4096 || fn > 4096 || tn > 4096 || fn == 0) return 5;
        char* text = text_end - n;
        char* from = from_end - fn;
        char* to = to_end - tn;
        if (n && fread(text, 1, n, in) != n) return 3;
        if (fread(from, 1, fn, in) != fn) return 3;
        if (tn && fread(to, 1, tn, in) != tn) return 3;
        long long count = k_replace_count(text, n, from, fn);
        long long room = (long long)n + count * ((long long)tn - (long long)fn);
        if (room < 0 || room > ROOM) return 6;
        char* out = out_end - room;
        long long wrote = k_replace_into(text, n, from, fn, to, tn, out);
        if (wrote != room) return 7;
        word((unsigned)count);
        word((unsigned)wrote);
        if (wrote) fwrite(out, 1, (size_t)wrote, stdout);
    }}
    return 0;
}}
"#
    );
    let dir = std::env::temp_dir().join(format!("kanso_replace_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let (c, bin, cases) = (dir.join("replace.c"), dir.join("replace"), dir.join("cases"));
    std::fs::write(&c, harness).expect("the harness writes");
    let built = Command::new("clang")
        .args(["-O1", "-g"])
        .arg(&c)
        .arg("-o")
        .arg(&bin)
        .output()
        .expect("clang runs");
    assert!(
        built.status.success(),
        "the lifted kernels do not compile on their own: {}",
        String::from_utf8_lossy(&built.stderr)
    );

    let mut rng = Rng(0x2026_1111_0e5c_a9ed);
    let mut asked = Vec::new();
    let mut wanted = Vec::new();
    asked.extend((NATIVE_CASES as u32).to_le_bytes());
    for _ in 0..NATIVE_CASES {
        let (text, from, to) = case(&mut rng);
        // the kernels work on bytes, so a case may cut a character in half
        let cutting = rng.below(4) == 0;
        let (text, from, to) = (text.as_bytes(), from.as_bytes(), to.as_bytes());
        let from = match cutting && from.len() > 1 {
            true => &from[..from.len() - 1],
            false => from,
        };
        for part in [text, from, to] {
            asked.extend((part.len() as u32).to_le_bytes());
        }
        for part in [text, from, to] {
            asked.extend(part);
        }
        let want = reference(text, from, to);
        let count = reference_count(text, from);
        wanted.extend(count.to_le_bytes());
        wanted.extend((want.len() as u32).to_le_bytes());
        wanted.extend(want);
    }
    std::fs::File::create(&cases).and_then(|mut f| f.write_all(&asked)).expect("the cases write");
    let run = Command::new(&bin).arg(&cases).output().expect("the harness runs");
    assert!(
        run.status.success(),
        "the harness stopped (status {}): {}",
        run.status,
        String::from_utf8_lossy(&run.stderr)
    );
    let first = run.stdout.iter().zip(&wanted).position(|(got, want)| got != want);
    assert!(
        run.stdout == wanted,
        "the runtime's replace and the reference part at byte {:?} of {} answered against {}",
        first,
        run.stdout.len(),
        wanted.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
