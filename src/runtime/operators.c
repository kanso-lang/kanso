static KValue k_sub_base(KValue v) {
    while (v.tag == K_SUB) v = ((KSub*)(intptr_t)v.payload)->inner;
    return v;
}

/* A builtin sees a subtype's value as its parent's, as the oracle's
   call_builtin does: `length` of a `type name string` counts the string. The
   emitter routes a builtin's arguments through this only in a program that
   declares a subtype, so no other program pays for it. */
KValue k_unsub(KValue v) {
    return v.tag == K_SUB ? k_sub_base(v) : v;
}

/* Whether an operand sends an operator to its user arms. A record does, and
   so does a subtype of one, which is the same value wearing a narrower name. */
long long k_routes_to_arms(KValue v) {
    /* a subtype is user-owned whatever it wraps, so an arm written for
       `type money int` is reachable where the int's builtin would otherwise
       answer first and the arm would never run */
    if (v.tag == K_SUB) return 1;
    return v.tag == K_REC;
}

KValue k_sub_wrap(long long type_id, KValue inner) {
    if (!k_not_failure(inner)) return inner;
    KSub* s = k_alloc(sizeof(KSub));
    s->type_id = type_id;
    s->inner = inner;
    KValue v; v.tag = K_SUB; v.payload = k_ptr(s); return v;
}

/* Two parents that are not one tag: `bool` is either of two, and `some` is
   any value that is neither none nor a failure. They sit below every
   -(tag + 1), which is what the other primitives are. */
#define K_WANT_BOOL (-100)
#define K_WANT_SOME (-101)

/* Whether a value that wraps nothing is the primitive a negative want
   names. */
static int k_want_prim(KValue v, long long want_id) {
    if (want_id == K_WANT_BOOL) return v.tag == K_TRUE || v.tag == K_FALSE;
    if (want_id == K_WANT_SOME) return v.tag != K_NONE && k_not_failure(v);
    if (want_id == -(K_INT + 1)) return K_IS_INT(v);
    return v.tag == -(want_id + 1);
}

/* Chain match for dispatch: 0 = exact, +1 per parent hop; -1 = no match.
   want_id >= 0 names a declared type; want_id < 0 encodes a primitive tag
   as -(tag + 1), so annotated primitive params accept wrapped values. */
long long k_sub_depth(KValue v, long long want_id) {
    long long depth = 0;
    while (v.tag == K_SUB) {
        KSub* s = (KSub*)(intptr_t)v.payload;
        if (want_id >= 0 && s->type_id == want_id) return depth;
        v = s->inner;
        depth++;
    }
    if (want_id < 0 && k_want_prim(v, want_id)) return depth;
    if (want_id >= 0 && v.tag == K_REC
        && ((KRec*)(intptr_t)v.payload)->type_id == want_id) return depth;
    return -1;
}

/* Dispatch checks for programs that declare subtypes: the annotated
   param accepts a wrapped value whose chain reaches the annotation. */
long long k_check_sub_tag(KValue v, long long tag) {
    return k_sub_depth(v, -(tag + 1)) >= 0;
}

long long k_check_sub_bool(KValue v) {
    KValue b = k_sub_base(v);
    return b.tag == K_TRUE || b.tag == K_FALSE;
}

long long k_check_sub_id(KValue v, long long type_id) {
    return k_sub_depth(v, type_id) >= 0;
}

long long k_check_sub_rec(KValue v, long long type_id, long long nfields) {
    KValue b = k_sub_base(v);
    if (b.tag != K_REC) return 0;
    KRec* r = (KRec*)(intptr_t)b.payload;
    return r->type_id == type_id && r->nfields == nfields;
}

/* "an int", "a string" — a diagnostic that fumbles its own grammar reads as
   carelessness about everything else in it. The rule is spelling, not
   pronunciation: a type name is a program's word rather than an English one,
   and the letter is what a reader has in front of them. `article` in diag.rs
   is the same rule for the other two engines. */
static const char* k_article(const char* word) {
    switch (word[0]) {
        case 'a': case 'e': case 'i': case 'o': case 'u': return "an";
        default: return "a";
    }
}

