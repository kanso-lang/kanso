/* utf-8 helpers: kanso strings are opaque utf-8, positions are codepoints */
static long k_cp_len(unsigned char b) {
    if (b < 0x80) return 1;
    if (b < 0xe0) return 2;
    if (b < 0xf0) return 3;
    return 4;
}

static KValue k_bytes_view(const unsigned char* data, long long len) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_sh_bytes += (long long)sizeof(KBytes);
    KBytes* b = k_alloc(sizeof(KBytes));
    b->len = len;
    b->data = data;
    b->cap = 0;
    KValue v; v.tag = K_BYTES; v.payload = k_ptr(b); return v;
}

KValue k_b_bytes(KValue sv) {
    if (!k_not_failure(sv)) return sv;
    if (sv.tag != K_STR) k_die("bytes takes a string");
    KStr* s = k_as_str(sv);
    return k_bytes_view((const unsigned char*)s->data, s->len);
}

/* `bytes ""` as the emitter writes it: an empty builder with 64 bytes of
   room, header and buffer in one bump. The codegen arm that calls this says
   why. The buffer is arena storage, marked so by bit 0 of `cap`, the regime
   `k_b_append_grow` gives a buffer whose header a rewind reclaims; a builder
   that outgrows it takes a buffer by the same rule any grow does. */
#define K_BYTES_SEED 64
KValue k_b_bytes_seed(void) {
    size_t head = (sizeof(KBytes) + 15) & ~(size_t)15;
    unsigned char* whole = k_alloc(head + sizeof(KBuf) + K_BYTES_SEED);
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_bytes += (long long)sizeof(KBytes);
    KBytes* b = (KBytes*)whole;
    KBuf* buf = (KBuf*)(whole + head);
    k_buf_set_cap(buf, K_BYTES_SEED, 0);
    buf->used = 0;
    b->len = 0;
    b->data = (const unsigned char*)(buf + 1);
    b->cap = K_BYTES_SEED | 1;
    KValue v; v.tag = K_BYTES; v.payload = k_ptr(b); return v;
}

/* `bytes` covers strings; this covers numbers, and it is what keeps byte data
   writable now that a list is never ambiently bytes. Loud outside 0-255 rather
   than truncating, which is the one place this differs from `append`'s single
   byte: there the number is a byte by position, here the list is the thing
   being claimed as bytes and a wrong element is a mistake in the claim. */
KValue k_b_to_bytes(KValue lv, const char* origin) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag != K_LIST) k_die("to_bytes takes a list of byte values");
    KList* l = k_as_list(lv);
    unsigned char* data = (unsigned char*)k_alloc((size_t)(l->len ? l->len : 1));
    for (long long i = 0; i < l->len; i++) {
        KValue item = l->items[i];
        if (!k_not_failure(item)) return item;
        if (item.tag != K_INT || item.payload < 0 || item.payload > 255) {
            /* a subtype of int is an int wherever one goes */
            KValue b = k_sub_base(item);
            if (item.tag == K_SUB && b.tag == K_INT && b.payload >= 0 && b.payload <= 255) {
                data[i] = (unsigned char)b.payload;
                continue;
            }
            return k_err(k_str("to_bytes takes byte values (0-255)"), origin);
        }
        data[i] = (unsigned char)item.payload;
    }
    return k_bytes_view(data, l->len);
}

/* A list or a bytes view, seen as a sequence of values. */
static long long k_seq_len(KValue v) {
    if (v.tag == K_LIST) return k_as_list(v)->len;
    if (v.tag == K_BYTES) return k_as_bytes(v)->len;
    k_die("concat takes two lists");
    return 0;
}

/* Write a list or bytes view into dst[0..len). A bytes view expands to int
   values in place, without first materializing an intermediate list. */
static void k_seq_into(KValue v, KValue* dst) {
    if (v.tag == K_LIST) {
        KList* l = k_as_list(v);
        memcpy(dst, l->items, sizeof(KValue) * l->len);
        return;
    }
    KBytes* b = k_as_bytes(v);
    for (long long i = 0; i < b->len; i++) dst[i] = k_int(b->data[i]);
}

KValue k_b_concat(KValue av, KValue bv) {
    if (__builtin_expect(!k_not_failure(av) || !k_not_failure(bv), 0)) return k_failed2(av, bv);
    long long alen = k_seq_len(av), blen = k_seq_len(bv);
    long long n = alen + blen;
    KValue* items = k_buf(n ? n : 1);
    k_seq_into(av, items);
    k_seq_into(bv, items + alen);
    return k_list_own(items, n);
}

/* Whether a run is ascii, which is the only question the wide pass below has
   to answer for nearly every token a document holds. It is asked 1,571,250
   times in one jsonbench run for 10,975,500 bytes, so the mean run is seven
   bytes and the width that matters is the short one.

   Every width is covered by at most two loads that overlap in the middle
   rather than by a walk. A run of eight or more reads whole words and then
   reads the LAST eight, which repeats bytes the loop already saw and needs no
   arithmetic to know how many. A shorter run has no word to read, so four and
   two bytes do the same trick a step down. Nothing here reads outside
   `data[0..len)`. */
static int k_all_ascii(const char* data, long long len) {
    const unsigned long long hi = 0x8080808080808080ULL;
    unsigned long long w;
    if (len >= 8) {
        for (long long j = 0; j + 8 <= len; j += 8) {
            memcpy(&w, data + j, sizeof w);
            if (w & hi) return 0;
        }
        memcpy(&w, data + len - 8, sizeof w);
        return (w & hi) == 0;
    }
    if (len >= 4) {
        uint32_t a, b;
        memcpy(&a, data, 4);
        memcpy(&b, data + len - 4, 4);
        return ((a | b) & 0x80808080u) == 0;
    }
    if (len >= 2) {
        uint16_t a, b;
        memcpy(&a, data, 2);
        memcpy(&b, data + len - 2, 2);
        return ((a | b) & (uint16_t)0x8080u) == 0;
    }
    if (len == 1) return (uint8_t)data[0] < 0x80;
    return 1;
}

#define K_UTF8_SCALAR_MAX 32


/* The grammar walked a byte at a time. It was the portable arm of the wide
   pass and nothing but a host without simd ever reached it; it is a function
   now because a short run is cheaper here than in the wide pass, which spends
   seven constant loads and two zeroed accumulators before its first block and
   then reads a trailing zero block whatever the length. 298 instructions to
   answer for seven bytes was what jsonbench measured. Every arm returns the
   same sentence the wide pass returns, so which one answered is not
   observable. */
static KValue k_utf8_bad_scalar(const char* data, long long len, const char* origin,
                                long long* chars) {
    long long i = 0;
    long long conts = 0;
    while (i < len) {
        unsigned char b0 = (unsigned char)data[i];
        if (b0 < 0x80) {
            /* A run reaches here holding at least one byte with the high bit
               set, and the ascii around it is still most of the bytes. A
               word at a time, as the door's predicate reads: a clean word
               is eight bytes in three instructions, and a word with a high
               bit in it says where, so the walk lands on that byte rather
               than testing its way there. The byte-at-a-time walk cost 190
               instructions a run on runbench's 134,442 short strings. */
            unsigned long long w, high;
            if (i + 8 <= len) {
                memcpy(&w, data + i, sizeof w);
                high = w & 0x8080808080808080ULL;
                if (!high) { i += 8; continue; }
                i += __builtin_ctzll(high) >> 3;
                continue;
            }
            if (len >= 8) {
                /* The tail, read as the run's last word with the bytes
                   already walked shifted out, the overlap k_all_ascii
                   makes. Runbench's short strings spent 444,609 single
                   steps here for 261,459 whole words. */
                long long back = len - 8;
                memcpy(&w, data + back, sizeof w);
                high = (w & 0x8080808080808080ULL) >> ((i - back) * 8);
                if (!high) { i = len; continue; }
                i += __builtin_ctzll(high) >> 3;
                continue;
            }
            i += 1;
            continue;
        }
        long w;
        unsigned lo = 0x80, hi = 0xBF;
        if (b0 >= 0xC2 && b0 <= 0xDF) { w = 2; }
        else if (b0 == 0xE0) { w = 3; lo = 0xA0; }
        else if (b0 >= 0xE1 && b0 <= 0xEC) { w = 3; }
        else if (b0 == 0xED) { w = 3; hi = 0x9F; }
        else if (b0 >= 0xEE && b0 <= 0xEF) { w = 3; }
        else if (b0 == 0xF0) { w = 4; lo = 0x90; }
        else if (b0 >= 0xF1 && b0 <= 0xF3) { w = 4; }
        else if (b0 == 0xF4) { w = 4; hi = 0x8F; }
        else return k_err(k_str("invalid utf-8"), origin);
        if (i + w > len) return k_err(k_str("invalid utf-8"), origin);
        unsigned char b1 = (unsigned char)data[i + 1];
        if (b1 < lo || b1 > hi) return k_err(k_str("invalid utf-8"), origin);
        for (long j = 2; j < w; j++) {
            if (((unsigned char)data[i + j] & 0xc0) != 0x80) return k_err(k_str("invalid utf-8"), origin);
        }
        conts += w - 1;
        i += w;
    }
    if (chars) *chars = len - conts;
    return k_none();
}

static KValue k_utf8_bad_wide(const char* data, long long len, const char* origin,
                              long long* chars);

/* The front door to utf-8 validation, and it answers without one for every
   token a json document holds. The wide pass below is nearly all setup —
   seven constant loads and two zeroed accumulators before the first block —
   where the ascii test above is two loads, so the test belongs at the call
   site and the pass belongs behind a call. It was inside the pass, which put
   both behind the same call: 83,092,800 instructions for 1,571,250 answers on
   jsonbench, 53 apiece for a mean run of seven ascii bytes. A run that is not
   ascii falls through and the wide pass reads it from the start, so nothing
   is scanned twice. */
static inline __attribute__((always_inline))
KValue k_utf8_bad(const char* data, long long len, const char* origin,
                  long long* chars) {
    if (K_COUNTING) k_stat_utf8_bytes += len;
    if (k_all_ascii(data, len)) {
        if (chars) *chars = len;
        return k_none();
    }
    if (len <= K_UTF8_SCALAR_MAX) return k_utf8_bad_scalar(data, len, origin, chars);
    return k_utf8_bad_wide(data, len, origin, chars);
}

/* The same door for a caller whose runs are ascii 98 times in 100: the two
   validators sit behind preserve_most wrappers, so the caller keeps no frame
   for them, and the 2 in 100 pay the wrapper's saves instead. */
static __attribute__((noinline, cold, preserve_most))
KValue k_utf8_bad_scalar_cold(const char* data, long long len, const char* origin,
                              long long* chars) {
    return k_utf8_bad_scalar(data, len, origin, chars);
}
static __attribute__((noinline, cold, preserve_most))
KValue k_utf8_bad_wide_cold(const char* data, long long len, const char* origin,
                            long long* chars) {
    return k_utf8_bad_wide(data, len, origin, chars);
}
static inline __attribute__((always_inline))
KValue k_utf8_bad_rare(const char* data, long long len, const char* origin,
                       long long* chars) {
    if (K_COUNTING) k_stat_utf8_bytes += len;
    if (k_all_ascii(data, len)) {
        if (chars) *chars = len;
        return k_none();
    }
    if (len <= K_UTF8_SCALAR_MAX) return k_utf8_bad_scalar_cold(data, len, origin, chars);
    return k_utf8_bad_wide_cold(data, len, origin, chars);
}

/* A string the validator just read has its character count in hand: every
   byte was ascii, or the pass counted the continuation bytes as it
   classified them. The memo `k_str_chars` writes lazily is written now, so
   `length` on the string never scans it. Runbench asked the length of 117
   encoder outputs and scanned 22,644,612 bytes a second time to answer. */
static inline void k_str_seed_count(KStr* s, long long chars) {
    if (s->cap == 0 && chars < 2147483647LL) s->cap = (int)(-chars - 1);
}
static KValue k_utf8_check(char* data, long long len, const char* origin);
static KValue k_utf8_finish(KValue bv, const char* origin);

/* utf8 of a slice, without the slice. A byte view is twenty-four bytes built
   to be read for a pointer and a length and dropped: k_utf8_finish takes a
   buffer without copying only when the view owns capacity, and a view carved
   out of somebody else's bytes owns none, so it always copies anyway. The
   decoder does this three million times, once per scalar token.

   Any shape this cannot answer for goes back through both calls, so the
   out-of-range window, the failing argument and the wrong type all behave
   exactly as they did. */
KValue k_b_slice(KValue container, KValue fromv, KValue tov);
static KValue k_b_slice_walk(KStr* s, long long from, long long to);
K_DOORCC KValue k_b_utf8(KValue lv, const char* origin);

/* The bytes arm of `utf8` over a slice, with the KValue convention taken off
   its inputs. Three KValues and a pointer want seven of the six integer
   registers the SysV ABI has, so the last argument spills to the stack and the
   callee reloads it; the arithmetic that follows is a clamp and two adds. The
   emitter reaches this door through the `k_b_utf8_slice_fast` shim, which
   tests the three tags itself; `k_b_utf8_slice` below is the same call for
   anything the shim turns away. */
/* A token of four to seven bytes comes back as the same permanent string the
   last one with those bytes did. The decoder reads the same few hundred keys
   and short values on every document, and building each one again was an
   allocation, a validation and a copy. The cache is direct-mapped over the
   token's own bytes and its length, and it never evicts: a slot that holds
   another token sends this one down the ordinary path, so the permanent
   storage it can take is bounded by its width, whatever the input does. A
   hit is valid utf-8 because the bytes it matched were validated when the
   slot was filled. A zero key never matches, since every key carries its
   length in the top byte. */
static uint64_t k_token_key[1 << K_TOKEN_BITS];
static KValue k_token_cache[1 << K_TOKEN_BITS];

