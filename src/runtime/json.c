/* The JSON library's string kernels: the decoder's walk to a string's
   closing quote, which undoes its escapes as it goes, and the encoder's
   writer, which escapes a string into a builder. Both copy sixteen bytes at a
   time where the target has vectors, and both read through text.c's scanners
   and append paths, so this part comes after it. */

/* The closing quote of a JSON string whose first backslash is at `q`. Each
   backslash takes the byte after it along, so an escaped quote is passed
   over. Zero when the string runs out first. The twin runs the same loop
   around the inlined scan. */
static long long k_json_quote_after(const unsigned char* d, long long len, long long q) {
    for (;;) {
        if (q + 1 > len) return 0;
        q = k_b_find2_raw(d, len, q + 2, 34, 92);
        if (q > len) return 0;
        if (d[q - 1] == 34) return q;
    }
}

/* THE STRING THE QUOTE WALK JUST UNDID. A JSON string with an escape in it was
   read twice: once by the walk to its closing quote, which answers
   `plain_string`, and again by `unescape`, which the decoder calls on the same
   range straight after. `k_json_escaped` does both in one walk. It writes the
   undone bytes at the arena's frontier as it goes, and when it reaches the
   quote it allocates the string `unescape` would have allocated, at the
   length `unescape` would have asked for, which lands where the bytes
   already are. The string is kept here, keyed by the bytes and the range, and
   `unescape` answers it when it is asked for that range. A `u` escape, any
   other escape, or a frontier without room for the string leaves no string
   behind, and the old walk finds the quote. */
static const unsigned char* k_unescaped_d;
static long long k_unescaped_from, k_unescaped_e;
static KValue k_unescaped_v;

/* The quotes and backslashes among the sixteen bytes at `d` as a mask, one
   bit a byte on x86-64 and four on arm64, with the sixteen copied to `o`
   whole and kept in `*xo`. Only those two targets have it; elsewhere the
   walk below goes a word at a time.

   `k_json_rest` stores the same sixteen moved down by `from` lanes, with
   zeros above them: the part of a block after an escape, written again
   behind what the escape wrote. A shuffle through `k_json_slide` does the
   move, so a block is loaded and classified once however many escapes it
   holds. The encoder's walk below uses both. */
#if defined(__x86_64__) || defined(__aarch64__)
static const unsigned char k_json_slide[32] = {
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
    0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80,
};
#endif
#if defined(__x86_64__)
#define K_UNDO_LANE 0
typedef __m128i KJsonLanes;
static inline uint64_t k_json_undo_block(char* o, const unsigned char* d, KJsonLanes* xo) {
    __m128i x = _mm_loadu_si128((const __m128i*)d);
    _mm_storeu_si128((__m128i*)o, x);
    *xo = x;
    __m128i m = _mm_or_si128(_mm_cmpeq_epi8(x, _mm_set1_epi8('"')),
                             _mm_cmpeq_epi8(x, _mm_set1_epi8('\\')));
    return (unsigned)_mm_movemask_epi8(m);
}
static inline void k_json_rest(char* o, KJsonLanes x, long long from) {
    __m128i by = _mm_loadu_si128((const __m128i*)(k_json_slide + from));
    _mm_storeu_si128((__m128i*)o, _mm_shuffle_epi8(x, by));
}
#elif defined(__aarch64__)
#define K_UNDO_LANE 2
typedef uint8x16_t KJsonLanes;
static inline uint64_t k_json_undo_block(char* o, const unsigned char* d, KJsonLanes* xo) {
    uint8x16_t x = vld1q_u8(d);
    vst1q_u8((uint8_t*)o, x);
    *xo = x;
    uint8x16_t m = vorrq_u8(vceqq_u8(x, vdupq_n_u8('"')), vceqq_u8(x, vdupq_n_u8('\\')));
    uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
    return vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
}
static inline void k_json_rest(char* o, KJsonLanes x, long long from) {
    vst1q_u8((uint8_t*)o, vqtbl1q_u8(x, vld1q_u8(k_json_slide + from)));
}
#endif

/* The closing quote of a string whose walk stopped at `p`, counted from zero,
   on a byte no backslash escapes. */
