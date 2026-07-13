#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>
#include <unistd.h>

/* ABI shared with emitted LLVM IR: %KValue = type { i64, i64 } */
typedef struct { long long tag; long long payload; } KValue;

enum { K_INT, K_FLOAT, K_TRUE, K_FALSE, K_NONE, K_ERR, K_STR, K_REC, K_DESC, K_LIST, K_MAP, K_CLOSURE, K_FNREF, K_BYTES };

typedef struct { long len; char* data; } KStr;
typedef struct { long long cap; long long used; } KBuf;
typedef struct { long long len; const unsigned char* data; } KBytes;
typedef struct { long long len; KValue* items; } KList;
/* A map is built by appending pairs to a frontier-shared buffer in O(1) (like
   lists), leaving them unsorted with possible duplicate keys. The canonical
   sorted-and-deduped view is computed once on first read and cached; appends
   invalidate it. Appending never reorders, so older shorter views stay valid;
   the sorted view is a fresh buffer, so no shared mutation. This turns object
   construction from O(n^2) copies into O(n) appends plus one O(n log n) sort. */
typedef struct {
    long long len;        /* raw appended pair count (unsorted, dups allowed) */
    KValue* pairs;        /* k_buf-backed, frontier-shared like list items */
    KValue* sorted;       /* cached sorted+deduped [k v k v...], or NULL */
    long long sorted_len; /* deduped entry count */
} KMap;
typedef struct { KValue (*fn)(void*, KValue); void* env; } KClosure;
typedef struct { long long type_id; long long nfields; KValue* fields; } KRec;
typedef struct KDesc KDesc;
struct KDesc { long long dtag; KValue x; KValue y; };
/* dtag: 0 print, 1 seq, 2 args, 3 stdin, 4 read_file, 5 write_file, 6 bind */

/* An err's propagation trace rides on the err value alone: the origin
   ("fn at file:line", interned at the construction site; NULL for
   executor-born errs) plus one hop per dispatcher failure pass-through,
   newest first. The happy path never allocates or touches any of this. */
typedef struct KHop { const char* fn; struct KHop* prev; } KHop;
typedef struct { KValue reason; const char* origin; KHop* hops; } KErrBox;

static KValue k_mklist(long long n, KValue* items);
static KValue k_call1(KValue f, KValue a);
static KValue* k_map_sorted(KMap* m, long long* out_len);

static char* k_arena = NULL;
static size_t k_arena_left = 0;

static void* k_alloc(size_t n) {
    n = (n + 15) & ~(size_t)15;
    if (n > k_arena_left) {
        size_t block = n > (1 << 20) ? n : (1 << 20);
        k_arena = malloc(block);
        if (!k_arena) { fputs("out of memory\n", stderr); exit(1); }
        k_arena_left = block;
    }
    void* p = k_arena;
    k_arena += n;
    k_arena_left -= n;
    return p;
}

/* Reference counting (Perceus, native backend only). Every heap object —
   the thing a KValue's payload points at — carries a 16-byte header with a
   refcount immediately before it. codegen is opaque about object layout, so
   this is entirely runtime-private. Commit 1: counts are maintained by
   k_dup/k_drop but nothing frees yet and codegen emits no calls, so behavior
   is unchanged. Immortal objects (interned strings, marker singletons) pin
   their count so dup/drop are no-ops on them. */
typedef struct { long long rc; long long pad; } KHeader;
#define K_IMMORTAL (1LL << 60)

static void* k_alloc_obj(size_t n) {
    KHeader* h = k_alloc(sizeof(KHeader) + n);
    h->rc = 1;
    return (void*)(h + 1);
}

static KHeader* k_hdr(long long payload) {
    return ((KHeader*)(intptr_t)payload) - 1;
}

static void k_make_immortal(long long payload) {
    k_hdr(payload)->rc = K_IMMORTAL;
}

static int k_is_heap(long long tag) {
    switch (tag) {
        case K_STR: case K_ERR: case K_REC: case K_DESC:
        case K_LIST: case K_MAP: case K_CLOSURE: case K_BYTES:
            return 1;
        default:
            return 0;
    }
}

KValue k_dup(KValue v) {
    if (k_is_heap(v.tag)) {
        KHeader* h = k_hdr(v.payload);
        if (h->rc < K_IMMORTAL) h->rc++;
    }
    return v;
}

void k_drop(KValue v) {
    if (k_is_heap(v.tag)) {
        KHeader* h = k_hdr(v.payload);
        if (h->rc < K_IMMORTAL && --h->rc == 0) {
            /* commit 1: no freeing yet — counts only. */
        }
    }
}

/* Diagnostics color from the site palette, only when stderr is a tty and
   NO_COLOR is unset: vermillion (24-bit 0xf03a00; 256-color 202) for the
   error kind, dim for trace lines. Piped output stays plain. */
static int k_color_mode(void) {
    static int mode = -1;
    if (mode >= 0) return mode;
    if (!isatty(2) || getenv("NO_COLOR")) return mode = 0;
    const char* ct = getenv("COLORTERM");
    if (ct && (strstr(ct, "truecolor") || strstr(ct, "24bit"))) return mode = 2;
    return mode = 1;
}

static const char* k_c_err(void) {
    switch (k_color_mode()) {
        case 2: return "\x1b[38;2;240;58;0m";
        case 1: return "\x1b[38;5;202m";
        default: return "";
    }
}

static const char* k_c_dim(void) { return k_color_mode() ? "\x1b[2m" : ""; }
static const char* k_c_off(void) { return k_color_mode() ? "\x1b[0m" : ""; }

