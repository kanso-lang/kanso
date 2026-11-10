//! `std/base64` is two kernels twice over: `kanso::base64` for the
//! interpreter and `k_base64_spell` and `k_base64_read` in
//! `src/runtime/text.c` for a built program. This holds all four to one
//! reference, written here apart from them: it spells by laying the bytes
//! out as bits and taking six at a time, and reads by looking each character
//! up in the alphabet and taking the bits back eight at a time.
//!
//! THE C IS LIFTED, NOT COPIED. The alphabet and the three functions are cut
//! out of the runtime's text and fed each case in a buffer that ends at a
//! page nothing may touch, in exactly the room the runtime promises, so a
//! read past the end of a ragged text or a write past the room a caller
//! gives stops the harness. It needs no sanitizer, which CI's clang cannot
//! link.
//!
//! Half the cases are bytes to spell, of every length from 0 to 67. The rest
//! are texts to read: a true spelling; a spelling with one or two characters
//! swapped for a letter, an `=`, or a character either side of each run of
//! the alphabet; a spelling cut short or run long by one to three characters;
//! and characters drawn from that pool at any length, ragged ones included.

use std::io::Write;
use std::process::Command;

const ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

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

fn reference_spell(bytes: &[u8]) -> String {
    let mut bits: Vec<u8> =
        bytes.iter().flat_map(|b| (0..8).rev().map(move |k| b >> k & 1)).collect();
    while !bits.len().is_multiple_of(6) {
        bits.push(0);
    }
    let letters = ALPHABET.as_bytes();
    let mut out: String = bits
        .chunks(6)
        .map(|six| letters[six.iter().fold(0usize, |n, bit| n << 1 | *bit as usize)] as char)
        .collect();
    while !out.len().is_multiple_of(4) {
        out.push('=');
    }
    out
}

/// A letter's place in the alphabet, or nothing for a character outside it.
fn six(c: u8) -> Option<u8> {
    ALPHABET.bytes().position(|letter| letter == c).map(|at| at as u8)
}

/// A third or fourth character: its place, nothing for the `=` that pads, or
/// a refusal of the whole text for anything else.
fn padded(c: u8) -> Option<Option<u8>> {
    if c == b'=' {
        return Some(None);
    }
    six(c).map(Some)
}

fn reference_read(text: &[u8]) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::new();
    for quad in text.chunks(4) {
        let mut letters = vec![six(quad[0])?, six(quad[1])?];
        let (third, fourth) = (padded(quad[2])?, padded(quad[3])?);
        if let Some(c) = third {
            letters.push(c);
            if let Some(d) = fourth {
                letters.push(d);
            }
        }
        let bits: Vec<u8> =
            letters.iter().flat_map(|l| (0..6).rev().map(move |k| l >> k & 1)).collect();
        for eight in bits.as_chunks::<8>().0 {
            out.push(eight.iter().fold(0u8, |n, bit| n << 1 | bit));
        }
    }
    Some(out)
}

/// Letters, the pad, and the characters either side of each run of the
/// alphabet, with a few that are nowhere near it.
const POOL: &[u8] = b"AZaz09+/==@[`{:.,*-_ \n\x00\x7f\x80\xff";

fn bytes_of(rng: &mut Rng) -> Vec<u8> {
    let n = rng.below(68);
    (0..n).map(|_| rng.next() as u8).collect()
}

fn text_of(rng: &mut Rng) -> Vec<u8> {
    let mut text = reference_spell(&bytes_of(rng)).into_bytes();
    match rng.below(20) {
        0..=7 => {}
        8..=13 => {
            for _ in 0..1 + rng.below(2) {
                if !text.is_empty() {
                    let at = rng.below(text.len() as u64);
                    text[at] = POOL[rng.below(POOL.len() as u64)];
                }
            }
        }
        14..=16 => {
            let by = 1 + rng.below(3);
            if rng.below(2) == 0 {
                text.truncate(text.len().saturating_sub(by));
            } else {
                text.extend((0..by).map(|_| POOL[rng.below(POOL.len() as u64)]));
            }
        }
        _ => {
            let n = rng.below(25);
            text = (0..n).map(|_| POOL[rng.below(POOL.len() as u64)]).collect();
        }
    }
    text
}

const CASES: u64 = 2_000_000;

