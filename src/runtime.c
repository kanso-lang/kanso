#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>
#include <unistd.h>

#if defined(__aarch64__)
#include <arm_neon.h>
#elif defined(__x86_64__)
#include <emmintrin.h>
#endif

/* ABI shared with emitted LLVM IR: %KValue = type { i64, i64 } */
typedef struct { long long tag; long long payload; } KValue;

enum { K_INT, K_FLOAT, K_TRUE, K_FALSE, K_NONE, K_ERR, K_STR, K_REC, K_DESC, K_LIST, K_MAP, K_CLOSURE, K_FNREF, K_BYTES, K_THUNK };

typedef struct { long len; char* data; } KStr;
typedef struct { long long cap; long long used; } KBuf;
/* cap == 0 is a borrowed view; cap != 0 marks data as the body of a
   KBuf-headed buffer this value may extend at its frontier. */
typedef struct { long long len; const unsigned char* data; long long cap; } KBytes;

/* Lazy v1 (design/lazy-v1-plan.md): a conditionally-demanded binding's
   pending computation. RC'd, malloc-backed, recycled through a free list --
   never the beat arenas, so a pending thunk can't pin a rewindable region.
   Captured args are copied in at creation; the site dispatcher
   (codegen-emitted d_thunk_eval) runs the computation at first force. */
typedef struct KThunk {
    long long rc;
    long long site;
    int forced;
    int argc;
    KValue result;
    struct KThunk* next_free;
    KValue args[8];
} KThunk;

static KThunk* k_thunk_free = 0;
static long long k_stat_thunk_allocs = 0;
static long long k_stat_thunk_forces = 0;
static long long k_stat_thunk_evals = 0;
static long long k_stat_thunk_frees = 0;
static long long k_stat_thunk_escaped = 0;
static long long k_stat_el_parses = 0;

extern KValue d_thunk_eval(long long site, KValue* args);

KValue k_thunk_new(long long site, int argc, ...) {
    KThunk* t = k_thunk_free;
    if (t) {
        k_thunk_free = t->next_free;
    } else {
        t = (KThunk*)malloc(sizeof(KThunk));
    }
    k_stat_thunk_allocs++;
    t->rc = 1;
    t->site = site;
    t->forced = 0;
    t->argc = argc;
    t->next_free = 0;
    va_list ap;
    va_start(ap, argc);
    for (int i = 0; i < argc; i++) {
        t->args[i] = va_arg(ap, KValue);
        /* a cell holding another cell keeps it alive */
        if (t->args[i].tag == K_THUNK) ((KThunk*)t->args[i].payload)->rc++;
    }
    va_end(ap);
    KValue v;
    v.tag = K_THUNK;
    v.payload = (long long)t;
    return v;
}

static void k_thunk_drop_args(KThunk* t);

static void k_thunk_release_cell(KThunk* t) {
    if (--t->rc > 0) return;
    k_thunk_drop_args(t);
    t->next_free = k_thunk_free;
    k_thunk_free = t;
    k_stat_thunk_frees++;
}

static void k_thunk_drop_args(KThunk* t) {
    for (int i = 0; i < t->argc; i++) {
        if (t->args[i].tag == K_THUNK) k_thunk_release_cell((KThunk*)t->args[i].payload);
    }
    t->argc = 0;
}

/* The frame epilogue for a releasable lazy binding: free the cell unless
   the frame's result IS the cell — the returned-thunk case, which escapes
   upward and is counted rather than freed. */
/* A cell handed onward in a tail call outlives its frame by design;
   count it so live-to-exit cells are always attributable. */
void k_thunk_note_escape(KValue cell) {
    if (cell.tag == K_THUNK) k_stat_thunk_escaped++;
}

KValue k_thunk_release_unless(KValue cell, KValue result) {
    if (cell.tag != K_THUNK) return result;
    if (result.tag == K_THUNK && result.payload == cell.payload) {
        k_stat_thunk_escaped++;
        return result;
    }
    k_thunk_release_cell((KThunk*)cell.payload);
    return result;
}

KValue k_render(KValue v, long long quote);
KValue k_b_render_value(KValue v) {
    return k_render(v, 0);
}

KValue k_force(KValue v) {
    if (v.tag != K_THUNK) return v;
    KThunk* t = (KThunk*)v.payload;
    k_stat_thunk_forces++;
    if (!t->forced) {
        k_stat_thunk_evals++;
        t->result = d_thunk_eval(t->site, t->args);
        t->forced = 1;
        /* the computation ran; its captures are done */
        k_thunk_drop_args(t);
    }
    return t->result;
}

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
typedef struct { KValue (*fn)(void*, KValue); void* env; long long ncaps; } KClosure;
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
static KValue* k_buf(long long cap);
static KValue k_list_own(KValue* items, long long n);
KValue k_call1(KValue f, KValue a);
static KValue* k_map_sorted(KMap* m, long long* out_len);

/* The arena is a chain of blocks, newest first; allocation bumps in the head
   block. k_beat_boundary rewinds the chain to a snapshot taken on its first
   call, retiring newer blocks to a spare pool for reuse — a steady-state loop
   recycles the same warm pages instead of marching through cold memory. If no
   boundary is ever signalled the arena only grows, exactly as before. */
typedef struct KBlock { struct KBlock* next; size_t cap; } KBlock;
static KBlock* k_blocks = NULL;
static KBlock* k_spare = NULL;
static char* k_arena = NULL;
static size_t k_arena_left = 0;

/* Cost counters: every value is an exact, machine-independent constant for a
   deterministic program, so they golden like output does. KANSO_COUNTERS=1
   dumps them to stderr at exit; CI diffs the dump against a committed cost
   golden — a performance ratchet with no clock in it. */
static long long k_stat_allocs = 0;
static long long k_stat_alloc_bytes = 0;
static long long k_stat_blocks = 0;
static long long k_stat_perm_allocs = 0;
static long long k_stat_beat_iters = 0;

static void k_stats_dump(void) {
    fprintf(stderr, "allocs=%lld\n", k_stat_allocs);
    fprintf(stderr, "alloc_bytes=%lld\n", k_stat_alloc_bytes);
    fprintf(stderr, "arena_blocks=%lld\n", k_stat_blocks);
    fprintf(stderr, "perm_allocs=%lld\n", k_stat_perm_allocs);
    fprintf(stderr, "beat_iters=%lld\n", k_stat_beat_iters);
    fprintf(stderr,
        "thunk_allocs=%lld\nthunk_forces=%lld\nthunk_evals=%lld\n"
        "thunk_frees=%lld\nthunk_escaped=%lld\nthunk_live_exit=%lld\n"
        "el_parses=%lld\n",
        k_stat_thunk_allocs, k_stat_thunk_forces, k_stat_thunk_evals,
        k_stat_thunk_frees, k_stat_thunk_escaped,
        k_stat_thunk_allocs - k_stat_thunk_frees, k_stat_el_parses);
}

static void k_arena_push(size_t need) {
    KBlock** link = &k_spare;
    while (*link && (*link)->cap < need) link = &(*link)->next;
    KBlock* b = *link;
    if (b) {
        *link = b->next;
    } else {
        b = malloc(sizeof(KBlock) + need);
        if (!b) { fputs("out of memory\n", stderr); exit(1); }
        b->cap = need;
        k_stat_blocks++;
    }
    b->next = k_blocks;
    k_blocks = b;
    k_arena = (char*)(b + 1);
    k_arena_left = b->cap;
}

static int k_stats_on = -1;

/* The refill path stays out of line; the bump inlines into every hot
   caller. Counters, when enabled, are exact: both paths count. */
static __attribute__((noinline)) void* k_alloc_refill(size_t n) {
    k_arena_push(n > (1 << 20) ? n : (size_t)(1 << 20));
    void* p = k_arena;
    k_arena += n;
    k_arena_left -= n;
    return p;
}

static inline __attribute__((always_inline)) void* k_alloc(size_t n) {
    n = (n + 15) & ~(size_t)15;
    if (__builtin_expect(k_stats_on != 0, 0)) {
        if (k_stats_on < 0) k_stats_on = getenv("KANSO_COUNTERS") != NULL;
        if (k_stats_on) {
            k_stat_allocs++;
            k_stat_alloc_bytes += (long long)n;
        }
    }
    if (__builtin_expect(n > k_arena_left, 0)) {
        return k_alloc_refill(n);
    }
    void* p = k_arena;
    k_arena += n;
    k_arena_left -= n;
    return p;
}

/* Beat marks form a stack, bracketing every entry into a compiler-proven
   beat loop. k_beat_push snapshots the frontier at entry; k_beat_iter rewinds
   to the innermost mark between iterations (the analysis guarantees the only
   values crossing an iteration are entry-threaded or scalar); k_beat_pop
   closes the entry, rewinding only when the loop's result is a non-heap
   scalar — a heap result keeps its region alive (a leak until an outer beat,
   traded knowingly for never freeing live data). Every operation is O(1) and
   every reclamation is a pointer reset; retired blocks recycle warm through
   the spare pool. Programs with no beat loop never call any of this and the
   arena only grows, exactly as before. */
static int k_is_heap(long long tag);

typedef struct { KBlock* block; char* ptr; size_t left; } KMark;
static void k_cache_reg_sweep(KMark* mark);
#define K_BEAT_MAX 64
static KMark k_beat_stack[K_BEAT_MAX];
static int k_beat_depth = 0;

static void k_beat_rewind(KMark* m) {
    while (k_blocks != m->block) {
        KBlock* b = k_blocks;
        k_blocks = b->next;
        b->next = k_spare;
        k_spare = b;
    }
    k_arena = m->ptr;
    k_arena_left = m->left;
}

void k_carry_clear(int depth);

void k_beat_push(void) {
    if (k_beat_depth < K_BEAT_MAX) {
        KMark* m = &k_beat_stack[k_beat_depth];
        m->block = k_blocks;
        m->ptr = k_arena;
        m->left = k_arena_left;
        k_carry_clear(k_beat_depth);
    }
    k_beat_depth++;
}

void k_beat_iter(void) {
    k_stat_beat_iters++;
    if (k_beat_depth > 0 && k_beat_depth <= K_BEAT_MAX) {
        k_cache_reg_sweep(&k_beat_stack[k_beat_depth - 1]);
        k_beat_rewind(&k_beat_stack[k_beat_depth - 1]);
    }
}

KValue k_beat_pop(KValue r);

/* The fold carry. A carry beat's loop-varying arguments (typically one
   accumulator) are freshly built each iteration, so the plain rewind would
   free them. Before rewinding, each staged value is deep-copied into a
   malloc'd carry buffer — copying runs strictly before the rewind, so source
   (arena) and destination (buffer) never overlap and no pointer needs
   rebasing. Two buffers per beat depth alternate: the deep copy severs all
   sharing with the previous carry, so the buffer it lived in retires the
   moment the new copy completes. Values already below the mark (the arena
   that survives the rewind) are shared, not copied — a threaded list inside
   a carried record costs nothing per iteration. At the pop, a heap result is
   copied out into the caller's arena and both buffers are freed. */
static long long k_ptr(void* p);

#define K_CARRY_MAX 8

typedef struct { char* data; size_t cap; size_t used; } KCarryBuf;
typedef struct { KCarryBuf from; KCarryBuf to; int used_flag; } KCarry;
static KCarry k_carries[K_BEAT_MAX];
static KValue k_carry_slots[K_CARRY_MAX];
static long long k_carry_n = 0;

/* Does p survive the innermost rewind — is it inside the live chain at or
   below the mark? mark == NULL means "the whole live chain", the test the
   pop's copy-out wants. */
static int k_survives(const void* p, KMark* m) {
    const char* q = (const char*)p;
    KBlock* b = m ? m->block : k_blocks;
    const char* frontier = m ? m->ptr : k_arena;
    for (; b; b = b->next) {
        const char* start = (const char*)(b + 1);
        const char* end = (b == (m ? m->block : k_blocks)) ? frontier : start + b->cap;
        if (q >= start && q < end) return 1;
        frontier = NULL;
    }
    return 0;
}

/* Sorted-view caches filled during a beat point above the mark; a rewind
   frees the view while the map header — below the mark, inside data the
   loop legitimately threads — survives holding the stale pointer. Fills
   register here, and every rewind resets the caches it just freed. */
#define K_CACHE_REG_MAX 65536
static KMap** k_cache_reg = NULL;
static int k_cache_reg_cap = 0;
static int k_cache_reg_n = 0;

static void k_cache_reg_add(KMap* m) {
    if (k_beat_depth <= 0) return;
    if (k_cache_reg_n == k_cache_reg_cap) {
        int cap = k_cache_reg_cap ? k_cache_reg_cap * 2 : 1024;
        if (cap > K_CACHE_REG_MAX) return;
        k_cache_reg = realloc(k_cache_reg, sizeof(KMap*) * cap);
        if (!k_cache_reg) { fputs("out of memory\n", stderr); exit(1); }
        k_cache_reg_cap = cap;
    }
    k_cache_reg[k_cache_reg_n++] = m;
}

static void k_cache_reg_sweep(KMark* mark) {
    int resets = 0;
    int w = 0;
    for (int i = 0; i < k_cache_reg_n; i++) {
        KMap* m = k_cache_reg[i];
        if (!k_survives(m, mark)) {
            continue; /* the header itself is being freed */
        }
        if (m->sorted && !k_survives(m->sorted, mark)) {
            m->sorted = NULL;
            m->sorted_len = 0;
            resets++;
            continue;
        }
        k_cache_reg[w++] = m;
    }
    k_cache_reg_n = w;
    (void)resets;
}

typedef struct { KCarryBuf* buf; KMark* mark; int to_arena; } KCopy;

static void* k_copy_alloc(KCopy* cp, size_t n) {
    n = (n + 15) & ~(size_t)15;
    if (cp->to_arena) return k_alloc(n);
    void* p = cp->buf->data + cp->buf->used;
    cp->buf->used += n;
    return p;
}

static size_t k_copy_size(KValue v, KMark* m);

static size_t k_copy_size_ptr(const void* p, size_t n, KMark* m) {
    (void)p;
    return (n + 15) & ~(size_t)15;
}

static size_t k_copy_size(KValue v, KMark* m) {
    if (!k_is_heap(v.tag)) return 0;
    const void* p = (const void*)(intptr_t)v.payload;
    if (k_survives(p, m)) return 0;
    size_t n = 0;
    switch (v.tag) {
        case K_STR: {
            KStr* s = (KStr*)p;
            n += k_copy_size_ptr(s, sizeof(KStr), m);
            if (!k_survives(s->data, m)) n += k_copy_size_ptr(s->data, (size_t)s->len + 1, m);
            break;
        }
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            n += k_copy_size_ptr(b, sizeof(KBytes), m);
            if (!k_survives(b->data, m)) n += k_copy_size_ptr(b->data, (size_t)b->len, m);
            break;
        }
        case K_LIST: {
            KList* l = (KList*)p;
            n += k_copy_size_ptr(l, sizeof(KList), m);
            n += k_copy_size_ptr(l->items, sizeof(KBuf) + sizeof(KValue) * (size_t)(l->len ? l->len : 1), m);
            for (long long i = 0; i < l->len; i++) n += k_copy_size(l->items[i], m);
            break;
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            n += k_copy_size_ptr(mp, sizeof(KMap), m);
            n += k_copy_size_ptr(mp->pairs, sizeof(KBuf) + sizeof(KValue) * (size_t)(2 * (mp->len ? mp->len : 1)), m);
            for (long long i = 0; i < 2 * mp->len; i++) n += k_copy_size(mp->pairs[i], m);
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            n += k_copy_size_ptr(r, sizeof(KRec), m);
            n += k_copy_size_ptr(r->fields, sizeof(KValue) * (size_t)(r->nfields ? r->nfields : 1), m);
            for (long long i = 0; i < r->nfields; i++) n += k_copy_size(r->fields[i], m);
            break;
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            n += k_copy_size_ptr(cl, sizeof(KClosure), m);
            n += k_copy_size_ptr(cl->env, sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1), m);
            for (long long i = 0; i < cl->ncaps; i++) n += k_copy_size(((KValue*)cl->env)[i], m);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            n += k_copy_size_ptr(d, sizeof(KDesc), m);
            n += k_copy_size(d->x, m);
            n += k_copy_size(d->y, m);
            break;
        }
        case K_ERR: {
            KErrBox* e = (KErrBox*)p;
            n += k_copy_size_ptr(e, sizeof(KErrBox), m);
            n += k_copy_size(e->reason, m);
            for (KHop* h = e->hops; h && !k_survives(h, m); h = h->prev)
                n += k_copy_size_ptr(h, sizeof(KHop), m);
            break;
        }
        default: break;
    }
    return n;
}

static KValue k_deep_copy(KValue v, KCopy* cp) {
    if (!k_is_heap(v.tag)) return v;
    void* p = (void*)(intptr_t)v.payload;
    if (k_survives(p, cp->mark)) return v;
    KValue out = v;
    switch (v.tag) {
        case K_STR: {
            KStr* s = (KStr*)p;
            KStr* ns = k_copy_alloc(cp, sizeof(KStr));
            ns->len = s->len;
            if (k_survives(s->data, cp->mark)) {
                ns->data = s->data;
            } else {
                ns->data = k_copy_alloc(cp, (size_t)s->len + 1);
                memcpy(ns->data, s->data, (size_t)s->len + 1);
            }
            out.payload = k_ptr(ns);
            break;
        }
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            KBytes* nb = k_copy_alloc(cp, sizeof(KBytes));
            nb->len = b->len;
            nb->cap = 0;
            if (k_survives(b->data, cp->mark)) {
                nb->data = b->data;
            } else {
                unsigned char* d = k_copy_alloc(cp, (size_t)b->len);
                memcpy(d, b->data, (size_t)b->len);
                nb->data = d;
            }
            out.payload = k_ptr(nb);
            break;
        }
        case K_LIST: {
            KList* l = (KList*)p;
            KList* nl = k_copy_alloc(cp, sizeof(KList));
            KBuf* buf = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)(l->len ? l->len : 1));
            buf->cap = l->len ? l->len : 1;
            buf->used = l->len;
            KValue* items = (KValue*)(buf + 1);
            for (long long i = 0; i < l->len; i++) items[i] = k_deep_copy(l->items[i], cp);
            nl->len = l->len;
            nl->items = items;
            out.payload = k_ptr(nl);
            break;
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            KMap* nm = k_copy_alloc(cp, sizeof(KMap));
            KBuf* buf = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)(2 * (mp->len ? mp->len : 1)));
            buf->cap = 2 * (mp->len ? mp->len : 1);
            buf->used = 2 * mp->len;
            KValue* pairs = (KValue*)(buf + 1);
            for (long long i = 0; i < 2 * mp->len; i++) pairs[i] = k_deep_copy(mp->pairs[i], cp);
            nm->len = mp->len;
            nm->pairs = pairs;
            nm->sorted = NULL;
            nm->sorted_len = 0;
            out.payload = k_ptr(nm);
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            KRec* nr = k_copy_alloc(cp, sizeof(KRec));
            KValue* fields = k_copy_alloc(cp, sizeof(KValue) * (size_t)(r->nfields ? r->nfields : 1));
            for (long long i = 0; i < r->nfields; i++) fields[i] = k_deep_copy(r->fields[i], cp);
            nr->type_id = r->type_id;
            nr->nfields = r->nfields;
            nr->fields = fields;
            out.payload = k_ptr(nr);
            break;
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            KClosure* nc = k_copy_alloc(cp, sizeof(KClosure));
            KValue* env = k_copy_alloc(cp, sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1));
            for (long long i = 0; i < cl->ncaps; i++) env[i] = k_deep_copy(((KValue*)cl->env)[i], cp);
            nc->fn = cl->fn;
            nc->env = env;
            nc->ncaps = cl->ncaps;
            out.payload = k_ptr(nc);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            KDesc* nd = k_copy_alloc(cp, sizeof(KDesc));
            nd->dtag = d->dtag;
            nd->x = k_deep_copy(d->x, cp);
            nd->y = k_deep_copy(d->y, cp);
            out.payload = k_ptr(nd);
            break;
        }
        case K_ERR: {
            KErrBox* e = (KErrBox*)p;
            KErrBox* ne = k_copy_alloc(cp, sizeof(KErrBox));
            ne->reason = k_deep_copy(e->reason, cp);
            ne->origin = e->origin;
            KHop** tail = &ne->hops;
            KHop* h = e->hops;
            for (; h && !k_survives(h, cp->mark); h = h->prev) {
                KHop* nh = k_copy_alloc(cp, sizeof(KHop));
                nh->fn = h->fn;
                *tail = nh;
                tail = &nh->prev;
            }
            *tail = h;
            out.payload = k_ptr(ne);
            break;
        }
        default: break;
    }
    return out;
}

void k_carry_clear(int depth) {
    /* buffers persist across entries at a depth for reuse; only the flag
       and fill levels reset */
    k_carries[depth].used_flag = 0;
    k_carries[depth].from.used = 0;
    k_carries[depth].to.used = 0;
}

/* Closing a beat entry. A non-heap result rewinds as always. A heap result
   keeps the region alive — and if the loop carried, the result may live in
   a carry buffer, so it is copied out into the caller's arena before the
   buffers go idle. */
KValue k_beat_pop(KValue r) {
    if (k_beat_depth > 0) {
        k_beat_depth--;
        if (k_beat_depth < K_BEAT_MAX) {
            KCarry* c = &k_carries[k_beat_depth];
            if (!k_is_heap(r.tag)) {
                k_cache_reg_sweep(&k_beat_stack[k_beat_depth]);
                k_beat_rewind(&k_beat_stack[k_beat_depth]);
            } else if (c->used_flag) {
                KCopy cp = { NULL, NULL, 1 };
                r = k_deep_copy(r, &cp);
            }
            c->used_flag = 0;
        }
    }
    return r;
}

void k_carry_reset(void) { k_carry_n = 0; }

void k_carry_stage(KValue v) {
    if (k_carry_n < K_CARRY_MAX) k_carry_slots[k_carry_n] = v;
    k_carry_n++;
}

KValue k_carry_take(long long i) { return k_carry_slots[i]; }

void k_beat_iter_carry(void) {
    k_stat_beat_iters++;
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return;
    for (long long i = 0; i < k_carry_n; i++) {
        long long tag = k_carry_slots[i].tag;
        if (tag == 4 || tag == 5) return; /* failure: the callee propagates it */
    }
    KMark* m = &k_beat_stack[k_beat_depth - 1];
    KCarry* c = &k_carries[k_beat_depth - 1];
    size_t need = 0;
    for (long long i = 0; i < k_carry_n; i++) need += k_copy_size(k_carry_slots[i], m);
    if (c->to.cap < need) {
        free(c->to.data);
        c->to.data = malloc(need ? need : 16);
        if (!c->to.data) { fputs("out of memory\n", stderr); exit(1); }
        c->to.cap = need ? need : 16;
    }
    c->to.used = 0;
    KCopy cp = { &c->to, m, 0 };
    for (long long i = 0; i < k_carry_n; i++)
        k_carry_slots[i] = k_deep_copy(k_carry_slots[i], &cp);
    k_cache_reg_sweep(m);
    k_beat_rewind(m);
    KCarryBuf swap = c->from;
    c->from = c->to;
    c->to = swap;
    c->used_flag = 1;
}

/* A permanent object: malloc'd, so it lives outside the beat arena and
   survives every rewind. Interned single-char strings and zero-field marker
   records are cached and reused across beats — an arena rewind moves the
   bump pointer, so caching arena storage and reusing it after a pop hands
   back reclaimed, since-reused memory. Permanent storage is the only cache
   that is sound across beats. */