void k_die(const char* msg) {
    fprintf(stderr, "%serror[runtime]:%s %s\n", k_c_err(), k_c_off(), msg);
    exit(1);
}

static long long k_ptr(void* p) { return (long long)(intptr_t)p; }
static KStr* k_as_str(KValue v) { return (KStr*)(intptr_t)v.payload; }
static KRec* k_as_rec(KValue v) { return (KRec*)(intptr_t)v.payload; }
static KList* k_as_list(KValue v) { return (KList*)(intptr_t)v.payload; }
static KBytes* k_as_bytes(KValue v) { return (KBytes*)(intptr_t)v.payload; }
static KMap* k_as_map(KValue v) { return (KMap*)(intptr_t)v.payload; }
static KDesc* k_as_desc(KValue v) { return (KDesc*)(intptr_t)v.payload; }

static double k_as_f(KValue v) { double d; memcpy(&d, &v.payload, 8); return d; }

KValue k_float(double d) {
    KValue v; v.tag = K_FLOAT; memcpy(&v.payload, &d, 8); return v;
}

KValue k_int(long long i) { KValue v; v.tag = K_INT; v.payload = i; return v; }
KValue k_bool(long long b) { KValue v; v.tag = b ? K_TRUE : K_FALSE; v.payload = 0; return v; }
KValue k_none(void) { KValue v; v.tag = K_NONE; v.payload = 0; return v; }

static KValue k_ascii_cache[128];
static char k_ascii_ready[128];

