/* Differential harness for the utf-8 validator's short-string fast path.
   The validator text is extracted from src/runtime.c at build time, never
   copied, so the harness cannot drift from the function it checks. */
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#define K_UTF8_HARNESS 1
#include "extracted.h"

/* the reference: a plain scalar decoder written independently of the
   vector path, straight from the utf-8 grammar in rfc 3629. */
static int ref_valid(const unsigned char* s, long long n) {
    long long i = 0;
    while (i < n) {
        unsigned c = s[i];
        if (c < 0x80) { i += 1; continue; }
        if (c >= 0xC2 && c <= 0xDF) {
            if (i + 1 >= n || (s[i+1] & 0xC0) != 0x80) return 0;
            i += 2; continue;
        }
        if (c == 0xE0) {
            if (i + 2 >= n || s[i+1] < 0xA0 || s[i+1] > 0xBF || (s[i+2] & 0xC0) != 0x80) return 0;
            i += 3; continue;
        }
        if (c >= 0xE1 && c <= 0xEC) {
            if (i + 2 >= n || (s[i+1] & 0xC0) != 0x80 || (s[i+2] & 0xC0) != 0x80) return 0;
            i += 3; continue;
        }
        if (c == 0xED) {
            if (i + 2 >= n || s[i+1] < 0x80 || s[i+1] > 0x9F || (s[i+2] & 0xC0) != 0x80) return 0;
            i += 3; continue;
        }
        if (c >= 0xEE && c <= 0xEF) {
            if (i + 2 >= n || (s[i+1] & 0xC0) != 0x80 || (s[i+2] & 0xC0) != 0x80) return 0;
            i += 3; continue;
        }
        if (c == 0xF0) {
            if (i + 3 >= n || s[i+1] < 0x90 || s[i+1] > 0xBF
                || (s[i+2] & 0xC0) != 0x80 || (s[i+3] & 0xC0) != 0x80) return 0;
            i += 4; continue;
        }
        if (c >= 0xF1 && c <= 0xF3) {
            if (i + 3 >= n || (s[i+1] & 0xC0) != 0x80
                || (s[i+2] & 0xC0) != 0x80 || (s[i+3] & 0xC0) != 0x80) return 0;
            i += 4; continue;
        }
        if (c == 0xF4) {
            if (i + 3 >= n || s[i+1] < 0x80 || s[i+1] > 0x8F
                || (s[i+2] & 0xC0) != 0x80 || (s[i+3] & 0xC0) != 0x80) return 0;
            i += 4; continue;
        }
        return 0;
    }
    return 1;
}


/* the second reference: the character count, arrived at by the other route.
   `harness_utf8_chars` counts the bytes that are not continuations; this
   walks the text a character at a time, taking each character's width from
   its lead byte the way rfc 3629 defines it. Two different questions with
   the same answer on valid utf-8, which is what the sweep below checks. */
static long long ref_chars(const unsigned char* s, long long n) {
    long long i = 0, count = 0;
    while (i < n) {
        unsigned c = s[i];
        i += c < 0x80 ? 1 : c < 0xE0 ? 2 : c < 0xF0 ? 3 : 4;
        count++;
    }
    return count;
}

/* One character of each width, so a sequence of widths names a string. */
static const unsigned char k_glyph[4][4] = {
    { 0x61 },                         /* a */
    { 0xC3, 0xA9 },                   /* e-acute */
    { 0xE2, 0x82, 0xAC },             /* euro sign */
    { 0xF0, 0x9F, 0x98, 0x80 },       /* grinning face */
};

/* Every arrangement of character widths that fits in `room` bytes, appended
   to what is already in `buf`. Twenty-four bytes covers three whole words
   plus the tails either side of each, which is where a counter reading eight
   at a time can go wrong. */
static void widths(unsigned char* buf, long long at, int room,
                   long long* checked, long long* bad) {
    long long got = harness_utf8_chars(buf, at);
    long long want = ref_chars(buf, at);
    (*checked)++;
    if (got != want) {
        if (*bad < 5) printf("CHARS MISMATCH len=%lld got=%lld want=%lld\n", at, got, want);
        (*bad)++;
    }
    for (int w = 1; w <= 4; w++) {
        if (w > room) continue;
        memcpy(buf + at, k_glyph[w - 1], (size_t)w);
        widths(buf, at + w, room - w, checked, bad);
    }
}