static void* k_alloc_perm(size_t n) {
    k_stat_perm_allocs++;
    void* p = malloc(n);
    if (!p) { fputs("out of memory\n", stderr); exit(1); }
    return p;
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

/* Hand-rolled lld formatting: the vfprintf machinery showed up hot in
   the encode profile, and a digit loop beats it several times over. */
static void k_itoa(char* buf, long long v) {
    char tmp[24];
    int n = 0;
    unsigned long long u = v < 0 ? (unsigned long long)(-(v + 1)) + 1 : (unsigned long long)v;
    do {
        tmp[n++] = (char)('0' + (u % 10));
        u /= 10;
    } while (u);
    char* w = buf;
    if (v < 0) *w++ = '-';
    while (n) *w++ = tmp[--n];
    *w = 0;
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
                KStr* s = k_alloc_perm(sizeof(KStr));
                s->len = 1;
                s->data = malloc(2);
                s->data[0] = (char)b;
                s->data[1] = 0;
                KValue v; v.tag = K_STR; v.payload = k_ptr(s);
                k_ascii_cache[b] = v;
                k_ascii_ready[b] = 1;
            }
            return k_ascii_cache[b];
        }
    }
    KStr* s = k_alloc(sizeof(KStr));
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
    KErrBox* box = k_alloc(sizeof(KErrBox));
    box->reason = reason;
    box->origin = origin;
    box->hops = NULL;
    KValue v; v.tag = K_ERR; v.payload = k_ptr(box); return v;
}

/* A dispatcher passing a failure through appends its name; none stays bare. */
KValue k_err_hop(KValue v, const char* fn) {
    if (v.tag != K_ERR) return v;
    KErrBox* old = k_err_box(v);
    KErrBox* box = k_alloc(sizeof(KErrBox));
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
            KRec* r = k_alloc_perm(sizeof(KRec));
            r->type_id = type_id;
            r->nfields = 0;
            r->fields = NULL;
            KValue v; v.tag = K_REC; v.payload = k_ptr(r);
            k_marker_cache[type_id] = v;
            k_marker_ready[type_id] = 1;
        }
        return k_marker_cache[type_id];
    }
    KRec* r = k_alloc(sizeof(KRec));
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

/* One allocation for a whole template: sums the piece lengths, copies
   once. A failure piece propagates; the profile showed chained k_concat
   quadratic-copying hot on the encode path. An array, not varargs —
   16-byte structs through va_arg differ between the arm64 and x86_64
   ABIs when the caller is emitted IR. */
KValue k_concat_arr(long long n, const KValue* parts) {
    long long total = 0;
    for (long long i = 0; i < n; i++) {
        KValue p = parts[i];
        if (!k_not_failure(p)) return p;
        total += k_as_str(p)->len;
    }
    KStr* s = k_alloc(sizeof(KStr));
    s->len = total;
    s->data = k_alloc(total + 1);
    long long at = 0;
    for (long long i = 0; i < n; i++) {
        KStr* ps = k_as_str(parts[i]);
        memcpy(s->data + at, ps->data, ps->len);
        at += ps->len;
    }
    s->data[total] = 0;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s);
    return v;
}

KValue k_concat(KValue a, KValue b) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    KStr* sa = k_as_str(a);
    KStr* sb = k_as_str(b);
    KStr* s = k_alloc(sizeof(KStr));
    s->len = sa->len + sb->len;
    s->data = k_alloc(s->len + 1);
    memmove(s->data, sa->data, sa->len);
    memmove(s->data + sa->len, sb->data, sb->len);
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