static __attribute__((noinline, cold, preserve_most)) KValue k_token_miss(const char* data, long long len,
                                                                          const char* origin,
                                                                          uint64_t key, unsigned slot) {
    long long chars;
    KValue bad = k_utf8_bad(data, len, origin, &chars);
    if (bad.tag == K_ERR) return bad;
    if (k_token_key[slot] != 0) slot ^= 1;
    if (k_token_key[slot] == 0) {
        /* Its bytes count as permanent the moment the slot is filled, and so
           does its byte in k_token_clean. */
        k_perm_live += (long long)sizeof(KToken) + 1;
        if (k_perm_live > k_perm_peak) k_perm_peak = k_perm_live;
        /* A token is at most seven bytes, so the three tests are asked of one
           word: the bytes past its end are 'A', which none of them flags. The
           byte walk this replaced cost about 17 instructions more a fill, over
           the 632 fills every benchmark makes. */
        uint64_t w = 0x4141414141414141ull;
        memcpy(&w, data, (size_t)len);
        const uint64_t ones = 0x0101010101010101ull, highs = 0x8080808080808080ull;
        uint64_t q = w ^ (ones * '"'), b = w ^ (ones * '\\');
        uint64_t hits = ((q - ones) & ~q) | ((b - ones) & ~b) | ((w - ones * 32) & ~w);
        unsigned char clean = (hits & highs) == 0;
        k_token_clean[slot] = clean;
        KStr* ps = &k_token_store[slot].s;
        ps->len = (int)len;
        ps->data = k_token_store[slot].data;
        memcpy(ps->data, data, (size_t)len);
        ps->data[len] = 0;
        ps->cap = (int)(-chars - 1);
        KValue pv; pv.tag = K_STR; pv.payload = k_ptr(ps);
        k_token_cache[slot] = pv;
        k_token_key[slot] = key;
        return pv;
    }
    KStr* s = k_str_alloc(len);
    memcpy(s->data, data, (size_t)len);
    s->data[len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

/* The cached string for a token of four to seven bytes. */
static inline KValue k_token_of(const char* data, long long len, const char* origin) {
    uint32_t a, b;
    memcpy(&a, data, 4);
    memcpy(&b, data + len - 4, 4);
    uint64_t key = (uint64_t)a | ((uint64_t)b << (8 * (len - 4))) | ((uint64_t)len << 56);
    unsigned slot = (unsigned)((key * 0x9E3779B97F4A7C15ull) >> (64 - K_TOKEN_BITS));
    if (k_token_key[slot] == key) return k_token_cache[slot];
    if (k_token_key[slot ^ 1] == key) return k_token_cache[slot ^ 1];
    return k_token_miss(data, len, origin, key, slot);
}

KValue k_b_utf8_slice_raw(const unsigned char* bytes, long long blen,
                          long long from, long long to, const char* origin) {
    const char* data = (const char*)bytes;
    long long len = 0;
    if (k_span_in(from, to, blen)) {
        data += from - 1;
        len = to - from + 1;
    }
    /* The count is not asked for here: these are the decoder's tokens,
       861,498 of them on runbench, and none of them is asked its length.
       Seeding each one cost 7,728,237 instructions there against 13,280,580
       the whole-string doors below save. */
    if (len >= 4 && len <= 7) return k_token_of(data, len, origin);
    KValue bad = k_utf8_bad_rare(data, len, origin, NULL);
    if (bad.tag == K_ERR) return bad;
    return k_str_n(data, len);
}

KValue k_b_utf8_slice(KValue container, KValue fromv, KValue tov, const char* origin) {
    if (container.tag == K_BYTES && fromv.tag == K_INT && tov.tag == K_INT) {
        KBytes* b = k_as_bytes(container);
        return k_b_utf8_slice_raw(b->data, b->len, fromv.payload, tov.payload,
                                  origin);
    }
    KValue sliced = k_b_slice(container, fromv, tov);
    return k_b_utf8(sliced, origin);
}

K_DOORCC KValue k_b_utf8(KValue lv, const char* origin) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag == K_BYTES) {
        return k_utf8_finish(lv, origin);
    }
    if (lv.tag != K_LIST) k_die("utf8 takes a list of byte values");
    KList* l = k_as_list(lv);
    /* build straight into the string's own buffer, then validate in place — the
       old path filled a scratch buffer and let k_str_n copy it a second time. */
    KStr* s = k_str_alloc(l->len);
    for (long long i = 0; i < l->len; i++) {
        KValue item = l->items[i];
        if (!k_not_failure(item)) return item;
        if (item.tag != K_INT || item.payload < 0 || item.payload > 255) {
            return k_err(k_str("utf8 takes byte values (0-255)"), origin);
        }
        s->data[i] = (char)item.payload;
    }
    s->data[l->len] = 0;
    long long chars;
    KValue bad = k_utf8_bad(s->data, l->len, origin, &chars);
    if (bad.tag == K_ERR) return bad;
    k_str_seed_count(s, chars);
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

/* Reached only by a run carrying a byte with the high bit set: the ascii test
   and the byte counter both live in `k_utf8_bad` above, inlined at each call
   site. */
static KValue k_utf8_bad_wide(const char* data, long long len, const char* origin,
                              long long* chars) {
#if defined(__aarch64__)
    /* keiser & lemire, "validating utf-8 in less than one instruction per
       byte" (2021): three nibble lookups classify every two-byte window,
       a saturating compare pins 3/4-byte continuation runs, and the
       whole document validates in one pass. an all-ascii block skips the
       classification. a trailing zero block terminates any sequence cut
       off at the end, so truncation needs no special case. */
    enum {
        TOO_SHORT = 1 << 0, TOO_LONG = 1 << 1, OVERLONG_3 = 1 << 2,
        TOO_LARGE = 1 << 3, SURROGATE = 1 << 4, OVERLONG_2 = 1 << 5,
        TOO_LARGE_1000 = 1 << 6, OVERLONG_4 = 1 << 6, TWO_CONTS = 1 << 7,
        CARRY = TOO_SHORT | TOO_LONG | TWO_CONTS,
    };
    static const uint8_t b1h[16] = {
        TOO_LONG, TOO_LONG, TOO_LONG, TOO_LONG,
        TOO_LONG, TOO_LONG, TOO_LONG, TOO_LONG,
        TWO_CONTS, TWO_CONTS, TWO_CONTS, TWO_CONTS,
        TOO_SHORT | OVERLONG_2, TOO_SHORT,
        TOO_SHORT | OVERLONG_3 | SURROGATE,
        TOO_SHORT | TOO_LARGE | TOO_LARGE_1000 | OVERLONG_4,
    };
    static const uint8_t b1l[16] = {
        CARRY | OVERLONG_2 | OVERLONG_3 | OVERLONG_4, CARRY | OVERLONG_2,
        CARRY, CARRY,
        CARRY | TOO_LARGE, CARRY | TOO_LARGE | TOO_LARGE_1000,
        CARRY | TOO_LARGE | TOO_LARGE_1000, CARRY | TOO_LARGE | TOO_LARGE_1000,
        CARRY | TOO_LARGE | TOO_LARGE_1000, CARRY | TOO_LARGE | TOO_LARGE_1000,
        CARRY | TOO_LARGE | TOO_LARGE_1000, CARRY | TOO_LARGE | TOO_LARGE_1000,
        CARRY | TOO_LARGE | TOO_LARGE_1000,
        CARRY | TOO_LARGE | TOO_LARGE_1000 | SURROGATE,
        CARRY | TOO_LARGE | TOO_LARGE_1000, CARRY | TOO_LARGE | TOO_LARGE_1000,
    };
    static const uint8_t b2h[16] = {
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT,
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT,
        TOO_LONG | OVERLONG_2 | TWO_CONTS | OVERLONG_3 | TOO_LARGE_1000 | OVERLONG_4,
        TOO_LONG | OVERLONG_2 | TWO_CONTS | OVERLONG_3 | TOO_LARGE,
        TOO_LONG | OVERLONG_2 | TWO_CONTS | SURROGATE | TOO_LARGE,
        TOO_LONG | OVERLONG_2 | TWO_CONTS | SURROGATE | TOO_LARGE,
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT,
    };
    uint8x16_t t1h = vld1q_u8(b1h), t1l = vld1q_u8(b1l), t2h = vld1q_u8(b2h);
    uint8x16_t prev = vdupq_n_u8(0);
    uint8x16_t error = vdupq_n_u8(0);
    long long i = 0;
    long long conts = 0;
    long long nblocks = (len + 15) / 16 + 1;
    for (long long blk = 0; blk < nblocks; blk++) {
        uint8_t buf[16];
        uint8x16_t cur;
        /* Four blocks of ascii at once; the sse arm below says why. */
        while (i + 64 <= len && vmaxvq_u8(prev) < 0x80) {
            uint8x16_t a = vld1q_u8((const uint8_t*)data + i);
            uint8x16_t b = vld1q_u8((const uint8_t*)data + i + 16);
            uint8x16_t c = vld1q_u8((const uint8_t*)data + i + 32);
            uint8x16_t d = vld1q_u8((const uint8_t*)data + i + 48);
            if (vmaxvq_u8(vorrq_u8(vorrq_u8(a, b), vorrq_u8(c, d))) >= 0x80) break;
            prev = d;
            i += 64;
            blk += 4;
        }
        if (blk >= nblocks) break;
        if (i + 16 <= len) {
            cur = vld1q_u8((const uint8_t*)data + i);
        } else {
            for (int j = 0; j < 16; j++)
                buf[j] = i + j < len ? (uint8_t)data[i + j] : 0;
            cur = vld1q_u8(buf);
        }
        i += 16;
        if (vmaxvq_u8(cur) < 0x80 && vmaxvq_u8(prev) < 0x80) {
            prev = cur;
            continue;
        }
        uint8x16_t prev1 = vextq_u8(prev, cur, 15);
        uint8x16_t sc = vandq_u8(
            vandq_u8(vqtbl1q_u8(t1h, vshrq_n_u8(prev1, 4)),
                     vqtbl1q_u8(t1l, vandq_u8(prev1, vdupq_n_u8(0x0F)))),
            vqtbl1q_u8(t2h, vshrq_n_u8(cur, 4)));
        uint8x16_t prev2 = vextq_u8(prev, cur, 14);
        uint8x16_t prev3 = vextq_u8(prev, cur, 13);
        uint8x16_t is3 = vqsubq_u8(prev2, vdupq_n_u8(0xDF));
        uint8x16_t is4 = vqsubq_u8(prev3, vdupq_n_u8(0xEF));
        uint8x16_t must23 = vcgtq_u8(vorrq_u8(is3, is4), vdupq_n_u8(0));
        uint8x16_t must23_80 = vandq_u8(must23, vdupq_n_u8(0x80));
        error = vorrq_u8(error, veorq_u8(sc, must23_80));
        /* a continuation byte is 10xxxxxx; only a block that reached the
           classification can hold one, so the ascii blocks above cost
           nothing here */
        uint8x16_t cont = vceqq_u8(vandq_u8(cur, vdupq_n_u8(0xC0)), vdupq_n_u8(0x80));
        conts += vaddvq_u8(vshrq_n_u8(cont, 7));
        prev = cur;
    }
    if (vmaxvq_u8(error) != 0) return k_err(k_str("invalid utf-8"), origin);
    if (chars) *chars = len - conts;
    return k_none();
#elif defined(__x86_64__)
    /* the same keiser & lemire structure as the neon path, on sse:
       pshufb is the nibble lookup, alignr the shifted windows, subs_epu8
       the saturating pins. */
    enum {
        TOO_SHORT = 1 << 0, TOO_LONG = 1 << 1, OVERLONG_3 = 1 << 2,
        TOO_LARGE = 1 << 3, SURROGATE = 1 << 4, OVERLONG_2 = 1 << 5,
        TOO_LARGE_1000 = 1 << 6, OVERLONG_4 = 1 << 6, TWO_CONTS = 1 << 7,
        CARRY = TOO_SHORT | TOO_LONG | TWO_CONTS,
    };
    __m128i t1h = _mm_setr_epi8(
        TOO_LONG, TOO_LONG, TOO_LONG, TOO_LONG,
        TOO_LONG, TOO_LONG, TOO_LONG, TOO_LONG,
        (char)TWO_CONTS, (char)TWO_CONTS, (char)TWO_CONTS, (char)TWO_CONTS,
        TOO_SHORT | OVERLONG_2, TOO_SHORT,
        TOO_SHORT | OVERLONG_3 | SURROGATE,
        TOO_SHORT | TOO_LARGE | TOO_LARGE_1000 | OVERLONG_4);
    __m128i t1l = _mm_setr_epi8(
        (char)(CARRY | OVERLONG_2 | OVERLONG_3 | OVERLONG_4),
        (char)(CARRY | OVERLONG_2), (char)CARRY, (char)CARRY,
        (char)(CARRY | TOO_LARGE), (char)(CARRY | TOO_LARGE | TOO_LARGE_1000),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000), (char)(CARRY | TOO_LARGE | TOO_LARGE_1000),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000), (char)(CARRY | TOO_LARGE | TOO_LARGE_1000),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000), (char)(CARRY | TOO_LARGE | TOO_LARGE_1000),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000 | SURROGATE),
        (char)(CARRY | TOO_LARGE | TOO_LARGE_1000), (char)(CARRY | TOO_LARGE | TOO_LARGE_1000));
    __m128i t2h = _mm_setr_epi8(
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT,
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT,
        (char)(TOO_LONG | OVERLONG_2 | TWO_CONTS | OVERLONG_3 | TOO_LARGE_1000 | OVERLONG_4),
        (char)(TOO_LONG | OVERLONG_2 | TWO_CONTS | OVERLONG_3 | TOO_LARGE),
        (char)(TOO_LONG | OVERLONG_2 | TWO_CONTS | SURROGATE | TOO_LARGE),
        (char)(TOO_LONG | OVERLONG_2 | TWO_CONTS | SURROGATE | TOO_LARGE),
        TOO_SHORT, TOO_SHORT, TOO_SHORT, TOO_SHORT);
    __m128i low_nib = _mm_set1_epi8(0x0F);
    __m128i prev = _mm_setzero_si128();
    __m128i error = _mm_setzero_si128();
    long long i = 0;
    __m128i counted = _mm_setzero_si128();
    long long nblocks = (len + 15) / 16 + 1;
    for (long long blk = 0; blk < nblocks; blk++) {
        unsigned char tail[16];
        __m128i cur;
        /* Four blocks of ascii at once, one mask for the four: an encoder's
           output is ascii but for the strings it copied through, and the
           block loop below paid its twelve instructions on every sixteen of
           them. The previous block must be ascii too, since a sequence that
           started in it needs the classification the skipped blocks would
           not run. */
        while (i + 64 <= len && !_mm_movemask_epi8(prev)) {
            __m128i a = _mm_loadu_si128((const __m128i*)(data + i));
            __m128i b = _mm_loadu_si128((const __m128i*)(data + i + 16));
            __m128i c = _mm_loadu_si128((const __m128i*)(data + i + 32));
            __m128i d = _mm_loadu_si128((const __m128i*)(data + i + 48));
            if (_mm_movemask_epi8(_mm_or_si128(_mm_or_si128(a, b), _mm_or_si128(c, d)))) break;
            prev = d;
            i += 64;
            blk += 4;
        }
        if (blk >= nblocks) break;
        if (i + 16 <= len) {
            cur = _mm_loadu_si128((const __m128i*)(data + i));
        } else {
            for (int j = 0; j < 16; j++)
                tail[j] = i + j < len ? (unsigned char)data[i + j] : 0;
            cur = _mm_loadu_si128((const __m128i*)tail);
        }
        i += 16;
        if (!_mm_movemask_epi8(_mm_or_si128(cur, prev))) {
            prev = cur;
            continue;
        }
        __m128i prev1 = _mm_alignr_epi8(cur, prev, 15);
        __m128i sc = _mm_and_si128(
            _mm_and_si128(
                _mm_shuffle_epi8(t1h, _mm_and_si128(_mm_srli_epi16(prev1, 4), low_nib)),
                _mm_shuffle_epi8(t1l, _mm_and_si128(prev1, low_nib))),
            _mm_shuffle_epi8(t2h, _mm_and_si128(_mm_srli_epi16(cur, 4), low_nib)));
        __m128i prev2 = _mm_alignr_epi8(cur, prev, 14);
        __m128i prev3 = _mm_alignr_epi8(cur, prev, 13);
        __m128i is3 = _mm_subs_epu8(prev2, _mm_set1_epi8((char)0xDF));
        __m128i is4 = _mm_subs_epu8(prev3, _mm_set1_epi8((char)0xEF));
        __m128i must23 = _mm_cmpgt_epi8(
            _mm_or_si128(is3, is4), _mm_setzero_si128());
        __m128i must23_80 = _mm_and_si128(must23, _mm_set1_epi8((char)0x80));
        error = _mm_or_si128(error, _mm_xor_si128(sc, must23_80));
        /* a continuation byte is 10xxxxxx; only a block that reached the
           classification can hold one, so the ascii blocks above cost
           nothing here. As a signed byte it is -128 to -65, one compare.
           The count stays in the register: psadbw sums the sixteen ones
           into two 64-bit lanes. The runtime is built for ssse3, which has
           no popcnt, and the movemask and bit-twiddled popcount this
           replaces were 22 of the block's 51 instructions. */
        __m128i cont = _mm_cmpgt_epi8(_mm_set1_epi8((char)-64), cur);
        __m128i ones = _mm_sub_epi8(_mm_setzero_si128(), cont);
        counted = _mm_add_epi64(counted, _mm_sad_epu8(ones, _mm_setzero_si128()));
        prev = cur;
    }
    if (_mm_movemask_epi8(_mm_cmpeq_epi8(error, _mm_setzero_si128())) != 0xFFFF)
        return k_err(k_str("invalid utf-8"), origin);
    long long conts = _mm_cvtsi128_si64(counted)
                    + _mm_cvtsi128_si64(_mm_unpackhi_epi64(counted, counted));
    if (chars) *chars = len - conts;
    return k_none();