static long long k_json_quote_from(const unsigned char* d, long long len, long long p) {
    long long q = k_b_find2_raw(d, len, p + 1, 34, 92);
    if (q > len) return 0;
    if (d[q - 1] == 34) return q;
    return k_json_quote_after(d, len, q);
}

/* `q` is the first backslash, found by the caller's scan from `from`; both
   count from one. Answers the closing quote's position, or zero when the
   string runs out first, as the walk it replaces did. */
long long k_json_escaped(const unsigned char* d, long long len, long long from, long long q) {
    static const char undo[256] = {
        ['"'] = '"', ['\\'] = '\\', ['/'] = '/', ['b'] = 8, ['f'] = 12, ['n'] = 10,
        ['r'] = 13, ['t'] = 9,
    };
    const uint64_t ones = 0x0101010101010101ULL, high = 0x8080808080808080ULL;
    const uint64_t slashes = 0x5C5C5C5C5C5C5C5CULL, quotes = 0x2222222222222222ULL;
    k_unescaped_d = NULL;
    char* o0 = k_arena + sizeof(KStr);
    char* lim = k_arena + k_arena_left;
    long long head = q - from;
    if (o0 + head + 9 > lim) return k_json_quote_after(d, len, q);
    /* the run before the first backslash, a word at a time; the word that
       reaches past it writes bytes the walk below writes over */
    const unsigned char* src = d + from - 1;
    long long i = 0;
    if (from - 1 + head + 8 <= len) {
        for (; i < head; i += 8) {
            uint64_t w;
            memcpy(&w, src + i, 8);
            memcpy(o0 + i, &w, 8);
        }
    } else {
        for (; i < head; i++) o0[i] = (char)src[i];
    }
    char* o = o0 + head;
    long long p = q - 1;
    for (;;) {
        /* d[p] is a backslash, and it takes the byte after it along */
        if (p + 1 >= len) return 0;
        char r = undo[d[p + 1]];
        if (!r) return k_json_quote_after(d, len, p + 1);
        *o++ = r;
        p += 2;
        /* The run to the next quote or backslash, sixteen bytes a block. On
           the run program it is about five bytes, so one block nearly always
           finds its end; the word loop below took one word and most of
           another. */
#if defined(__x86_64__) || defined(__aarch64__)
        while (p + 16 <= len) {
            if (o + 17 > lim) return k_json_quote_from(d, len, p);
            KJsonLanes x;
            uint64_t hit = k_json_undo_block(o, d + p, &x);
            if (!hit) {
                o += 16;
                p += 16;
                continue;
            }
            /* The escapes the block holds whole are answered from its one
               mask: the run before each is already in place, and after it
               the rest of the block is stored again behind the undone byte.
               A quote ends the string, and a backslash in the last lane has
               its partner in the next block, so both go to `stop`. A block
               with nothing left in it, or a frontier too near for the store,
               goes back to the loop at the byte after the escape, where its
               own test and block decide as they did before, so the string
               keeps its record exactly when it used to. */
            long long base = p, at = 0;
            for (;;) {
                long long k = __builtin_ctzll(hit) >> K_UNDO_LANE;
                o += k - at;
                p = base + k;
                if (d[p] == 34 || k == 15) goto stop;
                char u = undo[d[p + 1]];
                if (!u) return k_json_quote_after(d, len, p + 1);
                *o++ = u;
                at = k + 2;
                p = base + at;
                hit &= (~0ULL << ((at << K_UNDO_LANE) - 1)) << 1;
                if (!hit || o + 17 > lim) break;
                k_json_rest(o, x, at);
            }
        }
#endif
        while (p + 8 <= len) {
            if (o + 9 > lim) return k_json_quote_from(d, len, p);
            uint64_t w;
            memcpy(&w, d + p, 8);
            uint64_t xs = w ^ slashes, xq = w ^ quotes;
            uint64_t hit = (((xs - ones) & ~xs) | ((xq - ones) & ~xq)) & high;
            memcpy(o, &w, 8);
            if (hit) {
                int k = __builtin_ctzll(hit) >> 3;
                o += k;
                p += k;
                goto stop;
            }
            o += 8;
            p += 8;
        }
        while (p < len && d[p] != 34 && d[p] != 92) {
            if (o + 1 > lim) return k_json_quote_from(d, len, p);
            *o++ = (char)d[p++];
        }
        if (p >= len) return 0;
    stop:
        if (d[p] == 34) break;
    }
    long long e = p + 1;
    long long n = (long long)(o - o0);
    KStr* s = k_str_alloc(e - from);
    if (s->data != o0) memcpy(s->data, o0, (size_t)n);
    s->len = (int)n;
    s->data[n] = 0;
    k_unescaped_d = d;
    k_unescaped_from = from;
    k_unescaped_e = e;
    k_unescaped_v.tag = K_STR;
    k_unescaped_v.payload = k_ptr(s);
    return e;
}