/* Construction: `post_body s` — the inner value must be (or reach) the
   parent; want encodes a type id, or -(tag+1) for a primitive. */
KValue k_sub_ctor(long long type_id, long long want, KValue inner, const char* tyname, const char* parent) {
    if (!k_not_failure(inner)) return inner;
    if (k_sub_depth(inner, want) < 0) {
        fprintf(stderr, "%serror[runtime]:%s `%s` wraps %s %s\n", k_c_err(), k_c_off(), tyname,
                k_article(parent), parent);
        k_died();
    }
    return k_sub_wrap(type_id, inner);
}

/* The upcast: strip to the named ancestor; widening only. */
KValue k_upcast(KValue v, long long want, const char* tyname) {
    KValue cur = v;
    if (!k_not_failure(cur)) return cur;
    for (;;) {
        if (cur.tag == K_SUB) {
            KSub* sb = (KSub*)(intptr_t)cur.payload;
            if (want >= 0 && sb->type_id == want) return cur;
            cur = sb->inner;
            continue;
        }
        if (want < 0 && k_want_prim(cur, want)) return cur;
        if (want >= 0 && cur.tag == K_REC
            && ((KRec*)(intptr_t)cur.payload)->type_id == want) return cur;
        fprintf(stderr, "%serror[runtime]:%s `:%s` widens; this value is not %s %s\n",
                k_c_err(), k_c_off(), tyname, k_article(tyname), tyname);
        k_died();
    }
}

#define K_RENDER_PATH_MAX 4096
static const void* k_render_path[K_RENDER_PATH_MAX];
static int k_render_depth = 0;

/* Which values can reach a user's `render/to_string` arm. A compiled call
   site knows its argument's set and decides this at compile time; `print`
   handed over as a value does not, so the decision moves here. Coherence
   licenses the rest: no arm can exist for a primitive. */
long long k_render_dispatchable(KValue v) {
    return v.tag == K_REC || v.tag == K_NONE || v.tag == K_DESC || v.tag == K_SUB;
}

/* Rendering a value that IS a failure hands the failure back, because the
   interpolation asked for a failing value and infectiousness carries it. That
   is a property of the top of a render, not of every err it reaches: an err
   held INSIDE a list or a record is a thing the container holds, and the
   switch below renders it `err <reason>` the way the oracle does.

   The two used to be one function, and the early return shadowed the switch
   case, so printing a list of results handed the merged failure to the
   endpoint and took the successes with it. Native was alone in that: the
   interpreter and wasm both answer through `eval::render`, which has only the
   nested case. */
/* The digits of an int or a float, written into `buf` (64 bytes is enough
   for either) with the length returned and no string built. k_render_at is
   one reader; `append acc "{n}"` is the other, since 2026-09-09, and it
   copies the digits into the accumulator straight from here rather than
   through a KStr it would drop on the next line. */
static inline __attribute__((always_inline)) long long k_render_number(KValue v, char* buf) {
    long long nlen;
    switch (v.tag) {
        case K_INT:
            nlen = k_itoa(buf, v.payload);
            break;
        case K_FLOAT: {
            double d = k_as_f(v);
            if (d == floor(d) && fabs(d) < 1e15 && isfinite(d)) {
                /* An integral double under 1e15 casts exactly, so its digits
                   are an integer's digits and the fraction is exactly `.0`.
                   glibc's `%.1f` reaches __printf_fp for them anyway, which
                   is a multiprecision routine: it was 11.0% of the wide
                   benchmark. Negative zero is the one value the cast loses,
                   and `%.1f` writes `-0.0` for it. */
                char* o = buf;
                long long whole = (long long)d;
                if (whole == 0 && signbit(d)) *o++ = '-';
                o += k_itoa(o, whole);
                *o++ = '.';
                *o++ = '0';
                *o = 0;
                nlen = (long long)(o - buf);
                break;
            }
            /* shortest round-trip: %g trims trailing zeros, so probing
               15..17 yields byte-identical strings to probing 1..17 — a
               double never needs more, and rarely fewer, than 15 digits */
            /* the ryu digit core computes the true shortest round-trip
               representation directly — no probing, no dtoa */
            if (K_COUNTING) k_stat_ryu_renders++;
            if (d < 0) {
                /* Straight into the caller's buffer behind the sign. The
                   scratch this replaced was 63 bytes and a strcpy of the whole
                   rendering, paid on 430,400 of encodebench's 849,200 renders.
                   buf is 64 and render_ryu writes at most the 63 the scratch
                   held, so the byte the sign takes is the byte that was
                   spare. */
                buf[0] = '-';
                nlen = 1 + render_ryu(-d, buf + 1);
            } else {
                nlen = render_ryu(d, buf);
            }
            break;
        }
        default:
            k_die("k_render_number: not a number");
    }
    return nlen;
}