#else
    return k_utf8_bad_scalar(data, len, origin, chars);
#endif
}

static KValue k_utf8_check(char* data, long long len, const char* origin) {
    long long chars;
    KValue bad = k_utf8_bad(data, len, origin, &chars);
    if (bad.tag == K_ERR) return bad;
    KValue v = k_str_n(data, len);
    k_str_seed_count(k_as_str(v), chars);
    return v;
}

#define K_CHARS_UNKNOWN 0x7fffffffffffffffLL
static KValue k_utf8_take(KBytes* b, long long chars);

static KValue k_utf8_finish(KValue bv, const char* origin) {
    KBytes* b = k_as_bytes(bv);
    /* A short value shares its token with the fused door's, so a program that
       decodes a slice after asking whether it is none gets the same string. */
    if (b->len >= 4 && b->len <= 7) return k_token_of((const char*)b->data, b->len, origin);
    long long chars;
    KValue bad = k_utf8_bad((const char*)b->data, b->len, origin, &chars);
    if (bad.tag == K_ERR) return bad;
    return k_utf8_take(b, chars);
}

/* `built_text`: bytes the standard library assembled from text and ascii
   bytes alone. Every piece was text when it was appended, so the whole is,
   and the check `utf8` makes would read the bytes again to learn nothing.
   The JSON encoder ends this way, and on runbench that check read every
   document it wrote, 17 MB over 90 encodes. */
KValue k_b_built_text(KValue bv) {
    if (!k_not_failure(bv)) return bv;
    if (bv.tag != K_BYTES) k_die("built_text takes bytes");
    return k_utf8_take(k_as_bytes(bv), K_CHARS_UNKNOWN);
}

/* The builder-aware finish: when the bytes own a KBuf-headed buffer with
   room for the terminator, the string takes the buffer in place — no
   copy — and the frontier burns (used = cap) so a later append on any
   surviving bytes value must grow away rather than write under the
   string. `chars` is the character count when the caller has one, and
   K_CHARS_UNKNOWN when it does not, which leaves the count for `length` to
   take when it is asked. */
static KValue k_utf8_take(KBytes* b, long long chars) {
    long long bcap = b->cap & ~1LL;
    if (bcap && b->len < bcap) {
        KBuf* buf = ((KBuf*)b->data) - 1;
        if (buf->used == b->len) {
            ((unsigned char*)b->data)[b->len] = 0;
            buf->used = bcap;
            /* a malloc-backed chunk now belongs to a string that dies at
               the innermost rewind; register it so that rewind frees it */
            if (k_bytes_malloced(b) && k_beat_depth > 0 && k_beat_depth <= K_BEAT_MAX) {
                int d = k_beat_depth - 1;
                if (k_chunkreg_n[d] < 256) {
                    k_chunkreg[d][k_chunkreg_n[d]++] = ((KBuf*)b->data) - 1;
                } else {
                    k_chunkreg_spill[d]++;
                }
                k_reg_any_at(d) |= K_REG_CHUNK;
            }
            KStr* s = k_alloc(sizeof(KStr));
            s->len = (long)b->len;
            s->data = (char*)b->data;
            s->cap = 0;
            k_str_seed_count(s, chars);
            if (K_COUNTING) k_stat_utf8_zerocopy++;
            KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
        }
    }
    KValue v = k_str_n((const char*)b->data, b->len);
    k_str_seed_count(k_as_str(v), chars);
    return v;
}

KValue k_b_chars(KValue sv) {
    if (!k_not_failure(sv)) return sv;
    if (sv.tag != K_STR) k_die("chars takes a string");
    KStr* s = k_as_str(sv);
    long count = 0;
    for (long i = 0; i < s->len; i += k_cp_len((unsigned char)s->data[i])) count++;
    KValue* items = k_buf(count ? count : 1);
    long at = 0;
    for (long i = 0; i < count; i++) {
        long w = k_cp_len((unsigned char)s->data[at]);
        items[i] = k_str_n(s->data + at, w);
        at += w;
    }
    return k_list_own(items, count);
}

/* One pass over the bytes. utf-8 is self-synchronizing, so a separator that
   is itself valid utf-8 can only match where a codepoint begins, and a byte
   search needs no codepoint arithmetic to stay on boundaries. The scan this
   replaced asked `length` and `slice` at every position, both of which count
   codepoints from the start, so splitting a hundred kilobytes cost seconds. */
/* The first position at or after `from` where sep occurs, or -1. A match can
   only start where sep's first byte is, so memchr covers the ground and memcmp
   runs only at the candidates it returns. The loop this replaced called memcmp
   at every position: readbench split 188,698 bytes on "\n" that the bytes never
   contain and paid 75,479,200 calls at ten instructions each to learn it. */
static long k_split_find(const char* d, long len, const char* sep, long seplen, long from) {
    while (from + seplen <= len) {
        const char* p = memchr(d + from, sep[0], (size_t)(len - seplen - from + 1));
        if (!p) return -1;
        long at = (long)(p - d);
        if (seplen == 1 || memcmp(d + at, sep, (size_t)seplen) == 0) return at;
        from = at + 1;
    }
    return -1;
}

KValue k_b_split(KValue sv, KValue sepv) {
    if (__builtin_expect(!k_not_failure(sv) || !k_not_failure(sepv), 0)) return k_failed2(sv, sepv);
    if (sv.tag != K_STR || sepv.tag != K_STR) k_die("split takes two strings");
    KStr* s = k_as_str(sv);
    KStr* sep = k_as_str(sepv);
    if (sep->len <= 0) k_die("split needs a separator");
    long count = 1;
    for (long i = 0; ; ) {
        long at = k_split_find(s->data, s->len, sep->data, sep->len, i);
        if (at < 0) break;
        count++;
        i = at + sep->len;
    }
    /* No separator anywhere, so the one piece is the whole string. A KStr's
       data and len are written once at construction -- only its `cap` moves
       after, and that is the memoised codepoint count, which two holders would
       compute alike -- so the piece can be the input value rather than a copy
       of it. readbench splits 188,698 bytes holding no separator two hundred
       times, and the copy this drops was 82.66% of what the benchmark had left.

       Taken here rather than as a test on the last piece below. Written that
       way, `sv` stayed live to the tail and spilled: scanbench makes 501,500
       split calls, four of which reach this case, and every one of them paid
       twelve instructions for the liveness. Returning early leaves the loop
       below reading exactly what it read before. */
    if (count == 1) {
        KValue* one = k_buf(1);
        one[0] = sv;
        return k_list_own(one, 1);
    }
    KValue* items = k_buf(count);
    long at = 0, from = 0, n = 0;
    for (;;) {
        long hit = k_split_find(s->data, s->len, sep->data, sep->len, at);
        if (hit < 0) break;
        items[n++] = k_str_n(s->data + from, hit - from);
        at = hit + sep->len;
        from = at;
    }
    /* `from` is past a separator's end after any match, so from == 0 says the
       string held none and this last piece is the whole of it. A KStr's data
       and len are written once at construction -- only its `cap` moves after,
       and that is the memoised codepoint count, which two holders would
       compute alike -- so the piece can be the input value rather than a copy
       of it. readbench splits 188,698 bytes that hold no separator two hundred
       times, and the copy it no longer makes was 82.66% of what the benchmark
       had left. */
    items[n++] = k_str_n(s->data + from, s->len - from);
    return k_list_own(items, n);
}

KValue k_b_is_desc(KValue v) {
    if (!k_not_failure(v)) return v;
    return k_bool(v.tag == K_DESC);
}

static KValue k_wide_cache[256];
static uint32_t k_wide_key[256];
static unsigned char k_wide_ready[256];

/* `at` is what the builtin is called; indexing is what the reader wrote.
   Out of line because its message buffer is the largest thing in k_b_at's
   frame: with it inline every index paid a 280-byte frame and six saved
   registers to reach a character the cursor already stood next to. */
static __attribute__((noinline, cold)) void k_die_index(KValue container) {
    const char* hint = k_lazy_hint(container);
    char said[256];
    snprintf(said, sizeof said,
             "indexing takes a list or string with a 1-based position, or a map "
             "with a key%s",
             hint ? hint : "");
    k_die(said);
}

/* The two arms of an index that allocate or refuse: a wide character the
   cache does not hold, and a map or an unindexable value. They are a
   separate function so the arm every runbench index takes -- an ascii
   character out of k_str_n's cache -- carries no frame; with the calls in
   the same body the compiler opened every index with six pushes and a
   forty-byte reservation for paths a run takes six times. preserve_most on
   both, for the reason k_alloc_refill gives. */
static __attribute__((noinline, cold, preserve_most)) KValue k_b_at_wide_miss(uint32_t q, uint32_t key, unsigned slot, long w) {
    if (!k_wide_ready[slot]) {
        KStr* ps = k_alloc_const(sizeof(KStr) + 5);
        ps->len = w;
        ps->cap = -2;
        ps->data = (char*)(ps + 1);
        memcpy(ps->data, &q, 4);
        ps->data[w] = 0;
        KValue pv; pv.tag = K_STR; pv.payload = k_ptr(ps);
        k_wide_cache[slot] = pv;
        k_wide_key[slot] = key;
        k_wide_ready[slot] = 1;
        return pv;
    }
    KStr* os = k_str_alloc(w);
    memcpy(os->data, &q, 4);
    os->data[w] = 0;
    os->cap = -2;
    KValue one; one.tag = K_STR; one.payload = k_ptr(os);
    return one;
}

KValue k_b_at(KValue container, KValue index);
static __attribute__((noinline, cold, preserve_most)) KValue k_b_at_rest(KValue container, KValue index) {
    /* An index reads through a subtype to its base value, as every builtin
       does, so a subtype of int is a position and one of string is a key. */
    if (container.tag == K_SUB || index.tag == K_SUB) {
        /* Called through a pointer the optimiser cannot see through: a direct
           call let clang copy k_b_at's arms into this cold path, 1,200 bytes
           of text in every program that indexes. */
        KValue (*volatile again)(KValue, KValue) = k_b_at;
        return again(k_sub_base(container), k_sub_base(index));
    }
    /* A position past a word is past the end of anything memory can hold. */
    if (index.tag == K_BIG && (container.tag == K_LIST || container.tag == K_BYTES || container.tag == K_STR))
        return k_none();
    if (container.tag == K_MAP) {
        /* A map is indexed by an int or a string. Anything else, a none
           among them, is refused as the interpreter refuses it, where the
           search below would answer a miss. */
        if (!K_IS_INT(index) && index.tag != K_STR) {
            k_die_index(container);
            return k_none();
        }
        KMap* m = k_as_map(container);
        if (!m->sorted) k_map_sort_build(m);
        KValue* hit = k_map_find(m, index);
        return hit ? *hit : k_none();
    }
    k_die_index(container);
    return k_none();
}