int main(void) {
    unsigned char buf[32];
    long long checked = 0, bad = 0;
    /* every one-, two- and three-byte string: 16.8 million, exhaustive over
       the whole range the fast path can reach */
    for (int len = 0; len <= 3; len++) {
        long long total = 1;
        for (int k = 0; k < len; k++) total *= 256;
        for (long long v = 0; v < total; v++) {
            long long x = v;
            for (int k = 0; k < len; k++) { buf[k] = (unsigned char)(x & 0xFF); x >>= 8; }
            int got = harness_utf8_ok((const char*)buf, len);
            int want = ref_valid(buf, len);
            checked++;
            if (got != want) {
                if (bad < 5) {
                    printf("MISMATCH len=%d bytes=", len);
                    for (int k = 0; k < len; k++) printf("%02x ", buf[k]);
                    printf("got=%d want=%d\n", got, want);
                }
                bad++;
            }
        }
    }
    /* every length from four to twenty-four, sampled deterministically so a
       run is reproducible.

       The band used to stop at eight, which left the fast path's whole word
       region unprobed: for a run of nine to fifteen bytes the loop reads one
       word and something else has to answer for the remainder, and nothing
       here ever handed it such a run. A mutation that made that remainder
       answer for the wrong bytes passed this harness with 45 million cases
       and zero mismatches. Twenty-four reaches past the sixteen-byte vector
       boundary as well, so a lead byte can straddle either edge. */
    uint64_t st = 0x9E3779B97F4A7C15ull;
    for (long long t = 0; t < 20000000; t++) {
        st ^= st << 13; st ^= st >> 7; st ^= st << 17;
        int len = 4 + (int)(st % 21);
        for (int k = 0; k < len; k++) {
            st ^= st << 13; st ^= st >> 7; st ^= st << 17;
            /* bias toward ascii and toward lead bytes, where the edges live */
            buf[k] = (st & 3) ? (unsigned char)(st & 0x7F) : (unsigned char)(st & 0xFF);
        }
        int got = harness_utf8_ok((const char*)buf, len);
        int want = ref_valid(buf, len);
        checked++;
        if (got != want) {
            if (bad < 5) {
                printf("MISMATCH len=%d bytes=", len);
                for (int k = 0; k < len; k++) printf("%02x ", buf[k]);
                printf("got=%d want=%d\n", got, want);
            }
            bad++;
        }
    }
    /* the character counter, over every arrangement of character widths that
       fits in twenty-four bytes */
    {
        unsigned char wide[32];
        long long cchecked = 0, cbad = 0;
        widths(wide, 0, 24, &cchecked, &cbad);
        /* and over code points the four representatives above do not reach,
           at every length either side of a word boundary */
        for (long long t = 0; t < 200000; t++) {
            long long at = 0;
            int room = 4 + (int)(t % 21);
            while (room > 0) {
                st ^= st << 13; st ^= st >> 7; st ^= st << 17;
                int w = 1 + (int)(st % 4);
                if (w > room) w = 1;
                unsigned cp;
                if (w == 1) cp = (unsigned)(st % 0x80);
                else if (w == 2) cp = 0x80 + (unsigned)(st % (0x800 - 0x80));
                else if (w == 3) cp = 0x800 + (unsigned)(st % (0x10000 - 0x800));
                else cp = 0x10000 + (unsigned)(st % (0x110000 - 0x10000));
                if (w == 1) wide[at++] = (unsigned char)cp;
                else if (w == 2) {
                    wide[at++] = (unsigned char)(0xC0 | (cp >> 6));
                    wide[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                } else if (w == 3) {
                    wide[at++] = (unsigned char)(0xE0 | (cp >> 12));
                    wide[at++] = (unsigned char)(0x80 | ((cp >> 6) & 0x3F));
                    wide[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                } else {
                    wide[at++] = (unsigned char)(0xF0 | (cp >> 18));
                    wide[at++] = (unsigned char)(0x80 | ((cp >> 12) & 0x3F));
                    wide[at++] = (unsigned char)(0x80 | ((cp >> 6) & 0x3F));
                    wide[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                }
                room -= w;
            }
            long long got = harness_utf8_chars(wide, at);
            long long want = ref_chars(wide, at);
            cchecked++;
            if (got != want) {
                if (cbad < 5) printf("CHARS MISMATCH len=%lld got=%lld want=%lld\n", at, got, want);
                cbad++;
            }
        }
        printf("%lld counts checked, %lld mismatches\n", cchecked, cbad);
        bad += cbad;
        checked += cchecked;
    }
    /* The count each validator hands back. A string the validator accepts is
       given its character count from the same pass, and `length` reads that
       count without scanning again, so a wrong one is a wrong `length`. The
       door above runs with `chars` null and the counter above is a different
       function, so until this section the count the two passes return was
       read by nothing: a wide pass that stopped counting 0xBF as a
       continuation passed 53 million cases with no mismatch.

       Valid text only, from one byte to three hundred, so the wide pass runs
       across many sixteen-byte blocks and its four-block ascii skip. Half the
       characters are ascii runs of up to eighty bytes; the rest take every
       width with a random code point, so every continuation value from 0x80
       to 0xBF turns up. */
    {
        unsigned char text[320];
        long long vchecked = 0, vbad = 0;
        uint64_t vs = 0xD1B54A32D192ED03ull;
        for (long long t = 0; t < 400000; t++) {
            vs ^= vs << 13; vs ^= vs >> 7; vs ^= vs << 17;
            int room = 1 + (int)(vs % 300);
            long long at = 0;
            while (room > 0) {
                vs ^= vs << 13; vs ^= vs >> 7; vs ^= vs << 17;
                if (vs & 1) {
                    int run = 1 + (int)((vs >> 1) % 80);
                    if (run > room) run = room;
                    for (int k = 0; k < run; k++) text[at++] = (unsigned char)(0x20 + (k % 90));
                    room -= run;
                    continue;
                }
                int w = 2 + (int)((vs >> 1) % 3);
                if (w > room) { text[at++] = 0x61; room--; continue; }
                unsigned cp;
                if (w == 2) cp = 0x80 + (unsigned)((vs >> 3) % (0x800 - 0x80));
                else if (w == 3) {
                    cp = 0x800 + (unsigned)((vs >> 3) % (0x10000 - 0x800));
                    if (cp >= 0xD800 && cp <= 0xDFFF) cp = 0xE000;
                } else cp = 0x10000 + (unsigned)((vs >> 3) % (0x110000 - 0x10000));
                if (w == 2) {
                    text[at++] = (unsigned char)(0xC0 | (cp >> 6));
                    text[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                } else if (w == 3) {
                    text[at++] = (unsigned char)(0xE0 | (cp >> 12));
                    text[at++] = (unsigned char)(0x80 | ((cp >> 6) & 0x3F));
                    text[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                } else {
                    text[at++] = (unsigned char)(0xF0 | (cp >> 18));
                    text[at++] = (unsigned char)(0x80 | ((cp >> 12) & 0x3F));
                    text[at++] = (unsigned char)(0x80 | ((cp >> 6) & 0x3F));
                    text[at++] = (unsigned char)(0x80 | (cp & 0x3F));
                }
                room -= w;
            }
            long long want = ref_chars(text, at);
            long long wide_count = -1, scalar_count = -1;
            int wide_ok = harness_utf8_wide((const char*)text, at, &wide_count);
            int scalar_ok = harness_utf8_scalar((const char*)text, at, &scalar_count);
            vchecked++;
            if (!wide_ok || !scalar_ok || wide_count != want || scalar_count != want) {
                if (vbad < 5)
                    printf("VALIDATOR COUNT MISMATCH len=%lld want=%lld wide=%d/%lld scalar=%d/%lld\n",
                           at, want, wide_ok, wide_count, scalar_ok, scalar_count);
                vbad++;
            }
        }
        printf("%lld validator counts checked, %lld mismatches\n", vchecked, vbad);
        bad += vbad;
        checked += vchecked;
    }
    printf("%lld checked, %lld mismatches\n", checked, bad);
    return bad != 0;
}
