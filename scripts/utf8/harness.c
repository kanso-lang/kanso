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

int main(void) {
    unsigned char buf[8];
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
    /* four- and five-byte strings around the fast path's boundary, sampled
       deterministically so a run is reproducible */
    uint64_t st = 0x9E3779B97F4A7C15ull;
    for (long long t = 0; t < 20000000; t++) {
        st ^= st << 13; st ^= st >> 7; st ^= st << 17;
        int len = 4 + (int)(st % 5);
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
    printf("%lld checked, %lld mismatches\n", checked, bad);
    return bad != 0;
}