/* `.` field access: failures ride through; a non-record dies loudly. */
KValue k_b_field(KValue v, const char* name) {
    if (!k_not_failure(v)) return v;
    if (v.tag != K_REC) k_die("`.` reads a field of a record");
    KRec* r = k_as_rec(v);
    for (long long i = 0; i < r->nfields; i++) {
        if (!strcmp(k_type_field_name(r->type_id, i), name)) return r->fields[i];
    }
    k_die("no such field");
    KValue none; none.tag = K_NONE; none.payload = 0; return none;
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

/* ryu d2s tables (adams, PLDI 2018): 125-bit multipliers, generated exactly */
#define RYU_POW5_BITCOUNT 125
#define RYU_POW5_INV_BITCOUNT 125
static const uint64_t RYU_POW5[326][2] = {
    {0x0000000000000000ULL, 0x1000000000000000ULL},
    {0x0000000000000000ULL, 0x1400000000000000ULL},
    {0x0000000000000000ULL, 0x1900000000000000ULL},
    {0x0000000000000000ULL, 0x1f40000000000000ULL},
    {0x0000000000000000ULL, 0x1388000000000000ULL},
    {0x0000000000000000ULL, 0x186a000000000000ULL},
    {0x0000000000000000ULL, 0x1e84800000000000ULL},
    {0x0000000000000000ULL, 0x1312d00000000000ULL},
    {0x0000000000000000ULL, 0x17d7840000000000ULL},
    {0x0000000000000000ULL, 0x1dcd650000000000ULL},
    {0x0000000000000000ULL, 0x12a05f2000000000ULL},
    {0x0000000000000000ULL, 0x174876e800000000ULL},
    {0x0000000000000000ULL, 0x1d1a94a200000000ULL},
    {0x0000000000000000ULL, 0x12309ce540000000ULL},
    {0x0000000000000000ULL, 0x16bcc41e90000000ULL},
    {0x0000000000000000ULL, 0x1c6bf52634000000ULL},
    {0x0000000000000000ULL, 0x11c37937e0800000ULL},
    {0x0000000000000000ULL, 0x16345785d8a00000ULL},
    {0x0000000000000000ULL, 0x1bc16d674ec80000ULL},
    {0x0000000000000000ULL, 0x1158e460913d0000ULL},
    {0x0000000000000000ULL, 0x15af1d78b58c4000ULL},
    {0x0000000000000000ULL, 0x1b1ae4d6e2ef5000ULL},
    {0x0000000000000000ULL, 0x10f0cf064dd59200ULL},
    {0x0000000000000000ULL, 0x152d02c7e14af680ULL},
    {0x0000000000000000ULL, 0x1a784379d99db420ULL},
    {0x0000000000000000ULL, 0x108b2a2c28029094ULL},
    {0x0000000000000000ULL, 0x14adf4b7320334b9ULL},
    {0x4000000000000000ULL, 0x19d971e4fe8401e7ULL},
    {0x8800000000000000ULL, 0x1027e72f1f128130ULL},
    {0xaa00000000000000ULL, 0x1431e0fae6d7217cULL},
    {0xd480000000000000ULL, 0x193e5939a08ce9dbULL},
    {0xc9a0000000000000ULL, 0x1f8def8808b02452ULL},
    {0xbe04000000000000ULL, 0x13b8b5b5056e16b3ULL},
    {0xad85000000000000ULL, 0x18a6e32246c99c60ULL},
    {0xd8e6400000000000ULL, 0x1ed09bead87c0378ULL},
    {0x878fe80000000000ULL, 0x13426172c74d822bULL},
    {0x6973e20000000000ULL, 0x1812f9cf7920e2b6ULL},
    {0x03d0da8000000000ULL, 0x1e17b84357691b64ULL},
    {0x8262889000000000ULL, 0x12ced32a16a1b11eULL},
    {0x22fb2ab400000000ULL, 0x178287f49c4a1d66ULL},
    {0xabb9f56100000000ULL, 0x1d6329f1c35ca4bfULL},
    {0xcb54395ca0000000ULL, 0x125dfa371a19e6f7ULL},
    {0xbe2947b3c8000000ULL, 0x16f578c4e0a060b5ULL},
    {0x2db399a0ba000000ULL, 0x1cb2d6f618c878e3ULL},
    {0xfc90400474400000ULL, 0x11efc659cf7d4b8dULL},
    {0x7bb4500591500000ULL, 0x166bb7f0435c9e71ULL},
    {0xdaa16406f5a40000ULL, 0x1c06a5ec5433c60dULL},
    {0xa8a4de8459868000ULL, 0x118427b3b4a05bc8ULL},
    {0xd2ce16256fe82000ULL, 0x15e531a0a1c872baULL},
    {0x87819baecbe22800ULL, 0x1b5e7e08ca3a8f69ULL},
    {0xf4b1014d3f6d5900ULL, 0x111b0ec57e6499a1ULL},
    {0x71dd41a08f48af40ULL, 0x1561d276ddfdc00aULL},
    {0x0e549208b31adb10ULL, 0x1aba4714957d300dULL},
    {0x28f4db456ff0c8eaULL, 0x10b46c6cdd6e3e08ULL},
    {0x33321216cbecfb24ULL, 0x14e1878814c9cd8aULL},
    {0xbffe969c7ee839edULL, 0x1a19e96a19fc40ecULL},
    {0xf7ff1e21cf512434ULL, 0x105031e2503da893ULL},
    {0xf5fee5aa43256d41ULL, 0x14643e5ae44d12b8ULL},
    {0x337e9f14d3eec892ULL, 0x197d4df19d605767ULL},
    {0x005e46da08ea7ab6ULL, 0x1fdca16e04b86d41ULL},
    {0xa03aec4845928cb2ULL, 0x13e9e4e4c2f34448ULL},
    {0xc849a75a56f72fdeULL, 0x18e45e1df3b0155aULL},
    {0x7a5c1130ecb4fbd6ULL, 0x1f1d75a5709c1ab1ULL},
    {0xec798abe93f11d65ULL, 0x13726987666190aeULL},
    {0xa797ed6e38ed64bfULL, 0x184f03e93ff9f4daULL},
    {0x517de8c9c728bdefULL, 0x1e62c4e38ff87211ULL},
    {0xd2eeb17e1c7976b5ULL, 0x12fdbb0e39fb474aULL},
    {0x87aa5ddda397d462ULL, 0x17bd29d1c87a191dULL},
    {0xe994f5550c7dc97bULL, 0x1dac74463a989f64ULL},
    {0x11fd195527ce9dedULL, 0x128bc8abe49f639fULL},
    {0xd67c5faa71c24568ULL, 0x172ebad6ddc73c86ULL},
    {0x8c1b77950e32d6c2ULL, 0x1cfa698c95390ba8ULL},
    {0x57912abd28dfc639ULL, 0x121c81f7dd43a749ULL},
    {0xad75756c7317b7c8ULL, 0x16a3a275d494911bULL},
    {0x98d2d2c78fdda5baULL, 0x1c4c8b1349b9b562ULL},
    {0x9f83c3bcb9ea8794ULL, 0x11afd6ec0e14115dULL},
    {0x0764b4abe8652979ULL, 0x161bcca7119915b5ULL},
    {0x493de1d6e27e73d7ULL, 0x1ba2bfd0d5ff5b22ULL},
    {0x6dc6ad264d8f0866ULL, 0x1145b7e285bf98f5ULL},
    {0xc938586fe0f2ca80ULL, 0x159725db272f7f32ULL},
    {0x7b866e8bd92f7d20ULL, 0x1afcef51f0fb5effULL},
    {0xad34051767bdae34ULL, 0x10de1593369d1b5fULL},
    {0x9881065d41ad19c1ULL, 0x15159af804446237ULL},
    {0x7ea147f492186032ULL, 0x1a5b01b605557ac5ULL},
    {0x6f24ccf8db4f3c1fULL, 0x1078e111c3556cbbULL},
    {0x4aee003712230b27ULL, 0x14971956342ac7eaULL},
    {0xdda98044d6abcdf0ULL, 0x19bcdfabc13579e4ULL},
    {0x0a89f02b062b60b6ULL, 0x10160bcb58c16c2fULL},
    {0xcd2c6c35c7b638e4ULL, 0x141b8ebe2ef1c73aULL},
    {0x8077874339a3c71dULL, 0x1922726dbaae3909ULL},
    {0xe0956914080cb8e4ULL, 0x1f6b0f092959c74bULL},
    {0x6c5d61ac8507f38eULL, 0x13a2e965b9d81c8fULL},
    {0x4774ba17a649f072ULL, 0x188ba3bf284e23b3ULL},
    {0x1951e89d8fdc6c8fULL, 0x1eae8caef261aca0ULL},
    {0x0fd3316279e9c3d9ULL, 0x132d17ed577d0be4ULL},
    {0x13c7fdbb186434cfULL, 0x17f85de8ad5c4eddULL},
    {0x58b9fd29de7d4203ULL, 0x1df67562d8b36294ULL},
    {0xb7743e3a2b0e4942ULL, 0x12ba095dc7701d9cULL},
    {0xe5514dc8b5d1db92ULL, 0x17688bb5394c2503ULL},
    {0xdea5a13ae3465277ULL, 0x1d42aea2879f2e44ULL},
    {0x0b2784c4ce0bf38aULL, 0x1249ad2594c37cebULL},
    {0xcdf165f6018ef06dULL, 0x16dc186ef9f45c25ULL},
    {0x416dbf7381f2ac88ULL, 0x1c931e8ab871732fULL},
    {0x88e497a83137abd5ULL, 0x11dbf316b346e7fdULL},
    {0xeb1dbd923d8596caULL, 0x1652efdc6018a1fcULL},
    {0x25e52cf6cce6fc7dULL, 0x1be7abd3781eca7cULL},
    {0x97af3c1a40105dceULL, 0x1170cb642b133e8dULL},
    {0xfd9b0b20d0147542ULL, 0x15ccfe3d35d80e30ULL},
    {0x3d01cde904199292ULL, 0x1b403dcc834e11bdULL},
    {0x462120b1a28ffb9bULL, 0x1108269fd210cb16ULL},
    {0xd7a968de0b33fa82ULL, 0x154a3047c694fddbULL},
    {0xcd93c3158e00f923ULL, 0x1a9cbc59b83a3d52ULL},
    {0xc07c59ed78c09bb6ULL, 0x10a1f5b813246653ULL},
    {0xb09b7068d6f0c2a3ULL, 0x14ca732617ed7fe8ULL},
    {0xdcc24c830cacf34cULL, 0x19fd0fef9de8dfe2ULL},
    {0xc9f96fd1e7ec180fULL, 0x103e29f5c2b18bedULL},
    {0x3c77cbc661e71e13ULL, 0x144db473335deee9ULL},
    {0x8b95beb7fa60e598ULL, 0x1961219000356aa3ULL},
    {0x6e7b2e65f8f91efeULL, 0x1fb969f40042c54cULL},
    {0xc50cfcffbb9bb35fULL, 0x13d3e2388029bb4fULL},
    {0xb6503c3faa82a037ULL, 0x18c8dac6a0342a23ULL},
    {0xa3e44b4f95234844ULL, 0x1efb1178484134acULL},
    {0xe66eaf11bd360d2bULL, 0x135ceaeb2d28c0ebULL},
    {0xe00a5ad62c839075ULL, 0x183425a5f872f126ULL},
    {0x980cf18bb7a47493ULL, 0x1e412f0f768fad70ULL},
    {0x5f0816f752c6c8dcULL, 0x12e8bd69aa19cc66ULL},
    {0xf6ca1cb527787b13ULL, 0x17a2ecc414a03f7fULL},
    {0xf47ca3e2715699d7ULL, 0x1d8ba7f519c84f5fULL},
    {0xf8cde66d86d62026ULL, 0x127748f9301d319bULL},
    {0xf7016008e88ba830ULL, 0x17151b377c247e02ULL},
    {0xb4c1b80b22ae923cULL, 0x1cda62055b2d9d83ULL},
    {0x50f91306f5ad1b65ULL, 0x12087d4358fc8272ULL},
    {0xe53757c8b318623fULL, 0x168a9c942f3ba30eULL},
    {0x9e852dbadfde7acfULL, 0x1c2d43b93b0a8bd2ULL},
    {0xa3133c94cbeb0cc1ULL, 0x119c4a53c4e69763ULL},
    {0x8bd80bb9fee5cff1ULL, 0x16035ce8b6203d3cULL},
    {0xaece0ea87e9f43eeULL, 0x1b843422e3a84c8bULL},
    {0x4d40c9294f238a75ULL, 0x1132a095ce492fd7ULL},
    {0x2090fb73a2ec6d12ULL, 0x157f48bb41db7bcdULL},
    {0x68b53a508ba78856ULL, 0x1adf1aea12525ac0ULL},
    {0x417144725748b536ULL, 0x10cb70d24b7378b8ULL},
    {0x51cd958eed1ae283ULL, 0x14fe4d06de5056e6ULL},
    {0xe640faf2a8619b24ULL, 0x1a3de04895e46c9fULL},
    {0xefe89cd7a93d00f7ULL, 0x1066ac2d5daec3e3ULL},
    {0xebe2c40d938c4134ULL, 0x14805738b51a74dcULL},
    {0x26db7510f86f5181ULL, 0x19a06d06e2611214ULL},
    {0x9849292a9b4592f1ULL, 0x100444244d7cab4cULL},
    {0xbe5b73754216f7adULL, 0x1405552d60dbd61fULL},
    {0xadf25052929cb598ULL, 0x1906aa78b912cba7ULL},
    {0x996ee4673743e2ffULL, 0x1f485516e7577e91ULL},
    {0xffe54ec0828a6ddfULL, 0x138d352e5096af1aULL},
    {0xbfdea270a32d0957ULL, 0x18708279e4bc5ae1ULL},
    {0x2fd64b0ccbf84badULL, 0x1e8ca3185deb719aULL},
    {0x5de5eee7ff7b2f4cULL, 0x1317e5ef3ab32700ULL},
    {0x755f6aa1ff59fb1fULL, 0x17dddf6b095ff0c0ULL},
    {0x92b7454a7f3079e7ULL, 0x1dd55745cbb7ecf0ULL},
    {0x5bb28b4e8f7e4c30ULL, 0x12a5568b9f52f416ULL},
    {0xf29f2e22335ddf3cULL, 0x174eac2e8727b11bULL},
    {0xef46f9aac035570bULL, 0x1d22573a28f19d62ULL},
    {0xd58c5c0ab8215667ULL, 0x123576845997025dULL},
    {0x4aef730d6629ac01ULL, 0x16c2d4256ffcc2f5ULL},
    {0x9dab4fd0bfb41701ULL, 0x1c73892ecbfbf3b2ULL},
    {0xa28b11e277d08e60ULL, 0x11c835bd3f7d784fULL},
    {0x8b2dd65b15c4b1f9ULL, 0x163a432c8f5cd663ULL},
    {0x6df94bf1db35de77ULL, 0x1bc8d3f7b3340bfcULL},
    {0xc4bbcf772901ab0aULL, 0x115d847ad000877dULL},
    {0x35eac354f34215cdULL, 0x15b4e5998400a95dULL},
    {0x8365742a30129b40ULL, 0x1b221effe500d3b4ULL},
    {0xd21f689a5e0ba108ULL, 0x10f5535fef208450ULL},
    {0x06a742c0f58e894aULL, 0x1532a837eae8a565ULL},
    {0x4851137132f22b9dULL, 0x1a7f5245e5a2cebeULL},
    {0xed32ac26bfd75b42ULL, 0x108f936baf85c136ULL},
    {0xa87f57306fcd3212ULL, 0x14b378469b673184ULL},
    {0xd29f2cfc8bc07e97ULL, 0x19e056584240fde5ULL},
    {0xa3a37c1dd7584f1eULL, 0x102c35f729689eafULL},
    {0x8c8c5b254d2e62e6ULL, 0x14374374f3c2c65bULL},
    {0x6faf71eea079fb9fULL, 0x1945145230b377f2ULL},
    {0x0b9b4e6a48987a87ULL, 0x1f965966bce055efULL},
    {0x674111026d5f4c94ULL, 0x13bdf7e0360c35b5ULL},
    {0xc111554308b71fbaULL, 0x18ad75d8438f4322ULL},
    {0x7155aa93cae4e7a8ULL, 0x1ed8d34e547313ebULL},
    {0x26d58a9c5ecf10c9ULL, 0x13478410f4c7ec73ULL},
    {0xf08aed437682d4fbULL, 0x1819651531f9e78fULL},
    {0xecada89454238a3aULL, 0x1e1fbe5a7e786173ULL},
    {0x73ec895cb4963664ULL, 0x12d3d6f88f0b3ce8ULL},
    {0x90e7abb3e1bbc3fdULL, 0x1788ccb6b2ce0c22ULL},
    {0x352196a0da2ab4fdULL, 0x1d6affe45f818f2bULL},
    {0x0134fe24885ab11eULL, 0x1262dfeebbb0f97bULL},
    {0xc1823dadaa715d65ULL, 0x16fb97ea6a9d37d9ULL},
    {0x31e2cd19150db4bfULL, 0x1cba7de5054485d0ULL},
    {0x1f2dc02fad2890f7ULL, 0x11f48eaf234ad3a2ULL},
    {0xa6f9303b9872b535ULL, 0x1671b25aec1d888aULL},
    {0x50b77c4a7e8f6282ULL, 0x1c0e1ef1a724eaadULL},
    {0x5272adae8f199d91ULL, 0x1188d357087712acULL},
    {0x670f591a32e004f6ULL, 0x15eb082cca94d757ULL},
    {0x40d32f60bf980633ULL, 0x1b65ca37fd3a0d2dULL},
    {0x4883fd9c77bf03e0ULL, 0x111f9e62fe44483cULL},
    {0x5aa4fd0395aec4d8ULL, 0x156785fbbdd55a4bULL},
    {0x314e3c447b1a760eULL, 0x1ac1677aad4ab0deULL},
    {0xded0e5aaccf089c9ULL, 0x10b8e0acac4eae8aULL},
    {0x96851f15802cac3bULL, 0x14e718d7d7625a2dULL},
    {0xfc2666dae037d74aULL, 0x1a20df0dcd3af0b8ULL},
    {0x9d980048cc22e68eULL, 0x10548b68a044d673ULL},
    {0x84fe005aff2ba032ULL, 0x1469ae42c8560c10ULL},
    {0xa63d8071bef6883eULL, 0x198419d37a6b8f14ULL},
    {0xcfcce08e2eb42a4eULL, 0x1fe52048590672d9ULL},
    {0x21e00c58dd309a70ULL, 0x13ef342d37a407c8ULL},
    {0x2a580f6f147cc10dULL, 0x18eb0138858d09baULL},
    {0xb4ee134ad99bf150ULL, 0x1f25c186a6f04c28ULL},
    {0x7114cc0ec80176d2ULL, 0x137798f428562f99ULL},
    {0xcd59ff127a01d486ULL, 0x18557f31326bbb7fULL},
    {0xc0b07ed7188249a8ULL, 0x1e6adefd7f06aa5fULL},
    {0xd86e4f466f516e09ULL, 0x1302cb5e6f642a7bULL},
    {0xce89e3180b25c98bULL, 0x17c37e360b3d351aULL},
    {0x822c5bde0def3beeULL, 0x1db45dc38e0c8261ULL},
    {0xf15bb96ac8b58575ULL, 0x1290ba9a38c7d17cULL},
    {0x2db2a7c57ae2e6d2ULL, 0x1734e940c6f9c5dcULL},
    {0x391f51b6d99ba086ULL, 0x1d022390f8b83753ULL},
    {0x03b3931248014454ULL, 0x1221563a9b732294ULL},
    {0x04a077d6da019569ULL, 0x16a9abc9424feb39ULL},
    {0x45c895cc9081fac3ULL, 0x1c5416bb92e3e607ULL},
    {0x8b9d5d9fda513cbaULL, 0x11b48e353bce6fc4ULL},
    {0xae84b507d0e58be8ULL, 0x1621b1c28ac20bb5ULL},
    {0x1a25e249c51eeee3ULL, 0x1baa1e332d728ea3ULL},
    {0xf057ad6e1b33554dULL, 0x114a52dffc679925ULL},
    {0x6c6d98c9a2002aa1ULL, 0x159ce797fb817f6fULL},
    {0x4788fefc0a803549ULL, 0x1b04217dfa61df4bULL},
    {0x0cb59f5d8690214eULL, 0x10e294eebc7d2b8fULL},
    {0xcfe30734e83429a1ULL, 0x151b3a2a6b9c7672ULL},
    {0x83dbc9022241340aULL, 0x1a6208b50683940fULL},
    {0xb2695da15568c086ULL, 0x107d457124123c89ULL},
    {0x1f03b509aac2f0a7ULL, 0x149c96cd6d16cbacULL},
    {0x26c4a24c1573acd1ULL, 0x19c3bc80c85c7e97ULL},
    {0x783ae56f8d684c03ULL, 0x101a55d07d39cf1eULL},
    {0x16499ecb70c25f03ULL, 0x1420eb449c8842e6ULL},
    {0x9bdc067e4cf2f6c4ULL, 0x19292615c3aa539fULL},
    {0x82d3081de02fb476ULL, 0x1f736f9b3494e887ULL},
    {0xb1c3e512ac1dd0c9ULL, 0x13a825c100dd1154ULL},
    {0xde34de57572544fcULL, 0x18922f31411455a9ULL},
    {0x55c215ed2cee963bULL, 0x1eb6bafd91596b14ULL},
    {0xb5994db43c151de5ULL, 0x133234de7ad7e2ecULL},
    {0xe2ffa1214b1a655eULL, 0x17fec216198ddba7ULL},
    {0xdbbf89699de0feb6ULL, 0x1dfe729b9ff15291ULL},
    {0x2957b5e202ac9f31ULL, 0x12bf07a143f6d39bULL},
    {0xf3ada35a8357c6feULL, 0x176ec98994f48881ULL},
    {0x70990c31242db8bdULL, 0x1d4a7bebfa31aaa2ULL},
    {0x865fa79eb69c9376ULL, 0x124e8d737c5f0aa5ULL},
    {0xe7f791866443b854ULL, 0x16e230d05b76cd4eULL},
    {0xa1f575e7fd54a669ULL, 0x1c9abd04725480a2ULL},
    {0xa53969b0fe54e801ULL, 0x11e0b622c774d065ULL},
    {0x0e87c41d3dea2202ULL, 0x1658e3ab7952047fULL},
    {0xd229b5248d64aa82ULL, 0x1bef1c9657a6859eULL},
    {0x435a1136d85eea91ULL, 0x117571ddf6c81383ULL},
    {0x143095848e76a536ULL, 0x15d2ce55747a1864ULL},
    {0x193cbae5b2144e83ULL, 0x1b4781ead1989e7dULL},
    {0x2fc5f4cf8f4cb112ULL, 0x110cb132c2ff630eULL},
    {0xbbb77203731fdd56ULL, 0x154fdd7f73bf3bd1ULL},
    {0x2aa54e844fe7d4acULL, 0x1aa3d4df50af0ac6ULL},
    {0xdaa75112b1f0e4ebULL, 0x10a6650b926d66bbULL},
    {0xd15125575e6d1e26ULL, 0x14cffe4e7708c06aULL},
    {0x85a56ead360865b0ULL, 0x1a03fde214caf085ULL},
    {0x7387652c41c53f8eULL, 0x10427ead4cfed653ULL},
    {0x50693e7752368f71ULL, 0x14531e58a03e8be8ULL},
    {0x64838e1526c4334eULL, 0x1967e5eec84e2ee2ULL},
    {0xfda4719a70754022ULL, 0x1fc1df6a7a61ba9aULL},
    {0xde86c70086494815ULL, 0x13d92ba28c7d14a0ULL},
    {0x162878c0a7db9a1aULL, 0x18cf768b2f9c59c9ULL},
    {0x5bb296f0d1d280a1ULL, 0x1f03542dfb83703bULL},
    {0x194f9e5683239064ULL, 0x1362149cbd322625ULL},
    {0x5fa385ec23ec747eULL, 0x183a99c3ec7eafaeULL},
    {0xf78c67672ce7919dULL, 0x1e494034e79e5b99ULL},
    {0x3ab7c0a07c10bb02ULL, 0x12edc82110c2f940ULL},
    {0x4965b0c89b14e9c3ULL, 0x17a93a2954f3b790ULL},
    {0x5bbf1cfac1da2433ULL, 0x1d9388b3aa30a574ULL},
    {0xb957721cb92856a0ULL, 0x127c35704a5e6768ULL},
    {0xe7ad4ea3e7726c48ULL, 0x171b42cc5cf60142ULL},
    {0xa198a24ce14f075aULL, 0x1ce2137f74338193ULL},
    {0x44ff65700cd16498ULL, 0x120d4c2fa8a030fcULL},
    {0x563f3ecc1005bdbeULL, 0x16909f3b92c83d3bULL},
    {0x2bcf0e7f14072d2eULL, 0x1c34c70a777a4c8aULL},
    {0x5b61690f6c847c3dULL, 0x11a0fc668aac6fd6ULL},
    {0xf239c35347a59b4cULL, 0x16093b802d578bcbULL},
    {0xeec83428198f021fULL, 0x1b8b8a6038ad6ebeULL},
    {0x553d20990ff96153ULL, 0x1137367c236c6537ULL},
    {0x2a8c68bf53f7b9a8ULL, 0x1585041b2c477e85ULL},
    {0x752f82ef28f5a812ULL, 0x1ae64521f7595e26ULL},
    {0x093db1d57999890bULL, 0x10cfeb353a97dad8ULL},
    {0x0b8d1e4ad7ffeb4eULL, 0x1503e602893dd18eULL},
    {0x8e7065dd8dffe622ULL, 0x1a44df832b8d45f1ULL},
    {0xf9063faa78bfefd5ULL, 0x106b0bb1fb384bb6ULL},
    {0xb747cf9516efebcaULL, 0x1485ce9e7a065ea4ULL},
    {0xe519c37a5cabe6bdULL, 0x19a742461887f64dULL},
    {0xaf301a2c79eb7036ULL, 0x1008896bcf54f9f0ULL},
    {0xdafc20b798664c43ULL, 0x140aabc6c32a386cULL},
    {0x11bb28e57e7fdf54ULL, 0x190d56b873f4c688ULL},
    {0x1629f31ede1fd72aULL, 0x1f50ac6690f1f82aULL},
    {0x4dda37f34ad3e67aULL, 0x13926bc01a973b1aULL},
    {0xe150c5f01d88e019ULL, 0x187706b0213d09e0ULL},
    {0x19a4f76c24eb181fULL, 0x1e94c85c298c4c59ULL},
    {0xb0071aa39712ef13ULL, 0x131cfd3999f7afb7ULL},
    {0x9c08e14c7cd7aad8ULL, 0x17e43c8800759ba5ULL},
    {0x030b199f9c0d958eULL, 0x1ddd4baa0093028fULL},
    {0x61e6f003c1887d79ULL, 0x12aa4f4a405be199ULL},
    {0xba60ac04b1ea9cd7ULL, 0x1754e31cd072d9ffULL},
    {0xa8f8d705de65440dULL, 0x1d2a1be4048f907fULL},
    {0xc99b8663aaff4a88ULL, 0x123a516e82d9ba4fULL},
    {0xbc0267fc95bf1d2aULL, 0x16c8e5ca239028e3ULL},
    {0xab0301fbbb2ee474ULL, 0x1c7b1f3cac74331cULL},
    {0xeae1e13d54fd4ec9ULL, 0x11ccf385ebc89ff1ULL},
    {0x659a598caa3ca27bULL, 0x1640306766bac7eeULL},
    {0xff00efefd4cbcb1aULL, 0x1bd03c81406979e9ULL},
    {0x3f6095f5e4ff5ef0ULL, 0x116225d0c841ec32ULL},
    {0xcf38bb735e3f36acULL, 0x15baaf44fa52673eULL},
    {0x8306ea5035cf0457ULL, 0x1b295b1638e7010eULL},
    {0x11e4527221a162b6ULL, 0x10f9d8ede39060a9ULL},
    {0x565d670eaa09bb64ULL, 0x15384f295c7478d3ULL},
    {0x2bf4c0d2548c2a3dULL, 0x1a8662f3b3919708ULL},
    {0x1b78f88374d79a66ULL, 0x1093fdd8503afe65ULL},
    {0x625736a4520d8100ULL, 0x14b8fd4e6449bdfeULL},
    {0xfaed044d6690e140ULL, 0x19e73ca1fd5c2d7dULL},
    {0xbcd422b0601a8cc8ULL, 0x103085e53e599c6eULL},
    {0x6c092b5c78212ffaULL, 0x143ca75e8df0038aULL},
    {0x070b763396297bf8ULL, 0x194bd136316c046dULL},
    {0x48ce53c07bb3daf6ULL, 0x1f9ec583bdc70588ULL},
    {0x2d80f4584d5068daULL, 0x13c33b72569c6375ULL},
    {0x78e1316e60a48310ULL, 0x18b40a4eec437c52ULL},
};
static const uint64_t RYU_POW5_INV[292][2] = {
    {0x0000000000000001ULL, 0x2000000000000000ULL},
    {0x999999999999999aULL, 0x1999999999999999ULL},
    {0x47ae147ae147ae15ULL, 0x147ae147ae147ae1ULL},
    {0x6c8b4395810624deULL, 0x10624dd2f1a9fbe7ULL},
    {0x7a786c226809d496ULL, 0x1a36e2eb1c432ca5ULL},
    {0x61f9f01b866e43abULL, 0x14f8b588e368f084ULL},
    {0xb4c7f34938583622ULL, 0x10c6f7a0b5ed8d36ULL},
    {0x87a6520ec08d236aULL, 0x1ad7f29abcaf4857ULL},
    {0x9fb841a566d74f88ULL, 0x15798ee2308c39dfULL},
    {0xe62d01511f12a607ULL, 0x112e0be826d694b2ULL},
    {0xd6ae6881cb5109a4ULL, 0x1b7cdfd9d7bdbab7ULL},
    {0xdef1ed34a2a73aeaULL, 0x15fd7fe17964955fULL},
    {0x7f27f0f6e885c8bbULL, 0x119799812dea1119ULL},
    {0x650cb4be40d60df8ULL, 0x1c25c268497681c2ULL},
    {0xea70909833de7193ULL, 0x16849b86a12b9b01ULL},
    {0x21f3a6e0297ec143ULL, 0x1203af9ee756159bULL},
    {0x6985d7cd0f313537ULL, 0x1cd2b297d889bc2bULL},
    {0x2137dfd73f5a90f9ULL, 0x170ef54646d49689ULL},
    {0xe75fe645cc4873faULL, 0x12725dd1d243aba0ULL},
    {0xa5663d3c7a0d865dULL, 0x1d83c94fb6d2ac34ULL},
    {0x511e976394d79eb1ULL, 0x179ca10c9242235dULL},
    {0xda7edf82dd794bc1ULL, 0x12e3b40a0e9b4f7dULL},
    {0x2a6498d1625bac68ULL, 0x1e392010175ee596ULL},
    {0xeeb6e0a781e2f053ULL, 0x182db34012b25144ULL},
    {0x58924d52ce4f26a9ULL, 0x1357c299a88ea76aULL},
    {0x27507bb7b07ea441ULL, 0x1ef2d0f5da7dd8aaULL},
    {0x52a6c95fc0655034ULL, 0x18c240c4aecb13bbULL},
    {0x0eebd44c99eaa690ULL, 0x13ce9a36f23c0fc9ULL},
    {0xb17953adc3110a80ULL, 0x1fb0f6be50601941ULL},
    {0xc12ddc8b02740867ULL, 0x195a5efea6b34767ULL},
    {0x3424b06f3529a052ULL, 0x14484bfeebc29f86ULL},
    {0x901d59f290ee19dbULL, 0x1039d66589687f9eULL},
    {0x4cfbc31db4b0295fULL, 0x19f623d5a8a73297ULL},
    {0x3d9635b15d59bab2ULL, 0x14c4e977ba1f5bacULL},
    {0x97ab5e277de16228ULL, 0x109d8792fb4c4956ULL},
    {0xf2abc9d8c9689d0dULL, 0x1a95a5b7f87a0ef0ULL},
    {0x5bbca17a3aba173eULL, 0x154484932d2e725aULL},
    {0xafca1ac82efb45cbULL, 0x11039d428a8b8eaeULL},
    {0xb2dcf7a6b1920945ULL, 0x1b38fb9daa78e44aULL},
    {0xf57d92ebc141a104ULL, 0x15c72fb1552d836eULL},
    {0xc46475896767b403ULL, 0x116c262777579c58ULL},
    {0x6d6d88dbd8a5ecd2ULL, 0x1be03d0bf225c6f4ULL},
    {0x8abe071646eb23dbULL, 0x164cfda3281e38c3ULL},
    {0x6efe6c11d255b649ULL, 0x11d7314f534b609cULL},
    {0xb197134fb6ef8a0eULL, 0x1c8b821885456760ULL},
    {0x27ac0f72f8bfa1a5ULL, 0x16d601ad376ab91aULL},
    {0xb95672c260994e1eULL, 0x1244ce242c5560e1ULL},
    {0xf5571e03cdc21695ULL, 0x1d3ae36d13bbce35ULL},
    {0x2aac18030b01ababULL, 0x17624f8a762fd82bULL},
    {0xbbbce0026f348956ULL, 0x12b50c6ec4f31355ULL},
    {0x92c7ccd0b1eda889ULL, 0x1dee7a4ad4b81eefULL},
    {0xdbd30a408e57ba07ULL, 0x17f1fb6f10934bf2ULL},
    {0x7ca8d50071dfc806ULL, 0x1327fc58da0f6ff5ULL},
    {0xfaa7bb33e9660cd6ULL, 0x1ea6608e29b24cbbULL},
    {0x9552fc298784d711ULL, 0x18851a0b548ea3c9ULL},
    {0xaaa8c9bad2d0ac0eULL, 0x139dae6f76d88307ULL},
    {0xdddadc5e1e1aace3ULL, 0x1f62b0b257c0d1a5ULL},
    {0x7e48b04b4b488a4fULL, 0x191bc08eac9a4151ULL},
    {0xcb6d59d5d5d3a1d9ULL, 0x141633a556e1cddaULL},
    {0x3c577b1177dc817bULL, 0x1011c2eaabe7d7e2ULL},
    {0xc6f25e825960cf2aULL, 0x19b604aaaca62636ULL},
    {0x6bf518684780a5bbULL, 0x14919d5556eb51c5ULL},
    {0x232a79ed06008496ULL, 0x10747ddddf22a7d1ULL},
    {0xd1dd8fe1a3340756ULL, 0x1a53fc9631d10c81ULL},
    {0xa7e4731ae8f66c45ULL, 0x150ffd44f4a73d34ULL},
    {0x531d28e253f8569eULL, 0x10d9976a5d52975dULL},
    {0xeb61db03b98d5762ULL, 0x1af5bf109550f22eULL},
    {0xbc4e48cfc7a445e8ULL, 0x159165a6ddda5b58ULL},
    {0x6371d3d96c836b20ULL, 0x11411e1f17e1e2adULL},
    {0x9f1c8628ad9f11cdULL, 0x1b9b6364f3030448ULL},
    {0xe5b06b53be18db0bULL, 0x1615e91d8f359d06ULL},
    {0xeaf3890fcb4715a2ULL, 0x11ab20e472914a6bULL},
    {0x44b8db4c7871bc37ULL, 0x1c45016d841baa46ULL},
    {0x03c715d6c6c1635fULL, 0x169d9abe03495505ULL},
    {0x3638de456bcde919ULL, 0x1217aefe69077737ULL},
    {0x56c163a2461641c1ULL, 0x1cf2b1970e725858ULL},
    {0xdf011c81d1ab67ceULL, 0x17288e1271f51379ULL},
    {0x7f3416ce4155eca5ULL, 0x1286d80ec190dc61ULL},
    {0x6520247d3556476eULL, 0x1da48ce468e7c702ULL},
    {0xea801d30f7783925ULL, 0x17b6d71d20b96c01ULL},
    {0xbb99b0f3f92cfa84ULL, 0x12f8ac174d612334ULL},
    {0x5f5c4e532847f739ULL, 0x1e5aacf215683854ULL},
    {0x7f7d0b75b9d32c2eULL, 0x18488a5b44536043ULL},
    {0x9930d5f7c7dc2358ULL, 0x136d3b7c36a919cfULL},
    {0x8eb4898c72f9d226ULL, 0x1f152bf9f10e8fb2ULL},
    {0x722a07a38f2e41b8ULL, 0x18ddbcc7f40ba628ULL},
    {0xc1bb394fa5be9afaULL, 0x13e497065cd61e86ULL},
    {0x9c5ec2190930f7f6ULL, 0x1fd424d6faf030d7ULL},
    {0x49e56814075a5ff8ULL, 0x197683df2f268d79ULL},
    {0x6e51201005e1e660ULL, 0x145ecfe5bf520ac7ULL},
    {0xf1da800cd181851aULL, 0x104bd984990e6f05ULL},
    {0x4fc400148268d4f5ULL, 0x1a12f5a0f4e3e4d6ULL},
    {0xd96999aa01ed772bULL, 0x14dbf7b3f71cb711ULL},
    {0xadee1488018ac5bcULL, 0x10aff95cc5b09274ULL},
    {0x497ceda668de092cULL, 0x1ab328946f80ea54ULL},
    {0x3aca57b853e4d424ULL, 0x155c2076bf9a5510ULL},
    {0x623b7960431d7683ULL, 0x1116805effaeaa73ULL},
    {0x9d2bf566d1c8bd9eULL, 0x1b5733cb32b110b8ULL},
    {0x7dbcc452416d647fULL, 0x15df5ca28ef40d60ULL},
    {0xcafd69db678ab6ccULL, 0x117f7d4ed8c33de6ULL},
    {0xab2f0fc572778adfULL, 0x1bff2ee48e052fd7ULL},
    {0x88f273045b92d580ULL, 0x1665bf1d3e6a8cacULL},
    {0xd3f528d049424466ULL, 0x11eaff4a98553d56ULL},
    {0xb988414d4203a0a3ULL, 0x1cab3210f3bb9557ULL},
    {0x6139cdd76802e6e9ULL, 0x16ef5b40c2fc7779ULL},
    {0xe761717920025254ULL, 0x125915cd68c9f92dULL},
    {0xa568b58e999d5086ULL, 0x1d5b561574765b7cULL},
    {0x5120913ee14aa6d2ULL, 0x177c44ddf6c515fdULL},
    {0xa74d40ff1aa21f0eULL, 0x12c9d0b1923744caULL},
    {0x0baece64f769cb4aULL, 0x1e0fb44f50586e11ULL},
    {0x3c8bd850c5ee3c3bULL, 0x180c903f7379f1a7ULL},
    {0xca0979da37f1c9c9ULL, 0x133d4032c2c7f485ULL},
    {0xa9a8c2f6bfe942dbULL, 0x1ec866b79e0cba6fULL},
    {0x2153cf2bccba9be3ULL, 0x18a0522c7e709526ULL},
    {0x1aa9728970954982ULL, 0x13b374f06526ddb8ULL},
    {0xf775840f1a88759dULL, 0x1f8587e7083e2f8cULL},
    {0x5f9136727ba05e17ULL, 0x19379fec0698260aULL},
    {0x1940f85b9619e4dfULL, 0x142c7ff0054684d5ULL},
    {0xe100c6afab47ea4cULL, 0x1023998cd1053710ULL},
    {0xce67a44c453fdd47ULL, 0x19d28f47b4d524e7ULL},
    {0xd852e9d69dccb106ULL, 0x14a8729fc3ddb71fULL},
    {0x79dbee454b0a2738ULL, 0x1086c219697e2c19ULL},
    {0x295fe3a211a9d859ULL, 0x1a71368f0f30468fULL},
    {0xbab31c81a7bb137aULL, 0x15275ed8d8f36ba5ULL},
    {0x6228e39aec95a92fULL, 0x10ec4be0ad8f8951ULL},
    {0x9d0e38f7e0ef7517ULL, 0x1b13ac9aaf4c0ee8ULL},
    {0xb0d82d931a592a79ULL, 0x15a956e225d67253ULL},
    {0x8d79be0f4847552eULL, 0x11544581b7dec1dcULL},
    {0x158f967eda0bbb7cULL, 0x1bba08cf8c979c94ULL},
    {0x77a611ff14d62f97ULL, 0x162e6d72d6dfb076ULL},
    {0xf951a7ff43de8c79ULL, 0x11bebdf578b2f391ULL},
    {0xc21c3ffed2fdad8eULL, 0x1c6463225ab7ec1cULL},
    {0x01b0333242648ad8ULL, 0x16b6b5b5155ff017ULL},
    {0x0159c28e9b83a246ULL, 0x122bc490dde659acULL},
    {0xcef604175f3903a3ULL, 0x1d12d41afca3c2acULL},
    {0x725e69ac4c2d9c83ULL, 0x17424348ca1c9bbdULL},
    {0xf5185489d68ae39cULL, 0x129b69070816e2fdULL},
    {0xee8d540fbdab05c6ULL, 0x1dc574d80cf16b2fULL},
    {0xbed77672fe226b05ULL, 0x17d12a4670c1228cULL},
    {0xff12c528cb4ebc04ULL, 0x130dbb6b8d674ed6ULL},
    {0xcb513b74787df9a0ULL, 0x1e7c5f127bd87e24ULL},
    {0x090dc929f9fe614dULL, 0x18637f41fcad31b7ULL},
    {0xa0d7d42194cb810aULL, 0x1382cc34ca2427c5ULL},
    {0x67bfb9cf5478ce77ULL, 0x1f37ad21436d0c6fULL},
    {0x1fcc94a5dd2d71f9ULL, 0x18f9574dcf8a7059ULL},
    {0x7fd6dd517dbdf4c7ULL, 0x13faac3e3fa1f37aULL},
    {0xffbe2ee8c92fee0bULL, 0x1ff779fd329cb8c3ULL},
    {0x6631bf20a0f324d6ULL, 0x1992c7fdc216fa36ULL},
    {0xb827cc1a1a5c1d78ULL, 0x14756ccb01abfb5eULL},
    {0x935309ae7b7ce460ULL, 0x105df0a267bcc918ULL},
    {0x1eeb42b0c594a099ULL, 0x1a2fe76a3f9474f4ULL},
    {0xe58902270476e6e1ULL, 0x14f31f8832dd2a5cULL},
    {0xb7a0ce859d2bebe7ULL, 0x10c27fa028b0eeb0ULL},
    {0x59014a6f61dfdfd8ULL, 0x1ad0cc33744e4ab4ULL},
    {0xe0cdd525e7e64cadULL, 0x1573d68f903ea229ULL},
    {0x4d7177518651d6f1ULL, 0x11297872d9cbb4eeULL},
    {0x7be8bee8d6e957e8ULL, 0x1b758d848fac54b0ULL},
    {0xfcba3253df211320ULL, 0x15f7a46a0c89dd59ULL},
    {0x63c8284318e74280ULL, 0x1192e9ee706e4aaeULL},
    {0x060d0d3827d86a66ULL, 0x1c1e43171a4a1117ULL},
    {0x6b3da42cecad21ebULL, 0x167e9c127b6e7412ULL},
    {0x88fe1cf0bd574e56ULL, 0x11fee341fc585cdbULL},
    {0x419694b462254a23ULL, 0x1ccb0536608d615fULL},
    {0x67abaa29e81dd4e9ULL, 0x1708d0f84d3de77fULL},
    {0xb95621bb2017dd87ULL, 0x126d73f9d764b932ULL},
    {0xc223692b668c95a5ULL, 0x1d7becc2f23ac1eaULL},
    {0xce82ba891ed6de1dULL, 0x179657025b6234bbULL},
    {0xa53562074bdf1818ULL, 0x12deac01e2b4f6fcULL},
    {0x3b889cd87964f359ULL, 0x1e3113363787f194ULL},
    {0xfc6d4a46c783f5e1ULL, 0x18274291c6065adcULL},
    {0x30576e9f06032b1aULL, 0x13529ba7d19eaf17ULL},
    {0x1a257dcb3cd1de90ULL, 0x1eea92a61c311825ULL},
    {0x481dfe3c30a7e540ULL, 0x18bba884e35a79b7ULL},
    {0xd34b31c9c0865100ULL, 0x13c9539d82aec7c5ULL},
    {0x5211e942cda3b4cdULL, 0x1fa885c8d117a609ULL},
    {0x74db21023e1c90a4ULL, 0x19539e3a40dfb807ULL},
    {0xf715b401cb4a0d50ULL, 0x1442e4fb67196005ULL},
    {0xf8de299b09080aa7ULL, 0x103583fc527ab337ULL},
    {0x8e304291a80cddd7ULL, 0x19ef3993b72ab859ULL},
    {0x3e8d020e200a4b13ULL, 0x14bf6142f8eef9e1ULL},
    {0x653d9b3e80083c0fULL, 0x10991a9bfa58c7e7ULL},
    {0x6ec8f864000d2ce4ULL, 0x1a8e90f9908e0ca5ULL},
    {0x8bd3f9e999a423eaULL, 0x153eda614071a3b7ULL},
    {0x3ca994bae1501cbbULL, 0x10ff151a99f482f9ULL},
    {0xc775bac49bb3612bULL, 0x1b31bb5dc320d18eULL},
    {0xd2c4956a16291a89ULL, 0x15c162b168e70e0bULL},
    {0xdbd0778811ba7ba1ULL, 0x11678227871f3e6fULL},
    {0x2c80bf401c5d929bULL, 0x1bd8d03f3e9863e6ULL},
    {0xbd33cc3349e47549ULL, 0x16470cff6546b651ULL},
    {0xca8fd68f6e505dd4ULL, 0x11d270cc51055ea7ULL},
    {0x4419574be3b3c953ULL, 0x1c83e7ad4e6efdd9ULL},
    {0x0347790982f63aa9ULL, 0x16cfec8aa52597e1ULL},
    {0xcf6c60d468c4fbbaULL, 0x123ff06eea847980ULL},
    {0xe57a34870e07f92aULL, 0x1d331a4b10d3f59aULL},
    {0x512e906c0b399422ULL, 0x175c1508da432ae2ULL},
    {0xda8ba6bcd5c7a9b5ULL, 0x12b010d3e1cf5581ULL},
    {0x90df712e22d90f87ULL, 0x1de6815302e5559cULL},
    {0xda4c5a8b4f140c6cULL, 0x17eb9aa8cf1dde16ULL},
    {0xaea37ba2a5a9a38aULL, 0x1322e220a5b17e78ULL},
    {0x7dd25f6aa2a905a9ULL, 0x1e9e369aa2b59727ULL},
    {0x97db7f888220d154ULL, 0x187e92154ef7ac1fULL},
    {0x797c6606ce80a777ULL, 0x139874ddd8c6234cULL},
    {0x8f2d700ae4010bf1ULL, 0x1f5a549627a36badULL},
    {0x0c2459a25000d65aULL, 0x191510781fb5efbeULL},
    {0x701d1481d99a4515ULL, 0x1410d9f9b2f7f2feULL},
    {0xc017439b147b6a77ULL, 0x100d7b2e28c65bfeULL},
    {0xccf205c4ed9243f2ULL, 0x19af2b7d0e0a2ccaULL},
    {0x0a5b37d0be0e9cc2ULL, 0x148c22ca71a1bd6fULL},
    {0x0848f973cb3ee3ceULL, 0x10701bd527b4978cULL},
    {0xda0e5bec78649fb0ULL, 0x1a4cf9550c5425acULL},
    {0x7b3eaff060507fc0ULL, 0x150a6110d6a9b7bdULL},
    {0x95cbbff380406633ULL, 0x10d51a73deee2c97ULL},
    {0xefac665266cd7052ULL, 0x1aee90b964b04758ULL},
    {0x2623850eb8a459dbULL, 0x158ba6fab6f36c47ULL},
    {0x1e82d0d893b6ae49ULL, 0x113c85955f29236cULL},
    {0xfd9e1af41f8ab075ULL, 0x1b9408eefea838acULL},
    {0x97b1af29b2d559f7ULL, 0x16100725988693bdULL},
    {0xac8e25baf5777b2cULL, 0x11a66c1e139edc97ULL},
    {0x7a7d092b2258c513ULL, 0x1c3d79c9b8fe2dbfULL},
    {0x61fda0ef4ead6a76ULL, 0x169794a160cb57ccULL},
    {0xe7fe1a590bbdeec5ULL, 0x1212dd4de7091309ULL},
    {0xa6635d5b45fcb13aULL, 0x1ceafbafd80e84dcULL},
    {0x851c4aaf6b308dc8ULL, 0x172262f3133ed0b0ULL},
    {0xd0e36ef2bc26d7d4ULL, 0x1281e8c275cbda26ULL},
    {0xb49f17eac6a48c86ULL, 0x1d9ca79d894629d7ULL},
    {0x2a18dfef0550706bULL, 0x17b08617a104ee46ULL},
    {0x54e0b3259dd9f389ULL, 0x12f39e794d9d8b6bULL},
    {0x87cdeb6f62f65274ULL, 0x1e5297287c2f4578ULL},
    {0xd30b22bf825ea85dULL, 0x18421286c9bf6ac6ULL},
    {0x0f3c1bcc684bb9e4ULL, 0x13680ed23aff889fULL},
    {0x18602c7a4079296dULL, 0x1f0ce4839198da98ULL},
    {0x46b356c833942124ULL, 0x18d71d360e13e213ULL},
    {0x388f78a029434db6ULL, 0x13df4a91a4dcb4dcULL},
    {0x5a7f2766a86baf8aULL, 0x1fcbaa82a1612160ULL},
    {0x153285ebb9efbfa2ULL, 0x196fbb9bb44db44dULL},
    {0xaa8ed189618c994eULL, 0x145962e2f6a4903dULL},
    {0xeed8a7a11ad6e10cULL, 0x1047824f2bb6d9caULL},
    {0x7e27729b5e249b45ULL, 0x1a0c03b1df8af611ULL},
    {0xfe85f549181d4904ULL, 0x14d6695b193bf80dULL},
    {0xcb9e5dd4134aa0d0ULL, 0x10ab877c142ff9a4ULL},
    {0xdf63c9535211014dULL, 0x1aac0bf9b9e65c3aULL},
    {0x191ca10f74da6771ULL, 0x15566ffafb1eb02fULL},
    {0xadb080d92a4852c1ULL, 0x1111f32f2f4bc025ULL},
    {0x15e7348eaa0d5134ULL, 0x1b4feb7eb212cd09ULL},
    {0xab1f5d3eee710dc4ULL, 0x15d98932280f0a6dULL},
    {0xbc1917658b8da49dULL, 0x117ad428200c0857ULL},
    {0x2cf4f23c127c3a94ULL, 0x1bf7b9d9cce00d59ULL},
    {0xf0c3f4fcdb969543ULL, 0x165fc7e170b33de0ULL},
    {0x5a365d9716121103ULL, 0x11e6398126f5cb1aULL},
    {0x9056fc24f01ce804ULL, 0x1ca38f350b22de90ULL},
    {0xd9df301d8ce3ecd0ULL, 0x16e93f5da2824ba6ULL},
    {0xe17f59b13d8323daULL, 0x125432b14ecea2ebULL},
    {0x68cbc2b52f38395cULL, 0x1d53844ee47dd179ULL},
    {0x53d6355dbf602de3ULL, 0x177603725064a794ULL},
    {0xa9782ab165e68b1cULL, 0x12c4cf8ea6b6ec76ULL},
    {0x0f26aab56fd744faULL, 0x1e07b27dd78b13f1ULL},
    {0x3f52222abfdf6a62ULL, 0x18062864ac6f4327ULL},
    {0x65db4e88997f884eULL, 0x1338205089f29c1fULL},
    {0x6fc54a7428cc0d4aULL, 0x1ec033b40fea9365ULL},
    {0x596aa1f68709a43bULL, 0x1899c2f673220f84ULL},
    {0xadeee7f86c07b696ULL, 0x13ae3591f5b4d936ULL},
    {0x497e3ff3e00c5756ULL, 0x1f7d228322baf524ULL},
    {0xd464fff64cd6ac45ULL, 0x1930e868e89590e9ULL},
    {0x4383fff83d7889d1ULL, 0x14272053ed4473eeULL},
    {0xcf9cccc69793a174ULL, 0x101f4d0ff1038ff1ULL},
    {0x7f6147a425b90252ULL, 0x19cbae7fe805b31cULL},
    {0xcc4dd2e9b7c7350fULL, 0x14a2f1ffecd15c16ULL},
    {0x3d0b0f215fd290d9ULL, 0x10825b3323dab012ULL},
    {0x61ab4b689950e7c1ULL, 0x1a6a2b85062ab350ULL},
    {0x4e22a2ba1440b967ULL, 0x1521bc6a6b555c40ULL},
    {0x0b4ee894dd009453ULL, 0x10e7c9eebc4449cdULL},
    {0x1217da87c800ed51ULL, 0x1b0c764ac6d3a948ULL},
    {0xdb46486ca000bddaULL, 0x15a391d56bdc876cULL},
    {0x490506bd4ccd64afULL, 0x114fa7ddefe39f8aULL},
    {0xa8080ac87ae23ab1ULL, 0x1bb2a62fe638ff43ULL},
    {0x5339a239fbe82ef4ULL, 0x162884f31e93ff69ULL},
    {0x75c7b4fb2fecf25dULL, 0x11ba03f5b20fff87ULL},
    {0x22d92191e647ea2eULL, 0x1c5cd322b67fff3fULL},
    {0xb57a8141850654f2ULL, 0x16b0a8e891ffff65ULL},
    {0xc4620101373843f5ULL, 0x1226ed86db3332b7ULL},
    {0x3a366801f1f39feeULL, 0x1d0b15a491eb8459ULL},
    {0xfb5eb99b27f6198bULL, 0x173c115074bc69e0ULL},
    {0x2f7efae2865e7ad6ULL, 0x129674405d6387e7ULL},
    {0xe597f7d0d6fd9156ULL, 0x1dbd86cd6238d971ULL},
    {0x8479930d78cadaabULL, 0x17cad23de82d7ac1ULL},
    {0xd06142712d6f1556ULL, 0x1308a831868ac89aULL},
    {0x4d686a4eaf182222ULL, 0x1e74404f3daada91ULL},
    {0xa453883ef279b4e8ULL, 0x185d003f6488aedaULL},
    {0xe9dc6cff28615d87ULL, 0x137d99cc506d58aeULL},
    {0xa960ae650d6895a4ULL, 0x1f2f5c7a1a488de4ULL},
    {0xbab3beb73ded4483ULL, 0x18f2b061aea07183ULL},
    {0x2ef6322c318a9d36ULL, 0x13f559e7bee6c136ULL},
};

static inline uint64_t ryu_mulshift64(uint64_t m, const uint64_t* mul, int j) {
    __uint128_t b0 = (__uint128_t)m * mul[0];
    __uint128_t b2 = (__uint128_t)m * mul[1];
    return (uint64_t)(((b0 >> 64) + b2) >> (j - 64));
}

static inline int ryu_pow5bits(int e) { return (int)(((uint32_t)e * 1217359) >> 19) + 1; }
static inline uint32_t ryu_log10pow2(int e) { return ((uint32_t)e * 78913) >> 18; }
static inline uint32_t ryu_log10pow5(int e) { return ((uint32_t)e * 732923) >> 20; }

static inline int ryu_pow5factor(uint64_t v) {
    int count = 0;
    for (;;) {
        uint64_t q = v / 5;
        if (q * 5 != v) return count;
        v = q;
        count++;
    }
}
static inline int ryu_multiple_of_pow5(uint64_t v, int p) { return ryu_pow5factor(v) >= p; }
static inline int ryu_multiple_of_pow2(uint64_t v, int p) {
    return (v & ((1ULL << p) - 1)) == 0;
}

/* shortest digits + decimal exponent for a positive finite double; returns
   digit count, digits in dig[], value = dig * 10^*e10 */
static int ryu_d2d(double f, char* dig, int* e10) {
    uint64_t bits;
    __builtin_memcpy(&bits, &f, 8);
    uint64_t ieee_m = bits & ((1ULL << 52) - 1);
    uint32_t ieee_e = (uint32_t)(bits >> 52) & 0x7FF;
    int e2;
    uint64_t m2;
    if (ieee_e == 0) { e2 = 1 - 1023 - 52 - 2; m2 = ieee_m; }
    else { e2 = (int)ieee_e - 1023 - 52 - 2; m2 = (1ULL << 52) | ieee_m; }
    int even = (m2 & 1) == 0;
    int accept = even;

    uint64_t mv = 4 * m2;
    uint32_t mm_shift = ieee_m != 0 || ieee_e <= 1;

    uint64_t vr, vp, vm;
    int e10v;
    int vm_trailing = 0, vr_trailing = 0;
    uint8_t last_removed = 0;
    if (e2 >= 0) {
        uint32_t q = ryu_log10pow2(e2) - (e2 > 3);
        e10v = (int)q;
        int k = RYU_POW5_INV_BITCOUNT + ryu_pow5bits((int)q) - 1;
        int i = -e2 + (int)q + k;
        vr = ryu_mulshift64(mv, RYU_POW5_INV[q], i);
        vp = ryu_mulshift64(mv + 2, RYU_POW5_INV[q], i);
        vm = ryu_mulshift64(mv - 1 - mm_shift, RYU_POW5_INV[q], i);
        if (q <= 21) {
            if (mv % 5 == 0) vr_trailing = ryu_multiple_of_pow5(mv, (int)q);
            else if (accept) vm_trailing = ryu_multiple_of_pow5(mv - 1 - mm_shift, (int)q);
            else vp -= ryu_multiple_of_pow5(mv + 2, (int)q);
        }
    } else {
        uint32_t q = ryu_log10pow5(-e2) - (-e2 > 1);
        e10v = (int)q + e2;
        int i = -e2 - (int)q;
        int k = ryu_pow5bits(i) - RYU_POW5_BITCOUNT;
        int j = (int)q - k;
        vr = ryu_mulshift64(mv, RYU_POW5[i], j);
        vp = ryu_mulshift64(mv + 2, RYU_POW5[i], j);
        vm = ryu_mulshift64(mv - 1 - mm_shift, RYU_POW5[i], j);
        if (q <= 1) {
            vr_trailing = 1;
            if (accept) vm_trailing = mm_shift == 1;
            else vp -= 1;
        } else if (q < 63) {
            vr_trailing = ryu_multiple_of_pow2(mv, (int)q);
        }
    }

    int removed = 0;
    uint64_t output;
    if (vm_trailing || vr_trailing) {
        for (;;) {
            uint64_t vpd = vp / 10, vmd = vm / 10;
            if (vpd <= vmd) break;
            uint32_t vmm = (uint32_t)(vm % 10);
            uint64_t vrd = vr / 10;
            uint32_t vrm = (uint32_t)(vr % 10);
            vm_trailing &= vmm == 0;
            vr_trailing &= last_removed == 0;
            last_removed = (uint8_t)vrm;
            vr = vrd; vp = vpd; vm = vmd;
            removed++;
        }
        if (vm_trailing) {
            for (;;) {
                uint64_t vmd = vm / 10;
                uint32_t vmm = (uint32_t)(vm % 10);
                if (vmm != 0) break;
                uint64_t vpd = vp / 10;
                uint64_t vrd = vr / 10;
                uint32_t vrm = (uint32_t)(vr % 10);
                vr_trailing &= last_removed == 0;
                last_removed = (uint8_t)vrm;
                vr = vrd; vp = vpd; vm = vmd;
                removed++;
            }
        }
        if (vr_trailing && last_removed == 5 && vr % 2 == 0) {
            last_removed = 4;
        }
        output = vr + ((vr == vm && (!accept || !vm_trailing)) || last_removed >= 5);
    } else {
        int round_up = 0;
        uint64_t vpd100 = vp / 100, vmd100 = vm / 100;
        if (vpd100 > vmd100) {
            uint64_t vrd100 = vr / 100;
            uint32_t vrm100 = (uint32_t)(vr % 100);
            round_up = vrm100 >= 50;
            vr = vrd100; vp = vpd100; vm = vmd100;
            removed += 2;
        }
        for (;;) {
            uint64_t vpd = vp / 10, vmd = vm / 10;
            if (vpd <= vmd) break;
            uint64_t vrd = vr / 10;
            uint32_t vrm = (uint32_t)(vr % 10);
            round_up = vrm >= 5;
            vr = vrd; vp = vpd; vm = vmd;
            removed++;
        }
        output = vr + (vr == vm || round_up);
    }
    *e10 = e10v + removed;
    int n = 0;
    char tmp[20];
    while (output > 0) { tmp[n++] = (char)('0' + output % 10); output /= 10; }
    for (int a = 0; a < n; a++) dig[a] = tmp[n - 1 - a];
    dig[n] = 0;
    return n;
}

/* format (digits, k, e10) exactly as the probe would have: %g with
   precision max(15, k) — fixed vs exponent at X < -4 or X >= P */
static void render_ryu(double d, char* buf) {
    if (d == 0.0) { snprintf(buf, 64, "%.1f", d); return; }
    char dig[20];
    int e10;
    int k = ryu_d2d(d, dig, &e10);
    int x = e10 + k - 1; /* decimal exponent of the leading digit */
    int p = k > 15 ? k : 15;
    char* o = buf;
    if (x < -4 || x >= p) {
        *o++ = dig[0];
        if (k > 1) {
            *o++ = '.';
            for (int i = 1; i < k; i++) *o++ = dig[i];
        }
        *o++ = 'e';
        int ex = x;
        *o++ = ex < 0 ? '-' : '+';
        if (ex < 0) ex = -ex;
        if (ex >= 100) { *o++ = (char)('0' + ex / 100); ex %= 100; *o++ = (char)('0' + ex / 10); }
        else { *o++ = (char)('0' + ex / 10); }
        *o++ = (char)('0' + ex % 10);
        *o = 0;
    } else if (x >= 0) {
        int ip = x + 1; /* digits before the point */
        for (int i = 0; i < ip; i++) *o++ = i < k ? dig[i] : '0';
        if (k > ip) {
            *o++ = '.';
            for (int i = ip; i < k; i++) *o++ = dig[i];
        }
        *o = 0;
    } else {
        *o++ = '0'; *o++ = '.';
        for (int i = 0; i < -x - 1; i++) *o++ = '0';
        for (int i = 0; i < k; i++) *o++ = dig[i];
        *o = 0;
    }
}


KValue k_render(KValue v, long long quote) {
    // an err propagates through rendering (it is an exception); a none is a
    // value and renders its sentinel below
    if (v.tag == K_ERR) return v;
    char buf[64];
    switch (v.tag) {
        case K_INT:
            k_itoa(buf, v.payload);
            return k_str(buf);
        case K_FLOAT: {
            double d = k_as_f(v);
            if (d == floor(d) && fabs(d) < 1e15 && isfinite(d)) {
                snprintf(buf, sizeof buf, "%.1f", d);
                return k_str(buf);
            }
            /* shortest round-trip: %g trims trailing zeros, so probing
               15..17 yields byte-identical strings to probing 1..17 — a
               double never needs more, and rarely fewer, than 15 digits */
            /* the ryu digit core computes the true shortest round-trip
               representation directly — no probing, no dtoa */
            if (d < 0) {
                char inner[63];
                render_ryu(-d, inner);
                buf[0] = '-';
                strcpy(buf + 1, inner);
            } else {
                render_ryu(d, buf);
            }
            return k_str(buf);
        }
        case K_TRUE: return k_str("true");
        case K_FALSE: return k_str("false");
        case K_NONE: return k_str("<none>");
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
        case K_DESC: return k_str("<io>");
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
            if (n == 0) return k_str("{:}");
            KValue out = k_str("{ ");
            for (long long i = 0; i < n; i++) {
                if (i) out = k_concat(out, k_str(" "));
                out = k_concat(out, k_render(s[i * 2], 1));
                out = k_concat(out, k_str(":"));
                out = k_concat(out, k_render(s[i * 2 + 1], 1));
            }
            return k_concat(out, k_str(" }"));
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
    if (a.tag == K_INT && b.tag == K_FLOAT) return k_float((double)a.payload + k_as_f(b));
    if (a.tag == K_FLOAT && b.tag == K_INT) return k_float(k_as_f(a) + (double)b.payload);
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
    if (a.tag == K_INT && b.tag == K_FLOAT) return k_float((double)a.payload - k_as_f(b));
    if (a.tag == K_FLOAT && b.tag == K_INT) return k_float(k_as_f(a) - (double)b.payload);
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
    if (a.tag == K_INT && b.tag == K_FLOAT) return k_float((double)a.payload * k_as_f(b));
    if (a.tag == K_FLOAT && b.tag == K_INT) return k_float(k_as_f(a) * (double)b.payload);
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
    if ((a.tag == K_INT || a.tag == K_FLOAT) && (b.tag == K_INT || b.tag == K_FLOAT)) {
        double x = a.tag == K_INT ? (double)a.payload : k_as_f(a);
        double y = b.tag == K_INT ? (double)b.payload : k_as_f(b);
        if (y == 0.0) return k_err(k_str("division by zero"), origin);
        return k_float(x / y);
    }
    k_die("`/` is not defined for these values");
    return k_none();
}

KValue k_mod(KValue a, KValue b, const char* origin) {
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (a.tag == K_INT && b.tag == K_INT) {
        if (b.payload == 0) return k_err(k_str("modulo by zero"), origin);
        return k_int(a.payload % b.payload);
    }
    if ((a.tag == K_INT || a.tag == K_FLOAT) && (b.tag == K_INT || b.tag == K_FLOAT)) {
        double x = a.tag == K_INT ? (double)a.payload : k_as_f(a);
        double y = b.tag == K_INT ? (double)b.payload : k_as_f(b);
        if (y == 0.0) return k_err(k_str("modulo by zero"), origin);
        return k_float(fmod(x, y));
    }
    k_die("`%` is not defined for these values");
    return k_none();
}

static int k_order(KValue a, KValue b) {
    if (a.tag == K_INT && b.tag == K_INT) return (a.payload > b.payload) - (a.payload < b.payload);
    if (a.tag == K_FLOAT && b.tag == K_FLOAT) {
        double x = k_as_f(a);
        double y = k_as_f(b);
        return (x > y) - (x < y);
    }
    if ((a.tag == K_INT || a.tag == K_FLOAT) && (b.tag == K_INT || b.tag == K_FLOAT)) {
        double x = a.tag == K_INT ? (double)a.payload : k_as_f(a);
        double y = b.tag == K_INT ? (double)b.payload : k_as_f(b);
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
    KDesc* d = k_alloc(sizeof(KDesc));
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

/* Merge two failures: err + err becomes one err whose reason lists both
   reasons (origin-less: the merge has no single birthplace); a none adds
   nothing to an err; two nones stay none. Mirrors eval.rs exactly. */
static KValue k_accumulate_failures(KValue l, KValue r) {
    if (l.tag == K_ERR && r.tag == K_ERR) {
        KValue* items = k_buf(2);
        items[0] = k_err_inner(l);
        items[1] = k_err_inner(r);
        return k_err(k_list_own(items, 2), NULL);
    }
    if (l.tag == K_ERR) return l;
    if (r.tag == K_ERR) return r;
    return l;
}

/* a & b: join two descriptions to run with no order between them. Both sides
   are already evaluated; failures accumulate instead of short-circuiting. */
KValue k_desc_join(KValue a, KValue b) {
    int lf = !k_not_failure(a);
    int rf = !k_not_failure(b);
    if (lf && rf) return k_accumulate_failures(a, b);
    if (lf) return a;
    if (rf) return b;
    if (a.tag != K_DESC || b.tag != K_DESC) k_die("`&` joins two descriptions");
    return k_mkdesc(7, a, b);
}

KValue k_maybe_bind(KValue piped, KValue closure) {
    if (piped.tag == K_DESC) return k_mkdesc(6, piped, closure);
    return k_call1(closure, piped);
}

KValue k_desc_sleep(KValue ms) {
    if (!k_not_failure(ms)) return ms;
    if (ms.tag != K_INT) k_die("sleep takes milliseconds (an int)");
    return k_mkdesc(8, ms, k_none());
}

KValue k_desc_random(KValue n) {
    if (!k_not_failure(n)) return n;
    if (n.tag != K_INT) k_die("random takes a bound (an int)");
    return k_mkdesc(9, n, k_none());
}

static KValue k_desc_nil(void) { return k_mkdesc(10, k_none(), k_none()); }

/* SplitMix64, matching the interpreter's Rng. A real run seeds from
   entropy so dice roll differently each time; KANSO_SEED pins the stream,
   which is how the differential lattice and the goldens hold a concurrent
   program's output byte-identical across runs and engines. */
static uint64_t k_rng_state = 0;
static int k_rng_ready = 0;

static void k_rng_seed(void) {
    const char* s = getenv("KANSO_SEED");
    if (s) {
        k_rng_state = strtoull(s, NULL, 10);
    } else {
        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        uint64_t nanos = (uint64_t)ts.tv_sec * 1000000000ULL + (uint64_t)ts.tv_nsec;
        k_rng_state = nanos ^ ((uint64_t)getpid() << 32);
    }
    k_rng_ready = 1;
}

static uint64_t k_rng_next(void) {
    if (!k_rng_ready) k_rng_seed();
    k_rng_state += 0x9E3779B97F4A7C15ULL;
    uint64_t z = k_rng_state;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

static long long k_rng_below(long long n) {
    return n <= 0 ? 0 : (long long)(k_rng_next() % (uint64_t)n);
}

/* One step of a fiber: it finished (blocked=0, value set) or blocked on a
   `sleep` (blocked=1, ms + cont). Blocking threads the continuation up through
   Seq and Bind, so `sleep` may sit anywhere and suspension needs no coroutine.
   Mirrors eval.rs's Step exactly. */
typedef struct { int blocked; unsigned long long ms; KValue cont; KValue value; } KStep;

static KValue k_schedule(KDesc* join);
static KStep k_step(KDesc* d);

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
            /* a >> step is a beat: the left side's yield is discarded by
               contract, so everything it allocated dies here — unless it
               failed, in which case the err (and its region) survives. */
            k_beat_push();
            KValue left = k_exec(k_as_desc(d->x));
            if (left.tag == K_ERR) return k_beat_pop(left);
            k_beat_pop(k_none());
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
        case 7: {
            /* no order between the sides, and both always run — a failure on
               one never abandons the other; failures accumulate. each side is
               a beat: its yield is discarded on success, so only a failing
               side keeps its region. */
            return k_schedule(d);
        }
        case 8: {
            /* a bare sleep executed outside a group: pause for real */
            long long ms = d->x.tag == K_INT ? d->x.payload : 0;
            if (ms > 0) usleep((useconds_t)(ms * 1000));
            return k_none();
        }
        case 9: {
            long long n = d->x.tag == K_INT ? d->x.payload : 0;
            return k_int(k_rng_below(n));
        }
        case 10: {
            return k_none();
        }
        default: {
            /* a bind chain is the program's outer pulse, so it runs as one
               bracketed loop: each step executes, hands its yield to the
               continuation, and the returned description is evacuated
               through the carry buffers before the step's garbage is swept.
               memory stays flat across any chain length, and the chain
               costs constant C stack. the final result goes through the
               pop, which copies a heap survivor out of the buffers. */
            k_beat_push();
            KValue cur;
            cur.tag = K_DESC;
            cur.payload = k_ptr(d);
            for (;;) {
                KDesc* dd = k_as_desc(cur);
                if (dd->dtag != 6) {
                    return k_beat_pop(k_exec(dd));
                }
                KValue yielded = k_exec(k_as_desc(dd->x));
                KValue next = k_call1(dd->y, yielded);
                if (next.tag != K_DESC) {
                    return k_beat_pop(next);
                }
                k_carry_reset();
                k_carry_stage(next);
                k_beat_iter_carry();
                cur = k_carry_take(0);
            }
        }
    }
}

static KStep k_step(KDesc* d) {
    switch (d->dtag) {
        case 8: {
            long long ms = d->x.tag == K_INT ? d->x.payload : 0;
            KStep s = {1, (unsigned long long)(ms < 0 ? 0 : ms), k_desc_nil(), k_none()};
            return s;
        }
        case 1: {
            KStep l = k_step(k_as_desc(d->x));
            if (l.blocked) {
                KStep s = {1, l.ms, k_mkdesc(1, l.cont, d->y), k_none()};
                return s;
            }
            if (l.value.tag == K_ERR) {
                KStep s = {0, 0, k_none(), l.value};
                return s;
            }
            return k_step(k_as_desc(d->y));
        }
        case 6: {
            KStep in = k_step(k_as_desc(d->x));
            if (in.blocked) {
                KStep s = {1, in.ms, k_mkdesc(6, in.cont, d->y), k_none()};
                return s;
            }
            KValue next = k_call1(d->y, in.value);
            if (next.tag == K_DESC) return k_step(k_as_desc(next));
            KStep s = {0, 0, k_none(), next};
            return s;
        }
        default: {
            /* leaf effect or nested join: run to completion synchronously */
            KStep s = {0, 0, k_none(), k_exec(d)};
            return s;
        }
    }
}

static void k_flatten_join(KDesc* d, KValue* out, int* n, int cap) {
    if (d->dtag == 7) {
        k_flatten_join(k_as_desc(d->x), out, n, cap);
        k_flatten_join(k_as_desc(d->y), out, n, cap);
    } else {
        if (*n >= cap) k_die("parallel group exceeds the scheduler's fiber cap");
        KValue v; v.tag = K_DESC; v.payload = k_ptr(d);
        out[(*n)++] = v;
    }
}

/* Run a parallel group as cooperative green threads: each member a fiber,
   deterministic earliest-wake scheduling (ties by spawn order), `sleep`
   yields. Values are discarded (a group yields none); failures accumulate.
   The whole group runs under one beat — grow-only inside, everything the
   fibers allocate reclaimed at the end (their yields are garbage), which
   sidesteps any interaction between suspension and the beat mark-stack. */
/* signed milliseconds since a monotonic mark — the nsec delta may be
   negative when the nanosecond field wraps, so the math stays signed */
static long long k_ms_since(const struct timespec* t0) {
    struct timespec tn;
    clock_gettime(CLOCK_MONOTONIC, &tn);
    return (long long)(tn.tv_sec - t0->tv_sec) * 1000LL
        + (long long)(tn.tv_nsec - t0->tv_nsec) / 1000000LL;
}

static KValue k_schedule(KDesc* join) {
    int n = 0;
    KValue tmp[256];
    k_flatten_join(join, tmp, &n, 256);
    unsigned long long* wake = malloc(sizeof(unsigned long long) * n);
    KValue* fiber = malloc(sizeof(KValue) * n);
    int* done = malloc(sizeof(int) * n);
    for (int i = 0; i < n; i++) { wake[i] = 0; fiber[i] = tmp[i]; done[i] = 0; }
    unsigned long long now = 0;
    /* wall-credit: real time spent computing counts against a pending
       wait, so compute overlaps sleeps in wall-clock. the transcript
       stays purely logical — only the physical wait shrinks. */
    struct timespec sched_t0;
    clock_gettime(CLOCK_MONOTONIC, &sched_t0);
    KValue result = k_none();
    int remaining = n;
    k_beat_push();
    while (remaining > 0) {
        int pick = -1;
        for (int i = 0; i < n; i++) {
            if (!done[i] && (pick < 0 || wake[i] < wake[pick])) pick = i;
        }
        if (wake[pick] > now) {
            if (getenv("KANSO_SCHED_DEBUG")) {
                fprintf(stderr, "[sched] pick=%d wake=%llu elapsed=%lld\n",
                        pick, wake[pick], k_ms_since(&sched_t0));
            }
            /* loop to the deadline: usleep may return early on a signal */
            for (;;) {
                long long elapsed = k_ms_since(&sched_t0);
                if ((long long)wake[pick] <= elapsed) break;
                usleep((useconds_t)(((long long)wake[pick] - elapsed) * 1000));
            }
            now = wake[pick];
        }
        KStep s = k_step(k_as_desc(fiber[pick]));
        if (s.blocked) {
            wake[pick] = now + s.ms;
            fiber[pick] = s.cont;
        } else {
            done[pick] = 1;
            remaining--;
            if (!k_not_failure(s.value)) {
                result = result.tag == K_ERR
                    ? k_accumulate_failures(result, s.value)
                    : s.value;
            }
        }
    }
    free(wake); free(fiber); free(done);
    return k_beat_pop(result);
}

/* Exported (not static): the codegen prelude's inline k_truthy calls this on
   its cold path, so the die message lives in exactly one place. */
long long k_truthy_bad(void) {
    k_die("an if condition is true or false");
    return 0;
}

/* Fires on every `if` condition. The codegen prelude carries an
   alwaysinline IR twin of this body (internal linkage, so the symbols never
   collide) — LTO declined to inline across the .ll/.o boundary, leaving a
   real call on the hottest path; the IR twin makes the inline deterministic.
   This copy remains for the runtime's own internal callers. */
long long k_truthy(KValue v) {
    if (v.tag == K_TRUE) return 1;
    if (v.tag == K_FALSE) return 0;
    return k_truthy_bad();
}

/* ---- slice 2: lists, maps, closures, builtins ---- */

static KValue* k_buf(long long cap) {
    KBuf* b = k_alloc(sizeof(KBuf) + sizeof(KValue) * cap);
    b->cap = cap;
    b->used = 0;
    return (KValue*)(b + 1);
}

static KBuf* k_buf_of(KValue* items) { return ((KBuf*)items) - 1; }

/* Take ownership of an already-filled k_buf-backed item buffer as a list, with
   no copy: set its `used` to the length and wrap it. Callers that can build
   straight into a k_buf use this instead of k_mklist, whose job is to copy a
   caller's transient buffer into a fresh k_buf. */
static KValue k_list_own(KValue* items, long long n) {
    KList* l = k_alloc(sizeof(KList));
    l->len = n;
    l->items = items;
    k_buf_of(items)->used = n;
    KValue v; v.tag = K_LIST; v.payload = k_ptr(l); return v;
}

static KValue k_mklist(long long n, KValue* items) {
    KValue* buf = k_buf(n ? n : 1);
    memcpy(buf, items, sizeof(KValue) * n);
    return k_list_own(buf, n);
}

KValue k_list_lit(long long n, KValue* items) {
    return k_mklist(n, items);
}

KValue k_closure(KValue (*fn)(void*, KValue), long long ncaps, KValue* caps) {
    KClosure* c = k_alloc(sizeof(KClosure));
    KValue* env = k_alloc(sizeof(KValue) * (ncaps ? ncaps : 1));
    memcpy(env, caps, sizeof(KValue) * ncaps);
    c->fn = fn; c->env = env; c->ncaps = ncaps;
    KValue v; v.tag = K_CLOSURE; v.payload = k_ptr(c); return v;
}

KValue k_fnref(void* dispatcher) {
    KValue v; v.tag = K_FNREF; v.payload = (long long)(intptr_t)dispatcher; return v;
}

KValue k_env_get(void* env, long long i) { return ((KValue*)env)[i]; }

KValue k_call1(KValue f, KValue a) {
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

/* Calling a lambda value with more than one argument. The closure's fn pointer
   is stored generically; cast it to the arity the call site knows. Failures in
   the callable or any argument propagate before the body runs, matching the
   dispatcher. Arity is checked by the type system, so no runtime arity guard. */
KValue k_call2(KValue f, KValue a, KValue b) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (f.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        return ((KValue(*)(void*, KValue, KValue))c->fn)(c->env, a, b);
    }
    if (f.tag == K_FNREF) {
        return ((KValue(*)(KValue, KValue))(intptr_t)f.payload)(a, b);
    }
    k_die("this value is not callable");
    return k_none();
}

KValue k_call3(KValue f, KValue a, KValue b, KValue c) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (!k_not_failure(c)) return c;
    if (f.tag == K_CLOSURE) {
        KClosure* cl = (KClosure*)(intptr_t)f.payload;
        return ((KValue(*)(void*, KValue, KValue, KValue))cl->fn)(cl->env, a, b, c);
    }
    if (f.tag == K_FNREF) {
        return ((KValue(*)(KValue, KValue, KValue))(intptr_t)f.payload)(a, b, c);
    }
    k_die("this value is not callable");
    return k_none();
}

KValue k_call4(KValue f, KValue a, KValue b, KValue c, KValue d) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (!k_not_failure(c)) return c;
    if (!k_not_failure(d)) return d;
    if (f.tag == K_CLOSURE) {
        KClosure* cl = (KClosure*)(intptr_t)f.payload;
        return ((KValue(*)(void*, KValue, KValue, KValue, KValue))cl->fn)(cl->env, a, b, c, d);
    }
    if (f.tag == K_FNREF) {
        return ((KValue(*)(KValue, KValue, KValue, KValue))(intptr_t)f.payload)(a, b, c, d);
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
        k_cache_reg_add(m);
    }
    if (out_len) *out_len = m->sorted_len;
    return m->sorted;
}

KValue k_map_lit(long long n, KValue* flat_pairs) {
    /* literal keys arrive sorted and unique from the parser; k_map_sorted
       still recomputes on first read, cheaply (already sorted, no dups). */
    KMap* m = k_alloc(sizeof(KMap));
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
    KMap* out = k_alloc(sizeof(KMap));
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
    KValue* items = k_buf(n ? n : 1);
    for (long long i = 0; i < n; i++) {
        KValue* fields = k_alloc(sizeof(KValue) * 2);
        fields[0] = s[i * 2];
        fields[1] = s[i * 2 + 1];
        items[i] = k_rec(0, 2, fields);
    }
    return k_list_own(items, n);
}

/* utf-8 helpers: kanso strings are opaque utf-8, positions are codepoints */
static long k_cp_len(unsigned char b) {
    if (b < 0x80) return 1;
    if (b < 0xe0) return 2;
    if (b < 0xf0) return 3;
    return 4;
}

static KValue k_bytes_view(const unsigned char* data, long long len) {
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
    if (!k_not_failure(av)) return av;
    if (!k_not_failure(bv)) return bv;
    long long alen = k_seq_len(av), blen = k_seq_len(bv);
    long long n = alen + blen;
    KValue* items = k_buf(n ? n : 1);
    k_seq_into(av, items);
    k_seq_into(bv, items + alen);
    return k_list_own(items, n);
}

static KValue k_utf8_bad(const char* data, long long len, const char* origin);
static KValue k_utf8_check(char* data, long long len, const char* origin);

KValue k_b_utf8(KValue lv, const char* origin) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag == K_BYTES) {
        /* validate directly on the view (read-only) and let k_str_n do the one
           copy into the string — a pre-copy here would just be a second pass. */
        KBytes* b = k_as_bytes(lv);
        return k_utf8_check((char*)b->data, b->len, origin);
    }
    if (lv.tag != K_LIST) k_die("utf8 takes a list of byte values");
    KList* l = k_as_list(lv);
    /* build straight into the string's own buffer, then validate in place — the
       old path filled a scratch buffer and let k_str_n copy it a second time. */
    KStr* s = k_alloc(sizeof(KStr));
    s->len = (long)l->len;
    s->data = k_alloc(l->len + 1);
    for (long long i = 0; i < l->len; i++) {
        KValue item = l->items[i];
        if (!k_not_failure(item)) return item;
        if (item.tag != K_INT || item.payload < 0 || item.payload > 255) {
            return k_err(k_str("utf8 takes byte values (0-255)"), origin);
        }
        s->data[i] = (char)item.payload;
    }
    s->data[l->len] = 0;
    KValue bad = k_utf8_bad(s->data, l->len, origin);
    if (bad.tag == K_ERR) return bad;
    KValue v; v.tag = K_STR; v.payload = k_ptr(s); return v;
}