KValue k_render(KValue v, long long quote) { return k_render_at(v, quote, 0); }

/* A rendering is written into one buffer and made a string once. The
   containers used to join each element's text onto everything before it, and
   a join copies both halves into a fresh string, so a list's text was copied
   once per element and left behind in the arena each time: 4,000 small ints
   allocated 32,294,448 bytes to print 7,999, and a list of 20,000 maps
   outgrew the machine. The interpreter builds one String. The buffer is
   malloc'd and freed here, so a force inside the walk cannot rewind it. */
typedef struct { char* p; long long len; long long cap; } KRenderBuf;

static void k_rb_put(KRenderBuf* b, const char* s, long long n) {
    if (b->len + n > b->cap) {
        long long cap = b->cap ? b->cap : 64;
        while (cap < b->len + n) cap *= 2;
        char* p = realloc(b->p, (size_t)cap);
        if (!p) { fputs("out of memory\n", stderr); exit(1); }
        b->p = p;
        b->cap = cap;
    }
    memcpy(b->p + b->len, s, (size_t)n);
    b->len += n;
}

static void k_rb_lit(KRenderBuf* b, const char* s) { k_rb_put(b, s, (long long)strlen(s)); }

/* Everything k_render_at answers for a value inside a container, in the same
   order and with the same cycle path, appended rather than returned. */
static void k_render_into(KRenderBuf* b, KValue v, long long quote) {
    if (v.tag == K_SUB) { k_render_into(b, k_sub_base(v), quote); return; }
    if (v.tag == K_THUNK) {
        KThunk* cell = (KThunk*)(intptr_t)v.payload;
        for (int d = 0; d < k_render_depth; d++)
            if (k_render_path[d] == cell) { k_rb_lit(b, "<cycle>"); return; }
        if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = cell;
        k_render_into(b, k_force(v), quote);
        if (k_render_depth > 0) k_render_depth--;
        return;
    }
    char buf[64];
    switch (v.tag) {
        case K_INT:
        case K_FLOAT:
            k_rb_put(b, buf, k_render_number(v, buf));
            return;
        case K_BIG: {
            KStr* st = k_as_str(k_big_render(v));
            k_rb_put(b, st->data, st->len);
            return;
        }
        case K_TRUE: k_rb_lit(b, "true"); return;
        case K_FALSE: k_rb_lit(b, "false"); return;
        case K_NONE: k_rb_lit(b, "<none>"); return;
        case K_DONE: k_rb_lit(b, "<done>"); return;
        case K_ERR:
        case K_ERRV:
            k_rb_lit(b, "err ");
            k_render_into(b, k_err_inner(v), 1);
            return;
        case K_STR: {
            KStr* s = k_as_str(v);
            if (quote) k_rb_lit(b, "\"");
            k_rb_put(b, s->data, s->len);
            if (quote) k_rb_lit(b, "\"");
            return;
        }
        case K_REC: {
            KRec* r = k_as_rec(v);
            if (r->nfields > 0) {
                for (int d = 0; d < k_render_depth; d++)
                    if (k_render_path[d] == r) { k_rb_lit(b, "<cycle>"); return; }
                if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = r;
                k_rb_lit(b, k_type_shown(r->type_id));
                for (long long i = 0; i < r->nfields; i++) {
                    k_rb_lit(b, " ");
                    k_render_into(b, r->fields[i], 1);
                }
                k_render_depth--;
                return;
            }
            k_rb_lit(b, k_type_shown(r->type_id));
            return;
        }
        case K_DESC: k_rb_lit(b, "<io>"); return;
        case K_LIST: {
            KList* l = (KList*)(intptr_t)v.payload;
            for (int d = 0; d < k_render_depth; d++)
                if (k_render_path[d] == l) { k_rb_lit(b, "<cycle>"); return; }
            if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = l;
            k_rb_lit(b, "[");
            for (long long i = 0; i < l->len; i++) {
                if (i) k_rb_lit(b, " ");
                k_render_into(b, l->items[i], 1);
            }
            if (k_render_depth > 0) k_render_depth--;
            k_rb_lit(b, "]");
            return;
        }
        case K_MAP: {
            KMap* m = (KMap*)(intptr_t)v.payload;
            long long n;
            KValue* s = k_map_sorted(m, &n);
            if (n == 0) { k_rb_lit(b, "{}"); return; }
            for (int d = 0; d < k_render_depth; d++)
                if (k_render_path[d] == m) { k_rb_lit(b, "<cycle>"); return; }
            if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = m;
            k_rb_lit(b, "{ ");
            for (long long i = 0; i < n; i++) {
                if (i) k_rb_lit(b, " ");
                k_render_into(b, s[i * 2], 1);
                k_rb_lit(b, ":");
                k_render_into(b, s[i * 2 + 1], 1);
            }
            if (k_render_depth > 0) k_render_depth--;
            k_rb_lit(b, " }");
            return;
        }
        case K_BYTES: {
            KBytes* by = (KBytes*)(intptr_t)v.payload;
            k_rb_lit(b, "[");
            for (long long i = 0; i < by->len; i++) {
                if (i) k_rb_lit(b, " ");
                k_rb_put(b, buf, k_itoa(buf, (long long)by->data[i]));
            }
            k_rb_lit(b, "]");
            return;
        }
        case K_CLOSURE: case K_FNREF: k_rb_lit(b, "<fn>"); return;
        default: k_rb_lit(b, "<value>"); return;
    }
}