KValue k_b_at(KValue container, KValue index) {
    if (__builtin_expect(!k_not_failure(container) || !k_not_failure(index), 0)) return k_failed2(container, index);
    /* The STR arm is asked FIRST. `length s[i]` over text is the index this
       runtime meets most -- 690,000 calls on runbench, all of them from
       tally_4 -- and the LIST test in front of it was two instructions
       every one of them paid to be told no. A list index pays the same two
       instructions now, which is why this is measured on the whole work
       vein rather than on runbench alone. */
    if (container.tag == K_STR && index.tag == K_INT) {
        KStr* s = k_as_str(container);
        long long want = index.payload;
        long at;
        /* The step. A walk by index asks for the character after the one it
           just read, and the cursor stands on that one: the answer is the
           cursor's byte plus its character's width, which `k_str_seek` also
           finds, but only after asking whether the string is all ascii and
           whether the cursor's own character was wanted. On the run program's
           index phase, 690,000 indexes of a multibyte string, those questions
           cost 8,280,000 instructions. */
        if (s == k_seek_str && want == k_seek_char + 1 && s->cap != 0 && k_seek_byte < s->len) {
            if (K_COUNTING) k_stat_seek_resumes++;
            if (K_COUNTING) k_stat_seek_steps++;
            at = k_seek_byte + k_cp_len((unsigned char)s->data[k_seek_byte]);
            if (at >= s->len) return k_none();
            k_seek_char = want;
            k_seek_byte = at;
        } else {
            if (want < 1) return k_none();
            at = k_str_seek(s, want);
            if (at < 0) return k_none();
        }
        long w = k_cp_len((unsigned char)s->data[at]);
        if (w == 1) {
            /* an ascii character comes from the cache through k_str_n */
            KValue one = k_str_n(s->data + at, 1);
            KStr* os = k_as_str(one);
            if (os->cap == 0) os->cap = -2;
            return one;
        }
        /* A wide character is two, three or four bytes, and it goes over as
           four: the read past it stops at the string's own terminator at
           worst, since at + w <= len, and the write lands inside storage
           k_alloc rounded up to sixteen. glibc's memcpy took seventeen
           instructions to choose how to move three bytes, 345,000 times a
           run on runbench; a byte loop bounded by w cost more than the call.
           One character by construction, so its count is known without a
           scan: `length s[i]` asked k_utf8_chars to walk the bytes of every
           multi-byte character it was handed, 345,220 times on runbench. */
        uint32_t q;
        memcpy(&q, s->data + at, 4);
        /* The same character comes back from the same permanent string, the
           way an ascii one does from k_str_n's cache: a walk over text by
           index meets the same few characters at every step, and building
           each one in the arena was an allocation and a hundred instructions
           a step. The cache is direct-mapped over the character's own bytes
           and never evicts -- a slot that already holds another character
           sends this one down the arena path -- so the permanent storage it
           can hold is bounded by its width, whatever the text does. */
        uint32_t key = q & (0xffffffffu >> (8 * (4 - w)));
        unsigned slot = (unsigned)((key * 2654435761u) >> 24);
        if (k_wide_key[slot] == key && k_wide_ready[slot]) return k_wide_cache[slot];
        return k_b_at_wide_miss(q, key, slot, w);
    }
    if (container.tag == K_LIST && index.tag == K_INT) {
        KList* l = k_as_list(container);
        long long i = index.payload;
        if (i < 1 || i > l->len) return k_none();
        return l->items[i - 1];
    }
    if (container.tag == K_BYTES && index.tag == K_INT) {
        KBytes* b = k_as_bytes(container);
        long long i = index.payload;
        if (i < 1 || i > b->len) return k_none();
        return k_int(b->data[i - 1]);
    }
    return k_b_at_rest(container, index);
}

KValue k_index(KValue container, KValue key, const char* origin) {
    KValue found = k_b_at(container, key);
    if (found.tag == K_NONE) {
        return k_err(k_concat(k_str("missing index "), k_render(key, 1)), origin);
    }
    /* `!` says demand, so the element is forced here rather than at each
       builtin that later inspects it. */
    return k_force(found);
}

/* A header allocated before the current beat mark survives the rewind that
   ends a step of a bind chain; storage allocated after it does not. Writing
   through an older header leaves a surviving list pointing at a buffer the
   rewind reclaims, or holding an element it is about to take. Only a header
   born inside this beat may be mutated; every other one takes the copying
   path, which is what the in-place form is an optimisation of.

   The head block answers without a walk, which is where a hot loop's own
   allocations are; anything else falls back to the walk that settles it
   exactly, because guessing wrong there costs a copy on every push. */
static int k_born_this_beat(const void* p) {
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX || !k_blocks) return 1;
    KMark* m = &k_beat_stack[k_beat_depth - 1];
    const char* q = (const char*)p;
    if (q >= (const char*)(k_blocks + 1) && q < k_arena)
        return k_blocks != m->block || q >= (const char*)m->ptr;
    return !k_survives_x(p, m);
}

/* `proven` says the call site is one the linearity analysis licensed, so the
   list has no other holder and a header that predates this beat may still be
   mutated. Without it a proven in-place push silently stops being in-place the
   moment the value is a loop's accumulator — a fresh header per iteration, in
   the arena, threaded by the loop and freed by its rewind. The bytes append
   has always had this standing: `mutate` reaching it IS the proof, and no
   born-this-beat test stands in front of it. */
static KValue k_b_push_into_proven(KValue lv, KValue item, int mutate, int proven);

static KValue k_b_push_into(KValue lv, KValue item, int mutate) {
    return k_b_push_into_proven(lv, item, mutate, 0);
}

/* The refusal, out of line: its 128-byte message buffer sized the frame of
   every push that took the frontier slot. */
__attribute__((noreturn, noinline, cold)) static void k_die_push_takes(KValue lv) {
    const char* hint = k_lazy_hint(lv);
    char said[128];
    snprintf(said, sizeof said, "push takes a list and a value%s", hint ? hint : "");
    k_die(said);
}

static KValue k_b_push_grow(KValue lv, KList* l, KValue item, int mutate);

static KValue k_b_push_into_proven(KValue lv, KValue item, int mutate, int proven) {
    /* An err item is infectious like any argument: pushing one answers it.
       Only the list was asked until 2026-09-28, so the compiled engines stored
       the err as an element where the interpreter handed it on. */
    if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(item), 0)) return k_failed2(lv, item);
    if (lv.tag != K_LIST) k_die_push_takes(lv);
    KList* l = k_as_list(lv);
    if (mutate && !proven && !k_born_this_beat(l)) mutate = 0;
    if (mutate) item = k_big_kept(item, l);
    KBuf* buf = k_buf_of(l->items);
    if (buf->used == l->len && l->len < k_buf_cap(buf)) {
        /* this list is the frontier of its buffer: claim the next slot */
        l->items[l->len] = item;
        k_note_if_carried(item);
        buf->used++;
        if (mutate) {
            l->len++;
            return lv;
        }
        KList* out = k_alloc(sizeof(KList));
        out->len = l->len + 1;
        out->items = l->items;
        KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
    }
    return k_b_push_grow(lv, l, item, mutate);
}

/* A push that is not on its buffer's frontier: new storage, the elements
   copied over, the item after them. Reached from the general push once the
   frontier test has failed, and from the in-place push directly, which has
   already asked the same question and does not ask it twice. */
/* The outgrown buffer of an accumulator came from malloc, and its release
   is the rare arm of a grow: one call in sixteen on the run program. */
static __attribute__((noinline, cold, preserve_most)) void k_buf_release(KBuf* ob) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_bytes_freed++;
    k_perm_live -= (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)k_buf_cap(ob));
    free(ob);
}

static KValue k_b_push_grow(KValue lv, KList* l, KValue item, int mutate) {
    /* The smallest power of two that holds the new element, doubled again
       past sixteen, so the steps run 4, 8, 16, 64, 256 and a long
       accumulator still grows in few of them. A list of four that took a
       fifth element went straight to sixteen slots, and most lists the JSON
       decoder builds stop at five or six: 1,463 of them on one decode of
       bench/large.json, each 272 bytes where 144 hold it. The run program's
       arena peak fell 4,194,304 -> 3,670,032 bytes for +0.1925% of its
       instructions. Holding the doubling back only to eight gave the same
       peak but moved every longer list onto 8, 32, 128, which overshoots a
       thousand elements by twice as much and doubled the book's counters
       sample's permanent peak. */
    /* An accumulator's storage goes outside the arena, so the loop's rewind
       reclaims the iteration's garbage without reaching what the iteration
       was building. A transient's stays in the arena, where the rewind is
       exactly what should free it. */
    int perm = mutate && k_outlives_beat(l);
    long long cap = 4;
    while (cap < (l->len + 1)) cap <<= 1;
    /* The extra doubling pays for itself in the arena, where a grow is a
       fresh buffer and a copy. A buffer outside it grows by realloc, so it
       takes the smallest power of two: a 4,300-element accumulator held
       16,384 slots and now holds 8,192, and the run program's permanent
       peak halved for +37,769 instructions of extra regrows. */
    if (cap > 16 && !perm) cap <<= 1;
    if (perm && cap < 256) cap = 256;
    /* A buffer already out of the arena is grown where it is. The field it
       sits in was registered when it first left, and `l->items` is the same
       field after the realloc, so the registration still names it. */
    if (perm && k_buf_malloced(k_buf_of(l->items))) {
        KBuf* nb = k_buf_perm_regrow(k_buf_of(l->items), cap);
        KValue* grown = (KValue*)(nb + 1);
        grown[l->len] = item;
        nb->used = l->len + 1;
        l->items = grown;
        l->len++;
        return lv;
    }
    KValue* items = perm ? k_buf_perm(cap) : k_buf(cap);
    /* A list outgrows its literal's one slot far more often than it outgrows
       anything larger, and glibc's memcpy costs twenty-five instructions to
       move sixteen bytes. The same split k_rec makes. */
    if (l->len <= 4) {
        for (long long i = 0; i < l->len; i++) items[i] = l->items[i];
    } else {
        k_copy_cold(items, l->items, sizeof(KValue) * l->len);
    }
    items[l->len] = item;
    k_buf_of(items)->used = l->len + 1;
    if (mutate) {
        /* uniqueness is proven at mut sites, so the outgrown buffer has no
           other holder: an arena buffer goes to the shelf for the next
           collection, and one of ours is released outright. */
        if (k_buf_malloced(k_buf_of(l->items))) k_buf_release(k_buf_of(l->items));
        else k_buf_donate(l->items);
        l->items = items;
        l->len++;
        if (perm) k_permreg_add(l, &l->items);
        return lv;
    }
    KList* out = k_alloc(sizeof(KList));
    out->len = l->len + 1;
    out->items = items;
    KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
}

KValue k_b_push(KValue lv, KValue item) { return k_b_push_into(lv, item, 0); }

/* In-place push, emitted only where the linearity analysis proved the list is
   uniquely owned. On the frontier it mutates the header — no per-element
   allocation — which is the whole win; off the frontier it grows like a normal
   push (a uniquely-owned list is never off-frontier unless it just grew).

   THE BORN-THIS-BEAT TEST DECIDES A COUNTER AND NOTHING ELSE, which is why it
   is inside the `k_stats_on` guard. It used to stand in the fast arm's
   condition, and a call that failed it went to k_b_push_into_proven with
   mutate=1 and proven=1 — where the born line is skipped for being proven, the
   same frontier slot is claimed, the same header is incremented, and the same
   `lv` comes back. Two paths, one behaviour, and the second one paid for a
   call plus a beat-stack lookup plus, off the head block, the walk that
   settles tenure exactly. escapebench took that path 1,200,000 times a run
   against 3,000 on the fast arm.

   What the counters mean is unchanged: `push_mut_fast` still counts in-place
   writes onto a header born inside this beat and `push_mut_slow` still counts
   everything else, so all nine cost goldens are byte-identical across this
   change. They cost nothing when nobody is counting. */
KValue k_b_push_mut(KValue lv, KValue item) {
    if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(item), 0)) return k_failed2(lv, item);
    if (lv.tag != K_LIST) k_die_push_takes(lv);
    KList* l = k_as_list(lv);
    item = k_big_kept(item, l);
    KBuf* buf = k_buf_of(l->items);
    if (buf->used == l->len && l->len < k_buf_cap(buf)) {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
            if (k_born_this_beat(l)) k_stat_push_mut_fast++;
            else k_stat_push_mut_slow++;
        }
        l->items[l->len] = item;
        k_note_if_carried(item);
        buf->used++;
        l->len++;
        return lv;
    }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_push_mut_slow++;
    return k_b_push_grow(lv, l, item, 1);
}

/* In-place concat, emitted only where the linearity analysis proved the first
   list is uniquely owned. Each element of the second goes onto the first's
   frontier through the in-place push, which already knows how to grow, keep a
   bignum and register permanent storage. A fold written `concat acc chunk`
   built a fresh list every lap and held them all until the fold returned:
   4,000 laps of 25 ints peaked at 3 GB (kv's port, #1892). The analysis never
   proves `concat acc acc`, so the second list is not the one being grown. A
   first argument that is bytes keeps the copying concat. */
KValue k_b_concat_mut(KValue av, KValue bv) {
    if (__builtin_expect(!k_not_failure(av) || !k_not_failure(bv), 0)) return k_failed2(av, bv);
    if (av.tag != K_LIST) return k_b_concat(av, bv);
    long long blen = k_seq_len(bv);
    if (bv.tag == K_LIST) {
        KValue* items = k_as_list(bv)->items;
        for (long long i = 0; i < blen; i++) av = k_b_push_mut(av, items[i]);
        return av;
    }
    KBytes* b = k_as_bytes(bv);
    for (long long i = 0; i < blen; i++) av = k_b_push_mut(av, k_int(b->data[i]));
    return av;
}

/* A lazy sequence refused by a structural operation: the reader is one call
   from what they wanted, and every one of these messages used to stop at what
   it would not take. Answers NULL for anything that is not one. */
static const char* k_lazy_hint(KValue v) {
    if (v.tag != K_REC) return NULL;
    const char* ty = k_type_name(k_as_rec(v)->type_id);
    if (!ty) return NULL;
    /* std/list's adapter types, at whatever depth the import graph
       qualified them: "list/mapped" directly, "mid/list/mapped" one hop in */
    if (strncmp(ty, "list/", 5) != 0 && strstr(ty, "/list/") == NULL) return NULL;
    return " — a lazy sequence becomes a list with list/to_list";
}

/* How many characters a string holds. Counting codepoints is a walk of the
   whole string, and `length` sits in the guard of every loop a kanso program
   writes over text — `at > length s` — so counting it each time makes an
   ordinary character walk quadratic. Reading 128 kb one character at a time
   took 8.2 seconds before this.

   The count rides in `cap`, which an ordinary string does not use, encoded
   negative so nothing that asks `cap > 0` about a builder is confused. The
   header stays sixteen bytes: widening it would move the survivor ratio the
   cohort guard reads. A builder may still grow, so it counts every time. */