static KValue k_utf8_bad(const char* data, long long len, const char* origin) {
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
    long long nblocks = (len + 15) / 16 + 1;
    for (long long blk = 0; blk < nblocks; blk++) {
        uint8_t buf[16];
        uint8x16_t cur;
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
        prev = cur;
    }
    if (vmaxvq_u8(error) != 0) return k_err(k_str("invalid utf-8"), origin);
    return k_none();
#else
    long long i = 0;
    while (i < len) {
#if defined(__x86_64__)
        while (i + 16 <= len) {
            __m128i chunk = _mm_loadu_si128((const __m128i*)(data + i));
            if (_mm_movemask_epi8(chunk)) break;
            i += 16;
        }
        if (i >= len) break;
#endif
        long long block_end = i + 16 <= len ? i + 16 : len;
        while (i < block_end) {
            unsigned char b0 = (unsigned char)data[i];
            if (b0 < 0x80) { i += 1; continue; }
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
            i += w;
        }
    }
    return k_none();
#endif
}

static KValue k_utf8_check(char* data, long long len, const char* origin) {
    KValue bad = k_utf8_bad(data, len, origin);
    if (bad.tag == K_ERR) return bad;
    return k_str_n(data, len);
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
        KList* out = k_alloc(sizeof(KList));
        out->len = l->len + 1;
        out->items = l->items;
        KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
    }
    long long cap = l->len < 2 ? 4 : l->len * 2;
    KValue* items = k_buf(cap);
    memcpy(items, l->items, sizeof(KValue) * l->len);
    items[l->len] = item;
    k_buf_of(items)->used = l->len + 1;
    KList* out = k_alloc(sizeof(KList));
    out->len = l->len + 1;
    out->items = items;
    KValue v; v.tag = K_LIST; v.payload = k_ptr(out); return v;
}