/* A container's whole rendering, made a string once. */
static __attribute__((noinline)) KValue k_render_whole(KValue v) {
    KRenderBuf b = {0, 0, 0};
    k_render_into(&b, v, 1);
    KValue out = k_str_n(b.p, b.len);
    free(b.p);
    return out;
}


static KValue k_render_at(KValue v, long long quote, int held) {
    if (v.tag == K_ERR && !held) return v;
    if (v.tag == K_SUB) return k_render_at(k_sub_base(v), quote, held);
    /* Output is the demand, so rendering forces what it prints. The oracle
       answers the same way. */
    if (v.tag == K_THUNK) {
        KThunk* cell = (KThunk*)(intptr_t)v.payload;
        for (int d = 0; d < k_render_depth; d++)
            if (k_render_path[d] == cell) return k_str("<cycle>");
        if (k_render_depth < K_RENDER_PATH_MAX) k_render_path[k_render_depth++] = cell;
        /* the wire is the demand: output forces what it prints, and the path
           above is what stops a knot walking round */
        KValue out = k_render_at(k_force(v), quote, held);
        if (k_render_depth > 0) k_render_depth--;
        return out;
    }
    char buf[64];
    /* The two number arms write their digits into `buf` and set `nlen`, then
       leave the switch by the one exit below. They shared a `k_str(buf)` per
       return until the length came back from the writers; five k_str_n bodies
       inlined there is 1,264 bytes of .text where one is 253. */
    long long nlen;
    switch (v.tag) {
        case K_INT:
        case K_FLOAT:
            nlen = k_render_number(v, buf);
            break;
        case K_BIG: return k_big_render(v);
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("<none>");
        case K_DONE: return k_str("<done>");
        case K_ERR: return k_render_whole(v);
        case K_ERRV: return k_render_whole(v);
        case K_STR:
            if (!quote) return v;
            return k_concat(k_concat(k_str("\""), v), k_str("\""));
        case K_REC:
            if (k_as_rec(v)->nfields == 0) return k_str(k_type_shown(k_as_rec(v)->type_id));
            return k_render_whole(v);
        case K_DESC: return k_str("<io>");
        case K_LIST:
        case K_MAP:
        case K_BYTES:
            return k_render_whole(v);
        case K_CLOSURE: case K_FNREF: return k_str("<fn>");
        default: return k_str("<value>");
    }
    /* The one exit the number arms take. Every other tag returned above.
       The digits go over as sixteen bytes, eight more past fifteen, and the
       call for anything past twenty-three, which a number's rendering does
       not reach: buf is sixty-four bytes, so the reads past nlen are inside
       it, and k_alloc rounds a string's storage up to sixteen, so sixteen
       bytes fit a string of fifteen or fewer and twenty-four fit one of
       sixteen or more. glibc's memcpy took fifteen instructions to move a
       number's digits, 579,291 times a run on runbench, and a word loop
       bounded by nlen took twelve. */
    if (nlen == 1) return k_str_n(buf, 1);   /* a digit is in the cache */
    KStr* s = k_str_alloc(nlen);
    memcpy(s->data, buf, 16);
    if (nlen > 15) memcpy(s->data + 16, buf + 16, 8);
    if (nlen > 23) memcpy(s->data + 24, buf + 24, (size_t)(nlen - 24));
    s->data[nlen] = 0;
    KValue out; out.tag = K_STR; out.payload = k_ptr(s); return out;
}