/* The remembered position lives beside the arena, above `k_beat_rewind`. */

/* How many characters a run of valid utf-8 holds. A character's first byte
   is any byte whose top two bits are not `10`, so the count is the byte
   length minus the number of continuation bytes, and no byte needs to be
   decoded to know which it is.

   Eight bytes at a time, because the per-character walk this replaced took a
   branch for every byte and `length` on a document is the shape that pays
   for it: encodebench asks for the length of what it just encoded four
   hundred times, and that one call was 6.8% of the benchmark.

   The word is read with memcpy so an unaligned load is the compiler's
   problem, and byte order does not matter — every byte is tested in place
   and the answer is a population count, which is order-blind. */
/* The same count for a text shorter than 64 bytes, which the vector block
   below never reaches. It is a function of its own so that the scan built on
   it saves only the registers this loop needs; see k_str_chars_scan. */
static inline __attribute__((always_inline)) long long k_utf8_chars_short(
    const unsigned char* p, long long len) {
    long long i = 0;
    long long conts = 0;
    for (; i + 8 <= len; i += 8) {
        unsigned long long w;
        memcpy(&w, p + i, sizeof w);
        unsigned long long cont = w & ~(w << 1) & 0x8080808080808080ULL;
        conts += __builtin_popcountll(cont);
    }
#if defined(__clang__)
#pragma clang loop vectorize(disable) unroll(disable)
#endif
    for (; i < len; i++) conts += ((p[i] & 0xc0) == 0x80);
    return len - conts;
}

static long long k_utf8_chars(const unsigned char* p, long long len) {
    long long i = 0;
    long long conts = 0;
#if defined(__x86_64__)
    /* Sixteen bytes a compare: a continuation byte is 0x80..0xBF, which as a
       signed byte is below -64. Each lane counts in a byte for up to 63
       blocks of 64 before the sum of absolute differences against zero folds
       the lanes into two halves. The word loop below read 8 bytes in about
       seven instructions; this reads 64 in about fourteen. */
    {
        const __m128i below = _mm_set1_epi8((char)0xC0);
        const __m128i zero = _mm_setzero_si128();
        while (i + 64 <= len) {
            __m128i acc = zero;
            long long stop = i + 64LL * 63;
            if (stop > len) stop = len;
            for (; i + 64 <= stop; i += 64) {
                __m128i a = _mm_loadu_si128((const __m128i*)(p + i));
                __m128i b = _mm_loadu_si128((const __m128i*)(p + i + 16));
                __m128i c = _mm_loadu_si128((const __m128i*)(p + i + 32));
                __m128i d = _mm_loadu_si128((const __m128i*)(p + i + 48));
                acc = _mm_sub_epi8(acc, _mm_cmplt_epi8(a, below));
                acc = _mm_sub_epi8(acc, _mm_cmplt_epi8(b, below));
                acc = _mm_sub_epi8(acc, _mm_cmplt_epi8(c, below));
                acc = _mm_sub_epi8(acc, _mm_cmplt_epi8(d, below));
            }
            __m128i sums = _mm_sad_epu8(acc, zero);
            conts += _mm_cvtsi128_si64(sums) + _mm_cvtsi128_si64(_mm_unpackhi_epi64(sums, sums));
        }
    }
#endif
    for (; i + 8 <= len; i += 8) {
        unsigned long long w;
        memcpy(&w, p + i, sizeof w);
        /* bit 7 set and bit 6 clear, per byte */
        unsigned long long cont = w & ~(w << 1) & 0x8080808080808080ULL;
        conts += __builtin_popcountll(cont);
    }
    /* The tail is at most seven bytes, so widening it is code the linker
       carries and the processor never runs: 560 bytes of vector prologue
       per copy of this function, against a loop that trips seven times. */
#if defined(__clang__)
#pragma clang loop vectorize(disable) unroll(disable)
#endif
    for (; i < len; i++) conts += ((p[i] & 0xc0) == 0x80);
    return len - conts;
}

/* The scan runs once per string and the callers that ask -- an index, a
   slice -- are the hot ones, so it saves the registers it touches and they
   keep no frame for it.

   On x86-64 there are two, and `k_str_chars` picks by length; elsewhere
   there is no vector block to keep out of the way, and only the short scan
   is built. With the vector block in
   the only scan, every call saved the block's registers and took the word
   loop's general entry, short texts included: kq's `length` calls are on
   2,376 strings averaging eight bytes, and they cost 12,284 more
   instructions once the block was added. */
#if defined(__x86_64__)
static __attribute__((noinline, preserve_most)) long long k_str_chars_scan_wide(KStr* s) {
    if (K_COUNTING) k_stat_str_scans++;
    if (K_COUNTING) k_stat_str_scan_bytes += s->len;
    long long count = k_utf8_chars((const unsigned char*)s->data, s->len);
    if (s->cap == 0 && count < 2147483647LL) s->cap = (int)(-count - 1);
    return count;
}
#endif
static __attribute__((noinline, preserve_most)) long long k_str_chars_scan(KStr* s) {
    if (K_COUNTING) k_stat_str_scans++;
    if (K_COUNTING) k_stat_str_scan_bytes += s->len;
    long long count = k_utf8_chars_short((const unsigned char*)s->data, s->len);
    if (s->cap == 0 && count < 2147483647LL) s->cap = (int)(-count - 1);
    return count;
}
static long long k_str_chars(KStr* s) {
    if (s->cap < 0) return -(long long)s->cap - 1;
    if (s->cap > 0) return k_str_count(s);
#if defined(__x86_64__)
    if (s->len >= 64) return k_str_chars_scan_wide(s);
#endif
    return k_str_chars_scan(s);
}

/* The byte at which character `from` begins, or -1 when the text holds fewer
   characters than that. Answers the two questions `k_b_slice` learned to ask
   first, for the reader who asked for one character rather than a run.

   Reading a string one character at a time is the shape this exists for.
   Starting the walk at byte zero every time made `s[i]` in a loop quadratic:
   a string of two thousand characters cost 27,591,142 instructions and one of
   four thousand cost 106,971,148 — a doubling of the input for 3.88 times the
   work.

   The cursor is the one `k_b_slice` keeps, deliberately: a sweep that mixes
   `s[i]` and `text/slice` over the same subject stays linear because both
   hands move it. `k_b_slice` keeps its own combined walk rather than calling
   this, because it needs both ends of a run out of one pass and would
   otherwise cross the text twice. */
static long k_str_seek(KStr* s, long long from) {
    /* Every character one byte, so a position is an offset. `k_str_chars`
       caches its answer in the header, so this costs one scan per string. */
    if (k_str_chars(s) == (long long)s->len) {
        return from > (long long)s->len ? -1 : (long)(from - 1);
    }
    long at = 0;
    long long seen = 1;
    if (s == k_seek_str && s->cap != 0 && k_seek_byte < s->len) {
        if (K_COUNTING) k_stat_seek_resumes++;
        /* A walk by index asks for the character after the one it just
           read, at every step: the cursor's own character or the next
           answers without the general walk below and its bookkeeping. */
        if (from == k_seek_char) return k_seek_byte;
        if (from == k_seek_char + 1) {
            long nat = k_seek_byte + k_cp_len((unsigned char)s->data[k_seek_byte]);
            if (nat >= s->len) return -1;
            k_seek_char = from;
            k_seek_byte = nat;
            return nat;
        }
        at = k_seek_byte;
        seen = k_seek_char;
        /* Behind the cursor, step back to it rather than to the front: the
           same walk with the continuation bytes skipped, so the price is the
           distance moved rather than the position reached. */
        while (from < seen && at > 0) {
            at--;
            while (at > 0 && ((unsigned char)s->data[at] & 0xC0) == 0x80) at--;
            seen--;
        }
    }
    while (at < s->len) {
        if (seen == from) {
            k_seek_note(s);
            k_seek_char = from;
            k_seek_byte = at;
            return at;
        }
        at += k_cp_len((unsigned char)s->data[at]);
        seen++;
    }
    return -1;
}

KValue k_b_length(KValue v) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_LIST) return k_int(k_as_list(v)->len);
    if (v.tag == K_BYTES) return k_int(k_as_bytes(v)->len);
    if (v.tag == K_MAP) {
        /* the view's count and its side's, without merging the two */
        KMap* m = k_as_map(v);
        if ((uintptr_t)m->sorted & K_VIEW_SIDE)
            return k_int(m->sorted_len + *k_view_word(k_view_side(k_view_untagged(m->sorted))));
        long long n;
        k_map_sorted(m, &n);
        return k_int(n);
    }
    if (v.tag == K_STR) return k_int(k_str_chars(k_as_str(v)));
    /* A lazy sequence has no length to count — `naturals` would never
       finish — so refusing it is right. But the reader who wrote
       `length (map …)` is one call from what they wanted, and the message
       may as well say which. Built by hand because k_die_value puts the
       value last and the hint belongs after it. */
    if (v.tag == K_REC) {
        const char* ty = k_type_name(k_as_rec(v)->type_id);
        if (ty && (strncmp(ty, "list/", 5) == 0 || strstr(ty, "/list/") != NULL)) {
            KValue shown = k_render(v, 1);
            KStr* s = k_as_str(shown);
            char said[1024];
            snprintf(said, sizeof said,
                     "length takes a list, string, or map, not %.*s — a lazy "
                     "sequence becomes a list with list/to_list",
                     (int)s->len, s->data);
            k_die(said);
        }
    }
    k_die_value("length takes a list, string, or map", v);
    return k_none();
}

/* Sixteen bytes read from p stay inside p's page, so a vector load that runs
   past the end of a short string cannot fault: the page holding a valid byte
   is mapped whole. The two scanners below use it for a tail shorter than a
   vector, which they used to walk one byte at a time. On the run program
   8,042,860 bytes a run went through those byte loops at eight instructions
   each, because a json key or value is usually shorter than sixteen bytes
   and the vector loop never opened; one load and a mask over the bytes that
   are the string's answers the same question. The bytes past the end are
   loaded and then masked off, so nothing the answer depends on is read from
   outside the string. */
static inline int k_tail_window(const unsigned char* p) {
    return ((uintptr_t)p & 4095) <= 4096 - 16;
}

/* Scan for the first of two bytes at or after a 1-based position — the string
   scanner's inner loop, done as a tight pass instead of one boxed dispatch per
   byte. Returns the 1-based hit, or len+1 when neither byte appears. */
/* The scan, with the KValue convention taken off it. Every argument fits in a
   register here, where four KValues do not: three of them fill the six the
   SysV ABI has and the fourth arrives as a pointer to the stack, which the
   caller has to store and the callee has to load and unpack before it can
   splat the byte. That unpacking was fourteen of this function's fifty-four
   instructions on jsonbench and the scan itself was ten. The emitter reaches
   this door directly through the `k_b_find2_fast` shim; `k_b_find2` below is
   the same call for anything the shim's tag test turns away.

   always_inline, because the link is LTO and every emitted caller hands the
   two bytes as literals: inlined, the two broadcasts fold to constant vectors
   loaded from rodata, where the out-of-line door rebuilt them from registers
   on each of its 1,756,429 calls a run, and the hit was in the first sixteen
   bytes 99.99% of the time, so the setup was half the call. The scalar tail
   and the loop are small enough that eight copies cost the .text vein less
   than the calls cost the work vein. */
#ifndef K_HOT_ELSEWHERE
__attribute__((always_inline)) long long k_b_find2_raw(const unsigned char* d, long long len, long long from,
                        long long a, long long b) {
    if (K_COUNTING) k_stat_find2_calls++;
    long long p = from < 1 ? 0 : from - 1;
    unsigned char ca = (unsigned char)(a & 0xff);
    unsigned char cb = (unsigned char)(b & 0xff);
    long long i = p;
#if defined(__aarch64__)
    /* 16 bytes per step; the shrn-by-4 narrow turns the match vector into a
       64-bit mask (4 bits per byte), so ctz/4 names the first hit. */
    uint8x16_t va = vdupq_n_u8(ca), vb = vdupq_n_u8(cb);
    for (; i + 16 <= len; i += 16) {
        uint8x16_t chunk = vld1q_u8(d + i);
        uint8x16_t m = vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb));
        uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
        uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
        if (mask) return (i + (__builtin_ctzll(mask) >> 2) + 1);
    }
    if (i < len && k_tail_window(d + i)) {
        uint8x16_t chunk = vld1q_u8(d + i);
        uint8x16_t m = vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb));
        uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
        uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
        mask &= (1ULL << (4 * (len - i))) - 1;
        return mask ? (i + (__builtin_ctzll(mask) >> 2) + 1) : (len + 1);
    }
#elif defined(__x86_64__)
    __m128i va = _mm_set1_epi8((char)ca), vb = _mm_set1_epi8((char)cb);
    for (; i + 16 <= len; i += 16) {
        __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
        __m128i m = _mm_or_si128(_mm_cmpeq_epi8(chunk, va), _mm_cmpeq_epi8(chunk, vb));
        int mask = _mm_movemask_epi8(m);
        if (mask) return (i + __builtin_ctz(mask) + 1);
    }
    if (i < len && k_tail_window(d + i)) {
        __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
        __m128i m = _mm_or_si128(_mm_cmpeq_epi8(chunk, va), _mm_cmpeq_epi8(chunk, vb));
        int mask = _mm_movemask_epi8(m) & ((1 << (len - i)) - 1);
        return mask ? (i + __builtin_ctz(mask) + 1) : (len + 1);
    }
#endif
    for (; i < len; i++) {
        if (d[i] == ca || d[i] == cb) return (i + 1);
    }
    return len + 1;
}
#else
long long k_b_find2_raw(const unsigned char* d, long long len, long long from,
                        long long a, long long b);
#endif

KValue k_b_find2(KValue cs, KValue from, KValue a, KValue b) {
    if (!k_not_failure(cs)) { if (K_COUNTING) k_stat_find2_calls++; return cs; }
    if (!k_not_failure(from)) { if (K_COUNTING) k_stat_find2_calls++; return from; }
    if (!k_not_failure(a) || !k_not_failure(b)) {
        if (K_COUNTING) k_stat_find2_calls++;
        return k_both_or_either(a, b);
    }
    if (cs.tag != K_BYTES) { if (K_COUNTING) k_stat_find2_calls++; k_die("find2 takes bytes"); }
    KBytes* by = k_as_bytes(cs);
    return k_int(k_b_find2_raw(by->data, by->len, from.payload, a.payload,
                               b.payload));
}