/* In-place push, emitted only where the linearity analysis proved the list is
   uniquely owned. On the frontier it mutates the header — no per-element
   allocation — which is the whole win; off the frontier it grows like a normal
   push (a uniquely-owned list is never off-frontier unless it just grew). */
KValue k_b_push_mut(KValue lv, KValue item) {
    if (!k_not_failure(lv)) return lv;
    if (lv.tag != K_LIST) k_die("push takes a list and a value");
    KList* l = k_as_list(lv);
    KBuf* buf = k_buf_of(l->items);
    if (buf->used == l->len && l->len < buf->cap) {
        l->items[l->len] = item;
        buf->used++;
        l->len++;
        return lv;
    }
    return k_b_push(lv, item);
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

/* Scan for the first of two bytes at or after a 1-based position — the string
   scanner's inner loop, done as a tight pass instead of one boxed dispatch per
   byte. Returns the 1-based hit, or len+1 when neither byte appears. */
KValue k_b_find2(KValue cs, KValue from, KValue a, KValue b) {
    if (!k_not_failure(cs)) return cs;
    if (!k_not_failure(from)) return from;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (cs.tag != K_BYTES) k_die("find2 takes bytes");
    KBytes* by = k_as_bytes(cs);
    long long p = from.payload < 1 ? 0 : from.payload - 1;
    unsigned char ca = (unsigned char)(a.payload & 0xff);
    unsigned char cb = (unsigned char)(b.payload & 0xff);
    const unsigned char* d = by->data;
    long long i = p;
#if defined(__aarch64__)
    /* 16 bytes per step; the shrn-by-4 narrow turns the match vector into a
       64-bit mask (4 bits per byte), so ctz/4 names the first hit. */
    uint8x16_t va = vdupq_n_u8(ca), vb = vdupq_n_u8(cb);
    for (; i + 16 <= by->len; i += 16) {
        uint8x16_t chunk = vld1q_u8(d + i);
        uint8x16_t m = vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb));
        uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
        uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
        if (mask) return k_int(i + (__builtin_ctzll(mask) >> 2) + 1);
    }
#elif defined(__x86_64__)
    __m128i va = _mm_set1_epi8((char)ca), vb = _mm_set1_epi8((char)cb);
    for (; i + 16 <= by->len; i += 16) {
        __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
        __m128i m = _mm_or_si128(_mm_cmpeq_epi8(chunk, va), _mm_cmpeq_epi8(chunk, vb));
        int mask = _mm_movemask_epi8(m);
        if (mask) return k_int(i + __builtin_ctz(mask) + 1);
    }
#endif
    for (; i < by->len; i++) {
        if (d[i] == ca || d[i] == cb) return k_int(i + 1);
    }
    return k_int(by->len + 1);
}

/* The byte builder. Appends a string, a bytes value, or a single byte
   onto a bytes accumulator. The accumulator owns a KBuf-headed buffer and
   claims its frontier exactly as list push does, so a fold of appends is
   amortized linear while every intermediate value stays a real value. */
static KValue k_bytes_owned(long long len, const unsigned char* data, long long cap) {
    KBytes* b = k_alloc(sizeof(KBytes));
    b->len = len;
    b->data = data;
    b->cap = cap;
    KValue v; v.tag = K_BYTES; v.payload = k_ptr(b); return v;
}

KValue k_b_append(KValue acc, KValue x) {
    if (!k_not_failure(acc)) return acc;
    if (!k_not_failure(x)) return x;
    if (acc.tag != K_BYTES) k_die("append takes bytes and a string, bytes, or byte");
    KBytes* a = k_as_bytes(acc);
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
    } else {
        k_die("append takes bytes and a string, bytes, or byte");
        return k_none();
    }
    if (a->cap) {
        KBuf* buf = ((KBuf*)a->data) - 1;
        if (buf->used == a->len && a->len + n <= a->cap) {
            memcpy((unsigned char*)a->data + a->len, src, (size_t)n);
            buf->used = a->len + n;
            return k_bytes_owned(a->len + n, a->data, a->cap);
        }
    }
    long long cap = 2 * (a->len + n);
    if (cap < 64) cap = 64;
    KBuf* buf = k_alloc(sizeof(KBuf) + (size_t)cap);
    buf->cap = cap;
    buf->used = a->len + n;
    unsigned char* data = (unsigned char*)(buf + 1);
    memcpy(data, a->data, (size_t)a->len);
    memcpy(data + a->len, src, (size_t)n);
    return k_bytes_owned(a->len + n, data, cap);
}

/* find2 with a floor: also stops at the first byte below `lim`. Escape
   scanning wants this shape — quote, backslash, and control bytes all
   end a clean run, and one pass finds whichever comes first. */