#[test]
fn the_interpreter_s_base64_agrees_with_the_reference() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for case in 0..CASES {
        let bytes = bytes_of(&mut rng);
        assert_eq!(
            kanso::base64::spell(&bytes),
            reference_spell(&bytes),
            "case {case}: spelling {bytes:?}"
        );
        let text = text_of(&mut rng);
        assert_eq!(
            kanso::base64::read(&text),
            reference_read(&text),
            "case {case}: reading {:?}",
            String::from_utf8_lossy(&text)
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

const NATIVE_CASES: u64 = 400_000;

#[test]
fn the_runtime_s_base64_agrees_with_the_reference() {
    let src = kanso::RUNTIME_C;
    let letters = cut(src, "static const char k_base64_letters[64] =", ";");
    let spell = cut(src, "static long long k_base64_spell(", "\n    return o;\n}");
    let place = cut(src, "static unsigned k_base64_place(", "\n    return 255;\n}");
    let read = cut(src, "static long long k_base64_read(", "\n    return o;\n}");
    let harness = format!(
        r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

{letters}
{spell}
{place}
{read}

static void word(unsigned n) {{
    unsigned char w[4] = {{ n & 255, n >> 8 & 255, n >> 16 & 255, n >> 24 & 255 }};
    fwrite(w, 1, 4, stdout);
}}

/* A run of pages with one after it that nothing may touch. A buffer of `n`
   bytes is the last `n` before that page, so a kernel that reads or writes
   one byte past what it was promised stops the process. */
#define ROOM (1 << 16)
static unsigned char* fenced(void) {{
    long page = sysconf(_SC_PAGESIZE);
    unsigned char* at = mmap(0, ROOM + page, PROT_READ | PROT_WRITE,
                             MAP_PRIVATE | MAP_ANON, -1, 0);
    if (at == MAP_FAILED || mprotect(at + ROOM, page, PROT_NONE)) exit(4);
    return at + ROOM;
}}

/* Each case is a kind, a length and that many bytes. The input and the
   output each end at a fence, in exactly the room the runtime promises the
   kernel: four characters for every three bytes or part of three, and three
   bytes for every four characters. */
int main(int argc, char** argv) {{
    FILE* in = fopen(argv[1], "rb");
    if (argc < 2 || !in) return 2;
    unsigned char* in_end = fenced();
    unsigned char* out_end = fenced();
    unsigned char head[5];
    while (fread(head, 1, 5, in) == 5) {{
        unsigned len = head[1] | head[2] << 8 | head[3] << 16 | (unsigned)head[4] << 24;
        if (len > ROOM / 2) return 5;
        unsigned char* buf = in_end - len;
        if (len && fread(buf, 1, len, in) != len) return 3;
        long long n;
        unsigned char* out;
        if (head[0] == 'E') {{
            out = out_end - (len + 2) / 3 * 4;
            n = k_base64_spell(buf, len, (char*)out);
        }} else {{
            out = out_end - len / 4 * 3;
            n = k_base64_read(buf, len, out);
        }}
        word(n < 0 ? 0xffffffffu : (unsigned)n);
        if (n > 0) fwrite(out, 1, (size_t)n, stdout);
    }}
    return 0;
}}
"#
    );
    let dir = std::env::temp_dir().join(format!("kanso_base64_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("the harness directory is made");
    let (c, bin, cases) = (dir.join("base64.c"), dir.join("base64"), dir.join("cases"));
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

    let mut rng = Rng(0x2026_1110_0b64_0b64);
    let mut asked = Vec::new();
    let mut wanted = Vec::new();
    let frame = |to: &mut Vec<u8>, kind: Option<u8>, body: Option<&[u8]>| {
        to.extend(kind);
        let n = body.map_or(u32::MAX, |b| b.len() as u32);
        to.extend(n.to_le_bytes());
        to.extend(body.unwrap_or(&[]));
    };
    for _ in 0..NATIVE_CASES {
        let bytes = bytes_of(&mut rng);
        frame(&mut asked, Some(b'E'), Some(&bytes));
        frame(&mut wanted, None, Some(reference_spell(&bytes).as_bytes()));
        let text = text_of(&mut rng);
        frame(&mut asked, Some(b'D'), Some(&text));
        frame(&mut wanted, None, reference_read(&text).as_deref());
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
        "the runtime's base64 and the reference part at byte {:?} of {} answered against {}",
        first,
        run.stdout.len(),
        wanted.len()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