KValue k_str_n(const char* data, long long len) {
    if (len == 1) {
        unsigned char b = (unsigned char)data[0];
        if (b < 128 && b != 0) {
            if (!k_ascii_ready[b]) {
                KStr* s = k_alloc_obj(sizeof(KStr));
                s->len = 1;
                s->data = k_alloc(2);
                s->data[0] = (char)b;
                s->data[1] = 0;
                KValue v; v.tag = K_STR; v.payload = k_ptr(s);
                k_make_immortal(v.payload);
                k_ascii_cache[b] = v;
                k_ascii_ready[b] = 1;
            }
            return k_ascii_cache[b];
        }
    }
    KStr* s = k_alloc_obj(sizeof(KStr));
    s->len = (long)len;
    s->data = k_alloc(len + 1);
    memcpy(s->data, data, len);
    s->data[len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

static KValue k_str(const char* data) { return k_str_n(data, (long long)strlen(data)); }

long long k_not_failure(KValue v) { return v.tag != K_ERR && v.tag != K_NONE; }

static KErrBox* k_err_box(KValue v) { return (KErrBox*)(intptr_t)v.payload; }

KValue k_err(KValue reason, const char* origin) {
    if (!k_not_failure(reason)) return reason;
    KErrBox* box = k_alloc_obj(sizeof(KErrBox));
    box->reason = reason;
    box->origin = origin;
    box->hops = NULL;
    KValue v; v.tag = K_ERR; v.payload = k_ptr(box); return v;
}

/* A dispatcher passing a failure through appends its name; none stays bare. */
KValue k_err_hop(KValue v, const char* fn) {
    if (v.tag != K_ERR) return v;
    KErrBox* old = k_err_box(v);
    KErrBox* box = k_alloc_obj(sizeof(KErrBox));
    KHop* hop = k_alloc(sizeof(KHop));
    hop->fn = fn;
    hop->prev = old->hops;
    box->reason = old->reason;
    box->origin = old->origin;
    box->hops = hop;
    KValue out; out.tag = K_ERR; out.payload = k_ptr(box); return out;
}

/* Zero-field marker types (null, enum tags) have exactly one inhabitant, so
   every instance is interchangeable — intern one per type id instead of
   allocating. json's 2000+ nulls in a document collapse to a single record. */
#define K_MARKER_CACHE 256
static KValue k_marker_cache[K_MARKER_CACHE];
static char k_marker_ready[K_MARKER_CACHE];

KValue k_rec(long long type_id, long long n, KValue* args) {
    for (long long i = 0; i < n; i++) if (!k_not_failure(args[i])) return args[i];
    if (n == 0 && type_id >= 0 && type_id < K_MARKER_CACHE) {
        if (!k_marker_ready[type_id]) {
            KRec* r = k_alloc_obj(sizeof(KRec));
            r->type_id = type_id;
            r->nfields = 0;
            r->fields = NULL;
            KValue v; v.tag = K_REC; v.payload = k_ptr(r);
            k_make_immortal(v.payload);
            k_marker_cache[type_id] = v;
            k_marker_ready[type_id] = 1;
        }
        return k_marker_cache[type_id];
    }
    KRec* r = k_alloc_obj(sizeof(KRec));
    r->type_id = type_id;
    r->nfields = n;
    r->fields = k_alloc(sizeof(KValue) * n);
    memcpy(r->fields, args, sizeof(KValue) * n);
    KValue v; v.tag = K_REC; v.payload = k_ptr(r); return v;
}

KValue k_field(KValue v, long long i) { return k_as_rec(v)->fields[i]; }
KValue k_err_inner(KValue v) { return k_err_box(v)->reason; }

/* pattern checks: nonzero on match */
long long k_check_tag(KValue v, long long tag) { return v.tag == tag; }
long long k_check_int(KValue v, long long n) { return v.tag == K_INT && v.payload == n; }
long long k_check_rec(KValue v, long long type_id, long long nfields) {
    return v.tag == K_REC && k_as_rec(v)->type_id == type_id
        && k_as_rec(v)->nfields == nfields;
}
long long k_check_bool(KValue v) { return v.tag == K_TRUE || v.tag == K_FALSE; }

KValue k_concat(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    KStr* sa = k_as_str(a);
    KStr* sb = k_as_str(b);
    KStr* s = k_alloc_obj(sizeof(KStr));
    s->len = sa->len + sb->len;
    s->data = k_alloc(s->len + 1);
    memcpy(s->data, sa->data, sa->len);
    memcpy(s->data + sa->len, sb->data, sb->len);
    s->data[s->len] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

extern const char* k_type_name(long long type_id);
extern long long k_type_field_count(long long type_id);
extern const char* k_type_field_name(long long type_id, long long i);
KValue k_render(KValue v, long long quote);

/* keyed reads: `{ author: writer title } = post` — fields resolve by name
   against the record's declared type */
KValue k_keyed_check(KValue v, long long entries) {
    if (v.tag != K_REC) {
        KValue r = k_render(v, 1);
        fprintf(stderr, "%serror[runtime]:%s cannot read fields of %s; keyed reads take a record\n",
                k_c_err(), k_c_off(), k_as_str(r)->data);
        exit(1);
    }
    if (entries >= k_as_rec(v)->nfields)
        k_die("a keyed read omits at least one field; reading every field is the positional form");
    return v;
}

KValue k_keyed_field(KValue v, const char* name) {
    KRec* r = k_as_rec(v);
    long long n = k_type_field_count(r->type_id);
    for (long long i = 0; i < n; i++)
        if (!strcmp(k_type_field_name(r->type_id, i), name)) return r->fields[i];
    fprintf(stderr, "%serror[runtime]:%s `%s` has no field `%s`\n", k_c_err(), k_c_off(),
            k_type_name(r->type_id), name);
    exit(1);
}

KValue k_render(KValue v, long long quote) {
    if (!k_not_failure(v)) return v;
    char buf[64];
    switch (v.tag) {
        case K_INT:
            snprintf(buf, sizeof buf, "%lld", v.payload);
            return k_str(buf);
        case K_FLOAT: {
            double d = k_as_f(v);
            if (d == floor(d) && fabs(d) < 1e15 && isfinite(d)) {
                snprintf(buf, sizeof buf, "%.1f", d);
                return k_str(buf);
            }
            for (int prec = 1; prec <= 17; prec++) {
                snprintf(buf, sizeof buf, "%.*g", prec, d);
                if (strtod(buf, NULL) == d) break;
            }
            return k_str(buf);
        }
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("none");
        case K_ERR: return k_concat(k_str("err "), k_render(k_err_inner(v), 1));
        case K_STR:
            if (!quote) return v;
            return k_concat(k_concat(k_str("\""), v), k_str("\""));
        case K_REC: {
            KRec* r = k_as_rec(v);
            KValue out = k_str(k_type_name(r->type_id));
            for (long long i = 0; i < r->nfields; i++) {
                out = k_concat(out, k_str(" "));
                out = k_concat(out, k_render(r->fields[i], 1));
            }
            return out;
        }
        case K_DESC: return k_str("<description>");
        case K_LIST: {
            KList* l = (KList*)(intptr_t)v.payload;
            KValue out = k_str("[");
            for (long long i = 0; i < l->len; i++) {
                if (i) out = k_concat(out, k_str(" "));
                out = k_concat(out, k_render(l->items[i], 1));
            }
            return k_concat(out, k_str("]"));
        }
        case K_MAP: {
            KMap* m = (KMap*)(intptr_t)v.payload;
            long long n;
            KValue* s = k_map_sorted(m, &n);
            if (n == 0) return k_str("[:]");
            KValue out = k_str("[");
            for (long long i = 0; i < n; i++) {
                if (i) out = k_concat(out, k_str(" "));
                out = k_concat(out, k_render(s[i * 2], 1));
                out = k_concat(out, k_str(": "));
                out = k_concat(out, k_render(s[i * 2 + 1], 1));
            }
            return k_concat(out, k_str("]"));
        }
        case K_BYTES: {
            KBytes* b = (KBytes*)(intptr_t)v.payload;
            KValue out = k_str("[");
            char nbuf[8];
            for (long long i = 0; i < b->len; i++) {
                if (i) out = k_concat(out, k_str(" "));
                snprintf(nbuf, sizeof nbuf, "%d", (int)b->data[i]);
                out = k_concat(out, k_str(nbuf));
            }
            return k_concat(out, k_str("]"));
        }
        case K_CLOSURE: case K_FNREF: return k_str("<fn>");
    }
    return k_str("<value>");
}

static long long k_bytes_eq_list(KBytes* b, KList* l) {
    if (b->len != l->len) return 0;
    for (long long i = 0; i < b->len; i++) {
        if (l->items[i].tag != K_INT || l->items[i].payload != (long long)b->data[i]) return 0;
    }
    return 1;
}

static long long k_eq(KValue a, KValue b) {
    if (a.tag == K_BYTES && b.tag == K_LIST) return k_bytes_eq_list(k_as_bytes(a), k_as_list(b));
    if (a.tag == K_LIST && b.tag == K_BYTES) return k_bytes_eq_list(k_as_bytes(b), k_as_list(a));
    if (a.tag == K_BYTES && b.tag == K_BYTES) {
        KBytes* x = k_as_bytes(a); KBytes* y = k_as_bytes(b);
        return x->len == y->len && memcmp(x->data, y->data, x->len) == 0;
    }
    if (a.tag != b.tag) return 0;
    switch (a.tag) {
        case K_INT: return a.payload == b.payload;
        case K_FLOAT: return k_as_f(a) == k_as_f(b);
        case K_TRUE: case K_FALSE: case K_NONE: return 1;
        case K_STR: {
            KStr* sa = k_as_str(a);
            KStr* sb = k_as_str(b);
            return sa->len == sb->len && memcmp(sa->data, sb->data, sa->len) == 0;
        }
        case K_REC: {
            KRec* ra = k_as_rec(a);
            KRec* rb = k_as_rec(b);
            if (ra->type_id != rb->type_id) return 0;
            for (long long i = 0; i < ra->nfields; i++) {
                if (!k_eq(ra->fields[i], rb->fields[i])) return 0;
            }
            return 1;
        }
        case K_LIST: {
            KList* la = k_as_list(a);
            KList* lb = k_as_list(b);
            if (la->len != lb->len) return 0;
            for (long long i = 0; i < la->len; i++) {
                if (!k_eq(la->items[i], lb->items[i])) return 0;
            }
            return 1;
        }
        case K_MAP: {
            long long na, nb;
            KValue* sa = k_map_sorted(k_as_map(a), &na);
            KValue* sb = k_map_sorted(k_as_map(b), &nb);
            if (na != nb) return 0;
            for (long long i = 0; i < na * 2; i++) {
                if (!k_eq(sa[i], sb[i])) return 0;
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
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_add_overflow(a.payload, b.payload, &r)) k_die("integer overflow (int64 native build; spec int is arbitrary precision)");
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) + k_as_f(b));
    k_die("`+` is not defined for these values");
    return k_none();
}

KValue k_sub(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_sub_overflow(a.payload, b.payload, &r)) k_die("integer overflow (int64 native build; spec int is arbitrary precision)");
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) - k_as_f(b));
    k_die("`-` is not defined for these values");
    return k_none();
}