/* `plain_string cs p`, for a JSON string whose body starts at `p`. With no
   backslash before the closing quote it answers the text in front of the
   quote, when that is utf-8. With one, it answers the closing quote's
   position, for `unescape` to read the range. Anything else is none: the
   string runs out, or its bytes are not utf-8, and the caller reads it the
   long way, which is where the failure is named. The emitter reaches this
   through `k_b_plain_string_fast`, which tests the tags and runs the common
   case inline. */
KValue k_b_plain_string(KValue cs, KValue fromv) {
    if (__builtin_expect(!k_not_failure(cs) || !k_not_failure(fromv), 0))
        return k_failed2(cs, fromv);
    if (cs.tag != K_BYTES || fromv.tag != K_INT) k_die("plain_string takes bytes and a position");
    KBytes* b = k_as_bytes(cs);
    long long from = fromv.payload;
    if (from < 1) return k_none();
    long long q = k_b_find2_raw(b->data, b->len, from, 34, 92);
    if (q > b->len) return k_none();
    if (b->data[q - 1] == 92) {
        long long e = k_json_quote_after(b->data, b->len, q);
        return e ? k_int(e) : k_none();
    }
    KValue s = k_b_utf8_slice_raw(b->data, b->len, from, q - 1, k_site_now());
    return s.tag == K_ERR ? k_none() : s;
}

/* `unescape cs p e`: the bytes from `p` up to the closing quote at `e`, with
   JSON's eight one-letter escapes undone. A `u` escape, any other escape, or
   a range that is not one answers none, and the decoder reads the string the
   long way. The bytes are the decoder's, which are a string's, so what is
   between two escapes is whole characters and the answer is not validated
   again, as `built_text` does not validate what the decoder appends.

   The output is never longer than the input, so it is allocated at the
   input's length and the header takes what was written. Runs are copied
   eight bytes at a time and a backslash is found in a word by the zero-byte
   test on the word xor backslashes; the eight bytes are stored whole and the
   cursor moves only past the run, which stays inside the buffer because the
   output never gets ahead of the input. */