static long long k_bytes_eq_list(KBytes* b, KList* l) {
    if (b->len != l->len) return 0;
    for (long long i = 0; i < b->len; i++) {
        if (l->items[i].tag != K_INT || l->items[i].payload != (long long)b->data[i]) return 0;
    }
    return 1;
}

/* reused across comparisons so k_eq never mallocs on the acyclic path */
typedef struct { const void* a; const void* b; unsigned long long gen; } KEqSlot;
static KEqSlot* k_eq_slots = NULL;
static size_t k_eq_cap = 0;
static size_t k_eq_live = 0;
static unsigned long long k_eq_gen = 0;

static void k_eq_begin(void) {
    if (!k_eq_slots) {
        k_eq_cap = 1024;
        k_eq_slots = calloc(k_eq_cap, sizeof(KEqSlot));
        if (!k_eq_slots) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_eq_gen++;
    k_eq_live = 0;
}

static size_t k_eq_probe(const void* a, const void* b) {
    size_t h = (((uintptr_t)a >> 4) * 0x9E3779B97F4A7C15ULL)
             ^ (((uintptr_t)b >> 4) * 0xC2B2AE3D27D4EB4FULL);
    size_t i = h & (k_eq_cap - 1);
    while (k_eq_slots[i].gen == k_eq_gen
           && (k_eq_slots[i].a != a || k_eq_slots[i].b != b))
        i = (i + 1) & (k_eq_cap - 1);
    return i;
}

static void k_eq_grow(void) {
    KEqSlot* old = k_eq_slots;
    size_t old_cap = k_eq_cap;
    k_eq_cap *= 2;
    k_eq_slots = calloc(k_eq_cap, sizeof(KEqSlot));
    if (!k_eq_slots) { fputs("out of memory\n", stderr); exit(1); }
    for (size_t i = 0; i < old_cap; i++) {
        if (old[i].gen != k_eq_gen) continue;
        size_t j = k_eq_probe(old[i].a, old[i].b);
        k_eq_slots[j] = old[i];
    }
    free(old);
}

/* returns 1 if the pair was already assumed equal this comparison */
static int k_eq_assume(const void* a, const void* b) {
    if (k_eq_live * 10 >= k_eq_cap * 7) k_eq_grow();
    size_t i = k_eq_probe(a, b);
    if (k_eq_slots[i].gen == k_eq_gen) return 1;
    k_eq_slots[i].gen = k_eq_gen;
    k_eq_slots[i].a = a;
    k_eq_slots[i].b = b;
    k_eq_live++;
    return 0;
}

static long long k_eq_rec(KValue a, KValue b);

static long long k_eq(KValue a, KValue b) {
    k_eq_begin();
    return k_eq_rec(a, b);
}

/* Asking whether two functions are the same function is asking which one you
   were handed, and dispatch answers that. The same goes for an effect: if the
   answer would change what happens next, an arm names it. So the question is
   refused rather than answered falsely, the way `<` already refuses it. */
static int k_opaque_to_equality(KValue v) {
    if (v.tag == K_SUB) v = k_sub_base(v);
    return v.tag == K_CLOSURE || v.tag == K_FNREF || v.tag == K_DESC;
}

/* A whole number against a fractional one, decided exactly. Widening the
   integer first rounds it onto the nearest value a double can hold and
   reports two different numbers as equal — 9007199254740993 is the smallest
   integer where that happens. Comparing against the double's floor keeps the
   integer whole, and the fraction breaks the tie. Answers -1, 0 or 1 with the
   integer on the left. */
static int k_order_int_float(long long i, double d) {
    if (d != d) return -1;                      /* every NaN ranks last */
    if (d >= 9223372036854775808.0) return -1;  /* past every int64 */
    if (d < -9223372036854775808.0) return 1;
    double fl = floor(d);
    long long whole = (long long)fl;
    if (i != whole) return (i > whole) - (i < whole);
    return d > fl ? -1 : 0;
}

/* The builtin float order, ruled 2026-09-30. Every NaN equals every NaN and
   ranks above inf, whatever its sign bit and however it was made, and -0.0
   equals and ranks with 0.0, which C's comparisons already do. Until then
   equality was C's ==, so nan == nan was false, while the ordering answered
   "equal" for any pair holding a NaN. */
static int k_order_float(double x, double y) {
    int xn = x != x, yn = y != y;
    if (xn || yn) return xn - yn;
    return (x > y) - (x < y);
}

/* An int of either kind against a float, exactly, as the interpreter's
   `cmp_int_float` answers: the int against the float's floor, and a float
   with a fraction above its floor ranks above an int equal to that floor. */
static int k_order_intv_float(KValue i, double d) {
    if (i.tag == K_INT) return k_order_int_float(i.payload, d);
    if (d != d) return -1;
    if (d == INFINITY) return -1;
    if (d == -INFINITY) return 1;
    double fl = floor(d);
    int c = k_int_cmp(i, k_int_of_whole_f(fl));
    if (c == 0 && d > fl) return -1;
    return c;
}

static long long k_eq_rec(KValue a, KValue b) {
    if (k_opaque_to_equality(a) || k_opaque_to_equality(b))
        k_die("equality is not defined on a function or an effect — write an arm "
              "for the case you mean");
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    /* A cell is forced the way any demand forces it, so an ordinary lazy
       operand compares as its value. A cell's identity survives forcing, so
       arriving twice at the same pair is the cycle closing through one, and
       the pair is assumed equal — the assumption that makes bisimulation
       terminate, the same one the record case below makes. A cell demanded
       mid-construction still dies in the force: that one is not a value yet. */
    if (a.tag == K_THUNK && b.tag == K_THUNK) {
        if (k_eq_assume((void*)a.payload, (void*)b.payload)) return 1;
        return k_eq_rec(k_force(a), k_force(b));
    }
    if (a.tag == K_THUNK) return k_eq_rec(k_force(a), b);
    if (b.tag == K_THUNK) return k_eq_rec(a, k_force(b));
    if (a.tag == K_BYTES && b.tag == K_LIST) return k_bytes_eq_list(k_as_bytes(a), k_as_list(b));
    if (a.tag == K_LIST && b.tag == K_BYTES) return k_bytes_eq_list(k_as_bytes(b), k_as_list(a));
    if (a.tag == K_BYTES && b.tag == K_BYTES) {
        KBytes* x = k_as_bytes(a); KBytes* y = k_as_bytes(b);
        return x->len == y->len && memcmp(x->data, y->data, x->len) == 0;
    }
    /* an err out of its box equals another with an equal reason */
    if (a.tag == K_ERRV && b.tag == K_ERRV)
        return k_eq_rec(k_err_box(a)->reason, k_err_box(b)->reason);
    /* One numeric domain, which `<=` and `>=` already assert: both answer
       true for 1 and 1.0, and `<` answers false either way round because
       neither is strictly less. Equality was the one operator dissenting. A
       float cannot be a map key, so nothing needs them told apart. */
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_order_intv_float(a, k_as_f(b)) == 0;
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_order_intv_float(b, k_as_f(a)) == 0;
    if (a.tag == K_BIG && b.tag == K_BIG) {
        KBytes* x = k_as_bytes(a); KBytes* y = k_as_bytes(b);
        return x->len == y->len && memcmp(x->data, y->data, (size_t)x->len) == 0;
    }
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case K_INT: return a.payload == b.payload;
        case K_FLOAT: return k_order_float(k_as_f(a), k_as_f(b)) == 0;
        case K_TRUE: case K_FALSE: case K_NONE: case K_DONE: return 1;
        case K_STR: {
            KStr* sa = k_as_str(a);
            KStr* sb = k_as_str(b);
            return sa->len == sb->len && memcmp(sa->data, sb->data, sa->len) == 0;
        }
        case K_REC: {
            KRec* ra = k_as_rec(a);
            KRec* rb = k_as_rec(b);
            if (ra == rb) return 1;
            if (ra->type_id != rb->type_id) return 0;
            /* A cycle is one object however the program closed it, and it
               keeps the structural comparison bisimulation gives it.
               Assuming this pair equal is what makes the walk terminate;
               cycles through cells make the same assumption above. */
            if (k_eq_assume(ra, rb)) return 1;
            for (long long i = 0; i < ra->nfields; i++) {
                if (!k_eq_rec(ra->fields[i], rb->fields[i])) return 0;
            }
            return 1;
        }
        case K_LIST: {
            KList* la = k_as_list(a);
            KList* lb = k_as_list(b);
            if (la->len != lb->len) return 0;
            for (long long i = 0; i < la->len; i++) {
                if (!k_eq_rec(la->items[i], lb->items[i])) return 0;
            }
            return 1;
        }
        case K_MAP: {
            long long na, nb;
            KValue* sa = k_map_sorted(k_as_map(a), &na);
            KValue* sb = k_map_sorted(k_as_map(b), &nb);
            if (na != nb) return 0;
            for (long long i = 0; i < na * 2; i++) {
                if (!k_eq_rec(sa[i], sb[i])) return 0;
            }
            return 1;
        }
        default: return 0;
    }
}