KValue k_mul(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        long long r;
        if (__builtin_mul_overflow(a.payload, b.payload, &r)) k_die("integer overflow (int64 native build; spec int is arbitrary precision)");
        return k_int(r);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) return k_float(k_as_f(a) * k_as_f(b));
    k_die("`*` is not defined for these values");
    return k_none();
}

KValue k_div(KValue a, KValue b, const char* origin) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_err(k_str("division by zero"), origin);
        return k_int(a.payload / b.payload);
    }
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        if (k_as_f(b) == 0.0) return k_err(k_str("division by zero"), origin);
        return k_float(k_as_f(a) / k_as_f(b));
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        double x = k_as_f(a);
        double y = k_as_f(b);
        return (x > y) - (x < y);
    }
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
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
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

static KValue k_mkdesc(long long dtag, KValue x, KValue y) {
    KDesc* d = k_alloc_obj(sizeof(KDesc));
    d->dtag = dtag; d->x = x; d->y = y;
    KValue v; v.tag = K_DESC; v.payload = k_ptr(d); return v;
}

KValue k_desc_print(KValue text) {
    if (!k_not_failure(text)) return text;
    if (text.tag != K_STR) k_die("print takes a string; interpolate instead");
    return k_mkdesc(0, text, k_none());
}

KValue k_seq(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag != K_DESC || b.tag != K_DESC) k_die("`>>` sequences two effect descriptions");
    return k_mkdesc(1, a, b);
}

KValue k_desc_args(void) { return k_mkdesc(2, k_none(), k_none()); }
KValue k_desc_stdin(void) { return k_mkdesc(3, k_none(), k_none()); }

KValue k_b_read_file(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("read_file takes a path string");
    return k_mkdesc(4, path, k_none());
}

KValue k_b_write_file(KValue path, KValue content) {
    if (!k_not_failure(path)) return path;
    if (!k_not_failure(content)) return content;
    if (path.tag != K_STR || content.tag != K_STR) k_die("write_file takes strings");
    return k_mkdesc(5, path, content);
}

KValue k_maybe_bind(KValue piped, KValue closure) {
    if (piped.tag == K_DESC) return k_mkdesc(6, piped, closure);
    return k_call1(closure, piped);
}

static int k_argc_global = 0;
static char** k_argv_global = NULL;

static KValue k_exec(KDesc* d) {
    switch (d->dtag) {
        case 0: {
            KStr* s = k_as_str(d->x);
            fwrite(s->data, 1, s->len, stdout);
            fputc('\n', stdout);
            return k_none();
        }
        case 1: {
            KValue left = k_exec(k_as_desc(d->x));
            if (left.tag == K_ERR) return left;
            return k_exec(k_as_desc(d->y));
        }
        case 2: {
            long long n = k_argc_global > 1 ? k_argc_global - 1 : 0;
            KValue* items = k_alloc(sizeof(KValue) * (n ? n : 1));
            for (long long i = 0; i < n; i++) items[i] = k_str(k_argv_global[i + 1]);
            return k_mklist(n, items);
        }
        case 3: {
            size_t cap = 1 << 16, len = 0;
            char* data = malloc(cap);
            size_t got;
            while ((got = fread(data + len, 1, cap - len, stdin)) > 0) {
                len += got;
                if (len == cap) { cap *= 2; data = realloc(data, cap); }
            }
            KValue out = k_str_n(data, (long long)len);
            free(data);
            return out;
        }
        case 4: {
            KStr* p = k_as_str(d->x);
            FILE* fh = fopen(p->data, "rb");
            if (!fh) {
                return k_err(k_concat(k_concat(k_str("cannot read "), d->x),
                                      k_str(": no such file or unreadable")), NULL);
            }
            fseek(fh, 0, SEEK_END);
            long size = ftell(fh);
            fseek(fh, 0, SEEK_SET);
            char* data = malloc(size + 1);
            size_t got = fread(data, 1, size, fh);
            fclose(fh);
            KValue out = k_str_n(data, (long long)got);
            free(data);
            return out;
        }
        case 5: {
            KStr* p = k_as_str(d->x);
            KStr* c = k_as_str(d->y);
            FILE* fh = fopen(p->data, "wb");
            if (!fh) {
                return k_err(k_concat(k_str("cannot write "), d->x), NULL);
            }
            fwrite(c->data, 1, c->len, fh);
            fclose(fh);
            return k_none();
        }
        default: {
            KValue yielded = k_exec(k_as_desc(d->x));
            KValue next = k_call1(d->y, yielded);
            if (next.tag == K_DESC) return k_exec(k_as_desc(next));
            return next;
        }
    }
}