KValue k_b_unescape_raw(const unsigned char* d, long long len, long long from, long long e) {
    if (from < 1 || e < from || e > len + 1) return k_none();
    KStr* s = k_str_alloc(e - from);
    char* o = s->data;
    const unsigned char* p = d + (from - 1);
    const unsigned char* end = d + (e - 1);
    const uint64_t ones = 0x0101010101010101ULL, slashes = 0x5C5C5C5C5C5C5C5CULL;
    while (p < end) {
        while (p + 8 <= end) {
            uint64_t w;
            memcpy(&w, p, 8);
            uint64_t x = w ^ slashes;
            uint64_t hit = (x - ones) & ~x & (ones << 7);
            memcpy(o, &w, 8);
            if (hit) {
                int k = __builtin_ctzll(hit) >> 3;
                o += k;
                p += k;
                goto escape;
            }
            o += 8;
            p += 8;
        }
        while (p < end && *p != 92) *o++ = (char)*p++;
        if (p == end) break;
    escape:
        if (p + 1 == end) return k_none();
        char r;
        switch (p[1]) {
            case '"': r = '"'; break;
            case '\\': r = '\\'; break;
            case '/': r = '/'; break;
            case 'b': r = 8; break;
            case 'f': r = 12; break;
            case 'n': r = 10; break;
            case 'r': r = 13; break;
            case 't': r = 9; break;
            default: return k_none();
        }
        *o++ = r;
        p += 2;
    }
    long long n = o - s->data;
    s->len = (int)n;
    s->data[n] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

static __attribute__((noinline)) KValue k_b_unescape_rest(KValue cs, KValue fromv, KValue ev) {
    if (__builtin_expect(!k_not_failure(cs) || !k_not_failure(fromv) || !k_not_failure(ev), 0))
        return k_failed3(cs, fromv, ev);
    if (cs.tag != K_BYTES || fromv.tag != K_INT || ev.tag != K_INT)
        k_die("unescape takes bytes and two positions");
    KBytes* b = k_as_bytes(cs);
    return k_b_unescape_raw(b->data, b->len, fromv.payload, ev.payload);
}

/* The string `k_json_escaped` kept for this range is answered first, and
   without a frame; everything else is read as it always was. */
KValue k_b_unescape(KValue cs, KValue fromv, KValue ev) {
    if (cs.tag == K_BYTES && fromv.tag == K_INT && ev.tag == K_INT
        && k_as_bytes(cs)->data == k_unescaped_d && fromv.payload == k_unescaped_from
        && ev.payload == k_unescaped_e) {
        k_unescaped_d = NULL;
        return k_unescaped_v;
    }
    return k_b_unescape_rest(cs, fromv, ev);
}

/* The bytes of a word that a JSON string cannot hold as they stand -- a
   quote, a backslash, or a byte below a space -- as the high bit of each.
   The lowest bit set is exact; a borrow can only set one above it. */
static inline uint64_t k_json_specials(uint64_t w) {
    const uint64_t ones = 0x0101010101010101ULL;
    uint64_t q = w ^ (ones * '"'), b = w ^ (ones * '\\');
    return (((q - ones) & ~q) | ((b - ones) & ~b) | ((w - ones * 32) & ~w)) & (ones << 7);
}

/* Writes `len` bytes of `d` escaped for a JSON string into `o` and answers
   how many it wrote: at most six for each byte in, plus sixteen of slack,
   because a run is moved sixteen bytes at a time, the whole block stored and
   the cursor moved past the run alone. The escapes are lib/json's: `\"` and
   `\\`, the one-letter forms for 8, 9, 10, 12 and 13, and a `u` escape with
   lowercase hex digits for the other bytes below a space.

   A string the encoder escapes is short and has an escape every seven bytes
   or so on the run program, so the run before an escape was usually shorter
   than the eight bytes a word moved, and it went through a byte loop at nine
   instructions a byte. A block of sixteen covers most of those runs in one
   load. The last block of a string is read through `k_tail_window`, as the
   scanners read theirs, and the bytes past the string's end are ignored. */
static const char k_json_short[256] = {
    [8] = 'b', [9] = 't', [10] = 'n', [12] = 'f', [13] = 'r', ['"'] = '"', ['\\'] = '\\',
};
/* The bytes of the sixteen at `d` that a JSON string cannot hold as they
   stand -- a quote, a backslash, or a byte below a space -- as a mask, with
   the sixteen copied to `o` whole and kept in `*xo`. A byte is one bit of the
   mask on x86-64 and four on arm64, which is K_JSON_LANE as a shift and
   K_JSON_BYTE as the bits. */
#if defined(__x86_64__)
#define K_JSON_LANE 0
#define K_JSON_BYTE 1ULL
static inline uint64_t k_json_block(char* o, const unsigned char* d, KJsonLanes* xo) {
    __m128i x = _mm_loadu_si128((const __m128i*)d);
    _mm_storeu_si128((__m128i*)o, x);
    *xo = x;
    __m128i q = _mm_cmpeq_epi8(x, _mm_set1_epi8('"'));
    __m128i b = _mm_cmpeq_epi8(x, _mm_set1_epi8('\\'));
    __m128i c = _mm_cmpeq_epi8(_mm_min_epu8(x, _mm_set1_epi8(31)), x);
    return (unsigned)_mm_movemask_epi8(_mm_or_si128(_mm_or_si128(q, b), c));
}
#elif defined(__aarch64__)
#define K_JSON_LANE 2
#define K_JSON_BYTE 0xFULL
static inline uint64_t k_json_block(char* o, const unsigned char* d, KJsonLanes* xo) {
    uint8x16_t x = vld1q_u8(d);
    vst1q_u8((uint8_t*)o, x);
    *xo = x;
    uint8x16_t m = vorrq_u8(vorrq_u8(vceqq_u8(x, vdupq_n_u8('"')), vceqq_u8(x, vdupq_n_u8('\\'))),
                            vcleq_u8(x, vdupq_n_u8(31)));
    uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
    return vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
}
#endif

/* One byte the scan stopped on, escaped at `o`; answers the end of what it
   wrote. */
static inline char* k_json_escape_byte(char* o, unsigned char c) {
    static const char hex[] = "0123456789abcdef";
    char one = k_json_short[c];
    *o++ = '\\';
    if (one) {
        *o++ = one;
    } else {
        memcpy(o, "u00", 3);
        o[3] = hex[c >> 4];
        o[4] = hex[c & 15];
        o += 5;
    }
    return o;
}

/* The walk without blocks, a word and then a byte at a time, from `i` to the
   next byte to escape or to `len`, copying as it goes. It answers the
   position it stopped at and moves `*op` past what it wrote. Out of line: it
   runs where fewer than sixteen bytes are left and they end too close to a
   page's end to read as a block, and on a target with no blocks at all, and
   its word constants kept in registers cost the block loop a frame. */
static __attribute__((noinline)) long long k_json_walk(char** op, const unsigned char* d,
                                                       long long i, long long len) {
    char* o = *op;
    for (;;) {
        if (i + 8 <= len) {
            uint64_t w;
            memcpy(&w, d + i, 8);
            uint64_t h = k_json_specials(w);
            memcpy(o, &w, 8);
            if (h) {
                int k = __builtin_ctzll(h) >> 3;
                o += k;
                i += k;
                break;
            }
            o += 8;
            i += 8;
            continue;
        }
        while (i < len && d[i] != 34 && d[i] != 92 && d[i] >= 32) *o++ = (char)d[i++];
        break;
    }
    *op = o;
    return i;
}

static long long k_json_escape_into(char* o, const unsigned char* d, long long len) {
    char* start = o;
    long long i = 0;
    while (i < len) {
#if defined(__x86_64__) || defined(__aarch64__)
        long long left = len - i, span;
        KJsonLanes x;
        uint64_t hit;
        if (left >= 16) {
            span = 16;
            hit = k_json_block(o, d + i, &x);
        } else if (k_tail_window(d + i)) {
            span = left;
            hit = k_json_block(o, d + i, &x) & ((1ULL << (left << K_JSON_LANE)) - 1);
        } else {
            i = k_json_walk(&o, d, i, len);
            if (i == len) break;
            o = k_json_escape_byte(o, d[i++]);
            continue;
        }
        /* Every escape in the block is answered from the one mask. The run
           before an escape is already in place, and after it the rest of
           the block is stored again behind what the escape wrote. On the
           run program a block holds two or three escapes, and each used to
           load and classify a block of its own. */
        long long at = 0;
        while (hit) {
            long long k = __builtin_ctzll(hit) >> K_JSON_LANE;
            hit &= ~(K_JSON_BYTE << (k << K_JSON_LANE));
            o = k_json_escape_byte(o + (k - at), d[i + k]);
            at = k + 1;
            k_json_rest(o, x, at);
        }
        o += span - at;
        i += span;
#else
        i = k_json_walk(&o, d, i, len);
        if (i == len) break;
        o = k_json_escape_byte(o, d[i++]);
#endif
    }
    return o - start;
}

/* A builder without room takes the escaped bytes through a buffer on the
   stack. Out of line, because the buffer is most of a frame, and with it
   inline every string the encoder escaped into a builder with room saved six
   registers and reserved 792 bytes to reach a path it did not take. */
static __attribute__((noinline, cold)) KValue k_append_escaped_pieces(KValue acc,
        const unsigned char* d, long long len, int mutate) {
    char room[6 * 128 + 16];
    for (long long at = 0; at < len; at += 128) {
        long long piece = len - at < 128 ? len - at : 128;
        long long n = k_json_escape_into(room, d + at, piece);
        acc = k_b_append_range(acc, k_as_bytes(acc), (const unsigned char*)room, n, mutate);
    }
    return acc;
}

/* `append_escaped acc s`: `acc` with `s` appended as a JSON string holds it.
   The writer calls it for a string its scan found something to escape in;
   one with nothing is appended as it stands. Linearity treats it as it
   treats `append`, and a site it proves unique takes the `_mut` entry, which
   writes into the builder it was handed. The escaped bytes go through a
   buffer on the stack, 128 bytes of input at a time, so neither entry puts
   anything in the arena that `append` itself would not. A builder that ends
   at its buffer's frontier with room for the worst case is written into
   directly, as `k_b_append_rendered` writes its digits. */
static KValue k_append_escaped(KValue acc, KValue sv, int mutate) {
    if (!k_not_failure(acc) || !k_not_failure(sv)) return k_failed2(acc, sv);
    if (acc.tag != K_BYTES || sv.tag != K_STR) k_die("append_escaped takes bytes and a string");
    KStr* s = k_as_str(sv);
    const unsigned char* d = (const unsigned char*)s->data;
    long long len = s->len;
    KBytes* a = k_as_bytes(acc);
    long long acap = a->cap & ~1LL;
    if (acap) {
        KBuf* kb = ((KBuf*)a->data) - 1;
        if (kb->used == a->len && a->len + 6 * len + 16 <= acap) {
            long long n = k_json_escape_into((char*)a->data + a->len, d, len);
            kb->used = a->len + n;
            if (mutate) {
                a->len += n;
                return acc;
            }
            return k_bytes_owned(a->len + n, a->data, a->cap);
        }
    }
    return k_append_escaped_pieces(acc, d, len, mutate);
}

KValue k_b_append_escaped(KValue acc, KValue sv) { return k_append_escaped(acc, sv, 0); }

KValue k_b_append_escaped_mut(KValue acc, KValue sv) { return k_append_escaped(acc, sv, 1); }

/* `append_key acc k`: `acc` with `k` written as a JSON object key, quoted,
   escaped and followed by a colon. The emitter's twin claims the builder
   once for a key the decoder shared as a clean token; this is everything
   else, the three appends the key stands for. */
static KValue k_append_key(KValue acc, KValue sv, int mutate) {
    if (!k_not_failure(acc) || !k_not_failure(sv)) return k_failed2(acc, sv);
    if (acc.tag != K_BYTES || sv.tag != K_STR) k_die("append_key takes bytes and a string");
    KValue quote = k_int(34), colon = k_int(58);
    acc = mutate ? k_b_append_mut(acc, quote) : k_b_append(acc, quote);
    acc = k_append_escaped(acc, sv, mutate);
    acc = mutate ? k_b_append_mut(acc, quote) : k_b_append(acc, quote);
    return mutate ? k_b_append_mut(acc, colon) : k_b_append(acc, colon);
}

KValue k_b_append_key(KValue acc, KValue sv) { return k_append_key(acc, sv, 0); }

/* `append_string acc s`: `s` as a JSON string value, between its quotes. A
   builder at its frontier with room for the worst case takes the quotes and
   the escaped bytes in one claim; anything else is three appends. */
static KValue k_append_string(KValue acc, KValue sv, int mutate) {
    if (!k_not_failure(acc) || !k_not_failure(sv)) return k_failed2(acc, sv);
    if (acc.tag != K_BYTES || sv.tag != K_STR) k_die("append_string takes bytes and a string");
    KStr* s = k_as_str(sv);
    long long len = s->len;
    KBytes* a = k_as_bytes(acc);
    long long acap = a->cap & ~1LL;
    if (acap) {
        KBuf* kb = ((KBuf*)a->data) - 1;
        if (kb->used == a->len && a->len + 6 * len + 18 <= acap) {
            char* o = (char*)a->data + a->len;
            o[0] = '"';
            long long n = k_json_escape_into(o + 1, (const unsigned char*)s->data, len);
            o[n + 1] = '"';
            kb->used = a->len + n + 2;
            if (mutate) {
                a->len += n + 2;
                return acc;
            }
            return k_bytes_owned(a->len + n + 2, a->data, a->cap);
        }
    }
    KValue quote = k_int(34);
    acc = mutate ? k_b_append_mut(acc, quote) : k_b_append(acc, quote);
    acc = k_append_escaped(acc, sv, mutate);
    return mutate ? k_b_append_mut(acc, quote) : k_b_append(acc, quote);
}

KValue k_b_append_string(KValue acc, KValue sv) { return k_append_string(acc, sv, 0); }

K_DOORCC KValue k_b_append_string_mut(KValue acc, KValue sv) {
    return k_append_string(acc, sv, 1);
}

KValue k_b_append_key_mut(KValue acc, KValue sv) { return k_append_key(acc, sv, 1); }