long long k_check_str(KValue v, const char* data, long long len) {
    if (v.tag != K_STR) return 0;
    KStr* s = k_as_str(v);
    return s->len == len && memcmp(s->data, data, len) == 0;
}

KValue k_add(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_add_overflow(a.payload, b.payload, &r)) return k_int_add(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) + k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_add(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) + k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) + k_int_to_f(b));
    k_die("`+` is not defined for these values");
    return k_none();
}

KValue k_sub(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_sub_overflow(a.payload, b.payload, &r)) return k_int_sub(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) - k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_sub(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) - k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) - k_int_to_f(b));
    k_die("`-` is not defined for these values");
    return k_none();
}

KValue k_mul(KValue a, KValue b) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_mul_overflow(a.payload, b.payload, &r)) return k_int_mul(a, b);
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) * k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_mul(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_float(k_int_to_f(a) * k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return k_float(k_as_f(a) * k_int_to_f(b));
    k_die("`*` is not defined for these values");
    return k_none();
}

/* Division answers a value, not a failure, and the value's type is declared
   like any other — so its id is the compiler's to assign and this has to be
   told. The emitter calls this once before main. */
static long long k_math_dz = -1;
static long long k_math_mf = -1;

void k_math_ids(long long divide_by_zero, long long math_failure) {
    k_math_dz = divide_by_zero;
    k_math_mf = math_failure;
}

KValue k_sub_wrap(long long type_id, KValue inner);

static KValue k_math_failure(const char* reason) {
    KValue text = k_str(reason);
    /* A program that never names either type was not given their ids, because
       it cannot tell the wrapped value from the text it reads as. */
    if (k_math_dz < 0) return text;
    KValue root = k_sub_wrap(k_math_mf, text);
    return k_sub_wrap(k_math_dz, root);
}

KValue k_div(KValue a, KValue b, const char* origin) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_math_failure("division by zero");
        /* the one signed division that overflows: the least integer over -1
           is one past the greatest, which C leaves undefined and this machine
           answers by wrapping. The answer is a bignum. */
        if (a.payload == INT64_MIN && b.payload == -1) return k_int_div(a, b);
        return k_int(a.payload / b.payload);
    }
    if (K_IS_INT(a) && K_IS_INT(b)) {
        if (b.tag == K_INT && b.payload == 0) return k_math_failure("division by zero");
        return k_int_div(a, b);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        if (k_as_f(b) == 0.0) return k_math_failure("division by zero");
        return k_float(k_as_f(a) / k_as_f(b));
    }
    if ((K_IS_INT(a) || a.tag == K_FLOAT) && (K_IS_INT(b) || b.tag == K_FLOAT)) {
        double x = a.tag == K_FLOAT ? k_as_f(a) : k_int_to_f(a);
        double y = b.tag == K_FLOAT ? k_as_f(b) : k_int_to_f(b);
        if (y == 0.0) return k_math_failure("division by zero");
        return k_float(x / y);
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

KValue k_mod(KValue a, KValue b, const char* origin) {
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_math_failure("modulo by zero");
        /* the least integer modulo -1 is zero, and zero fits — but the
           quotient it is computed from does not, and x86's division traps
           on the pair. ARM answers without trapping, which is why this
           crashed only on the machine nobody was testing on. */
        if (a.payload == INT64_MIN && b.payload == -1) return k_int(0);
        return k_int(a.payload % b.payload);
    }
    if (K_IS_INT(a) && K_IS_INT(b)) {
        if (b.tag == K_INT && b.payload == 0) return k_math_failure("modulo by zero");
        return k_int_rem(a, b);
    }
    if ((K_IS_INT(a) || a.tag == K_FLOAT) && (K_IS_INT(b) || b.tag == K_FLOAT)) {
        double x = a.tag == K_FLOAT ? k_as_f(a) : k_int_to_f(a);
        double y = b.tag == K_FLOAT ? k_as_f(b) : k_int_to_f(b);
        if (y == 0.0) return k_math_failure("modulo by zero");
        return k_float(fmod(x, y));
    }
    k_die("`%` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_order_float(k_as_f(a), k_as_f(b));
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_cmp(a, b);
    if (K_IS_INT(a) && b.tag == K_FLOAT) return k_order_intv_float(a, k_as_f(b));
    if (a.tag == K_FLOAT && K_IS_INT(b)) return -k_order_intv_float(b, k_as_f(a));
    if (a.tag == K_STR && b.tag == K_STR) {
        KStr* sa = k_as_str(a);
        KStr* sb = k_as_str(b);
        long n = sa->len < sb->len ? sa->len : sb->len;
        int c = memcmp(sa->data, sb->data, n);
        if (c) return c > 0 ? 1 : -1;
        return (sa->len > sb->len) - (sa->len < sb->len);
    }
    k_die("comparison requires two values of one comparable type");
    return 0;
}

KValue k_cmp(KValue a, KValue b, long long op) {
    /* Two strings asked whether they are equal. k_eq opens a cycle-tracking
       generation and walks a dozen tag tests to reach the same length and
       byte compare it ends with here; std/regexp asks this of one character
       against a pattern literal at every step, 91,378 times on runbench's
       scan, where it cost ninety instructions a question. A one-character
       string comes from k_str_n's cache and a literal is permanent, so the
       same text is usually the same pointer. */
    if (op <= 1 && a.tag == K_STR && b.tag == K_STR) {
        KStr* sa = k_as_str(a);
        KStr* sb = k_as_str(b);
        int same = sa == sb
            || (sa->len == sb->len && memcmp(sa->data, sb->data, (size_t)sa->len) == 0);
        return k_bool(op == 0 ? same : !same);
    }
    if (a.tag == K_SUB) a = k_sub_base(a);
    if (b.tag == K_SUB) b = k_sub_base(b);
    if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
    if (op == 0) return k_bool(k_eq(a, b));
    if (op == 1) return k_bool(!k_eq(a, b));
    int c = k_order(a, b);
    switch (op) {
        case 2: return k_bool(c < 0);
        case 3: return k_bool(c <= 0);
        case 4: return k_bool(c > 0);
        default: return k_bool(c >= 0);
    }
}