long long k_truthy(KValue v) {
    if (v.tag == K_TRUE) return 1;
    if (v.tag == K_FALSE) return 0;
    k_die("an if condition is true or false");
    return 0;
}

/* ---- slice 2: lists, maps, closures, builtins ---- */

static KValue* k_buf(long long cap) {
    KBuf* b = k_alloc(sizeof(KBuf) + sizeof(KValue) * cap);
    b->cap = cap;
    b->used = 0;
    return (KValue*)(b + 1);
}

static KBuf* k_buf_of(KValue* items) { return ((KBuf*)items) - 1; }

static KValue k_mklist(long long n, KValue* items) {
    KList* l = k_alloc_obj(sizeof(KList));
    l->len = n;
    l->items = k_buf(n ? n : 1);
    memcpy(l->items, items, sizeof(KValue) * n);
    k_buf_of(l->items)->used = n;
    KValue v; v.tag = K_LIST; v.payload = k_ptr(l); return v;
}

KValue k_list_lit(long long n, KValue* items) {
    return k_mklist(n, items);
}

KValue k_closure(KValue (*fn)(void*, KValue), long long ncaps, KValue* caps) {
    KClosure* c = k_alloc_obj(sizeof(KClosure));
    KValue* env = k_alloc(sizeof(KValue) * (ncaps ? ncaps : 1));
    memcpy(env, caps, sizeof(KValue) * ncaps);
    c->fn = fn; c->env = env;
    KValue v; v.tag = K_CLOSURE; v.payload = k_ptr(c); return v;
}

KValue k_fnref(void* dispatcher) {
    KValue v; v.tag = K_FNREF; v.payload = (long long)(intptr_t)dispatcher; return v;
}

KValue k_env_get(void* env, long long i) { return ((KValue*)env)[i]; }

static KValue k_call1(KValue f, KValue a) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        if (!k_not_failure(a)) return a;
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        return c->fn(c->env, a);
    }
    if (f.tag == K_FNREF) {
        return ((KValue(*)(KValue))(intptr_t)f.payload)(a);
    }
    k_die("this value is not callable");
    return k_none();
}