KValue k_b_find2_below(KValue cs, KValue from, KValue a, KValue b, KValue lim) {
    if (!k_not_failure(cs)) return cs;
    if (!k_not_failure(from)) return from;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (!k_not_failure(lim)) return lim;
    if (cs.tag != K_BYTES) k_die("find2_below takes bytes");
    KBytes* by = k_as_bytes(cs);
    long long p = from.payload < 1 ? 0 : from.payload - 1;
    unsigned char ca = (unsigned char)(a.payload & 0xff);
    unsigned char cb = (unsigned char)(b.payload & 0xff);
    long long floor_v = lim.payload;
    const unsigned char* d = by->data;
    long long i = p;
    if (floor_v > 0 && floor_v <= 256) {
        unsigned char cl = (unsigned char)(floor_v - 1);
#if defined(__aarch64__)
        uint8x16_t va = vdupq_n_u8(ca), vb = vdupq_n_u8(cb), vl = vdupq_n_u8(cl);
        for (; i + 16 <= by->len; i += 16) {
            uint8x16_t chunk = vld1q_u8(d + i);
            uint8x16_t m = vorrq_u8(vorrq_u8(vceqq_u8(chunk, va), vceqq_u8(chunk, vb)),
                                    vcleq_u8(chunk, vl));
            uint8x8_t narrowed = vshrn_n_u16(vreinterpretq_u16_u8(m), 4);
            uint64_t mask = vget_lane_u64(vreinterpret_u64_u8(narrowed), 0);
            if (mask) return k_int(i + (__builtin_ctzll(mask) >> 2) + 1);
        }
#elif defined(__x86_64__)
        __m128i va = _mm_set1_epi8((char)ca), vb = _mm_set1_epi8((char)cb);
        __m128i vl = _mm_set1_epi8((char)cl);
        for (; i + 16 <= by->len; i += 16) {
            __m128i chunk = _mm_loadu_si128((const __m128i*)(d + i));
            __m128i low = _mm_cmpeq_epi8(_mm_min_epu8(chunk, vl), chunk);
            __m128i m = _mm_or_si128(_mm_or_si128(_mm_cmpeq_epi8(chunk, va),
                                                  _mm_cmpeq_epi8(chunk, vb)), low);
            int mask = _mm_movemask_epi8(m);
            if (mask) return k_int(i + __builtin_ctz(mask) + 1);
        }
#endif
    }
    for (; i < by->len; i++) {
        if (d[i] == ca || d[i] == cb || (long long)d[i] < floor_v) return k_int(i + 1);
    }
    return k_int(by->len + 1);
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
    KStr* os = k_alloc(sizeof(KStr));
    os->len = total;
    os->data = data;
    data[total] = 0;
    KValue out; out.tag = K_STR; out.payload = k_ptr(os);
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
    /* Strict [-]?digits{1,18} parses in a bare loop (18 digits cannot
       overflow i64); every other shape — longer runs, leading space or '+',
       junk — falls through to strtoll so behavior stays exactly libc's. */
    long long start = (len > 0 && data[0] == '-') ? 1 : 0;
    if (start < len && len - start <= 18) {
        long long acc = 0, j = start;
        for (; j < len; j++) {
            unsigned char c = (unsigned char)data[j] - '0';
            if (c > 9) break;
            acc = acc * 10 + c;
        }
        if (j == len) return k_int(start ? -acc : acc);
    }
    char* end = NULL;
    errno = 0;
    long long n = strtoll(data, &end, 10);
    if (errno == ERANGE) {
        /* strtoll saturates while consuming every digit — without this check
           an overflowing literal decodes as a silently wrong value. Loud
           limit beats quiet lie until native bignum tiering ships. */
        KValue str = k_str_n(data, len);
        return k_err(k_concat(k_concat(k_str("\""), str),
            k_str("\" overflows this engine's integers")), origin);
    }
    if (len == 0 || end != data + len) {
        KValue str = k_str_n(data, len);
        return k_err(k_concat(k_concat(k_str("\""), str), k_str("\" is not an integer")), origin);
    }
    return k_int(n);
}

KValue k_b_sqrt(KValue v) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_INT) return k_float(sqrt((double)v.payload));
    if (v.tag == K_FLOAT) return k_float(sqrt(k_as_f(v)));
    k_die("sqrt takes a number");
    return k_none();
}

KValue k_b_round(KValue v) {
    if (!k_not_failure(v)) return v;
    if (v.tag == K_INT) return v;
    if (v.tag == K_FLOAT) return k_int((long long)llround(k_as_f(v)));
    k_die("round takes a number");
    return k_none();
}

/* Generated: 10^q = (hi.lo / 2^128) * 2^e2 for q in [-342, 308], the
   128-bit significand MSB-normalized; negative powers rounded up per
   the eisel-lemire convention. Regeneration: compiler log 2026-07-23. */