/* Where a JSON number's bytes end. The answer is the first position at or
   after `from` whose byte cannot be part of a number -- anything but a digit,
   `+`, `-`, `.`, `e` or `E` -- and it comes back negated when a `.`, `e` or
   `E` went past, which is what makes the number a float. A position outside
   the bytes is its own answer, as the byte walk it replaces would stop there.
   lib/json walked a number one byte a step, a dispatch on every byte, and it
   cost the run program 23 instructions a character; sixteen bytes are
   classified here at once. */
static int k_number_byte(unsigned char c) {
    return (unsigned)(c - '0') < 10 || c == '+' || c == '-' || c == '.' || c == 'e' || c == 'E';
}

KValue k_b_number_span(KValue cs, KValue fromv) {
    if (K_COUNTING) k_stat_number_spans++;
    if (__builtin_expect(!k_not_failure(cs) || !k_not_failure(fromv), 0)) return k_failed2(cs, fromv);
    if (cs.tag != K_BYTES || !K_IS_INT(fromv)) k_die("number_span takes bytes and a position");
    if (fromv.tag == K_BIG) return fromv;
    KBytes* by = k_as_bytes(cs);
    const unsigned char* d = by->data;
    long long len = by->len;
    long long from = fromv.payload;
    if (from < 1 || from > len) return fromv;
    long long i = from - 1;
    int mark = 0;
#if defined(__x86_64__)
    const __m128i zero = _mm_set1_epi8('0');
    const __m128i nine = _mm_set1_epi8(9);
    while (i + 16 <= len) {
        __m128i v = _mm_loadu_si128((const __m128i*)(d + i));
        __m128i off = _mm_sub_epi8(v, zero);
        __m128i digit = _mm_cmpeq_epi8(_mm_min_epu8(off, nine), off);
        __m128i point = _mm_or_si128(
            _mm_cmpeq_epi8(v, _mm_set1_epi8('.')),
            _mm_or_si128(_mm_cmpeq_epi8(v, _mm_set1_epi8('e')),
                         _mm_cmpeq_epi8(v, _mm_set1_epi8('E'))));
        __m128i sign = _mm_or_si128(_mm_cmpeq_epi8(v, _mm_set1_epi8('+')),
                                    _mm_cmpeq_epi8(v, _mm_set1_epi8('-')));
        unsigned in = (unsigned)_mm_movemask_epi8(_mm_or_si128(digit, _mm_or_si128(point, sign)));
        unsigned marks = (unsigned)_mm_movemask_epi8(point);
        unsigned out = ~in & 0xffffu;
        if (out) {
            unsigned k = (unsigned)__builtin_ctz(out);
            if (marks & ((1u << k) - 1)) mark = 1;
            i += k;
            return k_int(mark ? -(i + 1) : i + 1);
        }
        if (marks) mark = 1;
        i += 16;
    }
#endif
    while (i < len && k_number_byte(d[i])) {
        if (d[i] == '.' || d[i] == 'e' || d[i] == 'E') mark = 1;
        i++;
    }
    return k_int(mark ? -(i + 1) : i + 1);
}


/* The byte builder. Appends a string, a bytes value, or a single byte
   onto a bytes accumulator. The accumulator owns a KBuf-headed buffer and
   claims its frontier exactly as list push does, so a fold of appends is
   amortized linear while every intermediate value stays a real value. */
static KValue k_bytes_owned(long long len, const unsigned char* data, long long cap) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_sh_bytes += (long long)sizeof(KBytes);
    KBytes* b = k_alloc(sizeof(KBytes));
    b->len = len;
    b->data = data;
    b->cap = cap;
    KValue v; v.tag = K_BYTES; v.payload = k_ptr(b); return v;
}

static __attribute__((noinline)) KValue k_b_append_grow(KValue acc, KBytes* a,
                                                        const unsigned char* src,
                                                        long long n, int mutate);

static __attribute__((noinline)) KValue k_b_append_wide(KValue acc, KBytes* a,
                                                        KValue x, int mutate);
static inline __attribute__((always_inline)) KValue k_b_append_range(KValue acc, KBytes* a,
                                                                     const unsigned char* src,
                                                                     long long n, int mutate);

/* A comma, a colon, a brace: three quarters of the appends the encoder makes
   are one byte into spare capacity, and this is the whole of that case.

   The general path below reaches its store through `src`, a pointer that is a
   phi of three predecessors, so a byte arriving in a register was written to a
   stack slot and read back — and the frame that slot needs is built by every
   append, of any shape, before anything is tested. Split, the byte case is a
   leaf: no frame, no callee-saved registers, and `a->len` read once into a
   local rather than three times, because a store through `unsigned char*`
   aliases every field of the header it is stored into. */
static KValue k_b_append_into(KValue acc, KValue x, int mutate) {
    if (__builtin_expect(!k_not_failure(acc) || !k_not_failure(x), 0)) return k_failed2(acc, x);
    if (acc.tag != K_BYTES) k_die("append takes bytes and a string, bytes, or byte");
    KBytes* a = k_as_bytes(acc);
    if (x.tag == K_INT) {
        long long alen = a->len;
        long long acap0 = a->cap;
        long long bcap = acap0 & ~1LL;
        unsigned char* data = (unsigned char*)a->data;
        if (bcap) {
            KBuf* bbuf = ((KBuf*)data) - 1;
            if (bbuf->used == alen && alen + 1 <= bcap) {
                if (K_COUNTING) k_stat_append_fast++;
                data[alen] = (unsigned char)(x.payload & 0xff);
                bbuf->used = alen + 1;
                if (mutate) {
                    a->len = alen + 1;
                    return acc;
                }
                return k_bytes_owned(alen + 1, data, acap0);
            }
        }
    }
    return k_b_append_wide(acc, a, x, mutate);
}

/* Everything the byte case above is not: a string, a byte string, and the byte
   that did not fit. Out of line so the common append stops paying its frame. */
static __attribute__((noinline)) KValue k_b_append_wide(KValue acc, KBytes* a,
                                                        KValue x, int mutate) {
    const unsigned char* src;
    long long n;
    unsigned char one;
    if (x.tag == K_STR) {
        KStr* s = k_as_str(x);
        src = (const unsigned char*)s->data;
        n = s->len;
    } else if (x.tag == K_BYTES) {
        KBytes* b = k_as_bytes(x);
        src = b->data;
        n = b->len;
    } else if (x.tag == K_INT) {
        one = (unsigned char)(x.payload & 0xff);
        src = &one;
        n = 1;
    } else if (x.tag == K_BIG) {
        /* the magnitude's low byte, as the interpreter takes it */
        KBigView big;
        k_big_view(x, &big);
        one = (unsigned char)(big.d[0] & 0xff);
        src = &one;
        n = 1;
    } else {
        k_die("append takes bytes and a string, bytes, or byte");
        return k_none();
    }
    return k_b_append_range(acc, a, src, n, mutate);
}

/* The copy itself, once the source is a pointer and a length. `append_wide`
   reaches it after unpacking a value; `append_slice` reaches it with a range
   of a bytes value it never boxed. */
static KValue k_b_append_fit(KValue acc, KBytes* a, const unsigned char* src,
                             long long n, int mutate);

/* A bytes value with no buffer of its own -- the empty accumulator an
   encoder starts from -- always grows, and asking that first, inline at each
   caller, keeps the grow off `k_b_append_fit`'s frame. With the two in one
   function, every such append pushed five registers, tested the capacity and
   popped them again on its way to the grow: 142,731 of them a run, fourteen
   instructions apiece, on the run program. */
static inline __attribute__((always_inline)) KValue k_b_append_range(KValue acc, KBytes* a,
                                                                     const unsigned char* src,
                                                                     long long n, int mutate) {
    if (!(a->cap & ~1LL)) return k_b_append_grow(acc, a, src, n, mutate);
    return k_b_append_fit(acc, a, src, n, mutate);
}

static __attribute__((noinline)) KValue k_b_append_fit(KValue acc, KBytes* a, const unsigned char* src,
                                                      long long n, int mutate) {
    long long acap = a->cap & ~1LL;
    if (acap) {
        KBuf* buf = ((KBuf*)a->data) - 1;
        if (buf->used == a->len && a->len + n <= acap) {
            if (K_COUNTING) k_stat_append_fast++;
            /* A comma, a colon, a brace: the encoder's commonest append is one
               byte, and a call into memcpy to move it costs more than the
               move. */
            if (n == 1) ((unsigned char*)a->data)[a->len] = *src;
            else memcpy((unsigned char*)a->data + a->len, src, (size_t)n);
            buf->used = a->len + n;
            /* The bytes went into the accumulator's own spare capacity, so a
               fresh header would differ only in its length. Where uniqueness
               is proven, write that length in place: the header was the
               largest single source of garbage the encoder made. */
            if (mutate) {
                a->len += n;
                return acc;
            }
            return k_bytes_owned(a->len + n, a->data, a->cap);
        }
    }
    return k_b_append_grow(acc, a, src, n, mutate);
}

/* The growth, out of line, so the fast path stops paying for it.

   One function used to hold both. The fast path — two failure tests, a tag
   dispatch, a capacity comparison and a store — needs two registers; the
   growth needs eight, because it decides which allocator the new buffer comes
   from, copies twice and frees the old header. So every append pushed six
   callee-saved registers and built a frame before it could test anything.
   Split, the common path pushes three, and the rare one pays a call. */
/* A grow's malloc regime and the free of a malloced predecessor are its
   rare arms -- 1,080 and 990 of 176,697 grows on the run program -- and
   both were plain calls the hot arm kept a frame for. */
static __attribute__((noinline, cold, preserve_most)) KBuf* k_bytes_buf_malloc(long long cap) {
    KBuf* buf = malloc(sizeof(KBuf) + (size_t)cap);
    if (!buf) { fputs("out of memory\n", stderr); exit(1); }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += (long long)(sizeof(KBuf) + (size_t)cap);
        if (K_COUNTING) k_stat_bytes_malloc++;
        k_stat_held_live += (long long)(sizeof(KBuf) + (size_t)cap);
        if (k_stat_held_live > k_stat_held_peak) k_stat_held_peak = k_stat_held_live;
    }
    return buf;
}
/* The unique builder's grow, where the old buffer has no other holder: one
   realloc in place of a malloc, a copy and a free. glibc extends the block
   where it can and, past its mmap threshold, remaps the pages rather than
   copying them; and the old and new buffers are never both held, which is
   the moment the run program's `held_peak_bytes` was taken. Counted as the
   malloc and the free it replaces, so the counters keep their meaning. */
static __attribute__((noinline, cold, preserve_most)) KBuf* k_bytes_buf_regrow(KBuf* old,
                                                                              long long cap) {
    long long was = (long long)sizeof(KBuf) + k_buf_cap(old);
    KBuf* buf = realloc(old, sizeof(KBuf) + (size_t)cap);
    if (!buf) { fputs("out of memory\n", stderr); exit(1); }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += (long long)(sizeof(KBuf) + (size_t)cap);
        if (K_COUNTING) k_stat_bytes_malloc++;
        if (K_COUNTING) k_stat_bytes_freed++;
        k_stat_held_live += (long long)(sizeof(KBuf) + (size_t)cap) - was;
        if (k_stat_held_live > k_stat_held_peak) k_stat_held_peak = k_stat_held_live;
    }
    return buf;
}
static __attribute__((noinline, cold, preserve_most)) void k_bytes_buf_release(KBuf* old) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_bytes_freed++;
        k_stat_held_live -= (long long)sizeof(KBuf) + k_buf_cap(old);
    }
    free(old);
}

static __attribute__((noinline)) KValue k_b_append_grow(KValue acc, KBytes* a,
                                                        const unsigned char* src,
                                                        long long n, int mutate) {
    if (K_COUNTING) k_stat_append_grow++;
    long long cap = 2 * (a->len + n);
    if (cap < 64) cap = 64;
    /* Where the grown buffer lives depends on where its header dies. A
       header the innermost rewind reclaims gets an arena buffer (negative
       cap marks the regime): the rewind frees it wholesale, and the carry
       copy-out already duplicates any bytes that must outlive it. A header
       below the mark — the licensed accumulator threading a beat — gets
       malloc, because arena storage above the mark would vanish under it.
       A fresh header always lands at the frontier, so the non-mut result
       dies with the frame whenever a beat is active. */
    int dies = 0;
    if (k_beat_depth > 0 && k_beat_depth <= K_BEAT_MAX) {
        KMark* inner = &k_beat_stack[k_beat_depth - 1];
        dies = mutate ? k_above_mark(a, inner) : 1;
    }
    /* Where the buffer comes from, and the sign the new cap carries. That is
       the whole of the difference between the two regimes, so it is decided
       here and the copy, the free and the two returns are written once
       below. A builder's buffer lives outside the arena: growth doubles, and
       an allocator that reclaims nothing pays for every intermediate size a
       builder passes through rather than the size it reached, where malloc
       plus a free on the owned path pays for one buffer at a time. */
    /* A malloc'd builder past 16 kb grows by half rather than doubling. Its
       buffer is what the run program's `held_peak_bytes` measures, and a
       doubled buffer is on average a quarter empty when the builder
       finishes: the 90 encodes of large.json each ended in a 277,522-byte
       buffer. Below 16 kb the extra grows cost more than the room saves,
       and the arena regime keeps doubling, since the arena reclaims none of
       the sizes a builder passes through. The cap stays even, which is how
       a malloc'd buffer is told from an arena one. */
    if (!dies && a->len + n > 16384) cap = ((a->len + n) * 3 / 2) & ~1LL;
    if (mutate && !dies && k_bytes_malloced(a)) {
        KBuf* grown = k_bytes_buf_regrow(((KBuf*)a->data) - 1, cap);
        k_buf_set_cap(grown, cap, 0);
        grown->used = a->len + n;
        unsigned char* at = (unsigned char*)(grown + 1);
        k_copy_short((char*)at + a->len, (const char*)src, n);
        a->len += n;
        a->data = at;
        a->cap = cap;
        return acc;
    }
    KBuf* buf;
    long long marked;
    if (dies) {
        buf = k_alloc(sizeof(KBuf) + (size_t)cap);
        marked = cap | 1;
    } else {
        buf = k_bytes_buf_malloc(cap);
        marked = cap;
    }
    k_buf_set_cap(buf, cap, 0);
    buf->used = a->len + n;
    unsigned char* data = (unsigned char*)(buf + 1);
    /* The first append into `text/bytes ""` grows from nothing: 176,697 of
       runbench's grows carry no bytes forward, and glibc's memcpy spent its
       dispatch deciding how to move zero of them. The run being appended is
       a token's worth, so it takes the short ladder. */
    if (a->len) k_copy_cold(data, a->data, (size_t)a->len);
    k_copy_short((char*)data + a->len, (const char*)src, n);
    if (mutate) {
        /* Only a buffer made here, from malloc, carries a clear bit 0 over a
           non-zero cap, which is what makes this free safe: uniqueness is
           proven at mut sites, so no other header shares the storage being
           released. */
        if (k_bytes_malloced(a)) k_bytes_buf_release(((KBuf*)a->data) - 1);
        a->len += n;
        a->data = data;
        a->cap = marked;
        return acc;
    }
    return k_bytes_owned(a->len + n, data, marked);
}