static int k_key_cmp(KValue a, KValue b) {
    if (a.tag != b.tag) return a.tag < b.tag ? -1 : 1;
    if (a.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    KStr* sa = k_as_str(a); KStr* sb = k_as_str(b);
    long n = sa->len < sb->len ? sa->len : sb->len;
    int c = memcmp(sa->data, sb->data, n);
    if (c) return c > 0 ? 1 : -1;
    return (sa->len > sb->len) - (sa->len < sb->len);
}

/* Sort indices by (key, insertion order) so equal keys stay ordered by when
   they were put — single-threaded, so a file-scope context pointer is fine. */
static KValue* k_msort_pairs;
static int k_msort_cmp(const void* pa, const void* pb) {
    long long ia = *(const long long*)pa, ib = *(const long long*)pb;
    int c = k_key_cmp(k_msort_pairs[ia * 2], k_msort_pairs[ib * 2]);
    if (c) return c;
    return ia < ib ? -1 : (ia > ib ? 1 : 0);
}

/* The canonical sorted+deduped view, computed once and cached on the map.
   Duplicate keys collapse keeping the last put (json's last-key-wins). */
static KValue* k_map_sorted(KMap* m, long long* out_len) {
    if (!m->sorted) {
        long long n = m->len;
        KValue* out = k_alloc(sizeof(KValue) * 2 * (n ? n : 1));
        if (n > 0) {
            long long* idx = k_alloc(sizeof(long long) * n);
            for (long long i = 0; i < n; i++) idx[i] = i;
            k_msort_pairs = m->pairs;
            qsort(idx, n, sizeof(long long), k_msort_cmp);
            long long w = 0;
            for (long long i = 0; i < n; i++) {
                long long si = idx[i];
                KValue k = m->pairs[si * 2], v = m->pairs[si * 2 + 1];
                if (w > 0 && k_key_cmp(k, out[(w - 1) * 2]) == 0) {
                    out[(w - 1) * 2 + 1] = v;
                } else {
                    out[w * 2] = k;
                    out[w * 2 + 1] = v;
                    w++;
                }
            }
            m->sorted_len = w;
        } else {
            m->sorted_len = 0;
        }
        m->sorted = out;
    }
    if (out_len) *out_len = m->sorted_len;
    return m->sorted;
}

KValue k_map_lit(long long n, KValue* flat_pairs) {
    /* literal keys arrive sorted and unique from the parser; k_map_sorted
       still recomputes on first read, cheaply (already sorted, no dups). */
    KMap* m = k_alloc_obj(sizeof(KMap));
    m->pairs = k_buf(2 * (n ? n : 1));
    memcpy(m->pairs, flat_pairs, sizeof(KValue) * 2 * n);
    k_buf_of(m->pairs)->used = 2 * n;
    m->len = n;
    m->sorted = NULL;
    m->sorted_len = 0;
    KValue mv; mv.tag = K_MAP; mv.payload = k_ptr(m); return mv;
}

KValue k_b_put(KValue mv, KValue key, KValue val) {
    if (!k_not_failure(mv)) return mv;
    if (!k_not_failure(key)) return key;
    if (!k_not_failure(val)) return val;
    if (mv.tag != K_MAP) k_die("put takes a map, a key, and a value");
    KMap* m = k_as_map(mv);
    KBuf* buf = k_buf_of(m->pairs);
    KMap* out = k_alloc_obj(sizeof(KMap));
    KValue ov; ov.tag = K_MAP; ov.payload = k_ptr(out);
    out->sorted = NULL;
    out->sorted_len = 0;
    if (buf->used == m->len * 2 && m->len * 2 + 2 <= buf->cap) {
        /* frontier-owned: claim the next pair slot in place (O(1)), leaving
           the key unsorted and any duplicate to be resolved on read */
        m->pairs[m->len * 2] = key;
        m->pairs[m->len * 2 + 1] = val;
        buf->used += 2;
        out->len = m->len + 1;
        out->pairs = m->pairs;
        return ov;
    }
    long long need = 2 * (m->len + 1);
    long long cap = need < 4 ? 4 : need * 2;
    KValue* np = k_buf(cap);
    memcpy(np, m->pairs, sizeof(KValue) * 2 * m->len);
    np[m->len * 2] = key;
    np[m->len * 2 + 1] = val;
    k_buf_of(np)->used = m->len * 2 + 2;
    out->len = m->len + 1;
    out->pairs = np;
    return ov;
}

KValue k_b_entries(KValue mv) {
    if (!k_not_failure(mv)) return mv;
    if (mv.tag != K_MAP) k_die("entries takes a map");
    KMap* m = k_as_map(mv);
    long long n;
    KValue* s = k_map_sorted(m, &n);
    KValue* items = k_alloc(sizeof(KValue) * (n ? n : 1));
    for (long long i = 0; i < n; i++) {
        KValue* fields = k_alloc(sizeof(KValue) * 2);
        fields[0] = s[i * 2];
        fields[1] = s[i * 2 + 1];
        items[i] = k_rec(0, 2, fields);
    }
    return k_mklist(n, items);
}

/* utf-8 helpers: kanso strings are opaque utf-8, positions are codepoints */
static long k_cp_len(unsigned char b) {
    if (b < 0x80) return 1;
    if (b < 0xe0) return 2;
    if (b < 0xf0) return 3;
    return 4;
}

static KValue k_bytes_view(const unsigned char* data, long long len) {
    KBytes* b = k_alloc_obj(sizeof(KBytes));
    b->len = len;
    b->data = data;
    KValue v; v.tag = K_BYTES; v.payload = k_ptr(b); return v;
}

KValue k_b_bytes(KValue sv) {
    if (!k_not_failure(sv)) return sv;
    if (sv.tag != K_STR) k_die("bytes takes a string");
    KStr* s = k_as_str(sv);
    return k_bytes_view((const unsigned char*)s->data, s->len);
}

static KValue k_view_to_list(KValue v) {
    if (v.tag != K_BYTES) return v;
    KBytes* b = k_as_bytes(v);
    KValue* items = k_alloc(sizeof(KValue) * (b->len ? b->len : 1));
    for (long long i = 0; i < b->len; i++) items[i] = k_int(b->data[i]);
    return k_mklist(b->len, items);
}

KValue k_b_concat(KValue av, KValue bv) {
    if (!k_not_failure(av)) return av;
    if (!k_not_failure(bv)) return bv;
    av = k_view_to_list(av);
    bv = k_view_to_list(bv);
    if (av.tag != K_LIST || bv.tag != K_LIST) k_die("concat takes two lists");
    KList* a = k_as_list(av);
    KList* b = k_as_list(bv);
    long long n = a->len + b->len;
    KValue* items = k_alloc(sizeof(KValue) * (n ? n : 1));
    memcpy(items, a->items, sizeof(KValue) * a->len);
    memcpy(items + a->len, b->items, sizeof(KValue) * b->len);
    return k_mklist(n, items);
}

static KValue k_utf8_check(char* data, long long len, const char* origin);

KValue k_b_utf8(KValue lv, const char* origin) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag == K_BYTES) {
        KBytes* b = k_as_bytes(lv);
        char* data = k_alloc(b->len + 1);
        memcpy(data, b->data, b->len);
        return k_utf8_check(data, b->len, origin);
    }
    if (lv.tag != K_LIST) k_die("utf8 takes a list of byte values");
    KList* l = k_as_list(lv);
    char* data = k_alloc(l->len + 1);
    for (long long i = 0; i < l->len; i++) {
        KValue item = l->items[i];
        if (!k_not_failure(item)) return item;
        if (item.tag != K_INT || item.payload < 0 || item.payload > 255) {
            return k_err(k_str("utf8 takes byte values (0-255)"), origin);
        }
        data[i] = (char)item.payload;
    }
    return k_utf8_check(data, l->len, origin);
}