static const struct { unsigned long long hi, lo; int e2; } k_el_pow10[] = {
    {0xeef453d6923bd65aULL, 0x113faa2906a13b40ULL, -1136},
    {0x9558b4661b6565f8ULL, 0x4ac7ca59a424c508ULL, -1132},
    {0xbaaee17fa23ebf76ULL, 0x5d79bcf00d2df64aULL, -1129},
    {0xe95a99df8ace6f53ULL, 0xf4d82c2c107973ddULL, -1126},
    {0x91d8a02bb6c10594ULL, 0x79071b9b8a4be86aULL, -1122},
    {0xb64ec836a47146f9ULL, 0x9748e2826cdee285ULL, -1119},
    {0xe3e27a444d8d98b7ULL, 0xfd1b1b2308169b26ULL, -1116},
    {0x8e6d8c6ab0787f72ULL, 0xfe30f0f5e50e20f8ULL, -1112},
    {0xb208ef855c969f4fULL, 0xbdbd2d335e51a936ULL, -1109},
    {0xde8b2b66b3bc4723ULL, 0xad2c788035e61383ULL, -1106},
    {0x8b16fb203055ac76ULL, 0x4c3bcb5021afcc32ULL, -1102},
    {0xaddcb9e83c6b1793ULL, 0xdf4abe242a1bbf3eULL, -1099},
    {0xd953e8624b85dd78ULL, 0xd71d6dad34a2af0eULL, -1096},
    {0x87d4713d6f33aa6bULL, 0x8672648c40e5ad69ULL, -1092},
    {0xa9c98d8ccb009506ULL, 0x680efdaf511f18c3ULL, -1089},
    {0xd43bf0effdc0ba48ULL, 0x0212bd1b2566def3ULL, -1086},
    {0x84a57695fe98746dULL, 0x014bb630f7604b58ULL, -1082},
    {0xa5ced43b7e3e9188ULL, 0x419ea3bd35385e2eULL, -1079},
    {0xcf42894a5dce35eaULL, 0x52064cac828675baULL, -1076},
    {0x818995ce7aa0e1b2ULL, 0x7343efebd1940994ULL, -1072},
    {0xa1ebfb4219491a1fULL, 0x1014ebe6c5f90bf9ULL, -1069},
    {0xca66fa129f9b60a6ULL, 0xd41a26e077774ef7ULL, -1066},
    {0xfd00b897478238d0ULL, 0x8920b098955522b5ULL, -1063},
    {0x9e20735e8cb16382ULL, 0x55b46e5f5d5535b1ULL, -1059},
    {0xc5a890362fddbc62ULL, 0xeb2189f734aa831eULL, -1056},
    {0xf712b443bbd52b7bULL, 0xa5e9ec7501d523e5ULL, -1053},
    {0x9a6bb0aa55653b2dULL, 0x47b233c92125366fULL, -1049},
    {0xc1069cd4eabe89f8ULL, 0x999ec0bb696e840bULL, -1046},
    {0xf148440a256e2c76ULL, 0xc00670ea43ca250eULL, -1043},
    {0x96cd2a865764dbcaULL, 0x380406926a5e5729ULL, -1039},
    {0xbc807527ed3e12bcULL, 0xc605083704f5ecf3ULL, -1036},
    {0xeba09271e88d976bULL, 0xf7864a44c633682fULL, -1033},
    {0x93445b8731587ea3ULL, 0x7ab3ee6afbe0211eULL, -1029},
    {0xb8157268fdae9e4cULL, 0x5960ea05bad82965ULL, -1026},
    {0xe61acf033d1a45dfULL, 0x6fb92487298e33beULL, -1023},
    {0x8fd0c16206306babULL, 0xa5d3b6d479f8e057ULL, -1019},
    {0xb3c4f1ba87bc8696ULL, 0x8f48a4899877186dULL, -1016},
    {0xe0b62e2929aba83cULL, 0x331acdabfe94de88ULL, -1013},
    {0x8c71dcd9ba0b4925ULL, 0x9ff0c08b7f1d0b15ULL, -1009},
    {0xaf8e5410288e1b6fULL, 0x07ecf0ae5ee44ddaULL, -1006},
    {0xdb71e91432b1a24aULL, 0xc9e82cd9f69d6151ULL, -1003},
    {0x892731ac9faf056eULL, 0xbe311c083a225cd3ULL, -999},
    {0xab70fe17c79ac6caULL, 0x6dbd630a48aaf407ULL, -996},
    {0xd64d3d9db981787dULL, 0x092cbbccdad5b109ULL, -993},
    {0x85f0468293f0eb4eULL, 0x25bbf56008c58ea6ULL, -989},
    {0xa76c582338ed2621ULL, 0xaf2af2b80af6f24fULL, -986},
    {0xd1476e2c07286faaULL, 0x1af5af660db4aee2ULL, -983},
    {0x82cca4db847945caULL, 0x50d98d9fc890ed4eULL, -979},
    {0xa37fce126597973cULL, 0xe50ff107bab528a1ULL, -976},
    {0xcc5fc196fefd7d0cULL, 0x1e53ed49a96272c9ULL, -973},
    {0xff77b1fcbebcdc4fULL, 0x25e8e89c13bb0f7bULL, -970},
    {0x9faacf3df73609b1ULL, 0x77b191618c54e9adULL, -966},
    {0xc795830d75038c1dULL, 0xd59df5b9ef6a2418ULL, -963},
    {0xf97ae3d0d2446f25ULL, 0x4b0573286b44ad1eULL, -960},
    {0x9becce62836ac577ULL, 0x4ee367f9430aec33ULL, -956},
    {0xc2e801fb244576d5ULL, 0x229c41f793cda740ULL, -953},
    {0xf3a20279ed56d48aULL, 0x6b43527578c11110ULL, -950},
    {0x9845418c345644d6ULL, 0x830a13896b78aaaaULL, -946},
    {0xbe5691ef416bd60cULL, 0x23cc986bc656d554ULL, -943},
    {0xedec366b11c6cb8fULL, 0x2cbfbe86b7ec8aa9ULL, -940},
    {0x94b3a202eb1c3f39ULL, 0x7bf7d71432f3d6aaULL, -936},
    {0xb9e08a83a5e34f07ULL, 0xdaf5ccd93fb0cc54ULL, -933},
    {0xe858ad248f5c22c9ULL, 0xd1b3400f8f9cff69ULL, -930},
    {0x91376c36d99995beULL, 0x23100809b9c21fa2ULL, -926},
    {0xb58547448ffffb2dULL, 0xabd40a0c2832a78bULL, -923},
    {0xe2e69915b3fff9f9ULL, 0x16c90c8f323f516dULL, -920},
    {0x8dd01fad907ffc3bULL, 0xae3da7d97f6792e4ULL, -916},
    {0xb1442798f49ffb4aULL, 0x99cd11cfdf41779dULL, -913},
    {0xdd95317f31c7fa1dULL, 0x40405643d711d584ULL, -910},
    {0x8a7d3eef7f1cfc52ULL, 0x482835ea666b2573ULL, -906},
    {0xad1c8eab5ee43b66ULL, 0xda3243650005eed0ULL, -903},
    {0xd863b256369d4a40ULL, 0x90bed43e40076a83ULL, -900},
    {0x873e4f75e2224e68ULL, 0x5a7744a6e804a292ULL, -896},
    {0xa90de3535aaae202ULL, 0x711515d0a205cb37ULL, -893},
    {0xd3515c2831559a83ULL, 0x0d5a5b44ca873e04ULL, -890},
    {0x8412d9991ed58091ULL, 0xe858790afe9486c3ULL, -886},
    {0xa5178fff668ae0b6ULL, 0x626e974dbe39a873ULL, -883},
    {0xce5d73ff402d98e3ULL, 0xfb0a3d212dc81290ULL, -880},
    {0x80fa687f881c7f8eULL, 0x7ce66634bc9d0b9aULL, -876},
    {0xa139029f6a239f72ULL, 0x1c1fffc1ebc44e81ULL, -873},
    {0xc987434744ac874eULL, 0xa327ffb266b56221ULL, -870},
    {0xfbe9141915d7a922ULL, 0x4bf1ff9f0062baa9ULL, -867},
    {0x9d71ac8fada6c9b5ULL, 0x6f773fc3603db4aaULL, -863},
    {0xc4ce17b399107c22ULL, 0xcb550fb4384d21d4ULL, -860},
    {0xf6019da07f549b2bULL, 0x7e2a53a146606a49ULL, -857},
    {0x99c102844f94e0fbULL, 0x2eda7444cbfc426eULL, -853},
    {0xc0314325637a1939ULL, 0xfa911155fefb5309ULL, -850},
    {0xf03d93eebc589f88ULL, 0x793555ab7eba27cbULL, -847},
    {0x96267c7535b763b5ULL, 0x4bc1558b2f3458dfULL, -843},
    {0xbbb01b9283253ca2ULL, 0x9eb1aaedfb016f17ULL, -840},
    {0xea9c227723ee8bcbULL, 0x465e15a979c1caddULL, -837},
    {0x92a1958a7675175fULL, 0x0bfacd89ec191ecaULL, -833},
    {0xb749faed14125d36ULL, 0xcef980ec671f667cULL, -830},
    {0xe51c79a85916f484ULL, 0x82b7e12780e7401bULL, -827},
    {0x8f31cc0937ae58d2ULL, 0xd1b2ecb8b0908811ULL, -823},
    {0xb2fe3f0b8599ef07ULL, 0x861fa7e6dcb4aa16ULL, -820},
    {0xdfbdcece67006ac9ULL, 0x67a791e093e1d49bULL, -817},
    {0x8bd6a141006042bdULL, 0xe0c8bb2c5c6d24e1ULL, -813},
    {0xaecc49914078536dULL, 0x58fae9f773886e19ULL, -810},
    {0xda7f5bf590966848ULL, 0xaf39a475506a899fULL, -807},
    {0x888f99797a5e012dULL, 0x6d8406c952429604ULL, -803},
    {0xaab37fd7d8f58178ULL, 0xc8e5087ba6d33b84ULL, -800},
    {0xd5605fcdcf32e1d6ULL, 0xfb1e4a9a90880a65ULL, -797},
    {0x855c3be0a17fcd26ULL, 0x5cf2eea09a550680ULL, -793},
    {0xa6b34ad8c9dfc06fULL, 0xf42faa48c0ea481fULL, -790},
    {0xd0601d8efc57b08bULL, 0xf13b94daf124da27ULL, -787},
    {0x823c12795db6ce57ULL, 0x76c53d08d6b70859ULL, -783},
    {0xa2cb1717b52481edULL, 0x54768c4b0c64ca6fULL, -780},
    {0xcb7ddcdda26da268ULL, 0xa9942f5dcf7dfd0aULL, -777},
    {0xfe5d54150b090b02ULL, 0xd3f93b35435d7c4dULL, -774},
    {0x9efa548d26e5a6e1ULL, 0xc47bc5014a1a6db0ULL, -770},
    {0xc6b8e9b0709f109aULL, 0x359ab6419ca1091cULL, -767},
    {0xf867241c8cc6d4c0ULL, 0xc30163d203c94b63ULL, -764},
    {0x9b407691d7fc44f8ULL, 0x79e0de63425dcf1eULL, -760},
    {0xc21094364dfb5636ULL, 0x985915fc12f542e5ULL, -757},
    {0xf294b943e17a2bc4ULL, 0x3e6f5b7b17b2939eULL, -754},
    {0x979cf3ca6cec5b5aULL, 0xa705992ceecf9c43ULL, -750},
    {0xbd8430bd08277231ULL, 0x50c6ff782a838354ULL, -747},
    {0xece53cec4a314ebdULL, 0xa4f8bf5635246429ULL, -744},
    {0x940f4613ae5ed136ULL, 0x871b7795e136be9aULL, -740},
    {0xb913179899f68584ULL, 0x28e2557b59846e40ULL, -737},
    {0xe757dd7ec07426e5ULL, 0x331aeada2fe589d0ULL, -734},
    {0x9096ea6f3848984fULL, 0x3ff0d2c85def7622ULL, -730},
    {0xb4bca50b065abe63ULL, 0x0fed077a756b53aaULL, -727},
    {0xe1ebce4dc7f16dfbULL, 0xd3e8495912c62895ULL, -724},
    {0x8d3360f09cf6e4bdULL, 0x64712dd7abbbd95dULL, -720},
    {0xb080392cc4349decULL, 0xbd8d794d96aacfb4ULL, -717},
    {0xdca04777f541c567ULL, 0xecf0d7a0fc5583a1ULL, -714},
    {0x89e42caaf9491b60ULL, 0xf41686c49db57245ULL, -710},
    {0xac5d37d5b79b6239ULL, 0x311c2875c522ced6ULL, -707},
    {0xd77485cb25823ac7ULL, 0x7d633293366b828cULL, -704},
    {0x86a8d39ef77164bcULL, 0xae5dff9c02033198ULL, -700},
    {0xa8530886b54dbdebULL, 0xd9f57f830283fdfdULL, -697},
    {0xd267caa862a12d66ULL, 0xd072df63c324fd7cULL, -694},
    {0x8380dea93da4bc60ULL, 0x4247cb9e59f71e6eULL, -690},
    {0xa46116538d0deb78ULL, 0x52d9be85f074e609ULL, -687},
    {0xcd795be870516656ULL, 0x67902e276c921f8cULL, -684},
    {0x806bd9714632dff6ULL, 0x00ba1cd8a3db53b7ULL, -680},
    {0xa086cfcd97bf97f3ULL, 0x80e8a40eccd228a5ULL, -677},
    {0xc8a883c0fdaf7df0ULL, 0x6122cd128006b2ceULL, -674},
    {0xfad2a4b13d1b5d6cULL, 0x796b805720085f82ULL, -671},
    {0x9cc3a6eec6311a63ULL, 0xcbe3303674053bb1ULL, -667},
    {0xc3f490aa77bd60fcULL, 0xbedbfc4411068a9dULL, -664},
    {0xf4f1b4d515acb93bULL, 0xee92fb5515482d45ULL, -661},
    {0x991711052d8bf3c5ULL, 0x751bdd152d4d1c4bULL, -657},
    {0xbf5cd54678eef0b6ULL, 0xd262d45a78a0635eULL, -654},
    {0xef340a98172aace4ULL, 0x86fb897116c87c35ULL, -651},
    {0x9580869f0e7aac0eULL, 0xd45d35e6ae3d4da1ULL, -647},
    {0xbae0a846d2195712ULL, 0x8974836059cca10aULL, -644},
    {0xe998d258869facd7ULL, 0x2bd1a438703fc94cULL, -641},
    {0x91ff83775423cc06ULL, 0x7b6306a34627ddd0ULL, -637},
    {0xb67f6455292cbf08ULL, 0x1a3bc84c17b1d543ULL, -634},
    {0xe41f3d6a7377eecaULL, 0x20caba5f1d9e4a94ULL, -631},
    {0x8e938662882af53eULL, 0x547eb47b7282ee9dULL, -627},
    {0xb23867fb2a35b28dULL, 0xe99e619a4f23aa44ULL, -624},
    {0xdec681f9f4c31f31ULL, 0x6405fa00e2ec94d5ULL, -621},
    {0x8b3c113c38f9f37eULL, 0xde83bc408dd3dd05ULL, -617},
    {0xae0b158b4738705eULL, 0x9624ab50b148d446ULL, -614},
    {0xd98ddaee19068c76ULL, 0x3badd624dd9b0958ULL, -611},
    {0x87f8a8d4cfa417c9ULL, 0xe54ca5d70a80e5d7ULL, -607},
    {0xa9f6d30a038d1dbcULL, 0x5e9fcf4ccd211f4dULL, -604},
    {0xd47487cc8470652bULL, 0x7647c32000696720ULL, -601},
    {0x84c8d4dfd2c63f3bULL, 0x29ecd9f40041e074ULL, -597},
    {0xa5fb0a17c777cf09ULL, 0xf468107100525891ULL, -594},
    {0xcf79cc9db955c2ccULL, 0x7182148d4066eeb5ULL, -591},
    {0x81ac1fe293d599bfULL, 0xc6f14cd848405531ULL, -587},
    {0xa21727db38cb002fULL, 0xb8ada00e5a506a7dULL, -584},
    {0xca9cf1d206fdc03bULL, 0xa6d90811f0e4851dULL, -581},
    {0xfd442e4688bd304aULL, 0x908f4a166d1da664ULL, -578},
    {0x9e4a9cec15763e2eULL, 0x9a598e4e043287ffULL, -574},
    {0xc5dd44271ad3cdbaULL, 0x40eff1e1853f29feULL, -571},
    {0xf7549530e188c128ULL, 0xd12bee59e68ef47dULL, -568},
    {0x9a94dd3e8cf578b9ULL, 0x82bb74f8301958cfULL, -564},
    {0xc13a148e3032d6e7ULL, 0xe36a52363c1faf02ULL, -561},
    {0xf18899b1bc3f8ca1ULL, 0xdc44e6c3cb279ac2ULL, -558},
    {0x96f5600f15a7b7e5ULL, 0x29ab103a5ef8c0baULL, -554},
    {0xbcb2b812db11a5deULL, 0x7415d448f6b6f0e8ULL, -551},
    {0xebdf661791d60f56ULL, 0x111b495b3464ad22ULL, -548},
    {0x936b9fcebb25c995ULL, 0xcab10dd900beec35ULL, -544},
    {0xb84687c269ef3bfbULL, 0x3d5d514f40eea743ULL, -541},
    {0xe65829b3046b0afaULL, 0x0cb4a5a3112a5113ULL, -538},
    {0x8ff71a0fe2c2e6dcULL, 0x47f0e785eaba72acULL, -534},
    {0xb3f4e093db73a093ULL, 0x59ed216765690f57ULL, -531},
    {0xe0f218b8d25088b8ULL, 0x306869c13ec3532dULL, -528},
    {0x8c974f7383725573ULL, 0x1e414218c73a13fcULL, -524},
    {0xafbd2350644eeacfULL, 0xe5d1929ef90898fbULL, -521},
    {0xdbac6c247d62a583ULL, 0xdf45f746b74abf3aULL, -518},
    {0x894bc396ce5da772ULL, 0x6b8bba8c328eb784ULL, -514},
    {0xab9eb47c81f5114fULL, 0x066ea92f3f326565ULL, -511},
    {0xd686619ba27255a2ULL, 0xc80a537b0efefebeULL, -508},
    {0x8613fd0145877585ULL, 0xbd06742ce95f5f37ULL, -504},
    {0xa798fc4196e952e7ULL, 0x2c48113823b73705ULL, -501},
    {0xd17f3b51fca3a7a0ULL, 0xf75a15862ca504c6ULL, -498},
    {0x82ef85133de648c4ULL, 0x9a984d73dbe722fcULL, -494},
    {0xa3ab66580d5fdaf5ULL, 0xc13e60d0d2e0ebbbULL, -491},
    {0xcc963fee10b7d1b3ULL, 0x318df905079926a9ULL, -488},
    {0xffbbcfe994e5c61fULL, 0xfdf17746497f7053ULL, -485},
    {0x9fd561f1fd0f9bd3ULL, 0xfeb6ea8bedefa634ULL, -481},
    {0xc7caba6e7c5382c8ULL, 0xfe64a52ee96b8fc1ULL, -478},
    {0xf9bd690a1b68637bULL, 0x3dfdce7aa3c673b1ULL, -475},
    {0x9c1661a651213e2dULL, 0x06bea10ca65c084fULL, -471},
    {0xc31bfa0fe5698db8ULL, 0x486e494fcff30a63ULL, -468},
    {0xf3e2f893dec3f126ULL, 0x5a89dba3c3efccfbULL, -465},
    {0x986ddb5c6b3a76b7ULL, 0xf89629465a75e01dULL, -461},
    {0xbe89523386091465ULL, 0xf6bbb397f1135824ULL, -458},
    {0xee2ba6c0678b597fULL, 0x746aa07ded582e2dULL, -455},
    {0x94db483840b717efULL, 0xa8c2a44eb4571cddULL, -451},
    {0xba121a4650e4ddebULL, 0x92f34d62616ce414ULL, -448},
    {0xe896a0d7e51e1566ULL, 0x77b020baf9c81d18ULL, -445},
    {0x915e2486ef32cd60ULL, 0x0ace1474dc1d122fULL, -441},
    {0xb5b5ada8aaff80b8ULL, 0x0d819992132456bbULL, -438},
    {0xe3231912d5bf60e6ULL, 0x10e1fff697ed6c6aULL, -435},
    {0x8df5efabc5979c8fULL, 0xca8d3ffa1ef463c2ULL, -431},
    {0xb1736b96b6fd83b3ULL, 0xbd308ff8a6b17cb3ULL, -428},
    {0xddd0467c64bce4a0ULL, 0xac7cb3f6d05ddbdfULL, -425},
    {0x8aa22c0dbef60ee4ULL, 0x6bcdf07a423aa96cULL, -421},
    {0xad4ab7112eb3929dULL, 0x86c16c98d2c953c7ULL, -418},
    {0xd89d64d57a607744ULL, 0xe871c7bf077ba8b8ULL, -415},
    {0x87625f056c7c4a8bULL, 0x11471cd764ad4973ULL, -411},
    {0xa93af6c6c79b5d2dULL, 0xd598e40d3dd89bd0ULL, -408},
    {0xd389b47879823479ULL, 0x4aff1d108d4ec2c4ULL, -405},
    {0x843610cb4bf160cbULL, 0xcedf722a585139bbULL, -401},
    {0xa54394fe1eedb8feULL, 0xc2974eb4ee658829ULL, -398},
    {0xce947a3da6a9273eULL, 0x733d226229feea33ULL, -395},
    {0x811ccc668829b887ULL, 0x0806357d5a3f5260ULL, -391},
    {0xa163ff802a3426a8ULL, 0xca07c2dcb0cf26f8ULL, -388},
    {0xc9bcff6034c13052ULL, 0xfc89b393dd02f0b6ULL, -385},
    {0xfc2c3f3841f17c67ULL, 0xbbac2078d443ace3ULL, -382},
    {0x9d9ba7832936edc0ULL, 0xd54b944b84aa4c0eULL, -378},
    {0xc5029163f384a931ULL, 0x0a9e795e65d4df12ULL, -375},
    {0xf64335bcf065d37dULL, 0x4d4617b5ff4a16d6ULL, -372},
    {0x99ea0196163fa42eULL, 0x504bced1bf8e4e46ULL, -368},
    {0xc06481fb9bcf8d39ULL, 0xe45ec2862f71e1d7ULL, -365},
    {0xf07da27a82c37088ULL, 0x5d767327bb4e5a4dULL, -362},
    {0x964e858c91ba2655ULL, 0x3a6a07f8d510f870ULL, -358},
    {0xbbe226efb628afeaULL, 0x890489f70a55368cULL, -355},
    {0xeadab0aba3b2dbe5ULL, 0x2b45ac74ccea842fULL, -352},
    {0x92c8ae6b464fc96fULL, 0x3b0b8bc90012929eULL, -348},
    {0xb77ada0617e3bbcbULL, 0x09ce6ebb40173745ULL, -345},
    {0xe55990879ddcaabdULL, 0xcc420a6a101d0516ULL, -342},
    {0x8f57fa54c2a9eab6ULL, 0x9fa946824a12232eULL, -338},
    {0xb32df8e9f3546564ULL, 0x47939822dc96abfaULL, -335},
    {0xdff9772470297ebdULL, 0x59787e2b93bc56f8ULL, -332},
    {0x8bfbea76c619ef36ULL, 0x57eb4edb3c55b65bULL, -328},
    {0xaefae51477a06b03ULL, 0xede622920b6b23f2ULL, -325},
    {0xdab99e59958885c4ULL, 0xe95fab368e45eceeULL, -322},
    {0x88b402f7fd75539bULL, 0x11dbcb0218ebb415ULL, -318},
    {0xaae103b5fcd2a881ULL, 0xd652bdc29f26a11aULL, -315},
    {0xd59944a37c0752a2ULL, 0x4be76d3346f04960ULL, -312},
    {0x857fcae62d8493a5ULL, 0x6f70a4400c562ddcULL, -308},
    {0xa6dfbd9fb8e5b88eULL, 0xcb4ccd500f6bb953ULL, -305},
    {0xd097ad07a71f26b2ULL, 0x7e2000a41346a7a8ULL, -302},
    {0x825ecc24c873782fULL, 0x8ed400668c0c28c9ULL, -298},
    {0xa2f67f2dfa90563bULL, 0x728900802f0f32fbULL, -295},
    {0xcbb41ef979346bcaULL, 0x4f2b40a03ad2ffbaULL, -292},
    {0xfea126b7d78186bcULL, 0xe2f610c84987bfa9ULL, -289},
    {0x9f24b832e6b0f436ULL, 0x0dd9ca7d2df4d7caULL, -285},
    {0xc6ede63fa05d3143ULL, 0x91503d1c79720dbcULL, -282},
    {0xf8a95fcf88747d94ULL, 0x75a44c6397ce912bULL, -279},
    {0x9b69dbe1b548ce7cULL, 0xc986afbe3ee11abbULL, -275},
    {0xc24452da229b021bULL, 0xfbe85badce996169ULL, -272},
    {0xf2d56790ab41c2a2ULL, 0xfae27299423fb9c4ULL, -269},
    {0x97c560ba6b0919a5ULL, 0xdccd879fc967d41bULL, -265},
    {0xbdb6b8e905cb600fULL, 0x5400e987bbc1c921ULL, -262},
    {0xed246723473e3813ULL, 0x290123e9aab23b69ULL, -259},
    {0x9436c0760c86e30bULL, 0xf9a0b6720aaf6522ULL, -255},
    {0xb94470938fa89bceULL, 0xf808e40e8d5b3e6aULL, -252},
    {0xe7958cb87392c2c2ULL, 0xb60b1d1230b20e05ULL, -249},
    {0x90bd77f3483bb9b9ULL, 0xb1c6f22b5e6f48c3ULL, -245},
    {0xb4ecd5f01a4aa828ULL, 0x1e38aeb6360b1af4ULL, -242},
    {0xe2280b6c20dd5232ULL, 0x25c6da63c38de1b1ULL, -239},
    {0x8d590723948a535fULL, 0x579c487e5a38ad0fULL, -235},
    {0xb0af48ec79ace837ULL, 0x2d835a9df0c6d852ULL, -232},
    {0xdcdb1b2798182244ULL, 0xf8e431456cf88e66ULL, -229},
    {0x8a08f0f8bf0f156bULL, 0x1b8e9ecb641b5900ULL, -225},
    {0xac8b2d36eed2dac5ULL, 0xe272467e3d222f40ULL, -222},
    {0xd7adf884aa879177ULL, 0x5b0ed81dcc6abb10ULL, -219},
    {0x86ccbb52ea94baeaULL, 0x98e947129fc2b4eaULL, -215},
    {0xa87fea27a539e9a5ULL, 0x3f2398d747b36225ULL, -212},
    {0xd29fe4b18e88640eULL, 0x8eec7f0d19a03aaeULL, -209},
    {0x83a3eeeef9153e89ULL, 0x1953cf68300424adULL, -205},
    {0xa48ceaaab75a8e2bULL, 0x5fa8c3423c052dd8ULL, -202},
    {0xcdb02555653131b6ULL, 0x3792f412cb06794eULL, -199},
    {0x808e17555f3ebf11ULL, 0xe2bbd88bbee40bd1ULL, -195},
    {0xa0b19d2ab70e6ed6ULL, 0x5b6aceaeae9d0ec5ULL, -192},
    {0xc8de047564d20a8bULL, 0xf245825a5a445276ULL, -189},
    {0xfb158592be068d2eULL, 0xeed6e2f0f0d56713ULL, -186},
    {0x9ced737bb6c4183dULL, 0x55464dd69685606cULL, -182},
    {0xc428d05aa4751e4cULL, 0xaa97e14c3c26b887ULL, -179},
    {0xf53304714d9265dfULL, 0xd53dd99f4b3066a9ULL, -176},
    {0x993fe2c6d07b7fabULL, 0xe546a8038efe402aULL, -172},
    {0xbf8fdb78849a5f96ULL, 0xde98520472bdd034ULL, -169},
    {0xef73d256a5c0f77cULL, 0x963e66858f6d4441ULL, -166},
    {0x95a8637627989aadULL, 0xdde7001379a44aa9ULL, -162},
    {0xbb127c53b17ec159ULL, 0x5560c018580d5d53ULL, -159},
    {0xe9d71b689dde71afULL, 0xaab8f01e6e10b4a7ULL, -156},
    {0x9226712162ab070dULL, 0xcab3961304ca70e9ULL, -152},
    {0xb6b00d69bb55c8d1ULL, 0x3d607b97c5fd0d23ULL, -149},
    {0xe45c10c42a2b3b05ULL, 0x8cb89a7db77c506bULL, -146},
    {0x8eb98a7a9a5b04e3ULL, 0x77f3608e92adb243ULL, -142},
    {0xb267ed1940f1c61cULL, 0x55f038b237591ed4ULL, -139},
    {0xdf01e85f912e37a3ULL, 0x6b6c46dec52f6689ULL, -136},
    {0x8b61313bbabce2c6ULL, 0x2323ac4b3b3da016ULL, -132},
    {0xae397d8aa96c1b77ULL, 0xabec975e0a0d081bULL, -129},
    {0xd9c7dced53c72255ULL, 0x96e7bd358c904a22ULL, -126},
    {0x881cea14545c7575ULL, 0x7e50d64177da2e55ULL, -122},
    {0xaa242499697392d2ULL, 0xdde50bd1d5d0b9eaULL, -119},
    {0xd4ad2dbfc3d07787ULL, 0x955e4ec64b44e865ULL, -116},
    {0x84ec3c97da624ab4ULL, 0xbd5af13bef0b113fULL, -112},
    {0xa6274bbdd0fadd61ULL, 0xecb1ad8aeacdd58fULL, -109},
    {0xcfb11ead453994baULL, 0x67de18eda5814af3ULL, -106},
    {0x81ceb32c4b43fcf4ULL, 0x80eacf948770ced8ULL, -102},
    {0xa2425ff75e14fc31ULL, 0xa1258379a94d028eULL, -99},
    {0xcad2f7f5359a3b3eULL, 0x096ee45813a04331ULL, -96},
    {0xfd87b5f28300ca0dULL, 0x8bca9d6e188853fdULL, -93},
    {0x9e74d1b791e07e48ULL, 0x775ea264cf55347eULL, -89},
    {0xc612062576589ddaULL, 0x95364afe032a819eULL, -86},
    {0xf79687aed3eec551ULL, 0x3a83ddbd83f52205ULL, -83},
    {0x9abe14cd44753b52ULL, 0xc4926a9672793543ULL, -79},
    {0xc16d9a0095928a27ULL, 0x75b7053c0f178294ULL, -76},
    {0xf1c90080baf72cb1ULL, 0x5324c68b12dd6339ULL, -73},
    {0x971da05074da7beeULL, 0xd3f6fc16ebca5e04ULL, -69},
    {0xbce5086492111aeaULL, 0x88f4bb1ca6bcf585ULL, -66},
    {0xec1e4a7db69561a5ULL, 0x2b31e9e3d06c32e6ULL, -63},
    {0x9392ee8e921d5d07ULL, 0x3aff322e62439fd0ULL, -59},
    {0xb877aa3236a4b449ULL, 0x09befeb9fad487c3ULL, -56},
    {0xe69594bec44de15bULL, 0x4c2ebe687989a9b4ULL, -53},
    {0x901d7cf73ab0acd9ULL, 0x0f9d37014bf60a11ULL, -49},
    {0xb424dc35095cd80fULL, 0x538484c19ef38c95ULL, -46},
    {0xe12e13424bb40e13ULL, 0x2865a5f206b06fbaULL, -43},
    {0x8cbccc096f5088cbULL, 0xf93f87b7442e45d4ULL, -39},
    {0xafebff0bcb24aafeULL, 0xf78f69a51539d749ULL, -36},
    {0xdbe6fecebdedd5beULL, 0xb573440e5a884d1cULL, -33},
    {0x89705f4136b4a597ULL, 0x31680a88f8953031ULL, -29},
    {0xabcc77118461cefcULL, 0xfdc20d2b36ba7c3eULL, -26},
    {0xd6bf94d5e57a42bcULL, 0x3d32907604691b4dULL, -23},
    {0x8637bd05af6c69b5ULL, 0xa63f9a49c2c1b110ULL, -19},
    {0xa7c5ac471b478423ULL, 0x0fcf80dc33721d54ULL, -16},
    {0xd1b71758e219652bULL, 0xd3c36113404ea4a9ULL, -13},
    {0x83126e978d4fdf3bULL, 0x645a1cac083126eaULL, -9},
    {0xa3d70a3d70a3d70aULL, 0x3d70a3d70a3d70a4ULL, -6},
    {0xccccccccccccccccULL, 0xcccccccccccccccdULL, -3},
    {0x8000000000000000ULL, 0x0000000000000000ULL, 1},
    {0xa000000000000000ULL, 0x0000000000000000ULL, 4},
    {0xc800000000000000ULL, 0x0000000000000000ULL, 7},
    {0xfa00000000000000ULL, 0x0000000000000000ULL, 10},
    {0x9c40000000000000ULL, 0x0000000000000000ULL, 14},
    {0xc350000000000000ULL, 0x0000000000000000ULL, 17},
    {0xf424000000000000ULL, 0x0000000000000000ULL, 20},
    {0x9896800000000000ULL, 0x0000000000000000ULL, 24},
    {0xbebc200000000000ULL, 0x0000000000000000ULL, 27},
    {0xee6b280000000000ULL, 0x0000000000000000ULL, 30},
    {0x9502f90000000000ULL, 0x0000000000000000ULL, 34},
    {0xba43b74000000000ULL, 0x0000000000000000ULL, 37},
    {0xe8d4a51000000000ULL, 0x0000000000000000ULL, 40},
    {0x9184e72a00000000ULL, 0x0000000000000000ULL, 44},
    {0xb5e620f480000000ULL, 0x0000000000000000ULL, 47},
    {0xe35fa931a0000000ULL, 0x0000000000000000ULL, 50},
    {0x8e1bc9bf04000000ULL, 0x0000000000000000ULL, 54},
    {0xb1a2bc2ec5000000ULL, 0x0000000000000000ULL, 57},
    {0xde0b6b3a76400000ULL, 0x0000000000000000ULL, 60},
    {0x8ac7230489e80000ULL, 0x0000000000000000ULL, 64},
    {0xad78ebc5ac620000ULL, 0x0000000000000000ULL, 67},
    {0xd8d726b7177a8000ULL, 0x0000000000000000ULL, 70},
    {0x878678326eac9000ULL, 0x0000000000000000ULL, 74},
    {0xa968163f0a57b400ULL, 0x0000000000000000ULL, 77},
    {0xd3c21bcecceda100ULL, 0x0000000000000000ULL, 80},
    {0x84595161401484a0ULL, 0x0000000000000000ULL, 84},
    {0xa56fa5b99019a5c8ULL, 0x0000000000000000ULL, 87},
    {0xcecb8f27f4200f3aULL, 0x0000000000000000ULL, 90},
    {0x813f3978f8940984ULL, 0x4000000000000000ULL, 94},
    {0xa18f07d736b90be5ULL, 0x5000000000000000ULL, 97},
    {0xc9f2c9cd04674edeULL, 0xa400000000000000ULL, 100},
    {0xfc6f7c4045812296ULL, 0x4d00000000000000ULL, 103},
    {0x9dc5ada82b70b59dULL, 0xf020000000000000ULL, 107},
    {0xc5371912364ce305ULL, 0x6c28000000000000ULL, 110},
    {0xf684df56c3e01bc6ULL, 0xc732000000000000ULL, 113},
    {0x9a130b963a6c115cULL, 0x3c7f400000000000ULL, 117},
    {0xc097ce7bc90715b3ULL, 0x4b9f100000000000ULL, 120},
    {0xf0bdc21abb48db20ULL, 0x1e86d40000000000ULL, 123},
    {0x96769950b50d88f4ULL, 0x1314448000000000ULL, 127},
    {0xbc143fa4e250eb31ULL, 0x17d955a000000000ULL, 130},
    {0xeb194f8e1ae525fdULL, 0x5dcfab0800000000ULL, 133},
    {0x92efd1b8d0cf37beULL, 0x5aa1cae500000000ULL, 137},
    {0xb7abc627050305adULL, 0xf14a3d9e40000000ULL, 140},
    {0xe596b7b0c643c719ULL, 0x6d9ccd05d0000000ULL, 143},
    {0x8f7e32ce7bea5c6fULL, 0xe4820023a2000000ULL, 147},
    {0xb35dbf821ae4f38bULL, 0xdda2802c8a800000ULL, 150},
    {0xe0352f62a19e306eULL, 0xd50b2037ad200000ULL, 153},
    {0x8c213d9da502de45ULL, 0x4526f422cc340000ULL, 157},
    {0xaf298d050e4395d6ULL, 0x9670b12b7f410000ULL, 160},
    {0xdaf3f04651d47b4cULL, 0x3c0cdd765f114000ULL, 163},
    {0x88d8762bf324cd0fULL, 0xa5880a69fb6ac800ULL, 167},
    {0xab0e93b6efee0053ULL, 0x8eea0d047a457a00ULL, 170},
    {0xd5d238a4abe98068ULL, 0x72a4904598d6d880ULL, 173},
    {0x85a36366eb71f041ULL, 0x47a6da2b7f864750ULL, 177},
    {0xa70c3c40a64e6c51ULL, 0x999090b65f67d924ULL, 180},
    {0xd0cf4b50cfe20765ULL, 0xfff4b4e3f741cf6dULL, 183},
    {0x82818f1281ed449fULL, 0xbff8f10e7a8921a4ULL, 187},
    {0xa321f2d7226895c7ULL, 0xaff72d52192b6a0dULL, 190},
    {0xcbea6f8ceb02bb39ULL, 0x9bf4f8a69f764490ULL, 193},
    {0xfee50b7025c36a08ULL, 0x02f236d04753d5b4ULL, 196},
    {0x9f4f2726179a2245ULL, 0x01d762422c946590ULL, 200},
    {0xc722f0ef9d80aad6ULL, 0x424d3ad2b7b97ef5ULL, 203},
    {0xf8ebad2b84e0d58bULL, 0xd2e0898765a7deb2ULL, 206},
    {0x9b934c3b330c8577ULL, 0x63cc55f49f88eb2fULL, 210},
    {0xc2781f49ffcfa6d5ULL, 0x3cbf6b71c76b25fbULL, 213},
    {0xf316271c7fc3908aULL, 0x8bef464e3945ef7aULL, 216},
    {0x97edd871cfda3a56ULL, 0x97758bf0e3cbb5acULL, 220},
    {0xbde94e8e43d0c8ecULL, 0x3d52eeed1cbea317ULL, 223},
    {0xed63a231d4c4fb27ULL, 0x4ca7aaa863ee4bddULL, 226},
    {0x945e455f24fb1cf8ULL, 0x8fe8caa93e74ef6aULL, 230},
    {0xb975d6b6ee39e436ULL, 0xb3e2fd538e122b44ULL, 233},
    {0xe7d34c64a9c85d44ULL, 0x60dbbca87196b616ULL, 236},
    {0x90e40fbeea1d3a4aULL, 0xbc8955e946fe31cdULL, 240},
    {0xb51d13aea4a488ddULL, 0x6babab6398bdbe41ULL, 243},
    {0xe264589a4dcdab14ULL, 0xc696963c7eed2dd1ULL, 246},
    {0x8d7eb76070a08aecULL, 0xfc1e1de5cf543ca2ULL, 250},
    {0xb0de65388cc8ada8ULL, 0x3b25a55f43294bcbULL, 253},
    {0xdd15fe86affad912ULL, 0x49ef0eb713f39ebeULL, 256},
    {0x8a2dbf142dfcc7abULL, 0x6e3569326c784337ULL, 260},
    {0xacb92ed9397bf996ULL, 0x49c2c37f07965404ULL, 263},
    {0xd7e77a8f87daf7fbULL, 0xdc33745ec97be906ULL, 266},
    {0x86f0ac99b4e8dafdULL, 0x69a028bb3ded71a3ULL, 270},
    {0xa8acd7c0222311bcULL, 0xc40832ea0d68ce0cULL, 273},
    {0xd2d80db02aabd62bULL, 0xf50a3fa490c30190ULL, 276},
    {0x83c7088e1aab65dbULL, 0x792667c6da79e0faULL, 280},
    {0xa4b8cab1a1563f52ULL, 0x577001b891185938ULL, 283},
    {0xcde6fd5e09abcf26ULL, 0xed4c0226b55e6f86ULL, 286},
    {0x80b05e5ac60b6178ULL, 0x544f8158315b05b4ULL, 290},
    {0xa0dc75f1778e39d6ULL, 0x696361ae3db1c721ULL, 293},
    {0xc913936dd571c84cULL, 0x03bc3a19cd1e38e9ULL, 296},
    {0xfb5878494ace3a5fULL, 0x04ab48a04065c723ULL, 299},
    {0x9d174b2dcec0e47bULL, 0x62eb0d64283f9c76ULL, 303},
    {0xc45d1df942711d9aULL, 0x3ba5d0bd324f8394ULL, 306},
    {0xf5746577930d6500ULL, 0xca8f44ec7ee36479ULL, 309},
    {0x9968bf6abbe85f20ULL, 0x7e998b13cf4e1ecbULL, 313},
    {0xbfc2ef456ae276e8ULL, 0x9e3fedd8c321a67eULL, 316},
    {0xefb3ab16c59b14a2ULL, 0xc5cfe94ef3ea101eULL, 319},
    {0x95d04aee3b80ece5ULL, 0xbba1f1d158724a12ULL, 323},
    {0xbb445da9ca61281fULL, 0x2a8a6e45ae8edc97ULL, 326},
    {0xea1575143cf97226ULL, 0xf52d09d71a3293bdULL, 329},
    {0x924d692ca61be758ULL, 0x593c2626705f9c56ULL, 333},
    {0xb6e0c377cfa2e12eULL, 0x6f8b2fb00c77836cULL, 336},
    {0xe498f455c38b997aULL, 0x0b6dfb9c0f956447ULL, 339},
    {0x8edf98b59a373fecULL, 0x4724bd4189bd5eacULL, 343},
    {0xb2977ee300c50fe7ULL, 0x58edec91ec2cb657ULL, 346},
    {0xdf3d5e9bc0f653e1ULL, 0x2f2967b66737e3edULL, 349},
    {0x8b865b215899f46cULL, 0xbd79e0d20082ee74ULL, 353},
    {0xae67f1e9aec07187ULL, 0xecd8590680a3aa11ULL, 356},
    {0xda01ee641a708de9ULL, 0xe80e6f4820cc9495ULL, 359},
    {0x884134fe908658b2ULL, 0x3109058d147fdcddULL, 363},
    {0xaa51823e34a7eedeULL, 0xbd4b46f0599fd415ULL, 366},
    {0xd4e5e2cdc1d1ea96ULL, 0x6c9e18ac7007c91aULL, 369},
    {0x850fadc09923329eULL, 0x03e2cf6bc604ddb0ULL, 373},
    {0xa6539930bf6bff45ULL, 0x84db8346b786151cULL, 376},
    {0xcfe87f7cef46ff16ULL, 0xe612641865679a63ULL, 379},
    {0x81f14fae158c5f6eULL, 0x4fcb7e8f3f60c07eULL, 383},
    {0xa26da3999aef7749ULL, 0xe3be5e330f38f09dULL, 386},
    {0xcb090c8001ab551cULL, 0x5cadf5bfd3072cc5ULL, 389},
    {0xfdcb4fa002162a63ULL, 0x73d9732fc7c8f7f6ULL, 392},
    {0x9e9f11c4014dda7eULL, 0x2867e7fddcdd9afaULL, 396},
    {0xc646d63501a1511dULL, 0xb281e1fd541501b8ULL, 399},
    {0xf7d88bc24209a565ULL, 0x1f225a7ca91a4226ULL, 402},
    {0x9ae757596946075fULL, 0x3375788de9b06958ULL, 406},
    {0xc1a12d2fc3978937ULL, 0x0052d6b1641c83aeULL, 409},
    {0xf209787bb47d6b84ULL, 0xc0678c5dbd23a49aULL, 412},
    {0x9745eb4d50ce6332ULL, 0xf840b7ba963646e0ULL, 416},
    {0xbd176620a501fbffULL, 0xb650e5a93bc3d898ULL, 419},
    {0xec5d3fa8ce427affULL, 0xa3e51f138ab4cebeULL, 422},
    {0x93ba47c980e98cdfULL, 0xc66f336c36b10137ULL, 426},
    {0xb8a8d9bbe123f017ULL, 0xb80b0047445d4184ULL, 429},
    {0xe6d3102ad96cec1dULL, 0xa60dc059157491e5ULL, 432},
    {0x9043ea1ac7e41392ULL, 0x87c89837ad68db2fULL, 436},
    {0xb454e4a179dd1877ULL, 0x29babe4598c311fbULL, 439},
    {0xe16a1dc9d8545e94ULL, 0xf4296dd6fef3d67aULL, 442},
    {0x8ce2529e2734bb1dULL, 0x1899e4a65f58660cULL, 446},
    {0xb01ae745b101e9e4ULL, 0x5ec05dcff72e7f8fULL, 449},
    {0xdc21a1171d42645dULL, 0x76707543f4fa1f73ULL, 452},
    {0x899504ae72497ebaULL, 0x6a06494a791c53a8ULL, 456},
    {0xabfa45da0edbde69ULL, 0x0487db9d17636892ULL, 459},
    {0xd6f8d7509292d603ULL, 0x45a9d2845d3c42b6ULL, 462},
    {0x865b86925b9bc5c2ULL, 0x0b8a2392ba45a9b2ULL, 466},
    {0xa7f26836f282b732ULL, 0x8e6cac7768d7141eULL, 469},
    {0xd1ef0244af2364ffULL, 0x3207d795430cd926ULL, 472},
    {0x8335616aed761f1fULL, 0x7f44e6bd49e807b8ULL, 476},
    {0xa402b9c5a8d3a6e7ULL, 0x5f16206c9c6209a6ULL, 479},
    {0xcd036837130890a1ULL, 0x36dba887c37a8c0fULL, 482},
    {0x802221226be55a64ULL, 0xc2494954da2c9789ULL, 486},
    {0xa02aa96b06deb0fdULL, 0xf2db9baa10b7bd6cULL, 489},
    {0xc83553c5c8965d3dULL, 0x6f92829494e5acc7ULL, 492},
    {0xfa42a8b73abbf48cULL, 0xcb772339ba1f17f9ULL, 495},
    {0x9c69a97284b578d7ULL, 0xff2a760414536efbULL, 499},
    {0xc38413cf25e2d70dULL, 0xfef5138519684abaULL, 502},
    {0xf46518c2ef5b8cd1ULL, 0x7eb258665fc25d69ULL, 505},
    {0x98bf2f79d5993802ULL, 0xef2f773ffbd97a61ULL, 509},
    {0xbeeefb584aff8603ULL, 0xaafb550ffacfd8faULL, 512},
    {0xeeaaba2e5dbf6784ULL, 0x95ba2a53f983cf38ULL, 515},
    {0x952ab45cfa97a0b2ULL, 0xdd945a747bf26183ULL, 519},
    {0xba756174393d88dfULL, 0x94f971119aeef9e4ULL, 522},
    {0xe912b9d1478ceb17ULL, 0x7a37cd5601aab85dULL, 525},
    {0x91abb422ccb812eeULL, 0xac62e055c10ab33aULL, 529},
    {0xb616a12b7fe617aaULL, 0x577b986b314d6009ULL, 532},
    {0xe39c49765fdf9d94ULL, 0xed5a7e85fda0b80bULL, 535},
    {0x8e41ade9fbebc27dULL, 0x14588f13be847307ULL, 539},
    {0xb1d219647ae6b31cULL, 0x596eb2d8ae258fc8ULL, 542},
    {0xde469fbd99a05fe3ULL, 0x6fca5f8ed9aef3bbULL, 545},
    {0x8aec23d680043beeULL, 0x25de7bb9480d5854ULL, 549},
    {0xada72ccc20054ae9ULL, 0xaf561aa79a10ae6aULL, 552},
    {0xd910f7ff28069da4ULL, 0x1b2ba1518094da04ULL, 555},
    {0x87aa9aff79042286ULL, 0x90fb44d2f05d0842ULL, 559},
    {0xa99541bf57452b28ULL, 0x353a1607ac744a53ULL, 562},
    {0xd3fa922f2d1675f2ULL, 0x42889b8997915ce8ULL, 565},
    {0x847c9b5d7c2e09b7ULL, 0x69956135febada11ULL, 569},
    {0xa59bc234db398c25ULL, 0x43fab9837e699095ULL, 572},
    {0xcf02b2c21207ef2eULL, 0x94f967e45e03f4bbULL, 575},
    {0x8161afb94b44f57dULL, 0x1d1be0eebac278f5ULL, 579},
    {0xa1ba1ba79e1632dcULL, 0x6462d92a69731732ULL, 582},
    {0xca28a291859bbf93ULL, 0x7d7b8f7503cfdcfeULL, 585},
    {0xfcb2cb35e702af78ULL, 0x5cda735244c3d43eULL, 588},
    {0x9defbf01b061adabULL, 0x3a0888136afa64a7ULL, 592},
    {0xc56baec21c7a1916ULL, 0x088aaa1845b8fdd0ULL, 595},
    {0xf6c69a72a3989f5bULL, 0x8aad549e57273d45ULL, 598},
    {0x9a3c2087a63f6399ULL, 0x36ac54e2f678864bULL, 602},
    {0xc0cb28a98fcf3c7fULL, 0x84576a1bb416a7ddULL, 605},
    {0xf0fdf2d3f3c30b9fULL, 0x656d44a2a11c51d5ULL, 608},
    {0x969eb7c47859e743ULL, 0x9f644ae5a4b1b325ULL, 612},
    {0xbc4665b596706114ULL, 0x873d5d9f0dde1feeULL, 615},
    {0xeb57ff22fc0c7959ULL, 0xa90cb506d155a7eaULL, 618},
    {0x9316ff75dd87cbd8ULL, 0x09a7f12442d588f2ULL, 622},
    {0xb7dcbf5354e9beceULL, 0x0c11ed6d538aeb2fULL, 625},
    {0xe5d3ef282a242e81ULL, 0x8f1668c8a86da5faULL, 628},
    {0x8fa475791a569d10ULL, 0xf96e017d694487bcULL, 632},
    {0xb38d92d760ec4455ULL, 0x37c981dcc395a9acULL, 635},
    {0xe070f78d3927556aULL, 0x85bbe253f47b1417ULL, 638},
    {0x8c469ab843b89562ULL, 0x93956d7478ccec8eULL, 642},
    {0xaf58416654a6babbULL, 0x387ac8d1970027b2ULL, 645},
    {0xdb2e51bfe9d0696aULL, 0x06997b05fcc0319eULL, 648},
    {0x88fcf317f22241e2ULL, 0x441fece3bdf81f03ULL, 652},
    {0xab3c2fddeeaad25aULL, 0xd527e81cad7626c3ULL, 655},
    {0xd60b3bd56a5586f1ULL, 0x8a71e223d8d3b074ULL, 658},
    {0x85c7056562757456ULL, 0xf6872d5667844e49ULL, 662},
    {0xa738c6bebb12d16cULL, 0xb428f8ac016561dbULL, 665},
    {0xd106f86e69d785c7ULL, 0xe13336d701beba52ULL, 668},
    {0x82a45b450226b39cULL, 0xecc0024661173473ULL, 672},
    {0xa34d721642b06084ULL, 0x27f002d7f95d0190ULL, 675},
    {0xcc20ce9bd35c78a5ULL, 0x31ec038df7b441f4ULL, 678},
    {0xff290242c83396ceULL, 0x7e67047175a15271ULL, 681},
    {0x9f79a169bd203e41ULL, 0x0f0062c6e984d386ULL, 685},
    {0xc75809c42c684dd1ULL, 0x52c07b78a3e60868ULL, 688},
    {0xf92e0c3537826145ULL, 0xa7709a56ccdf8a82ULL, 691},
    {0x9bbcc7a142b17ccbULL, 0x88a66076400bb691ULL, 695},
    {0xc2abf989935ddbfeULL, 0x6acff893d00ea435ULL, 698},
    {0xf356f7ebf83552feULL, 0x0583f6b8c4124d43ULL, 701},
    {0x98165af37b2153deULL, 0xc3727a337a8b704aULL, 705},
    {0xbe1bf1b059e9a8d6ULL, 0x744f18c0592e4c5cULL, 708},
    {0xeda2ee1c7064130cULL, 0x1162def06f79df73ULL, 711},
    {0x9485d4d1c63e8be7ULL, 0x8addcb5645ac2ba8ULL, 715},
    {0xb9a74a0637ce2ee1ULL, 0x6d953e2bd7173692ULL, 718},
    {0xe8111c87c5c1ba99ULL, 0xc8fa8db6ccdd0437ULL, 721},
    {0x910ab1d4db9914a0ULL, 0x1d9c9892400a22a2ULL, 725},
    {0xb54d5e4a127f59c8ULL, 0x2503beb6d00cab4bULL, 728},
    {0xe2a0b5dc971f303aULL, 0x2e44ae64840fd61dULL, 731},
    {0x8da471a9de737e24ULL, 0x5ceaecfed289e5d2ULL, 735},
    {0xb10d8e1456105dadULL, 0x7425a83e872c5f47ULL, 738},
    {0xdd50f1996b947518ULL, 0xd12f124e28f77719ULL, 741},
    {0x8a5296ffe33cc92fULL, 0x82bd6b70d99aaa6fULL, 745},
    {0xace73cbfdc0bfb7bULL, 0x636cc64d1001550bULL, 748},
    {0xd8210befd30efa5aULL, 0x3c47f7e05401aa4eULL, 751},
    {0x8714a775e3e95c78ULL, 0x65acfaec34810a71ULL, 755},
    {0xa8d9d1535ce3b396ULL, 0x7f1839a741a14d0dULL, 758},
    {0xd31045a8341ca07cULL, 0x1ede48111209a050ULL, 761},
    {0x83ea2b892091e44dULL, 0x934aed0aab460432ULL, 765},
    {0xa4e4b66b68b65d60ULL, 0xf81da84d5617853fULL, 768},
    {0xce1de40642e3f4b9ULL, 0x36251260ab9d668eULL, 771},
    {0x80d2ae83e9ce78f3ULL, 0xc1d72b7c6b426019ULL, 775},
    {0xa1075a24e4421730ULL, 0xb24cf65b8612f81fULL, 778},
    {0xc94930ae1d529cfcULL, 0xdee033f26797b627ULL, 781},
    {0xfb9b7cd9a4a7443cULL, 0x169840ef017da3b1ULL, 784},
    {0x9d412e0806e88aa5ULL, 0x8e1f289560ee864eULL, 788},
    {0xc491798a08a2ad4eULL, 0xf1a6f2bab92a27e2ULL, 791},
    {0xf5b5d7ec8acb58a2ULL, 0xae10af696774b1dbULL, 794},
    {0x9991a6f3d6bf1765ULL, 0xacca6da1e0a8ef29ULL, 798},
    {0xbff610b0cc6edd3fULL, 0x17fd090a58d32af3ULL, 801},
    {0xeff394dcff8a948eULL, 0xddfc4b4cef07f5b0ULL, 804},
    {0x95f83d0a1fb69cd9ULL, 0x4abdaf101564f98eULL, 808},
    {0xbb764c4ca7a4440fULL, 0x9d6d1ad41abe37f1ULL, 811},
    {0xea53df5fd18d5513ULL, 0x84c86189216dc5edULL, 814},
    {0x92746b9be2f8552cULL, 0x32fd3cf5b4e49bb4ULL, 818},
    {0xb7118682dbb66a77ULL, 0x3fbc8c33221dc2a1ULL, 821},
    {0xe4d5e82392a40515ULL, 0x0fabaf3feaa5334aULL, 824},
    {0x8f05b1163ba6832dULL, 0x29cb4d87f2a7400eULL, 828},
    {0xb2c71d5bca9023f8ULL, 0x743e20e9ef511012ULL, 831},
    {0xdf78e4b2bd342cf6ULL, 0x914da9246b255416ULL, 834},
    {0x8bab8eefb6409c1aULL, 0x1ad089b6c2f7548eULL, 838},
    {0xae9672aba3d0c320ULL, 0xa184ac2473b529b1ULL, 841},
    {0xda3c0f568cc4f3e8ULL, 0xc9e5d72d90a2741eULL, 844},
    {0x8865899617fb1871ULL, 0x7e2fa67c7a658892ULL, 848},
    {0xaa7eebfb9df9de8dULL, 0xddbb901b98feeab7ULL, 851},
    {0xd51ea6fa85785631ULL, 0x552a74227f3ea565ULL, 854},
    {0x8533285c936b35deULL, 0xd53a88958f87275fULL, 858},
    {0xa67ff273b8460356ULL, 0x8a892abaf368f137ULL, 861},
    {0xd01fef10a657842cULL, 0x2d2b7569b0432d85ULL, 864},
    {0x8213f56a67f6b29bULL, 0x9c3b29620e29fc73ULL, 868},
    {0xa298f2c501f45f42ULL, 0x8349f3ba91b47b8fULL, 871},
    {0xcb3f2f7642717713ULL, 0x241c70a936219a73ULL, 874},
    {0xfe0efb53d30dd4d7ULL, 0xed238cd383aa0110ULL, 877},
    {0x9ec95d1463e8a506ULL, 0xf4363804324a40aaULL, 881},
    {0xc67bb4597ce2ce48ULL, 0xb143c6053edcd0d5ULL, 884},
    {0xf81aa16fdc1b81daULL, 0xdd94b7868e94050aULL, 887},
    {0x9b10a4e5e9913128ULL, 0xca7cf2b4191c8326ULL, 891},
    {0xc1d4ce1f63f57d72ULL, 0xfd1c2f611f63a3f0ULL, 894},
    {0xf24a01a73cf2dccfULL, 0xbc633b39673c8cecULL, 897},
    {0x976e41088617ca01ULL, 0xd5be0503e085d813ULL, 901},
    {0xbd49d14aa79dbc82ULL, 0x4b2d8644d8a74e18ULL, 904},
    {0xec9c459d51852ba2ULL, 0xddf8e7d60ed1219eULL, 907},
    {0x93e1ab8252f33b45ULL, 0xcabb90e5c942b503ULL, 911},
    {0xb8da1662e7b00a17ULL, 0x3d6a751f3b936243ULL, 914},
    {0xe7109bfba19c0c9dULL, 0x0cc512670a783ad4ULL, 917},
    {0x906a617d450187e2ULL, 0x27fb2b80668b24c5ULL, 921},
    {0xb484f9dc9641e9daULL, 0xb1f9f660802dedf6ULL, 924},
    {0xe1a63853bbd26451ULL, 0x5e7873f8a0396973ULL, 927},
    {0x8d07e33455637eb2ULL, 0xdb0b487b6423e1e8ULL, 931},
    {0xb049dc016abc5e5fULL, 0x91ce1a9a3d2cda62ULL, 934},
    {0xdc5c5301c56b75f7ULL, 0x7641a140cc7810fbULL, 937},
    {0x89b9b3e11b6329baULL, 0xa9e904c87fcb0a9dULL, 941},
    {0xac2820d9623bf429ULL, 0x546345fa9fbdcd44ULL, 944},
    {0xd732290fbacaf133ULL, 0xa97c177947ad4095ULL, 947},
    {0x867f59a9d4bed6c0ULL, 0x49ed8eabcccc485dULL, 951},
    {0xa81f301449ee8c70ULL, 0x5c68f256bfff5a74ULL, 954},
    {0xd226fc195c6a2f8cULL, 0x73832eec6fff3111ULL, 957},
    {0x83585d8fd9c25db7ULL, 0xc831fd53c5ff7eabULL, 961},
    {0xa42e74f3d032f525ULL, 0xba3e7ca8b77f5e55ULL, 964},
    {0xcd3a1230c43fb26fULL, 0x28ce1bd2e55f35ebULL, 967},
    {0x80444b5e7aa7cf85ULL, 0x7980d163cf5b81b3ULL, 971},
    {0xa0555e361951c366ULL, 0xd7e105bcc332621fULL, 974},
    {0xc86ab5c39fa63440ULL, 0x8dd9472bf3fefaa7ULL, 977},
    {0xfa856334878fc150ULL, 0xb14f98f6f0feb951ULL, 980},
    {0x9c935e00d4b9d8d2ULL, 0x6ed1bf9a569f33d3ULL, 984},
    {0xc3b8358109e84f07ULL, 0x0a862f80ec4700c8ULL, 987},
    {0xf4a642e14c6262c8ULL, 0xcd27bb612758c0faULL, 990},
    {0x98e7e9cccfbd7dbdULL, 0x8038d51cb897789cULL, 994},
    {0xbf21e44003acdd2cULL, 0xe0470a63e6bd56c3ULL, 997},
    {0xeeea5d5004981478ULL, 0x1858ccfce06cac74ULL, 1000},
    {0x95527a5202df0ccbULL, 0x0f37801e0c43ebc8ULL, 1004},
    {0xbaa718e68396cffdULL, 0xd30560258f54e6baULL, 1007},
    {0xe950df20247c83fdULL, 0x47c6b82ef32a2069ULL, 1010},
    {0x91d28b7416cdd27eULL, 0x4cdc331d57fa5441ULL, 1014},
    {0xb6472e511c81471dULL, 0xe0133fe4adf8e952ULL, 1017},
    {0xe3d8f9e563a198e5ULL, 0x58180fddd97723a6ULL, 1020},
    {0x8e679c2f5e44ff8fULL, 0x570f09eaa7ea7648ULL, 1024},
};
#define K_EL_POW10_MIN (-342)
#define K_EL_POW10_MAX (308)