KValue k_b_append(KValue acc, KValue x) { return k_b_append_into(acc, x, 0); }

/* The same append at a site the linearity analysis proved unique. */
KValue k_b_append_mut(KValue acc, KValue x) { return k_b_append_into(acc, x, 1); }

/* The arm `k_b_append_mut_word` refuses: a counting build, or a builder
   without eight bytes of room at its frontier. It builds the literal and
   appends it in place, out of line so the word's fast path stays small where
   it is inlined.

   In place, and not through `k_b_append` as the string arm of
   `k_b_append_mut_byte` does. That arm refuses only a builder with no room for
   the literal, and `k_b_append` then grows it. This one also refuses a
   builder with room for the literal but not for a whole word, and there
   `k_b_append` claims the room and hands back a new header in the arena. A
   builder a beat loop carries by identity keeps the old one, the loop's
   rewind reclaims the new one, and the next append read a header the arena
   had handed to something else: encodebench's counting build crashed on it. */
__attribute__((noinline, cold)) KValue k_b_append_word_slow(KValue acc, const char* lit,
                                                           long long n, KValue* cell) {
    return k_b_append_mut(acc, k_str_lit(lit, n, cell));
}

/* `append acc (slice cs from to)` where both sides are bytes. The slice is
   built to be copied and dropped, and a view is a header the arena has to
   hand out; this reads the range straight out of `cs` and hands
   k_b_append_into a pointer and a length, so nothing is allocated for it.

   An out-of-range or inverted range is the empty slice, which appends
   nothing, and `acc` still has to be bytes for that to be legal. Anything the
   tags turn away goes the long way. A range past the end is none since #1890,
   so a program cannot hand one to `append`: this door is written only for the
   standard library's own ranges, which come from a scan of the same bytes. */
KValue k_b_append_slice(KValue acc, KValue cs, KValue fromv, KValue tov,
                        long long mutate) {
    if (__builtin_expect(!k_not_failure(acc) || !k_not_failure(cs) || !k_not_failure(fromv)
                         || !k_not_failure(tov), 0)) {
        KValue xs[4] = {acc, cs, fromv, tov};
        return k_failed_of(4, xs);
    }
    if (cs.tag != K_BYTES || fromv.tag != K_INT || tov.tag != K_INT) {
        return k_b_append_into(acc, k_b_slice(cs, fromv, tov), (int)mutate);
    }
    if (acc.tag != K_BYTES) k_die("append takes bytes and a string, bytes, or byte");
    KBytes* a = k_as_bytes(acc);
    KBytes* src = k_as_bytes(cs);
    long long from = fromv.payload, to = tov.payload;
    /* The empty slice appends nothing, and the view it would have been is not
       built: a fifth of the runs between two escapes are empty, and a header
       allocated to carry zero bytes is 204,450 allocations a decode. */
    if (!k_span_in(from, to, src->len)) {
        return k_b_append_range(acc, a, src->data, 0, (int)mutate);
    }
    return k_b_append_range(acc, a, src->data + (from - 1), to - from + 1,
                            (int)mutate);
}

/* `append acc "{x}"` where `x` is an int or a float. The template rendered
   `x` into a KStr and the append copied that string into `acc`, so every
   scalar the JSON encoder wrote was built once to be copied once: 379,530
   strings a runbench, 5.5% of its instructions in the render alone. This
   writes the digits into a stack buffer and hands k_b_append_range the
   pointer and the length, so nothing is allocated for them. Any other
   value takes the road it always took, through k_render, so the two
   spellings cannot disagree on a byte. */
K_DOORCC KValue k_b_append_rendered(KValue acc, KValue v, long long mutate) {
    if (!k_not_failure(acc)) return acc;
    if (v.tag != K_INT && v.tag != K_FLOAT) {
        return k_b_append_into(acc, k_render(v, 0), (int)mutate);
    }
    if (acc.tag != K_BYTES) k_die("append takes bytes and a string, bytes, or byte");
    char buf[64];
    long long n = k_render_number(v, buf);
    if (K_COUNTING) k_stat_append_rendered++;
    KBytes* a = k_as_bytes(acc);
    long long acap = a->cap & ~1LL;
    if (acap) {
        KBuf* kb = ((KBuf*)a->data) - 1;
        /* A number is at most 24 bytes (`-1.7976931348623157e308`), and buf
           holds 64, so where the accumulator has 24 to spare the copy is
           three words whatever the length, and the call into memcpy for a
           handful of digits is not made. */
        if (kb->used == a->len && a->len + 24 <= acap) {
            if (K_COUNTING) k_stat_append_fast++;
            memcpy((unsigned char*)a->data + a->len, buf, 24);
            kb->used = a->len + n;
            if (mutate) {
                a->len += n;
                return acc;
            }
            return k_bytes_owned(a->len + n, a->data, a->cap);
        }
    }
    return k_b_append_range(acc, a, (const unsigned char*)buf, n, (int)mutate);
}

/* find2 with a floor: also stops at the first byte below `lim`. Escape
   scanning wants this shape — quote, backslash, and control bytes all
   end a clean run, and one pass finds whichever comes first. */
/* The scan behind `find2_below`, with the KValue convention taken off its
   inputs. FIVE KValues are ten register-sized arguments where the SysV ABI has
   six, so four of them arrive on the caller's stack and the callee loads and
   unpacks each before it can splat a byte — the largest spill of the five
   doors the emitter declares over six registers. This takes a pointer, a
   length and four integers: six, and nothing spills. The emitter reaches it
   through the `k_b_find2_below_fast` shim, which tests the tags itself;
   `k_b_find2_below` below is the same scan for anything the shim turns away.

   always_inline for the reason k_b_find2_raw gives: the byte pair and the
   limit are literals at every emitted site, and the broadcasts fold. */
#ifndef K_HOT_ELSEWHERE
__attribute__((always_inline)) long long k_b_find2_below_raw(const unsigned char* d, long long len,
                              long long from, long long a, long long b,
                              long long floor_v) {
    /* The JSON writer's scan, over a shared token already known to hold none
       of the three. The decoder hands the writer the same few hundred keys on
       every document, and the run program scans 942,750 strings a run
       through this door. The byte triple is a literal at every emitted site,
       so the test folds away everywhere else. A view that starts inside a token reads the whole
       token's flag, which is safe: a clean token has no dirty sub-range.
       `find2_calls` is counted after this, so it counts scans, and a skip
       that stops firing shows there. */
    if (a == 34 && b == 92 && floor_v == 32) {
        uintptr_t off = (uintptr_t)((const char*)d - (const char*)k_token_store);
        if (off < sizeof(k_token_store) && k_token_clean[off / sizeof(KToken)]) return len + 1;
    }
    if (K_COUNTING) k_stat_find2_calls++;
    long long p = from < 1 ? 0 : from - 1;
    unsigned char ca = (unsigned char)(a & 0xff);
    unsigned char cb = (unsigned char)(b & 0xff);
    long long i = p;
    if (floor_v > 0 && floor_v <= 256) {
        unsigned char cl = (unsigned char)(floor_v - 1);
#if defined(__aarch64__)
        uint8x16_t va = vdupq_n_u8(ca), vb = vdupq_n_u8(cb), vl = vdupq_n_u8(cl);
        for (; i + 16 <= len; i += 16) {
            uint8x16_t chunk = vld1q_u8(d + i);
            uint8x16_t m = vorrq_u8(vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb)),
                                    vcleq_u8(chunk, vl));
            uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
            uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
            if (mask) return (i + (__builtin_ctzll(mask) >> 2) + 1);
        }
        if (i < len && k_tail_window(d + i)) {
            uint8x16_t chunk = vld1q_u8(d + i);
            uint8x16_t m = vorrq_u8(vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb)),
                                    vcleq_u8(chunk, vl));
            uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
            uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
            mask &= (1ULL << (4 * (len - i))) - 1;
            return mask ? (i + (__builtin_ctzll(mask) >> 2) + 1) : (len + 1);
        }
#elif defined(__x86_64__)
        __m128i va = _mm_set1_epi8((char)ca), vb = _mm_set1_epi8((char)cb);
        __m128i vl = _mm_set1_epi8((char)cl);
        for (; i + 16 <= len; i += 16) {
            __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
            __m128i low = _mm_cmpeq_epi8(_mm_min_epu8(chunk, vl), chunk);
            __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, va),
                                                  _mm_cmpeq_epi8(chunk, vb)), low);
            int mask = _mm_movemask_epi8(m);
            if (mask) return (i + __builtin_ctz(mask) + 1);
        }
        if (i < len && k_tail_window(d + i)) {
            __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
            __m128i low = _mm_cmpeq_epi8(_mm_min_epu8(chunk, vl), chunk);
            __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, va),
                                                  _mm_cmpeq_epi8(chunk, vb)), low);
            int mask = _mm_movemask_epi8(m) & ((1 << (len - i)) - 1);
            return mask ? (i + __builtin_ctz(mask) + 1) : (len + 1);
        }
#endif
    }
    for (; i < len; i++) {
        if (d[i] == ca || d[i] == cb || (long long)d[i] < floor_v) return (i + 1);
    }
    return (len + 1);
}
#else
long long k_b_find2_below_raw(const unsigned char* d, long long len,
                              long long from, long long a, long long b,
                              long long floor_v);
#endif

KValue k_b_find2_below(KValue cs, KValue from, KValue a, KValue b, KValue lim) {
    if (!k_not_failure(cs)) { if (K_COUNTING) k_stat_find2_calls++; return cs; }
    if (!k_not_failure(from)) { if (K_COUNTING) k_stat_find2_calls++; return from; }
    if (!k_not_failure(a) || !k_not_failure(b)) {
        if (K_COUNTING) k_stat_find2_calls++;
        return k_both_or_either(a, b);
    }
    if (!k_not_failure(lim)) { if (K_COUNTING) k_stat_find2_calls++; return lim; }
    if (cs.tag != K_BYTES) { if (K_COUNTING) k_stat_find2_calls++; k_die("find2_below takes bytes"); }
    KBytes* by = k_as_bytes(cs);
    return k_int(k_b_find2_below_raw(by->data, by->len, from.payload, a.payload,
                                     b.payload, lim.payload));
}

/* The bytes arm of `slice`, with the KValue convention taken off its inputs.
   A view is a header and two words, so the arithmetic here is four compares
   and an add; everything before it in `k_b_slice` is the boxing. The emitter
   reaches this door through the `k_b_slice_fast` shim, which tests the three
   tags itself; `k_b_slice` below is the same call for anything the shim turns
   away, and for the list and string containers, which do not come here. */
KValue k_b_slice_raw(const unsigned char* data, long long len, long long from,
                     long long to) {
    if (__builtin_expect(to > len || from > len + 1, 0)) return k_none();
    if (!k_span_in(from, to, len)) return k_bytes_view(data, 0);
    return k_bytes_view(data + (from - 1), to - from + 1);
}

/* Count characters a word at a time from `*at`, which is on a character
   boundary with `*seen` characters before it, and stop short of the
   character numbered `next`, so the caller's character loop still lands on
   it. A word's characters are its bytes less its continuation bytes; a
   character cut by the word's end is given back, and `*at` stays on a
   boundary. */
static void k_slice_skip(KStr* s, long* at, long long* seen, long long next) {
    while (*at + 8 <= s->len) {
        unsigned long long w;
        memcpy(&w, s->data + *at, sizeof w);
        unsigned long long cont = w & ~(w << 1) & 0x8080808080808080ULL;
        /* Only the byte-high bits can be set, so moving each to its byte's
           low bit and multiplying by 0x0101...01 sums the eight bytes into
           the top one: a shift, a multiply and a shift. The runtime is built
           for ssse3, which has no popcnt, and `__builtin_popcountll` became
           a dozen shifts, masks and adds. k_utf8_chars keeps the builtin:
           the same multiply there cost pendbench 1,642,235 instructions,
           for a reason not isolated. */
        long long leads = 8 - (long long)(((cont >> 7) * 0x0101010101010101ULL) >> 56);
        if (*seen + leads >= next) break;
        long nat = *at + 8;
        while (((unsigned char)s->data[nat] & 0xC0) == 0x80) nat--;
        if (nat == *at) break;
        if (nat < *at + 8) leads--;
        *seen += leads;
        *at = nat;
    }
}