static KValue k_utf8_check(char* data, long long len, const char* origin) {
    long long i = 0;
    while (i < len) {
        unsigned char b0 = (unsigned char)data[i];
        long w = b0 < 0x80 ? 1 : b0 < 0xc2 ? 0 : b0 < 0xe0 ? 2 : b0 < 0xf0 ? 3 : b0 < 0xf5 ? 4 : 0;
        if (w == 0 || i + w > len) return k_err(k_str("invalid utf-8"), origin);
        for (long j = 1; j < w; j++) {
            if (((unsigned char)data[i + j] & 0xc0) != 0x80) return k_err(k_str("invalid utf-8"), origin);
        }
        i += w;
    }
    return k_str_n(data, len);
}

KValue k_b_chars(KValue sv) {
    if (!k_not_failure(sv)) return sv;
    if (sv.tag != K_STR) k_die("chars takes a string");
    KStr* s = k_as_str(sv);
    long count = 0;
    for (long i = 0; i < s->len; i += k_cp_len((unsigned char)s->data[i])) count++;
    KValue* items = k_alloc(sizeof(KValue) * (count ? count : 1));
    long at = 0;
    for (long i = 0; i < count; i++) {
        long w = k_cp_len((unsigned char)s->data[at]);
        items[i] = k_str_n(s->data + at, w);
        at += w;
    }
    return k_mklist(count, items);
}

KValue k_b_at(KValue container, KValue index) {
    if (!k_not_failure(container)) return container;
    if (!k_not_failure(index)) return index;
    if (container.tag == K_LIST && index.tag == K_INT) {
        KList* l = k_as_list(container);
        long long i = index.payload;
        if (i < 1 || i > l->len) return k_none();
        return l->items[i - 1];
    }
    if (container.tag == K_STR && index.tag == K_INT) {
        KStr* s = k_as_str(container);
        long long want = index.payload;
        if (want < 1) return k_none();
        long at = 0;
        long long seen = 0;
        while (at < s->len) {
            long w = k_cp_len((unsigned char)s->data[at]);
            seen++;
            if (seen == want) return k_str_n(s->data + at, w);
            at += w;
        }
        return k_none();
    }
    if (container.tag == K_BYTES && index.tag == K_INT) {
        KBytes* b = k_as_bytes(container);
        long long i = index.payload;
        if (i < 1 || i > b->len) return k_none();
        return k_int(b->data[i - 1]);
    }
    if (container.tag == K_MAP) {
        KMap* m = k_as_map(container);
        long long n;
        KValue* s = k_map_sorted(m, &n);
        long long lo = 0, hi = n - 1;
        while (lo <= hi) {
            long long mid = (lo + hi) / 2;
            int c = k_key_cmp(index, s[mid * 2]);
            if (c == 0) return s[mid * 2 + 1];
            if (c < 0) hi = mid - 1; else lo = mid + 1;
        }
        return k_none();
    }
    k_die("at takes a list or string with a 1-based position, or a map with a key");
    return k_none();
}

KValue k_index(KValue container, KValue key, const char* origin) {
    KValue found = k_b_at(container, key);
    if (found.tag == K_NONE) {
        return k_err(k_concat(k_str("missing index "), k_render(key, 1)), origin);
    }
    return found;
}

KValue k_b_push(KValue lv, KValue item) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag != K_LIST) k_die("push takes a list and a value");
    KList* l = k_as_list(lv);
    KBuf* buf = k_buf_of(l->items);
    if (buf->used == l->len && l->len < buf->cap) {
        /* this list is the frontier of its buffer: claim the next slot */
        l->items[l->len] = item;
        buf->used++;
        KList* out = k_alloc_obj(sizeof(KList));
        out->len = l->len + 1;
        out->items = l->items;
        KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
    }
    long long cap = l->len < 2 ? 4 : l->len * 2;
    KValue* items = k_buf(cap);
    memcpy(items, l->items, sizeof(KValue) * l->len);
    items[l->len] = item;
    k_buf_of(items)->used = l->len + 1;
    KList* out = k_alloc_obj(sizeof(KList));
    out->len = l->len + 1;
    out->items = items;
    KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
}

KValue k_b_length(KValue v) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_LIST) return k_int(k_as_list(v)->len);
    if (v.tag == K_BYTES) return k_int(k_as_bytes(v)->len);
    if (v.tag == K_MAP) {
        long long n;
        k_map_sorted(k_as_map(v), &n);
        return k_int(n);
    }
    if (v.tag == K_STR) {
        KStr* s = k_as_str(v);
        long count = 0;
        for (long i = 0; i < s->len; i += k_cp_len((unsigned char)s->data[i])) count++;
        return k_int(count);
    }
    k_die("length takes a list or string");
    return k_none();
}

KValue k_b_slice(KValue container, KValue fromv, KValue tov) {
    if (!k_not_failure(container)) return container;
    if (!k_not_failure(fromv)) return fromv;
    if (!k_not_failure(tov)) return tov;
    if (fromv.tag != K_INT || tov.tag != K_INT) k_die("slice takes 1-based inclusive positions");
    long long from = fromv.payload, to = tov.payload;
    if (container.tag == K_BYTES) {
        KBytes* b = k_as_bytes(container);
        if (from < 1 || from > to || to > b->len) return k_bytes_view(b->data, 0);
        return k_bytes_view(b->data + (from - 1), to - from + 1);
    }
    if (container.tag == K_LIST) {
        KList* l = k_as_list(container);
        if (from < 1 || from > to || to > l->len) return k_mklist(0, NULL);
        return k_mklist(to - from + 1, l->items + (from - 1));
    }
    if (container.tag == K_STR) {
        KStr* s = k_as_str(container);
        long start = -1, end = -1, at = 0;
        long long seen = 0;
        while (at <= s->len) {
            seen++;
            if (seen == from) start = at;
            if (seen == to + 1) { end = at; break; }
            if (at == s->len) break;
            at += k_cp_len((unsigned char)s->data[at]);
        }
        if (from < 1 || from > to || start < 0) return k_str_n("", 0);
        if (end < 0) end = s->len;
        if (seen < to) return k_str_n("", 0);
        return k_str_n(s->data + start, end - start);
    }
    k_die("slice takes a list or string");
    return k_none();
}