/* Correctly-rounded decimal-to-double on the fast path (eisel-lemire,
   "number parsing at a gigabyte per second", 2021), with the exact
   small-case (clinger, PLDI 1990) in front and strtod behind: returns 1
   and writes *out when certain, 0 to defer. w is the digit significand,
   q the decimal exponent — value = w * 10^q. */
static int k_el_parse(unsigned long long w, long long q, double* out) {
    k_stat_el_parses++;
    if (w == 0) { *out = 0.0; return 1; }
    static const double exact10[23] = {
        1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11,
        1e12, 1e13, 1e14, 1e15, 1e16, 1e17, 1e18, 1e19, 1e20, 1e21, 1e22
    };
    if (w < (1ULL << 53) && q >= -22 && q <= 22) {
        double d = (double)w;
        *out = q >= 0 ? d * exact10[q] : d / exact10[-q];
        return 1;
    }
    if (q < K_EL_POW10_MIN || q > K_EL_POW10_MAX) {
        /* certain under/overflow */
        if (q < K_EL_POW10_MIN) { *out = 0.0; return 1; }
        *out = 1.0 / 0.0;
        return 1;
    }
    int lz = __builtin_clzll(w);
    unsigned long long wn = w << lz;
    const unsigned long long mhi = k_el_pow10[q - K_EL_POW10_MIN].hi;
    const unsigned long long mlo = k_el_pow10[q - K_EL_POW10_MIN].lo;
    __uint128_t p = (__uint128_t)wn * mhi;
    unsigned long long phi = (unsigned long long)(p >> 64);
    unsigned long long plo = (unsigned long long)p;
    /* refine with the low table word when the truncated product is too
       close to a rounding boundary to be trusted */
    if ((phi & 0x1FF) == 0x1FF) {
        __uint128_t p2 = (__uint128_t)wn * mlo;
        unsigned long long carry = plo + (unsigned long long)(p2 >> 64) < plo;
        plo += (unsigned long long)(p2 >> 64);
        phi += carry;
        if ((phi & 0x1FF) == 0x1FF && plo + 1 == 0) return 0;
    }
    /* value = (phi + plo/2^64) * 2^(e2t - lz), phi's leading bit at
       62+msb: binary exponent E = 62 + msb + e2t - lz. */
    int msb = (int)(phi >> 63);
    int e_val = 62 + msb + k_el_pow10[q - K_EL_POW10_MIN].e2 - lz;
    int biased = e_val + 1023;
    if (biased <= 0 || biased >= 2047) return 0; /* subnormal/inf: defer */
    int shift = 10 + msb;
    unsigned long long m = phi >> shift;
    unsigned long long round_bit = (phi >> (shift - 1)) & 1;
    unsigned long long sticky = (phi & ((1ULL << (shift - 1)) - 1)) | plo;
    if (round_bit && sticky == 0) return 0; /* halfway under truncation: defer */
    m += round_bit & (sticky != 0 || (m & 1));
    if (m >= (1ULL << 53)) {
        m >>= 1;
        biased += 1;
        if (biased >= 2047) return 0;
    }
    unsigned long long bits =
        ((unsigned long long)biased << 52) | (m & ((1ULL << 52) - 1));
    double d;
    __builtin_memcpy(&d, &bits, 8);
    *out = d;
    return 1;
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
    /* the fast path: a plain decimal scanned into (w, q) and parsed by
       eisel-lemire; anything it can't be certain about — overlong digits,
       exotic forms, halfway cases — falls through to strtod, which stays
       the semantic authority */
    if (len > 0) {
        const char* p = data;
        const char* stop = data + len;
        int neg = 0;
        if (*p == '-' || *p == '+') { neg = *p == '-'; p++; }
        unsigned long long w = 0;
        long long q = 0;
        int digits = 0, any = 0, ok = 1;
        while (p < stop && *p >= '0' && *p <= '9') {
            any = 1;
            if (digits < 19) { w = w * 10 + (unsigned long long)(*p - '0'); if (w) digits++; }
            else { q++; }
            p++;
        }
        if (p < stop && *p == '.') {
            p++;
            while (p < stop && *p >= '0' && *p <= '9') {
                any = 1;
                if (digits < 19) {
                    w = w * 10 + (unsigned long long)(*p - '0');
                    if (w) digits++;
                    q--;
                }
                p++;
            }
        }
        if (p < stop && (*p == 'e' || *p == 'E')) {
            p++;
            int esign = 1;
            if (p < stop && (*p == '-' || *p == '+')) { esign = *p == '-' ? -1 : 1; p++; }
            long long e = 0;
            int edigits = 0;
            while (p < stop && *p >= '0' && *p <= '9') {
                if (e < 100000) e = e * 10 + (*p - '0');
                edigits++;
                p++;
            }
            if (!edigits) ok = 0;
            q += esign * e;
        }
        if (ok && any && p == stop) {
            double out;
            if (k_el_parse(w, q, &out)) {
                return k_float(neg ? -out : out);
            }
        }
    }
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
    if (getenv("KANSO_COUNTERS")) atexit(k_stats_dump);
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