KValue k_b_slice(KValue container, KValue fromv, KValue tov) {
    if (__builtin_expect(!k_not_failure(container) || !k_not_failure(fromv) || !k_not_failure(tov), 0)) return k_failed3(container, fromv, tov);
    if (!K_IS_INT(fromv) || !K_IS_INT(tov)) k_die("slice takes 1-based inclusive positions");
    /* A bound past a word reads as zero, which no slice starts or ends at,
       as the interpreter reads one that is no position at all. */
    long long from = fromv.tag == K_INT ? fromv.payload : 0, to = tov.tag == K_INT ? tov.payload : 0;
    if (container.tag == K_BYTES) {
        KBytes* b = k_as_bytes(container);
        return k_b_slice_raw(b->data, b->len, from, to);
    }
    if (container.tag == K_LIST) {
        KList* l = k_as_list(container);
        if (to > l->len || from > l->len + 1) return k_none();
        if (!k_span_in(from, to, l->len)) return k_mklist(0, NULL);
        return k_mklist(to - from + 1, l->items + (from - 1));
    }
    if (container.tag == K_STR) {
        KStr* s = k_as_str(container);
        /* A range that runs past the end, or starts past it, answers none,
           as an index past the end does (#1890). The count is the one the
           ascii test below asks for, cached on the string. */
        long long chars = k_str_chars(s);
        if (to > chars || from > chars + 1) return k_none();
        /* Every character one byte, so a position is an offset and the walk
           that finds it is unnecessary. This is the case for html, source and
           anything else a gate reads. */
        if (chars == (long long)s->len) {
            if (!k_span_in(from, to, (long long)s->len)) return k_str_n("", 0);
            /* One character first: a matcher slices one at a time, a million
               times a run on scanbench, and the ascii cache answers it. The
               length test below cost those four instructions apiece when it
               came first. */
            if (to == from) return k_str_n(s->data + (from - 1), 1);
            if (to - from + 1 >= K_STR_VIEW_MIN && s->cap <= 0)
                return k_str_view(s, from - 1, to - from + 1, to - from + 1);
            return k_str_n(s->data + (from - 1), to - from + 1);
        }
        return k_b_slice_walk(s, from, to);
    }
    k_die("slice takes a list or string");
    return k_none();
}

/* The walk keeps a dozen values live and paid seven pushes at k_b_slice's
   door, on every slice of any shape; as its own function in tail position
   the ascii and list arms open with none. */
static __attribute__((noinline)) KValue k_b_slice_walk(KStr* s, long long from, long long to) {
    {
        /* Multibyte text still needs the walk that turns a character
           position into a byte offset, and starting it at zero every time is
           what made reading a page one character at a time quadratic — a page
           with em-dashes in it is not ascii, so the direct path above never
           fires for prose.

           Every walk over text goes forward, so one remembered position is
           enough to make the whole sweep linear: the next slice resumes where
           the last one stopped rather than from the front. A builder qualifies
           too: it only ever appends, and the cursor is an offset into what is
           already there, which appending does not move — not even across a
           grow, because the header is the same object and the offset is a
           number rather than a pointer. */
        long start = -1, end = -1, at = 0;
        long long seen = 0;
        if (s == k_seek_str && s->cap != 0) {
            if (K_COUNTING) k_stat_seek_resumes++;
            at = k_seek_byte;
            /* `seen` counts characters already passed, and the loop increments
               before it compares — so resuming AT the remembered character
               means one fewer has been passed. */
            seen = k_seek_char - 1;
            /* Behind the cursor, step back to it rather than to the front. A
               matcher does not read a page in one direction: it backtracks,
               and a rewind to byte zero on every step back is the whole cost.
               Walking back is the same walk with the continuation bytes
               skipped, so the price is the distance moved rather than the
               position reached. */
            while (from <= seen && at > 0) {
                at--;
                while (at > 0 && ((unsigned char)s->data[at] & 0xC0) == 0x80) at--;
                seen--;
            }
        }
        /* Between the two positions the walk is looking for, the bytes are
           only counted, and a distance of ten characters or more is counted
           a word at a time: the index phase's subject is 1.4 MB of two- and
           four-byte characters, sliced once from the front, and the
           character loop below took fourteen instructions a character over
           it. A short slice -- the matcher's, a few characters at a time --
           never enters the skip, so it pays nothing for it. */
        if (from - seen > 9) k_slice_skip(s, &at, &seen, from);
        while (at <= s->len) {
            seen++;
            if (seen == from) {
                start = at;
                if (to + 1 - seen > 9 && at < s->len) {
                    at += k_cp_len((unsigned char)s->data[at]);
                    k_slice_skip(s, &at, &seen, to + 1);
                    continue;
                }
            }
            if (seen == to + 1) { end = at; break; }
            if (at == s->len) break;
            at += k_cp_len((unsigned char)s->data[at]);
        }
        if (start >= 0 && from >= 1) {
            k_seek_note(s);
            k_seek_char = from;
            k_seek_byte = start;
        }
        if (from < 1 || from > to || start < 0) return k_str_n("", 0);
        if (end < 0) end = s->len;
        if (seen <= to) return k_str_n("", 0);
        if (end - start >= K_STR_VIEW_MIN && s->cap <= 0)
            return k_str_view(s, start, end - start, to - from + 1);
        return k_str_n(s->data + start, end - start);
    }
}

/* The pieces are asked in their own pass, which stops at the first piece
   without a count: pend joins four thousand rendered numbers, none of which
   has been counted, and asking each of them inside the copying loop cost that
   benchmark 4,803,400 instructions. */
static void k_join_seed_count(KStr* os, KList* l, KStr* ss) {
    long long chars = 0;
    if (ss->len != 0) {
        if (ss->cap >= 0) return;
        chars = (-(long long)ss->cap - 1) * (l->len ? l->len - 1 : 0);
    }
    for (long long i = 0; i < l->len; i++) {
        KStr* part = k_as_str(l->items[i]);
        if (part->cap >= 0) return;
        chars += -(long long)part->cap - 1;
    }
    k_str_seed_count(os, chars);
}

/* A join whose list holds something other than a string from position `from`
   on. A subtype of string is a string wherever one goes, so this finishes the
   walk reading through wrappers and builds the answer from their base
   strings; anything else is refused as before. Out of line, so a join of
   plain strings keeps its loop. */
static __attribute__((noinline, cold)) KValue k_join_wrapped(KList* l, KStr* ss, long long from) {
    for (long long i = from; i < l->len; i++) {
        if (l->items[i].tag == K_THUNK) {
            l->items[i] = k_force(l->items[i]);
            k_note_if_carried(l->items[i]);
        }
        if (!k_not_failure(l->items[i])) return l->items[i];
        if (k_sub_base(l->items[i]).tag != K_STR) k_die("join takes a list of strings");
    }
    long total = 0;
    for (long long i = 0; i < l->len; i++) {
        total += k_as_str(k_sub_base(l->items[i]))->len;
        if (i) total += ss->len;
    }
    char* data = k_alloc(total + 1);
    long at = 0;
    for (long long i = 0; i < l->len; i++) {
        if (i) {
            memcpy(data + at, ss->data, (size_t)ss->len);
            at += ss->len;
        }
        KStr* is = k_as_str(k_sub_base(l->items[i]));
        memcpy(data + at, is->data, (size_t)is->len);
        at += is->len;
    }
    data[total] = 0;
    KStr* os = k_alloc(sizeof(KStr));
    os->len = total;
    os->data = data;
    os->cap = 0;
    KValue out; out.tag = K_STR; out.payload = k_ptr(os);
    return out;
}

KValue k_b_join(KValue lv, KValue sep) {
    if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(sep), 0)) return k_failed2(lv, sep);
    if (lv.tag != K_LIST || sep.tag != K_STR) k_die("join takes a list of strings and a separator");
    KList* l = k_as_list(lv);
    KStr* ss = k_as_str(sep);
    long total = 0;
    for (long long i = 0; i < l->len; i++) {
        /* Joining demands each string, so a cell is forced here — and the
           answer is written back, because the copying loop below reads the
           slot again. Forcing twice would run the computation twice, and
           k_alloc for the output buffer runs between the two passes. */
        if (l->items[i].tag == K_THUNK) {
            /* Only a force that replaced a thunk puts a pointer in the slot
               that was not there before; a slot already forced is written
               back with the bits it held. Asking behind the thunk test costs
               runbench nothing, and asking in front of it cost 3,617,214. */
            l->items[i] = k_force(l->items[i]);
            k_note_if_carried(l->items[i]);
        }
        if (!k_not_failure(l->items[i])) return l->items[i];
        if (l->items[i].tag != K_STR) return k_join_wrapped(l, ss, i);
        total += k_as_str(l->items[i])->len;
        if (i) total += ss->len;
    }
    char* data = k_alloc(total + 1);
    long at = 0;
    for (long long i = 0; i < l->len; i++) {
        /* Both copies are one byte or none in every shipped caller measured:
           `join [s s] ""` gives a zero-length separator and `join digits " "`
           joins one-byte strings with a one-byte separator. Through the call
           a one-byte separator costs 16.0 instructions and a one-byte item
           23.6, over 400,019 calls in runbench; storing the byte where the
           length says one saves 5.5 on an average call. */
        if (i) {
            k_copy_short(data + at, ss->data, ss->len);
            at += ss->len;
        }
        KStr* is = k_as_str(l->items[i]);
        k_copy_short(data + at, is->data, is->len);
        at += is->len;
    }
    KStr* os = k_alloc(sizeof(KStr));
    os->len = total;
    os->data = data;
    os->cap = 0;
    /* A string's character count is memoised in `cap` the first time
       `length` or an index asks for it, and a join of pieces that each
       carry one knows the sum without reading a byte. The run program's
       index phase doubles its subject by joining it to itself, asking
       `length` after every doubling, and each ask scanned the whole new
       string: 2,458,767 instructions a run, for counts the pieces held. */
    k_join_seed_count(os, l, ss);
    data[total] = 0;
    KValue out; out.tag = K_STR; out.payload = k_ptr(os);
    return out;
}

KValue k_b_map(KValue lv, KValue f) {
    if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(f), 0)) return k_failed2(lv, f);
    if (lv.tag != K_LIST) k_die("map takes a list");
    KList* l = k_as_list(lv);
    KValue* items = k_alloc(sizeof(KValue) * (l->len ? l->len : 1));
    for (long long i = 0; i < l->len; i++) items[i] = k_call1(f, l->items[i]);
    return k_mklist(l->len, items);
}

KValue k_b_filter(KValue lv, KValue f) {
    if (__builtin_expect(!k_not_failure(lv) || !k_not_failure(f), 0)) return k_failed2(lv, f);
    if (lv.tag != K_LIST) k_die("filter takes a list");
    KList* l = k_as_list(lv);
    KValue* items = k_alloc(sizeof(KValue) * (l->len ? l->len : 1));
    long long kept = 0;
    for (long long i = 0; i < l->len; i++) {
        KValue verdict = k_call1(f, l->items[i]);
        if (verdict.tag == K_TRUE) items[kept++] = l->items[i];
        else if (verdict.tag != K_FALSE) k_die("a filter predicate returns true or false");
    }
    return k_mklist(kept, items);
}

static int k_sort_cmp(const void* pa, const void* pb) {
    return k_order(*(const KValue*)pa, *(const KValue*)pb);
}

KValue k_b_sort(KValue lv) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag != K_LIST) k_die("sort takes a list");
    KList* l = k_as_list(lv);
    KValue* items = k_alloc(sizeof(KValue) * (l->len ? l->len : 1));
    memcpy(items, l->items, sizeof(KValue) * l->len);
    qsort(items, l->len, sizeof(KValue), k_sort_cmp);
    return k_mklist(l->len, items);
}

KValue k_b_sum(KValue lv) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag != K_LIST) k_die("sum takes a list");
    KList* l = k_as_list(lv);
    long long total = 0;
    for (long long i = 0; i < l->len; i++) {
        if (!k_not_failure(l->items[i])) return l->items[i];
        if (!K_IS_INT(l->items[i])) k_die("sum takes a list of int");
        long long r;
        if (l->items[i].tag == K_BIG || __builtin_add_overflow(total, l->items[i].payload, &r)) {
            /* past a word: the rest of the sum is a bignum's, and a failure
               later in the list still answers in place of it */
            KValue big = k_int(total);
            for (long long j = i; j < l->len; j++) {
                if (!k_not_failure(l->items[j])) return l->items[j];
                if (!K_IS_INT(l->items[j])) k_die("sum takes a list of int");
                big = k_int_add(big, l->items[j]);
            }
            return big;
        }
        total = r;
    }
    return k_int(total);
}

KValue k_b_char_code(KValue cv) {
    if (!k_not_failure(cv)) return cv;
    if (cv.tag != K_STR) k_die("char_code takes a one-character string");
    KStr* s = k_as_str(cv);
    unsigned char b0 = (unsigned char)s->data[0];
    long w = k_cp_len(b0);
    if (s->len != w) k_die("char_code takes a one-character string");
    long cp;
    if (w == 1) cp = b0;
    else if (w == 2) cp = ((b0 & 0x1f) << 6) | (s->data[1] & 0x3f);
    else if (w == 3) cp = ((b0 & 0x0f) << 12) | ((s->data[1] & 0x3f) << 6) | (s->data[2] & 0x3f);
    else cp = ((b0 & 0x07) << 18) | ((s->data[1] & 0x3f) << 12) | ((s->data[2] & 0x3f) << 6) | (s->data[3] & 0x3f);
    return k_int(cp);
}

KValue k_b_from_code(KValue nv, const char* origin) {
    if (!k_not_failure(nv)) return nv;
    if (!K_IS_INT(nv)) k_die("from_code takes an int");
    long long cp = nv.tag == K_INT ? nv.payload : -1;
    if (cp < 0 || cp > 0x10ffff || (cp >= 0xd800 && cp <= 0xdfff)) {
        return k_err(k_str("not a unicode scalar value"), origin);
    }
    char data[4];
    long w;
    if (cp < 0x80) { data[0] = (char)cp; w = 1; }
    else if (cp < 0x800) { data[0] = (char)(0xc0 | (cp >> 6)); data[1] = (char)(0x80 | (cp & 0x3f)); w = 2; }
    else if (cp < 0x10000) { data[0] = (char)(0xe0 | (cp >> 12)); data[1] = (char)(0x80 | ((cp >> 6) & 0x3f)); data[2] = (char)(0x80 | (cp & 0x3f)); w = 3; }
    else { data[0] = (char)(0xf0 | (cp >> 18)); data[1] = (char)(0x80 | ((cp >> 12) & 0x3f)); data[2] = (char)(0x80 | ((cp >> 6) & 0x3f)); data[3] = (char)(0x80 | (cp & 0x3f)); w = 4; }
    return k_str_n(data, w);
}

/* A number's digits sit in a null-terminated buffer (KStr data, or a bytes
   view over it), and strtoll/strtod halt at the first non-digit — which the
   scanner guarantees is the delimiter at data[len]. So we parse in place
   straight from the bytes, skipping the string the scanner would otherwise
   allocate per number. */
/* the bytes libc's isspace answers yes to in the C locale */
static int k_num_space(char c) {
    return c == ' ' || (c >= '\t' && c <= '\r');
}