KValue k_b_join(KValue lv, KValue sep) {
    if (!k_not_failure(lv)) return lv;
    if (!k_not_failure(sep)) return sep;
    if (lv.tag != K_LIST || sep.tag != K_STR) k_die("join takes a list of strings and a separator");
    KList* l = k_as_list(lv);
    KStr* ss = k_as_str(sep);
    long total = 0;
    for (long long i = 0; i < l->len; i++) {
        if (!k_not_failure(l->items[i])) return l->items[i];
        if (l->items[i].tag != K_STR) k_die("join takes a list of strings");
        total += k_as_str(l->items[i])->len;
        if (i) total += ss->len;
    }
    char* data = k_alloc(total + 1);
    long at = 0;
    for (long long i = 0; i < l->len; i++) {
        if (i) { memcpy(data + at, ss->data, ss->len); at += ss->len; }
        KStr* is = k_as_str(l->items[i]);
        memcpy(data + at, is->data, is->len);
        at += is->len;
    }
    KValue out = k_str_n(data, total);
    return out;
}

KValue k_b_map(KValue lv, KValue f) {
    if (!k_not_failure(lv)) return lv;
    if (!k_not_failure(f)) return f;
    if (lv.tag != K_LIST) k_die("map takes a list");
    KList* l = k_as_list(lv);
    KValue* items = k_alloc(sizeof(KValue) * (l->len ? l->len : 1));
    for (long long i = 0; i < l->len; i++) items[i] = k_call1(f, l->items[i]);
    return k_mklist(l->len, items);
}

KValue k_b_filter(KValue lv, KValue f) {
    if (!k_not_failure(lv)) return lv;
    if (!k_not_failure(f)) return f;
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
        if (l->items[i].tag != K_INT) k_die("sum takes a list of int");
        long long r;
        if (__builtin_add_overflow(total, l->items[i].payload, &r)) k_die("integer overflow (int64 native build; spec int is arbitrary precision)");
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
    if (nv.tag != K_INT) k_die("from_code takes an int");
    long long cp = nv.payload;
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
KValue k_b_to_int(KValue sv, const char* origin) {
    if (!k_not_failure(sv)) return sv;
    if (sv.tag == K_INT) return sv;
    if (sv.tag != K_STR && sv.tag != K_BYTES) k_die("to_int takes a string");
    const char* data;
    long long len;
    if (sv.tag == K_STR) { KStr* s = k_as_str(sv); data = s->data; len = s->len; }
    else { KBytes* b = k_as_bytes(sv); data = (const char*)b->data; len = b->len; }
    char* end = NULL;
    long long n = strtoll(data, &end, 10);
    if (len == 0 || end != data + len) {
        KValue str = k_str_n(data, len);
        return k_err(k_concat(k_concat(k_str("\""), str), k_str("\" is not an integer")), origin);
    }
    return k_int(n);
}

KValue k_b_to_float(KValue v, const char* origin) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_FLOAT) return v;
    if (v.tag == K_INT) return k_float((double)v.payload);
    if (v.tag != K_STR && v.tag != K_BYTES) k_die("to_float takes a string or int");
    const char* data;
    long long len;
    if (v.tag == K_STR) { KStr* s = k_as_str(v); data = s->data; len = s->len; }
    else { KBytes* b = k_as_bytes(v); data = (const char*)b->data; len = b->len; }
    char* end = NULL;
    double d = strtod(data, &end);
    if (len == 0 || end != data + len) {
        KValue str = k_str_n(data, len);
        return k_err(k_concat(k_concat(k_str("\""), str), k_str("\" is not a number")), origin);
    }
    return k_float(d);
}

extern KValue k_user_main(void);

static void k_report_err(KValue e, const char* reached) {
    KValue r = k_render(k_err_inner(e), 1);
    fprintf(stderr, "%serror[endpoint]:%s unhandled err reached %s: %s\n",
            k_c_err(), k_c_off(), reached, k_as_str(r)->data);
    KErrBox* box = k_err_box(e);
    if (box->origin) {
        fprintf(stderr, "%s  born in %s%s\n", k_c_dim(), box->origin, k_c_off());
    }
    if (box->hops) {
        fprintf(stderr, "%s  passed through ", k_c_dim());
        for (KHop* hop = box->hops; hop; hop = hop->prev) {
            fprintf(stderr, hop->prev ? "%s \xe2\x86\x90 " : "%s", hop->fn);
        }
        fprintf(stderr, "%s\n", k_c_off());
    }
}

int main(int argc, char** argv) {
    k_argc_global = argc;
    k_argv_global = argv;
    KValue v = k_user_main();
    if (v.tag == K_DESC) {
        KValue y = k_exec(k_as_desc(v));
        if (y.tag == K_ERR) {
            k_report_err(y, "the executor");
            return 1;
        }
        return 0;
    }
    if (v.tag == K_ERR) {
        k_report_err(v, "main");
        return 1;
    }
    if (v.tag == K_NONE) {
        fprintf(stderr, "%serror[endpoint]:%s unhandled none reached main\n", k_c_err(), k_c_off());
        return 1;
    }
    return 0;
}
