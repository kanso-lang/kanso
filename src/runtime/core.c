#include <stdio.h>
#include <stdarg.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <stdint.h>
#include <errno.h>
#include <time.h>
#include <dirent.h>
#include <sys/stat.h>
#include <unistd.h>
#include <sys/wait.h>

/* Whether this runtime object carries its twenty-seven counter sites. The
   `-DKANSO_COUNTERS_BUILD` comes from the caller, so `cached_runtime_object`
   keys on it; an object built one way handed to a build that wanted the other
   is a binary whose counters are half there.

   Each of the twenty-seven asks this before it asks `k_stats_on`. In a
   counting build that is a constant 1 and the test reads exactly as it did;
   in a shipped build it is a constant 0 and the compiler folds the whole
   condition away, which is worth 13,187,834 instructions on the run program
   (0.6159%) and 2,944 bytes of .text, measured by compiling this file both
   ways and linking the same IR against each.

   It sits this high up because `k_stats_dump` reads it too, and a macro used
   before it is defined is a build error rather than a wrong answer -- which is
   how the first cut of the refusal below was caught.

   `k_stats_on` itself stays defined either way. The emitted IR declares it
   `external global` in both builds -- the shipped one just has no reader
   left -- and removing the definition would be a link error rather than a
   saving. */
#ifdef KANSO_COUNTERS_BUILD
#define K_COUNTING 1
#else
#define K_COUNTING 0
#endif

#include <sys/socket.h>
#include <netinet/in.h>
#include <fcntl.h>
#include <signal.h>

#ifndef __wasm__
#include <setjmp.h>
/* Work-ahead's catch (#1856, ruled 2026-10-11). Pooled work runs while a
   fiber is parked, ahead of output the program as written prints first, so a
   death inside it cannot be allowed to end the process there. While the pool
   runs, every exit in this file comes back to `k_ahead_try` instead; the try
   empties the cells the work left half-built, and the program as written
   computes them again where it reads them and fails at that point, as it
   would have without the pool. */
static volatile int k_ahead_pooling = 0;
static sigjmp_buf k_ahead_out;
__attribute__((noreturn)) static void k_ahead_exit(int code) {
    if (k_ahead_pooling) siglongjmp(k_ahead_out, 1);
    exit(code);
}
#define exit(c) k_ahead_exit(c)
#endif

#if defined(__aarch64__)
#include <arm_neon.h>
#elif defined(__x86_64__)
#include <tmmintrin.h>
#endif

/* ABI shared with emitted LLVM IR: %KValue = type { i64, i64 } */
typedef struct { long long tag; long long payload; } KValue;

enum { K_INT, K_FLOAT, K_TRUE, K_FALSE, K_NONE, K_ERR, K_STR, K_REC, K_DESC, K_LIST, K_MAP, K_CLOSURE, K_FNREF, K_BYTES, K_THUNK, K_SUB, K_DONE, K_BIG };
/* An int of either kind: a word, or a bignum past one. */
#define K_IS_INT(v) ((v).tag == K_INT || (v).tag == K_BIG)

/* `cap` is spare room the string may grow into. Zero for every string built
   the ordinary way; positive only for a builder, whose storage is malloc'd
   rather than taken from the arena so a rewind cannot reach it — the byte
   builder's arrangement. Fields are ordered and narrowed to hold the header at
   sixteen bytes: a string is the most numerous heap object a decode makes, and
   widening it moves the survivor ratio the cohort guard reads. */
typedef struct { char* data; int len; int cap; } KStr;

/* A builder's storage carries its character count in the eight bytes before
   the data, where `k_buf_of` keeps a list's header for the same reason. A
   string that will not grow caches its count in `cap`; a builder's `cap` is
   already spoken for by the room, so without this a read walks the whole
   string every time — linear per read, quadratic in a loop, and invisible to
   every allocation counter because the walk allocates nothing. The count is
   kept as bytes arrive, so nothing ever walks a builder at all. */
#define K_STR_HEAD ((long long)sizeof(long long))
#define k_str_count(s) (((long long*)(void*)(s)->data)[-1])
static long long k_str_chars(KStr* s);
static long k_str_seek(KStr* s, long long from);
/* `capw` is the capacity doubled, plus one when the storage came from malloc
   rather than the arena. The regime used to be the sign, and every push asked
   for the magnitude with a negate and a conditional move before it could
   compare; doubled, the question "is there room for `need`" is `2 * need <=
   capw` in either regime, one shifted add. Nothing else reads the word: the
   capacity goes through k_buf_cap and the regime through k_buf_malloced. */
typedef struct { long long capw; long long used; } KBuf;
static inline long long k_buf_cap(const KBuf* b) { return b->capw >> 1; }
static inline int k_buf_malloced(const KBuf* b) { return (int)(b->capw & 1); }
static inline void k_buf_set_cap(KBuf* b, long long cap, int malloced) {
    b->capw = 2 * cap + (malloced ? 1 : 0);
}
/* cap == 0 is a borrowed view; cap != 0 marks data as the body of a
   KBuf-headed buffer this value may extend at its frontier. */
typedef struct { long long len; const unsigned char* data; long long cap; } KBytes;

/* `cap` carries the room and the regime in one field, and the regime rides in
   BIT 0 rather than in the sign. Bit 0 is free because every capacity this
   field ever holds comes from k_b_append_grow's `2 * (len + n)`, clamped up to
   64 -- always even. Reading the room is then one `and` where stripping a sign
   was a `neg` and a `cmovs`, and the emitter inlines that read at ninety-three
   sites: on the run program they cost 18,024,553 instructions, 0.86%.

   0 is still a borrowed view, so `cap != 0` reads as it always did. A set bit
   marks arena storage, which the innermost rewind reclaims wholesale; a clear
   bit on a non-zero cap marks malloc, which this file frees by hand. That is
   the one question the sign used to answer, and `k_bytes_malloced` is now the
   only place that asks it. */
static inline int k_bytes_malloced(const KBytes* b) {
    return b->cap != 0 && !(b->cap & 1);
}

/* Lazy v1 (design/lazy-v1-plan.md): a conditionally-demanded binding's
   pending computation. RC'd, malloc-backed, recycled through a free list --
   never the beat arenas, so a pending thunk can't pin a rewindable region.
   Captured args are copied in at creation; the site dispatcher
   (codegen-emitted d_thunk_eval) runs the computation at first force. */
/* `forced` carries three states rather than two. See k_force for why the
   third one is a flag on the cell and not a list somewhere else. */
#define K_MEMO_NONE 0
#define K_MEMO_KEPT 1
#define K_MEMO_AT_RISK 2

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
static long long k_stat_ryu_renders = 0;
/* renders whose digits the short path found, without ryu's multiplies; a
   merge that loses the path takes this to zero */
static long long k_stat_ryu_short = 0;
static long long k_stat_utf8_bytes = 0;
long long k_stat_find2_calls = 0;
/* JSON number spans classified by k_b_number_span, the presence counter for
   that scan: a decoder that walks its numbers a byte at a time again reads
   zero here. */
long long k_stat_number_spans = 0;
static long long k_stat_append_fast = 0;
static long long k_stat_append_rendered = 0;
static long long k_stat_append_grow = 0;
/* The tenure allocator mallocs outside every other accounting: its blocks
   move no allocation counter, no arena counter and no peak. So a change that
   released none of them read GREEN on all nine counter gates and read as a
   WIN on the instruction vein, because skipping the frees is cheaper than
   doing them. Found on 2026-09-01 by breaking k_ten_release on purpose. A
   free counter is what makes the leak visible: the block count is unmoved by
   it, the free count goes to nought. */
static long long k_stat_ten_blocks = 0;
static long long k_stat_ten_frees = 0;
/* Where a tenure block dies, which the pair above cannot say. A hand-up moves
   a depth's blocks to the depth outside, which frees them later; a release
   frees them here. Either way the block was allocated once and freed once, so
   `ten_blocks` and `ten_frees` read the same on both. Replacing the hand-up
   with a release therefore left every .mem golden byte-identical -- including
   the fixture whose own comment said such a change would turn it red. Found
   on 2026-09-22 by making the change and running the vein. */
static long long k_stat_ten_handups = 0;
static long long k_stat_chain_finds = 0;
static long long k_stat_utf8_zerocopy = 0;
/* Characters counted by walking, which a string that can cache its count
   pays once and a builder pays on every read. Quadratic when a builder is
   read in a loop, and invisible to every allocation counter — the scan
   allocates nothing. */
static long long k_stat_str_scans = 0;
static long long k_stat_str_scan_bytes = 0;
/* A position-to-offset walk that started from the remembered cursor instead
   of from the front of the string. A scan that rewinds between positions
   loses the cursor if the rewind forgets it, and the walk then starts from
   the front at every position: nothing else moves, and this reads nought. */
static long long k_stat_seek_resumes = 0;
/* Indexes answered by stepping the cursor one character on, without the
   general seek: a walk by index takes this path at every character after its
   first. A merge that loses the step reads zero here. */
static long long k_stat_seek_steps = 0;
static long long k_stat_carry_dedup = 0;
static long long k_stat_bytes_malloc = 0;
/* Where arena bytes go, by value shape. The totals above say how much was
   allocated; these say what it was, which is the first question every memory
   diagnosis asks. Headers and payloads count into the shape that owns them. */
static long long k_stat_sh_str = 0;
static long long k_stat_sh_rec = 0;
static long long k_stat_sh_buf = 0;
static long long k_stat_sh_map = 0;
static long long k_stat_sh_bytes = 0;
/* Bytes of block the spare list holds at its fullest. A rewind hands its
   blocks there and the next allocation takes them back, so a program whose
   blocks are reused keeps this near one block; a block nothing can reuse
   stays until the cohort ends, and this is the only counter that sees it,
   since the arena's own peak counts only blocks in use. */
static long long k_spare_bytes = 0;
static long long k_stat_spare_peak = 0;
/* Work-ahead (#1856): constants the pool forced while a fiber was parked, and
   those it dropped because the work died. A program whose pool never ran
   prints neither, so every vein without a sleep reads as it did. */
static long long k_stat_ahead_forced = 0, k_stat_ahead_dropped = 0;
static int k_ahead_ran = 0;
static long long k_stat_bytes_freed = 0;
static long long k_stat_buf_reuse = 0;
/* A map's cached sorted view is malloc'd, so no rewind reclaims it and only an
   explicit free ever will. Counting both ends makes the gap a diffed fact
   rather than a belief: view_allocs minus view_frees is memory the process is
   still holding, and on a program that builds a map per loop iteration it grew
   without bound before anything could see it. */
static long long k_stat_view_allocs = 0;
static long long k_stat_view_frees = 0;
/* Malloc-backed storage currently live, and its high-water mark. Arena
   storage rewinds away; these bytes only leave through free(), so a peak that
   scales with iteration count is a leak by definition.

   Builder buffers and map views both. The definition above always covered
   views — they are malloc'd and only free() releases them — and for a while
   the implementation counted only builders, which left the objective weighing
   memory it could not see. A term that measures less than it claims is worse
   than a missing one, because the gap reads as a zero. */
static long long k_stat_held_live = 0;
static long long k_stat_held_peak = 0;

extern KValue d_thunk_eval(long long site, KValue* args);

KValue k_thunk_new(long long site, int argc, ...) {
    KThunk* t = k_thunk_free;
    if (t) {
        k_thunk_free = t->next_free;
    } else {
        t = (KThunk*)malloc(sizeof(KThunk));
    }
    if (K_COUNTING) k_stat_thunk_allocs++;
    t->rc = 1;
    t->site = site;
    t->forced = K_MEMO_NONE;
    t->argc = argc;
    t->next_free = 0;
    va_list ap;
    va_start(ap, argc);
    for (int i = 0; i < argc; i++) {
#ifdef __wasm__
        /* The emitted IR passes each argument as a first-class `%KValue`,
           which the wasm backend writes into the variadic buffer as its two
           words. The wasm32 C ABI would read a struct through a pointer, so
           the words are read as the backend wrote them. */
        t->args[i].tag = va_arg(ap, long long);
        t->args[i].payload = va_arg(ap, long long);
#else
        t->args[i] = va_arg(ap, KValue);
#endif
        /* a cell holding another cell keeps it alive */
        if (t->args[i].tag == K_THUNK) ((KThunk*)t->args[i].payload)->rc++;
    }
    va_end(ap);
    KValue v;
    v.tag = K_THUNK;
    v.payload = (long long)t;
    return v;
}

/* A constant that names itself hands its own cell out while it is still
   being built, and the cell has to answer for itself. `k_caf_init` seeds
   every cell with one of these before it runs any builder, so a mention
   during the build loads a thunk rather than the zeroed global — which is an
   integer zero, and reads as one. Demanding it is the failure the oracle
   reports; holding it without demanding it renders as any pending cell. */
/* It calls exit, and saying so is worth instructions rather than tidiness.
   Unmarked, clang inlines the fprintf and the exit into every caller that
   validates a tag — so a hot runtime entry builds a frame and saves the
   registers that error path needs before it can test anything, and pays for
   them on the path that never dies. */
__attribute__((noreturn, noinline)) void k_die(const char* msg);

enum { K_SITE_BLACKHOLE = -1 };

KValue k_caf_blackhole(void) {
    KThunk* t = (KThunk*)malloc(sizeof(KThunk));
    if (!t) { fputs("out of memory\n", stderr); exit(1); }
    t->rc = 1;
    t->site = K_SITE_BLACKHOLE;
    t->forced = K_MEMO_NONE;
    t->argc = 0;
    t->next_free = 0;
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
    if (K_COUNTING) k_stat_thunk_frees++;
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
    if (K_COUNTING && cell.tag == K_THUNK) k_stat_thunk_escaped++;
}

KValue k_thunk_release_unless(KValue cell, KValue result) {
    if (cell.tag != K_THUNK) return result;
    if (result.tag == K_THUNK && result.payload == cell.payload) {
        if (K_COUNTING) k_stat_thunk_escaped++;
        return result;
    }
    k_thunk_release_cell((KThunk*)cell.payload);
    return result;
}

KValue k_render(KValue v, long long quote);
static KValue k_bytes_view(const unsigned char* data, long long len);
/* Whether `from..to`, 1-based and inclusive, lies inside `len` elements:
   1 <= from <= to <= len. A length is never negative, so as unsigned
   compares this is `from - 1 < to` and `to <= len`: a `from` below one wraps
   to the top of the range and fails the first, a negative `to` fails the
   second. The signed spelling was three compares folded through `setcc` and
   `or`, 861,498 times a run at the head of the decoder's token path. */
static inline __attribute__((always_inline)) int k_span_in(long long from, long long to,
                                                           long long len) {
    return (unsigned long long)from - 1 < (unsigned long long)to &&
           (unsigned long long)to <= (unsigned long long)len;
}

static KValue k_utf8_bad(const char* data, long long len, const char* origin,
                         long long* chars);
static inline void k_str_seed_count(KStr* s, long long chars);
static KValue k_render_at(KValue v, long long quote, int held);
KValue k_b_render_value(KValue v);

/* A cell whose value was built inside the innermost beat cannot be memoized
   outright: the loop rewinds between iterations, and the memo would point at
   reclaimed arena while the cell still said it was forced. Declining it
   outright is what that used to mean, and the cost is one whole evaluation on
   every later force — in the streaming shape, once a block. sha256 forces one
   cell sixty-four times a block and evaluated it sixty-four times.

   So the memo is kept and marked AT RISK instead. A rewind may take the answer
   away, and a carry evacuation may move it somewhere a rewind cannot reach —
   k_deep_copy's thunk arm re-points `result` for exactly that reason — and
   which of the two happened is a question with a cheap answer: is the storage
   still in the live chain. Asked at the next force, where it costs a walk of a
   block list rather than a re-evaluation, and answered once per force rather
   than once per rewind.

   A list of at-risk cells checked at each rewind would be the other shape and
   is worse: a cell is refcounted and can be freed and handed out again while
   the list still names it, so the list would have to be kept in step with the
   free list. The flag rides on the cell and cannot go stale.

   The captures are NOT dropped for an at-risk memo, because a cell that may be
   asked to run again has to still have them. */
static int k_memo_outlives(KValue result);

/* Is an at-risk memo's answer still where it was left? A non-heap answer never
   moved. A heap one is gone if the rewind retired the block it sat in, or left
   the bump pointer below it. NULL mark means the whole live chain, which is
   the question here: not "does it outlive some mark" but "is it still there".
   Storage outside the arena — a builder's malloc, tenured bytes — reads as
   gone, which costs a recompute and is never wrong. */
static int k_still_live(const void* p);
static int k_is_heap(long long tag);

static int k_memo_still_there(KThunk* t) {
    if (!k_is_heap(t->result.tag)) return 1;
    return k_still_live((const void*)(intptr_t)t->result.payload);
}

/* The cold half, kept out of line. Everything but "the memo is standing" is
   here, because k_force is called once per demanded cell and the caller pays
   for its size: growing it past what the inliner will take cost deepbench
   4,375,985 instructions, 0.60% of the run, with no counter moving at all. */
KValue k_force(KValue v);

static __attribute__((noinline)) KValue k_force_slow(KThunk* t) {
    if (t->forced == K_MEMO_AT_RISK) {
        if (k_memo_still_there(t)) return t->result;
        t->forced = K_MEMO_NONE;
    }
    if (t->site == K_SITE_BLACKHOLE) k_die("a lazy binding demands its own value");
    if (K_COUNTING) k_stat_thunk_evals++;
    KValue answer = d_thunk_eval(t->site, t->args);
    /* The computation can answer with a cell of its own: a deferred binding
       handed back through a pass-through arm is the next call's deferred
       binding. Scrutiny wants the value at the end of that chain, which is
       what the interpreter's force has always walked to. */
    answer = k_force(answer);
    t->result = answer;
    if (!k_memo_outlives(answer)) {
        t->forced = K_MEMO_AT_RISK;
        return t->result;
    }
    t->forced = K_MEMO_KEPT;
    /* the computation ran; its captures are done */
    k_thunk_drop_args(t);
    return t->result;
}

KValue k_force(KValue v) {
    if (v.tag != K_THUNK) return v;
    KThunk* t = (KThunk*)v.payload;
    if (K_COUNTING) k_stat_thunk_forces++;
    if (t->forced == K_MEMO_KEPT) return t->result;
    return k_force_slow(t);
}

/* A constructor slot is where a knot ties, so a field still being computed is
   stored rather than demanded. Every other site demands: reaching a blackhole
   anywhere else is the error the blackhole exists to report. */
KValue k_force_unless_black(KValue v) {
    if (v.tag == K_THUNK) {
        KThunk* t = (KThunk*)v.payload;
        if (!t->forced && t->site == K_SITE_BLACKHOLE) return v;
    }
    return k_force(v);
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
/* The closure calling convention. `preserve_none` (LLVM 19) lets a closure
   body clobber every register, so its callee-saved prologue disappears and
   the caller keeps what it needs instead -- worth 3.06% of encodebench and
   1.57% of the consolidated run program, measured 2026-09-06. The host's
   clang is whatever the host has, so the build PROBES for the keyword and
   defines this to nothing when it is absent; every site that stores, passes
   or calls a closure pointer carries it, and a mismatch is a miscompile
   rather than a slowdown. The fnref family (`r->fn`, no env parameter) is
   deliberately NOT converted: the emitted fast arm tests for a closure tag
   before it calls, so the two families never meet. */
#ifdef KANSO_PRESERVE_NONE
#define K_CLOSCC __attribute__((preserve_none))
#else
#define K_CLOSCC
#endif

/* A few of the doors the program calls take the same convention, so they
   save nothing the caller does not keep. Each was measured on its own on the
   run program under clang 19, against the program's functions already on
   preserve_none: `append_rendered` -4,998,510 instructions, `entries`
   -4,571,478, `utf8` -2,157,799 and `to_float_slice` -1,113,156. The same
   trial made `at` dearer by 9,209,935 and `utf8_slice_raw` by 4,306,698,
   because their callers keep more live across the call than the callee used
   to save, so they stay on the C convention. The emitter writes the keyword
   on the declares and calls of exactly these, in `PRESERVE_NONE_DOORS`, and
   `tests/the_doors_the_program_calls_agree.rs` holds the two lists equal. */
#define K_DOORCC K_CLOSCC

/* `fn` is the wrapper C calls through. `body` is the lambda itself on the
   tailcc convention, for a call in tail position to jump to: the wrapper is
   a frame of its own, and a loop that hands itself on through a lambda ran
   the toml port out of stack. Runtime-built closures have no body, and a
   call to one goes through `fn`. The two counts are ints so the body fits
   in the 32 bytes the struct already took, and no allocation grows. */
typedef struct {
    KValue (K_CLOSCC *fn)(void*, KValue);
    void* env;
    int ncaps;
    int arity;
    void* body;
} KClosure;
/* A named group handed out as a value. The emitter writes one of these per
   wrapper as a static, so the collector never sees it, and it carries the
   name because the group's diagnostic says which group. `builtin` says which
   sentence a wrong argument count gets: a group's arms are what did not
   match, where a builtin takes one count and can say which. */
typedef struct { void* fn; long long arity; const char* name; long long builtin; } KFnref;
typedef struct { long long type_id; long long nfields; KValue* fields; } KRec;
/* A nominal subtype wrapper: transparent to every consumer, visible only
   to dispatch, which walks the chain and prefers nearer declarations. */
typedef struct { long long type_id; KValue inner; } KSub;
typedef struct KDesc KDesc;
struct KDesc { long long dtag; KValue x; KValue y; };
/* dtag: 0 print, 2 args, 3 stdin, 4 read_file, 5 write_file, 6 bind,
   7 join, 8 sleep, 9 random, 10 nil, 11 write (stdout, no newline),
   12 write_err, 13 env, 14 exists, 15 list_dir, 16 now, 17 run,
   18 is_dir, 19 make_dir, 20 listen, 21 accept, 22 net_read, 23 net_write,
   24 net_close, 25 await (a started child, inside a group), 26 start,
   27 kill, 28 net_port, 29 rescue,
   30 read_bytes,
   31 settled (a box already holding its answer: what `effect v` was handed),
   32 read_line, 33 modified */

/* An err's propagation trace rides on the err value alone: the origin
   ("fn at file:line", interned at the construction site; NULL for
   executor-born errs) plus one hop per dispatcher failure pass-through,
   newest first. The happy path never allocates or touches any of this. */
typedef struct KHop { const char* fn; struct KHop* prev; } KHop;
typedef struct KErrBox KErrBox;
/* `merged` says the reason is a list *of reasons* rather than one reason
   that happens to be a list, so merging is a fold: three failures answer
   three reasons however they were grouped, and `err ["a" "b"]` stays one. */
/* `hako` is the package that RAISED this err, which is what an arm asks
   about: a group may not see a failure its own package made. It is not stored
   separately — codegen hands one literal holding both halves, "<hako>\0<fn> at
   <file>:<line>", so `hako` points at the literal and `origin` just past its
   NUL. Nineteen runtime signatures carry an origin and nine codegen sites emit
   one; threading a second argument through all of them to move four bytes of
   information was not worth it. NULL for an err with no frame, which therefore
   belongs to no package and matches every arm. */
struct KErrBox { KValue reason; const char* origin; const char* hako;
                 KHop* hops; KErrBox* cause; long long merged; };

static KValue k_mklist(long long n, KValue* items);
static KValue* k_buf(long long cap);
static KValue k_list_own(KValue* items, long long n);
KValue k_call1(KValue f, KValue a);
static KValue k_partial_apply(KClosure* p, long long n, KValue* fresh);
static KValue k_partial_run(KClosure* p);
KValue k_closure(KValue (K_CLOSCC *fn)(void*, KValue), long long arity, long long ncaps, KValue* caps, void* body);
KValue k_env_get(void* env, long long i);
static KValue* k_map_sorted(KMap* m, long long* out_len);

/* The arena is a chain of blocks, newest first; allocation bumps in the head
   block. k_beat_boundary rewinds the chain to a snapshot taken on its first
   call, retiring newer blocks to a spare pool for reuse — a steady-state loop
   recycles the same warm pages instead of marching through cold memory. If no
   boundary is ever signalled the arena only grows, exactly as before. */
/* `host` is set on a tail: the unused end of a block, split off to stay the
   bump region when an oversize allocation is given a block of its own. Real
   blocks leave it null. The pad keeps the arena that follows the header on a
   sixteen-byte boundary. */
typedef struct KBlock { struct KBlock* next; size_t cap; struct KBlock* host; size_t pad; } KBlock;
KBlock* k_blocks = NULL;
static KBlock* k_spare = NULL;

/* The live chain sorted by address, for the survival questions once the chain
   is long. Each block's `pad` holds its position in the chain counted from the
   oldest, so "is this block older than the mark's" is one compare. A program
   whose arena only grows -- an interpreter that never frees its heap -- built
   a chain of hundreds of blocks, and every survival check walked it: the lox
   port's loop of 40,000 passes took 30.6 s with the lap carry and 1.9 s with
   64 MiB blocks. Below K_BIDX_WALK blocks the walk is kept, because the hot
   paths ask with a chain of one or two and a search would only cost them. */
#define K_BIDX_WALK 8
static KBlock** k_bidx = NULL;
static long long k_bidx_n = 0, k_bidx_cap = 0;

static void k_bidx_link(KBlock* b) {
    b->pad = b->next ? b->next->pad + 1 : 0;
    if (k_bidx_n == k_bidx_cap) {
        k_bidx_cap = k_bidx_cap ? 2 * k_bidx_cap : 64;
        k_bidx = realloc(k_bidx, sizeof(KBlock*) * (size_t)k_bidx_cap);
        if (!k_bidx) { fputs("out of memory\n", stderr); exit(1); }
    }
    long long i = k_bidx_n;
    while (i > 0 && k_bidx[i - 1] > b) { k_bidx[i] = k_bidx[i - 1]; i--; }
    k_bidx[i] = b;
    k_bidx_n++;
}

static void k_bidx_unlink(KBlock* b) {
    long long i = 0;
    while (i < k_bidx_n && k_bidx[i] != b) i++;
    if (i == k_bidx_n) return;
    for (; i + 1 < k_bidx_n; i++) k_bidx[i] = k_bidx[i + 1];
    k_bidx_n--;
}

/* The live block holding q, or NULL. */
static __attribute__((noinline)) KBlock* k_bidx_find(const char* q) {
    if (K_COUNTING) k_stat_chain_finds++;
    long long lo = 0, hi = k_bidx_n;
    while (hi - lo > 1) {
        long long mid = (lo + hi) / 2;
        if ((const char*)k_bidx[mid] <= q) lo = mid; else hi = mid;
    }
    if (k_bidx_n == 0 || (const char*)k_bidx[lo] > q) return NULL;
    KBlock* b = k_bidx[lo];
    const char* start = (const char*)(b + 1);
    return q >= start && q < start + b->cap ? b : NULL;
}

static inline int k_chain_long(void) {
    return k_blocks && k_blocks->pad >= K_BIDX_WALK;
}
/* bytes held by the live chain, and the most it ever held: the process's
   deterministic peak, the number the one-shot welfare term watches */
long long k_live_block_bytes = 0;
static long long k_stat_peak_block_bytes = 0;
static long long k_stat_cohort_frees = 0;
static long long k_stat_cohort_kept = 0;
/* One remembered position, so a forward sweep over a string does not restart
   at the front. It names its string by address, which is only meaningful for
   as long as that address means that string — so `k_beat_rewind` forgets it
   when the string sits above the mark, because the arena hands those
   addresses back and the next allocation there is a different string.

   This used to say a reset was unnecessary, on the grounds that a string
   whose bytes changed is a builder and a builder never qualifies. The string
   whose bytes change across a rewind is not a builder; it is somebody else,
   wearing the address. `text/slice s 5 5` then answered `g` where the
   interpreter answered `e`.

   A single cell is safe because the scheduler is green — every fibre runs on
   the one thread — and it would need to move into the fibre if that ever
   stopped being true. */
KStr* k_seek_str = NULL;
static long long k_seek_char = 0;
static long k_seek_byte = 0;

char* k_arena = NULL;
/* The emitted IR reads and writes this as an i64, which is size_t on a 64-bit
   host and twice it on wasm32. The native declaration stays size_t: spelled
   `unsigned long long`, the same width under another name, it took 16 bytes
   off every benchmark's machine code and moved twelve of the fourteen work
   rows, deepbench up by 324,000 instructions. */
#ifdef __wasm__
unsigned long long k_arena_left = 0;
#else
size_t k_arena_left = 0;
#endif

/* Cost counters: every value is an exact, machine-independent constant for a
   deterministic program, so they golden like output does. KANSO_COUNTERS=1
   dumps them to stderr at exit; CI diffs the dump against a committed cost
   golden — a performance ratchet with no clock in it. */
static long long k_stat_allocs = 0;
static long long k_stat_alloc_bytes = 0;
static long long k_stat_blocks = 0;
static long long k_stat_perm_allocs = 0;
long long k_stat_beat_iters = 0;
static long long k_stat_evac_bytes = 0;
static long long k_stat_evac_allocs = 0;
/* What DECIDING costs, where evac_allocs and evac_bytes count what copying
   costs. k_slots_survive reads a node's whole immediate interior on every ask
   and nothing in the tree bounds how often it is asked, so a loop carrying a
   list that grows with its input asks a question that grows with it. On the
   8,192-byte digest, with the carry tier admitted on the branch that was
   declined for other reasons, that was 33,024 asks of 8,256 slots —
   272,646,144 examinations finding zero heap slots — against 33,827
   allocations that actually evacuated. Every counter beside this one read that
   workload as nearly free.

   Slots examined rather than calls made, because the calls were linear and the
   slots were not; and slots rather than bytes, because this counts an
   algorithm-level step that no platform can widen.

   It costs about two retired instructions per slot examined, which is the
   branch itself: basket rises 32,004 on 16,002 slots and pendbench 236,166 on
   118,087, both exactly twice. Accumulating once at the exit instead was
   measured and is not cheaper — same two per slot on both — so the count
   stays where it is read. */
static long long k_stat_survive_slots = 0;
/* The two in-place writers are the decode kernels, and neither had a presence
   counter: a doubling of either is invisible to `allocs`, because the fast
   path allocates nothing. Counted by PATH, so the diff also says when a write
   stops taking the fast one. */
static long long k_stat_put_mut_fast = 0;
static long long k_stat_put_mut_grow = 0;
static long long k_stat_push_mut_fast = 0;
static long long k_stat_push_mut_slow = 0;

static void k_stats_dump(void);

static void k_stats_on_signal(int sig) {
    (void)sig;
    fprintf(stderr, "--- counters on request ---\n");
    k_stats_dump();
}

/* Live and peak bytes of malloc-backed accumulator storage — the dimension
   every counter here was blind to while a real workload held 3.6 GB. */
static long long k_perm_live = 0, k_perm_peak = 0;

static void k_stats_dump(void) {
    /* A shipped binary has no counters to report, and the dangerous answer is
       not zero -- it is a block that looks right. The runtime's own sites are
       what this function prints, and they are compiled out here while the
       emitted fast paths were already gone, so every row a runtime site owns
       would read correctly and the ones an inlined path owns would not. On
       escapebench that was push_mut_fast 0 against 3,000 and push_mut_slow
       12,000 against 1,200,000, with twenty-odd rows agreeing either way: a
       reader has no reason to distrust the two that do not. So say so and
       print nothing. */
    if (!K_COUNTING) {
        fprintf(stderr,
            "counters: this binary was built without them, so there is "
            "nothing to report.\nbuild it with `kanso build --counters` "
            "and run that.\n");
        return;
    }
    fprintf(stderr, "allocs=%lld\n", k_stat_allocs);
    fprintf(stderr, "alloc_bytes=%lld\n", k_stat_alloc_bytes);
    fprintf(stderr, "arena_blocks=%lld\n", k_stat_blocks);
    fprintf(stderr, "arena_peak_bytes=%lld\n", k_stat_peak_block_bytes);
    fprintf(stderr, "cohort_frees=%lld\ncohort_kept=%lld\n", k_stat_cohort_frees, k_stat_cohort_kept);
    fprintf(stderr, "perm_allocs=%lld\n", k_stat_perm_allocs);
    fprintf(stderr, "beat_iters=%lld\n", k_stat_beat_iters);
    fprintf(stderr, "evac_allocs=%lld\nevac_bytes=%lld\nsurvive_slots=%lld\n",
            k_stat_evac_allocs, k_stat_evac_bytes, k_stat_survive_slots);
    fprintf(stderr,
        "put_mut_fast=%lld\nput_mut_grow=%lld\npush_mut_fast=%lld\npush_mut_slow=%lld\n",
        k_stat_put_mut_fast, k_stat_put_mut_grow,
        k_stat_push_mut_fast, k_stat_push_mut_slow);
    fprintf(stderr,
        "thunk_allocs=%lld\nthunk_forces=%lld\nthunk_evals=%lld\n"
        "thunk_frees=%lld\nthunk_escaped=%lld\nthunk_live_exit=%lld\n"
        "el_parses=%lld\nryu_renders=%lld\nryu_short=%lld\nutf8_bytes=%lld\n"
        "find2_calls=%lld\nnumber_spans=%lld\nappend_fast=%lld\nappend_grow=%lld\n"
        "append_rendered=%lld\n"
        "utf8_zerocopy=%lld\ncarry_dedup=%lld\nbytes_malloc=%lld\nbytes_freed=%lld\n"
        "perm_live_bytes=%lld\nperm_peak_bytes=%lld\n",
        k_stat_thunk_allocs, k_stat_thunk_forces, k_stat_thunk_evals,
        k_stat_thunk_frees, k_stat_thunk_escaped,
        k_stat_thunk_allocs - k_stat_thunk_frees, k_stat_el_parses,
        k_stat_ryu_renders, k_stat_ryu_short, k_stat_utf8_bytes, k_stat_find2_calls,
        k_stat_number_spans,
        k_stat_append_fast, k_stat_append_grow, k_stat_append_rendered,
        k_stat_utf8_zerocopy,
        k_stat_carry_dedup, k_stat_bytes_malloc, k_stat_bytes_freed,
        k_perm_live, k_perm_peak);
    fprintf(stderr, "str_scans=%lld\nstr_scan_bytes=%lld\nseek_resumes=%lld\nseek_steps=%lld\n",
            k_stat_str_scans, k_stat_str_scan_bytes, k_stat_seek_resumes, k_stat_seek_steps);
    fprintf(stderr, "buf_reuse=%lld\nheld_peak_bytes=%lld\n", k_stat_buf_reuse, k_stat_held_peak);
    fprintf(stderr, "view_allocs=%lld\nview_frees=%lld\n",
            k_stat_view_allocs, k_stat_view_frees);
    fprintf(stderr, "ten_blocks=%lld\nten_frees=%lld\nten_handups=%lld\nchain_finds=%lld\n",
            k_stat_ten_blocks, k_stat_ten_frees, k_stat_ten_handups, k_stat_chain_finds);
    fprintf(stderr,
        "sh_str=%lld\nsh_rec=%lld\nsh_buf=%lld\nsh_map=%lld\nsh_bytes=%lld\n",
        k_stat_sh_str, k_stat_sh_rec, k_stat_sh_buf, k_stat_sh_map, k_stat_sh_bytes);
    fprintf(stderr, "spare_peak_bytes=%lld\n", k_stat_spare_peak);
    if (k_ahead_ran) {
        fprintf(stderr, "ahead_forced=%lld\nahead_dropped=%lld\n",
                k_stat_ahead_forced, k_stat_ahead_dropped);
    }
}

/* A one-shot boundary returns its garbage to the allocator. A cohort
   rewind's spare blocks have no hot loop waiting to recycle them, and
   megabytes of dead pages held in spare are exactly the peak the license
   exists to cut. Hot-loop rewinds never call this; they keep their warm
   pages. */
static void k_spare_release(int keep) {
    KBlock** link = &k_spare;
    for (KBlock* b = *link; b; b = *link) {
        if (keep > 0) {
            keep--;
            link = &b->next;
            continue;
        }
        *link = b->next;
        if (K_COUNTING) k_spare_bytes -= (long long)b->cap;
        free(b);
    }
}

/* An oversize block is the exact size of the value it was opened for, so a
   value that grows a little each pass -- a string a loop appends to -- asks
   for a few bytes more than any block the last rewind gave back. Every pass
   opened a new block while the old ones waited in the spare list for the
   cohort to end. A loop appending a byte a pass to a 1 MiB string held 777 MB
   at 6,000 passes with an arena peak of 3 MB. So when an oversize request
   misses, the oversize spares smaller than it are freed. Standard 1 MiB spares
   are kept for the refills they serve. */
static __attribute__((noinline, cold)) void k_spare_drop_short(size_t need) {
    KBlock** link = &k_spare;
    for (KBlock* b = *link; b; b = *link) {
        if (b->cap > (size_t)(1 << 20) && b->cap < need) {
            *link = b->next;
            if (K_COUNTING) k_spare_bytes -= (long long)b->cap;
            free(b);
            continue;
        }
        link = &b->next;
    }
}

static void k_arena_push(size_t need) {
    KBlock** link = &k_spare;
    while (*link && (*link)->cap < need) link = &(*link)->next;
    KBlock* b = *link;
    if (b) {
        *link = b->next;
        if (K_COUNTING) k_spare_bytes -= (long long)b->cap;
    } else {
        if (need > (size_t)(1 << 20)) k_spare_drop_short(need);
        b = malloc(sizeof(KBlock) + need);
        if (!b) { fputs("out of memory\n", stderr); exit(1); }
        b->cap = need;
        b->host = NULL;
        if (K_COUNTING) k_stat_blocks++;
    }
    b->next = k_blocks;
    k_blocks = b;
    k_bidx_link(b);
    k_live_block_bytes += (long long)b->cap;
    if (K_COUNTING && k_live_block_bytes > k_stat_peak_block_bytes) {
        k_stat_peak_block_bytes = k_live_block_bytes;
    }
    k_arena = (char*)(b + 1);
    k_arena_left = b->cap;
}

int k_stats_on = -1;

/* The counter switch is read once, before main and before anything a program
   does can consult it.

   It used to initialise itself lazily on the first allocation, inside
   k_alloc — which is inlined into every hot caller there is. So every loop
   that allocated carried a WRITE to this global, and a loop that writes a
   global cannot have any read of that global hoisted out of it. Every
   counter check in every hot loop reloaded and re-tested it once an
   iteration, on release runs where the counters are off and the answer never
   changes. Setting it here, from a constructor that runs ahead of every entry
   point this file has, leaves the global written-once and lets the loads
   hoist.

   It also fixes what the counted run reports. Every counting site tests
   `k_stats_on > 0`, and -1 fails that test, so everything that happened
   before the first arena allocation went uncounted — one byte malloc and
   sixteen bytes of shared string on basket, and thirty-two to forty-eight
   bytes of shared buffer on three others. Those are the counters moving to
   what was always true. */
__attribute__((constructor)) static void k_stats_switch(void) {
    k_stats_on = getenv("KANSO_COUNTERS") != NULL;
}

/* The refill path stays out of line; the bump inlines into every hot
   caller. Counters, when enabled, are exact: both paths count.

   preserve_most makes the refill save every register it touches, so a
   caller keeps its live values in caller-saved registers across the call.
   Without it a hot function whose only call was this one pinned its values
   in callee-saved registers and opened with pushes for a path a run takes
   about a thousand times: four instructions a call left k_b_slice_raw,
   k_map_lit, k_rec, k_b_append_slice and k_b_push on runbench, and the
   refill paid twelve a call over 1,278 calls. The six cold helpers that
   carry the attribute are the ones a hot function calls and nothing else. */
/* An allocation larger than a block gets a block of exactly its size, and
   that block used to become the bump region with nothing left in it, so the
   next small allocation opened a fresh 1 MiB block while the one before still
   had room. The run program's index shape builds a 1,572,864-byte string and
   then allocates a header, and that header's block was the top of the whole
   program's peak. So when the current block has a useful tail, the tail is
   split off as a block of its own -- its host shrinks to the part in use, so
   no two blocks cover the same bytes -- and pushed back on top of the
   oversize block to go on serving small allocations. A rewind that pops a
   tail gives its bytes back to the host rather than to the spare list, since
   they were never a block malloc handed out. */
#define K_TAIL_MIN 4096
static __attribute__((noinline)) void* k_alloc_oversize(size_t n) {
    KBlock* host = k_blocks;
    char* tail = k_arena;
    size_t left = k_arena_left;
    if (!host || left < K_TAIL_MIN + sizeof(KBlock)) {
        k_arena_push(n);
        void* p = k_arena;
        k_arena += n;
        k_arena_left -= n;
        return p;
    }
    /* The host keeps the bytes in use and the tail takes the rest, header
       and all, so the split leaves the live total where it was. */
    host->cap = (size_t)(tail - (char*)(host + 1));
    k_arena_push(n);
    void* p = k_arena;
    KBlock* t = (KBlock*)tail;
    t->cap = left - sizeof(KBlock);
    t->host = host;
    t->next = k_blocks;
    k_blocks = t;
    k_bidx_link(t);
    k_arena = (char*)(t + 1);
    k_arena_left = t->cap;
    return p;
}

static __attribute__((noinline, preserve_most)) void* k_alloc_refill(size_t n) {
    if (n > (1 << 20)) return k_alloc_oversize(n);
    k_arena_push((size_t)(1 << 20));
    void* p = k_arena;
    k_arena += n;
    k_arena_left -= n;
    return p;
}

static inline __attribute__((always_inline)) void* k_alloc(size_t n) {
    n = (n + 15) & ~(size_t)15;
    /* The same `> 0` test every other counting site makes. Written as
       `!= 0` around a second `if`, it compiled to a compare and two branches
       at every inlined allocation, one of them for the -1 the switch has
       not held since the constructor above began setting it: 7.3 million
       allocations a run on the run program, one instruction each. */
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += (long long)n;
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

/* `reg_any` summarises the three registries for THIS mark: a bitmask of
   K_REG_CHUNK/VIEW/PERM, set when something is registered at this depth and
   cleared by the rewind. It lived in a parallel `k_reg_any[K_BEAT_MAX]` until
   2026-09-17. The rewind's fast path had the mark pointer in hand and had to
   turn it back into a depth to index that array -- a subtract and a shift on
   the hot path, once an iteration -- and reading it off the mark costs one
   displacement instead. */
typedef struct { KBlock* block; char* ptr; size_t left; long long bytes;
                 int reg_any; } KMark;
#define K_BEAT_MAX 64
KMark k_beat_stack[K_BEAT_MAX];
int k_beat_depth = 0;
/* The innermost mark, or `k_beat_none` where there is no mark to rewind to:
   at depth zero, and past the deepest mark the stack holds. It is what
   `k_beat_depth` says and is kept beside it because the rewind's fast path
   wants the pointer and never the number: deriving it took a load, a
   decrement, a range test and three address instructions, once an iteration,
   to arrive at a value that does not change for the life of the loop. Every
   write to the depth goes through `k_beat_set_depth`, and the counting build
   checks the two agree at every iteration -- see
   `tests/the_cached_beat_top_tracks_the_depth.rs`.

   `k_beat_none` stands where NULL used to, so the rewind asks no null
   question at every iteration. Its `reg_any` is set, which sends it past the
   fast path to `k_beat_rewind_slow`, and the slow path returns at once for
   it: a beat past the deepest mark keeps count and rewinds nothing, as it
   always did. */
KMark k_beat_none = { .reg_any = 1 };
KMark* k_beat_top = &k_beat_none;

/* The innermost mark the seek cursor's string lies under: allocated before
   that mark was pushed, so a rewind to it or to any mark pushed later cannot
   hand the string's address back. A rewind to an outer mark can, and forgets
   the cursor. Working this out when the cursor is set, which a scan does once
   per string it starts on, leaves the rewind one comparison, which every loop
   pays at every iteration.

   At depth zero no mark exists yet, so every mark ever pushed comes after the
   string. Past the deepest mark the stack holds, nothing is known, and the
   answer is one past the stack's end so that every rewind forgets. With a
   mark in hand, a string in the range the mark would hand back sits above it,
   and so does any string once a new block has been taken, since the range is
   then more than one block and one test cannot bound it. */
KMark* k_seek_under = k_beat_stack;

static inline void k_seek_note(KStr* s) {
    k_seek_str = s;
    KMark* top = k_beat_top;
    if (k_beat_depth == 0) { k_seek_under = k_beat_stack; return; }
    if (top == &k_beat_none) { k_seek_under = k_beat_stack + K_BEAT_MAX; return; }
    int above = k_blocks != top->block
        || (uintptr_t)s - (uintptr_t)top->ptr < (uintptr_t)k_arena - (uintptr_t)top->ptr;
    k_seek_under = above ? top + 1 : top;
}

static inline void k_beat_set_depth(int d) {
    k_beat_depth = d;
    /* `d > 0 && d <= K_BEAT_MAX` as one unsigned compare: a depth of zero
       wraps to a huge index and fails the same test a depth past the top
       does. Two compares and two branches become one compare and a cmov. */
    k_beat_top = ((unsigned)(d - 1) < (unsigned)K_BEAT_MAX)
               ? &k_beat_stack[d - 1] : &k_beat_none;
}

/* Whether anything is on the buffer shelf. The shelf is twelve pointers and
   the flush is a 96-byte memset, which a beat loop paid once an iteration to
   clear a shelf that was already empty. Programs that never donate a buffer
   never write it, so the memset becomes one test. */
int k_buf_dirty = 0;
static void k_buf_flush(void);

/* Malloc-backed builder chunks handed to strings by the zerocopy finish.
   The string dies at its frame's rewind; the chunk would outlive it as a
   leak — one burned buffer per iteration was 85 MB of encodebench RSS.
   Registration happens at the handoff, and the rewind frees the frame's
   chunks wholesale: the carry copy-out never shares malloc'd data (it
   fails k_survives and is copied), so nothing can point here afterward. */
/* The same shape for a map's sorted view, and for the same reason. The view
   is malloc'd so a below-mark header can never hold a pointer a rewind frees,
   and the price was that a transient map's view had no owner at all: a loop
   building one map per iteration allocated 200,000 views and freed none.

   A header born this beat dies at the rewind, and both copy paths null the
   view rather than share it, so the rewind is exactly where the view can go.
   HEADERS are registered rather than views, which buys two things: a view
   that outgrows its buffer is followed without a fixup, and freeing through
   the header nulls it, so a header registered twice frees once. */
/* The registry grows rather than spilling. A fixed bank with an overflow
   counter is what k_chunkreg does, and it is wrong here for a reason the
   corpus showed: a fixture registered more than a bank's worth inside one
   beat and the excess silently went back to leaking, which is the behaviour
   this exists to remove. A cap that quietly stops working is worse than no
   cap, so this one has none. */
/* One word per depth standing for the three per-depth registries, so a rewind
   reads one array element where it read four. Each registry owns a bit and
   clears its own: a summary that any of them could clear would go stale the
   first time a pop emptied one and left another holding entries.

   It says only whether a registry HAS entries, never how many, because that is
   the whole of what the fast path asks. The counts stay where they were — the
   flush loops need them, and a summary that tried to carry them would be a
   second copy of state to keep honest. */
#define K_REG_CHUNK 1
#define K_REG_VIEW  2
#define K_REG_PERM  4
#define k_reg_any_at(d) (k_beat_stack[d].reg_any)

static KMap** k_viewreg[K_BEAT_MAX];
static long long k_viewreg_n[K_BEAT_MAX];
static long long k_viewreg_cap[K_BEAT_MAX];
static void k_view_free(KValue* view);
static void k_map_view_free(KValue* view);

static void k_viewreg_push(int d, KMap* m) {
    if (k_viewreg_n[d] == k_viewreg_cap[d]) {
        long long grown = k_viewreg_cap[d] ? k_viewreg_cap[d] * 2 : 256;
        KMap** wider = realloc(k_viewreg[d], sizeof(KMap*) * (size_t)grown);
        if (!wider) { fputs("out of memory\n", stderr); exit(1); }
        k_viewreg[d] = wider;
        k_viewreg_cap[d] = grown;
    }
    k_viewreg[d][k_viewreg_n[d]++] = m;
    k_reg_any_at(d) |= K_REG_VIEW;
}

static void k_viewreg_flush(int d) {
    for (long long i = 0; i < k_viewreg_n[d]; i++) {
        KMap* m = k_viewreg[d][i];
        if (m->sorted) {
            if (m->sorted != m->pairs) k_map_view_free(m->sorted);
            m->sorted = NULL;
            m->sorted_len = 0;
        }
    }
    k_viewreg_n[d] = 0;
}

/* A pop that keeps its region keeps its maps, so their views hand up with
   the frame's chunks rather than being freed under a live header.
   The registry is empty on every pop a decode makes — no map it builds is
   ever asked for a sorted view — so the test that finds it empty is worth
   more inline than the call that would find it empty. Measured on the decode
   benchmark: 632,550 pops, every one of them over an empty registry. */
static void k_viewreg_migrate_held(int d) {
    if (d > 0) {
        for (long long i = 0; i < k_viewreg_n[d]; i++) k_viewreg_push(d - 1, k_viewreg[d][i]);
    }
    k_viewreg_n[d] = 0;
}

static inline __attribute__((always_inline)) void k_viewreg_migrate(int d) {
    if (d < 0 || d >= K_BEAT_MAX || k_viewreg_n[d] == 0) return;
    k_reg_any_at(d) &= ~K_REG_VIEW;
    k_viewreg_migrate_held(d);
}

static int k_born_this_beat(const void* p);

static void k_viewreg_add(KMap* m) {
    int d = k_beat_depth - 1;
    if (d < 0 || d >= K_BEAT_MAX) return;
    k_viewreg_push(d, m);
}

static int k_frozen_holds(const void* p);
static int k_survives(const void* p, KMark* m);
static int k_frozen_n;

/* The view goes to the beat whose rewind reclaims its map's header, asked from
   the innermost depth out. Asking only the innermost mark, as
   `k_born_this_beat` does, missed a map read inside a region call one depth
   below the loop that built it: the header lay under the region's mark, so
   the view was never registered, and mal's evaluator built 209,210 views at
   two hundred thousand iterations and freed 10,519. A header in no arena
   block -- a carry buffer, a tenure block -- registers at the innermost depth,
   which rewinds no later than its storage goes; a frozen constant keeps its
   view for the life of the program, as before. */
static __attribute__((noinline)) void k_viewreg_own(KMap* m) {
    if (k_frozen_n && k_frozen_holds(m)) return;
    for (int d = k_beat_depth - 1; d >= 0; d--) {
        if (d >= K_BEAT_MAX) continue;
        if (!k_survives(m, &k_beat_stack[d])) {
            k_viewreg_push(d, m);
            return;
        }
    }
}

/* An accumulator whose storage was pushed outside the arena had, until now,
   nobody to release it. The growth path frees each buffer it outgrows, so a
   single list costs nothing; what leaked was the LAST buffer of every escaped
   list, once per list. A loop building one list per iteration therefore leaked
   linearly — 200 lists, 200 final buffers, and vse grew at 355 MB/s.

   The owner's FIELD is registered rather than the buffer, the same choice the
   view registry made and for the same two reasons: a buffer that is outgrown
   is followed with no fixup, so the growth path keeps its free untouched, and
   freeing through the field nulls it, so a field registered once per growth is
   freed once.

   The depth is the innermost mark the owning header does not survive — the
   beat whose rewind reclaims the header itself, which is exactly when nothing
   can reach the storage any more. A header that predates every live mark
   belongs to no beat and finds no depth; it is left alone, and leaks as it
   always did. */
static KValue*** k_permreg[K_BEAT_MAX];
static long long k_permreg_n[K_BEAT_MAX];
static long long k_permreg_cap[K_BEAT_MAX];
/* Every rewind and every pop asks whether there is anything here, and for most
   programs the answer is no forever — a decode registers nothing at all and
   still pops 632,550 times. Two tests rather than one: `any` is a single hot
   int that stays set once a program has registered anything, so a program that
   never does pays a register-resident branch and never reads the per-depth
   array at all; `n[d]` is the second, for programs that do. Splitting the call
   away from its empty test is what the view registry's migrate does, and this
   is that measured one step further — the first shape cost 0.68% on a
   benchmark whose perm_peak_bytes is zero. */
static int k_permreg_any = 0;
static void k_permreg_flush_held(int d);

static inline __attribute__((always_inline)) void k_permreg_flush(int d) {
    if (!k_permreg_any || k_permreg_n[d] == 0) return;
    k_reg_any_at(d) &= ~K_REG_PERM;
    k_permreg_flush_held(d);
}

static void k_permreg_migrate_held(int d);

static inline __attribute__((always_inline)) void k_permreg_migrate(int d) {
    if (!k_permreg_any || d < 0 || d >= K_BEAT_MAX || k_permreg_n[d] == 0) return;
    k_reg_any_at(d) &= ~K_REG_PERM;
    k_permreg_migrate_held(d);
}

static void k_permreg_push(int d, KValue** slot) {
    if (k_permreg_n[d] == k_permreg_cap[d]) {
        long long grown = k_permreg_cap[d] ? k_permreg_cap[d] * 2 : 64;
        KValue*** wider = realloc(k_permreg[d], sizeof(KValue**) * (size_t)grown);
        if (!wider) { fputs("out of memory\n", stderr); exit(1); }
        k_permreg[d] = wider;
        k_permreg_cap[d] = grown;
    }
    k_permreg_any = 1;
    k_permreg[d][k_permreg_n[d]++] = slot;
    k_reg_any_at(d) |= K_REG_PERM;
}

static void k_permreg_migrate_held(int d) {
    if (d > 0) {
        for (long long i = 0; i < k_permreg_n[d]; i++) k_permreg_push(d - 1, k_permreg[d][i]);
    }
    k_permreg_n[d] = 0;
}

static KBuf* k_chunkreg[K_BEAT_MAX][256];
static int k_chunkreg_n[K_BEAT_MAX];
static long long k_chunkreg_spill[K_BEAT_MAX];

static void k_chunkreg_flush(int d) {
    for (int i = 0; i < k_chunkreg_n[d]; i++) {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
            if (K_COUNTING) k_stat_bytes_freed++;
            k_stat_held_live -= (long long)sizeof(KBuf) + k_buf_cap(k_chunkreg[d][i]);
        }
        free(k_chunkreg[d][i]);
    }
    k_chunkreg_n[d] = 0;
    k_chunkreg_spill[d] = 0;
    k_reg_any_at(d) &= ~K_REG_CHUNK;
}

/* A pop that keeps its region alive keeps the region's strings alive, so
   the frame's chunks now die with the enclosing frame instead: hand them
   up. With no enclosing frame they live until exit, as they always did. */
static void k_chunkreg_migrate(int d) {
    if (d < 0 || d >= K_BEAT_MAX) return;
    if (d > 0) {
        int up = d - 1;
        for (int i = 0; i < k_chunkreg_n[d]; i++) {
            if (k_chunkreg_n[up] < 256) {
                k_chunkreg[up][k_chunkreg_n[up]++] = k_chunkreg[d][i];
            } else {
                k_chunkreg_spill[up]++;
            }
            k_reg_any_at(up) |= K_REG_CHUNK;
        }
    }
    k_chunkreg_n[d] = 0;
    k_chunkreg_spill[d] = 0;
    k_reg_any_at(d) &= ~K_REG_CHUNK;
}

static long long k_pend_n;
static void k_pend_drop_above(KMark* m);
void k_beat_rewind_slow(KMark* m) {
    if (m == &k_beat_none) return;
    if (k_pend_n) k_pend_drop_above(m);
    k_buf_flush();
    long long d = m - k_beat_stack;
    if (d >= 0 && d < K_BEAT_MAX) {
        k_chunkreg_flush((int)d);
        k_viewreg_flush((int)d);
        k_permreg_flush((int)d);
        k_reg_any_at(d) = 0;
    }
    while (k_blocks != m->block) {
        KBlock* b = k_blocks;
        k_blocks = b->next;
        k_bidx_unlink(b);
        if (b->host) {
            /* a tail goes back to the block it was cut from */
            b->host->cap += sizeof(KBlock) + b->cap;
            continue;
        }
        k_live_block_bytes -= (long long)b->cap;
        b->next = k_spare;
        k_spare = b;
        if (K_COUNTING) k_spare_bytes += (long long)b->cap;
        if (K_COUNTING && k_spare_bytes > k_stat_spare_peak) k_stat_spare_peak = k_spare_bytes;
    }
    k_arena = m->ptr;
    k_arena_left = m->left;
    /* Every address above the mark is free to be handed out again, so the
       string the seek cursor names may not be there any more. One store. */
    k_seek_str = NULL;
}

/* What a rewind does in the common case is write two words, and a third when
   the seek cursor's string could be handed back. Everything else
   above — the buffer shelf, the three registries, retiring blocks — is
   conditional on state most programs never reach, and the call that finds all
   of it empty still pays the six callee-saved pushes those loops need. So the
   empty test moves out to the call site and the frame stays behind, which is
   the same split the view registry's migrate and the permanent registry's
   flush already make one level down. It is worth more here: a beat loop
   rewinds once per iteration, where those are once per pop.

   Measured before the split, on three programs whose beat loops dominate:
   escapebench spent 80,970,118 of 185,475,445 instructions in this function,
   43.66%, across 1,206,001 rewinds — 67 instructions each, every one of them
   over an empty shelf and three empty registries. basket spent 16.21%. A
   200,000-element fused map/select/sum spent 32.92%.

   No counter moves: the three flush loops and the block retire are the only
   code here that touches a statistic, and the fast path is taken exactly when
   all four would do nothing, so every cost golden stays byte-identical.

   How much of that the corpus can actually see was measured rather than
   assumed, by dropping each test in turn. Three are pinned: without the buffer
   shelf's test five counter gates segfault, because a shelf entry that
   outlives its arena region hands a freed pointer to the next grower; without
   the chunk registry's, encode's counters move; without the block test,
   encode's and scan's do. The other three are not, and the reason is one fact:
   on every program in the corpus, a rewind that finds a registry non-empty is
   also a rewind that took a new block, so the block test reaches the slow path
   first and the registry tests never decide anything. Counted over the eleven
   benchmarks, chunkreg_spill is non-zero at no rewind at all; escapebench has
   3,000 rewinds with a non-empty permanent registry and the same 3,000 have
   moved on from their mark's block; a fixture written to isolate the view
   registry got 500 and 500. That the two travel together is a property of the
   programs, not of the code, so the tests stay. */
static inline void k_beat_rewind(KMark* m) {
    /* ONE test for the shelf and the registries together. Both are zero on
       essentially every rewind -- the comment above counts how rarely either
       decides anything -- so the two branches were two predicted-taken jumps
       where an `or` of two zero words is one. The bounds check the fast path
       used to carry went with the array: every caller derives `m` from a
       depth it has already ranged, and `k_beat_rewind_slow` keeps its own. */
    if (__builtin_expect(!(k_buf_dirty | m->reg_any), 1)) {
        /* Nothing was allocated since the mark: the same pointer means the
           same room left, so the two stores below would write back what is
           already there, and no string can lie above a mark nothing was
           allocated past, so the cursor has nothing to forget. A loop that
           only pushes into a list it owns takes this exit every iteration;
           escapebench's 1.55 million did. The pointer is asked before the
           block because it answers for both: blocks never overlap, so a
           pointer equal to the mark's lies in the mark's block, and a loop
           that allocated nothing no longer loads the chain's head to be
           told so. */
        if (__builtin_expect(k_arena == m->ptr, 1)) return;
    }
    if (__builtin_expect(!(k_buf_dirty | m->reg_any) && k_blocks == m->block, 1)) {
        /* The cursor names a string by its header's address, and the rewind
           hands back every address from the mark up. A string under this
           mark is not handed back, so a scan over a string that arrived from
           outside the loop keeps its place; forgetting it there made every
           position walk from the front, which is quadratic in the subject.
           `k_seek_note` recorded which marks the string lies under. */
        if (m < k_seek_under)
            k_seek_str = NULL;
        k_arena = m->ptr;
        k_arena_left = m->left;
        return;
    }
    k_beat_rewind_slow(m);
}

void k_carry_clear(int depth);
void k_beat_push_deep(void);


/* Inlined into every loop that rewinds, through LTO. It was a call before,
   because the loops that make it most -- the run program's tally is one --
   are large enough to spend the inliner's budget first. When the rewind grew
   a test for the seek cursor, the run program paid 13,453,975 instructions
   for it, five an iteration; taking the call away gives back 7,298,706 of
   them. */
/* K_HOT_ELSEWHERE: a release build compiles this file as machine code and
   defines this function and the two byte scans in a small bitcode unit of
   their own, which the LTO link inlines into the program. `hot_source` in
   src/main.rs builds that unit from this file's own text, so these three and
   everything they touch stay here, with the globals they read given external
   linkage for its sake. */
#ifndef K_HOT_ELSEWHERE
__attribute__((always_inline)) void k_beat_iter(void) {
    if (K_COUNTING) k_stat_beat_iters++;
    KMark* m = k_beat_top;
    if (K_COUNTING && m != ((k_beat_depth > 0 && k_beat_depth <= K_BEAT_MAX)
                            ? &k_beat_stack[k_beat_depth - 1] : &k_beat_none)) {
        k_die("the cached beat top and the beat depth disagree");
    }
    k_beat_rewind(m);
}
#else
void k_beat_iter(void);
#endif

static long long k_ptr(void* p);

/* A rewind with bignums among the loop's arguments, plain or carried. A
   bignum the iteration made sits above the mark the rewind returns to, so
   before it each one moves to malloc'd storage, and after it each moves back
   into the arena, above the mark again, and the copy is freed. Only an
   argument that is a bignum when the loop reaches its edge is moved, and the
   emitter calls these only then. */
void k_big_lift_out(KValue* vs, long long n) {
    for (long long i = 0; i < n; i++) {
        if (vs[i].tag != K_BIG) continue;
        KBytes* b = (KBytes*)(intptr_t)vs[i].payload;
        KBytes* r = malloc(sizeof(KBytes) + (size_t)b->len);
        if (!r) { fputs("out of memory\n", stderr); exit(1); }
        r->data = (const unsigned char*)(r + 1);
        r->cap = 0;
        r->len = b->len;
        memcpy((unsigned char*)(r + 1), b->data, (size_t)b->len);
        vs[i].payload = k_ptr(r);
    }
}

/* The common edge lifts one argument, and takes it and hands it back by
   value rather than through a slot the caller writes and reads. */
KValue k_big_lift_out1(KValue v) {
    k_big_lift_out(&v, 1);
    return v;
}

KValue k_big_lift_in1(KValue v);

void k_big_lift_in(KValue* vs, long long n) {
    for (long long i = 0; i < n; i++) {
        if (vs[i].tag != K_BIG) continue;
        KBytes* h = (KBytes*)(intptr_t)vs[i].payload;
        KBytes* r = k_alloc(sizeof(KBytes) + (size_t)h->len);
        r->data = (const unsigned char*)(r + 1);
        r->cap = 0;
        r->len = h->len;
        memcpy((unsigned char*)(r + 1), h->data, (size_t)h->len);
        free(h);
        vs[i].payload = k_ptr(r);
    }
}

KValue k_big_lift_in1(KValue v) {
    k_big_lift_in(&v, 1);
    return v;
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
/* at_arena and at_blocks record where the arena stood at this depth's last
   stage, so a chain step can tell how far its region has drifted since;
   they were one pair of globals shared by every depth until 2026-09-07, and
   an inner chain read the outer chain's last stage as its own.

   skipped says a lap went by without staging since the last stage did. The
   pop reads it: see k_beat_pop_slow. */
typedef struct {
    KCarryBuf from; KCarryBuf to; int used_flag;
    const char* at_arena; const void* at_blocks; int skipped;
} KCarry;
KCarry k_carries[K_BEAT_MAX];

/* A beat entered past the stack's last mark keeps count and nothing else. */
void k_beat_push_deep(void) {
    k_beat_set_depth(k_beat_depth + 1);
}

/* A mark whose pointer and remaining count no longer meet the end of its block
   hands out memory past that end, and the damage surfaces later in an
   unrelated allocation — as a glibc abort on linux, and as nothing at all on
   macOS. The check used to sit in the rewind, where its comment called it one
   comparison; on x86-64 it is eight instructions, and a beat loop pays them
   once an iteration. It reads only the mark's own two words against its own
   block, and a mark is written here and never again, so asking at the rewind
   asks a question already answered: the same bad marks are caught, once per
   loop entry instead of once per iteration, and a mark broken before the push
   is now reported at the push.

   K_HOT_ELSEWHERE: a release build defines this in the hot unit, beside
   k_beat_iter, so the encoder's loops take their mark without a call. The
   over-deep case stays here as a call of its own. */
#ifndef K_HOT_ELSEWHERE
__attribute__((always_inline)) void k_beat_push(void) {
    if (k_beat_depth < K_BEAT_MAX) {
        KMark* m = &k_beat_stack[k_beat_depth];
        m->block = k_blocks;
        m->ptr = k_arena;
        m->left = k_arena_left;
        m->bytes = k_live_block_bytes;
        if (k_blocks && m->ptr + m->left != (char*)(k_blocks + 1) + k_blocks->cap) {
            k_die("a beat mark and the arena disagree about the room that is left");
        }
        k_carries[k_beat_depth].used_flag = 0;
        k_carries[k_beat_depth].skipped = 0;
        k_carries[k_beat_depth].from.used = 0;
        k_carries[k_beat_depth].to.used = 0;
        /* In range by the test above, so the new top is the mark just written
           and the general setter's range test would be dead code. */
        k_beat_depth++;
        k_beat_top = m;
        return;
    }
    k_beat_push_deep();
}
#else
void k_beat_push(void);
#endif

static KValue k_carry_slots[K_CARRY_MAX];
/* Slots the compiler proved hold a string builder this cycle owns. The copy
   below strips a positive cap on purpose — the copy owns no room — so an
   accumulator that crossed a beat used to arrive roomless and be re-seeded,
   which copies the whole string once a lap. A kept slot crosses by identity
   instead. Only a slot `builder_params` names is kept, so nothing that merely
   happens to have capacity is aliased. */
static unsigned char k_carry_kept[K_CARRY_MAX];
static long long k_carry_n = 0;

/* Tenured carry storage. A carried value is deep-copied into the pair above on
   every iteration, and the copy lands in malloc'd memory that k_survives — which
   knows only about arena blocks — answers no for, so the next iteration copies
   it again. A document threaded through a loop is therefore copied once per
   iteration, which is quadratic in its length.

   The discrimination that fixes it is survival, not size. A value found inside
   the previous iteration's buffer has already lived a whole lap, so it is
   promoted here, where nothing overwrites it, and every later iteration shares
   it. A value the loop built fresh this lap is not there and goes to the pair
   as before — which is what keeps a loop that builds a megabyte per iteration
   from accumulating megabytes, the shape that produced the 83 GB runaway.

   Blocks are appended and never realloc'd, so growth cannot move what they
   hold. Membership is a hash set rather than a walk of the block list: the
   walk would be linear in the blocks and asked once per node, which is the
   quadratic again by another road. Both are released at the pop, after the
   result has been copied out into the caller's arena. */


/* Does p survive the innermost rewind — is it inside the live chain at or
   below the mark? mark == NULL means "the whole live chain", the test the
   pop's copy-out wants. */
/* Tenured storage lies below every mark by construction — it is malloc'd, and
   a rewind cannot reach it. Saying so here rather than at the two walks is
   what keeps the repair path working: a tenured node whose field was later
   made to point at fresh arena still fails k_interior_survives and is
   repaired, where an early return would have shared a node holding a pointer
   the next rewind frees.

   A null mark means the whole live chain, which is the copy-out at the pop.
   Tenured storage is released immediately after that walk, so there it must
   be copied, and the test is skipped. */
static int k_ten_any = 0;
static int k_ten_holds(const void* p, KMark* m);

static int k_survives(const void* p, KMark* m) {
    const char* q = (const char*)p;
    KBlock* b = m ? m->block : k_blocks;
    const char* frontier = m ? m->ptr : k_arena;
    if (__builtin_expect(k_chain_long(), 0) && b) {
        const char* start = (const char*)(b + 1);
        if (q >= start && q < frontier) return 1;
        KBlock* at = k_bidx_find(q);
        return at && at != b && at->pad < b->pad;
    }
    for (; b; b = b->next) {
        const char* start = (const char*)(b + 1);
        const char* end = (b == (m ? m->block : k_blocks)) ? frontier : start + b->cap;
        if (q >= start && q < end) return 1;
        frontier = NULL;
    }
    return 0;
}

/* Is this address still in the live chain at all — the question an at-risk
   memo asks after some rewind may have happened. A NULL mark is the whole
   chain up to the bump pointer, which is exactly "not reclaimed". */
static int k_still_live(const void* p) {
    return k_survives(p, NULL);
}

/* Does this outlive the rewind, counting storage the carry tenured? Asked by
   the copy machinery and by k_born_this_beat, and by nothing else.
   k_survives itself stays the pure arena walk, because the mutation fast paths
   ask it per append and putting a second test inside it cost between 1.2 and
   5.5 per cent on four benchmarks that tenure nothing at all.
   The other callers are safe without it, and each for its own reason:
   k_memo_outlives and k_carry_stage_kept answer no and copy, which is extra
   work rather than wrong; k_outlives_beat answers no and takes the general
   path; k_b_append_mut's `dies` answers no and mallocs, which is what a header
   below the mark wants anyway. k_born_this_beat is the one that would be
   wrong — answering yes there licenses growing a tenured list in place, and it
   is shared. */
static int k_ten_holds(const void* p, KMark* m);

/* Is p in the arena above the mark -- storage the rewind reclaims? Such a
   pointer cannot be tenured, since tenure blocks are malloc'd, so the walk of
   the tenure blocks is skipped for the one kind of pointer the sizing walk
   asks about most: a node the loop built this lap. The blocks newer than the
   mark's are visited from the head of the chain, and the mark's own block
   from the mark's pointer to its end. */
static int k_above_mark(const void* p, KMark* m) {
    const char* q = (const char*)p;
    for (KBlock* b = k_blocks; b; b = b->next) {
        const char* start = (const char*)(b + 1);
        const char* end = start + b->cap;
        if (b == m->block) return q >= m->ptr && q < end;
        if (q >= start && q < end) return 1;
    }
    return 0;
}

/* Where a pointer stands against a mark, in one walk of the chain: below it
   (the arena keeps it), above it (the rewind takes it), or in no block at all
   (malloc'd, or a tenure block's). k_survives and k_above_mark each answered
   one of those questions with a walk of their own, and the sizing walk asked
   both of every node it sized, 330,000 times a run on runbench. */
enum { K_WHERE_OUTSIDE = 0, K_WHERE_BELOW = 1, K_WHERE_ABOVE = 2 };

static int k_ten_holds_outside(const void* p);

static int k_where(const void* p, KMark* m) {
    const char* q = (const char*)p;
    KBlock* mb = m->block;
    /* The mark's own block first: a node the loop built this lap sits above
       the mark in it, and a node the lap before left sits below, and those
       two are most of what the sizing walk asks about. Walking from the head
       reached that block only after every block newer than it, which a beat
       whose block is not the head paid on every ask: pendbench 182,000 asks
       a run, 3,037,237 instructions when the walk was reordered. A mark with
       no block -- the freeze's -- finds everything above, as before. */
    if (mb) {
        const char* start = (const char*)(mb + 1);
        if (q >= start && q < start + mb->cap)
            return q < m->ptr ? K_WHERE_BELOW : K_WHERE_ABOVE;
    }
    if (__builtin_expect(k_chain_long(), 0)) {
        KBlock* at = k_bidx_find(q);
        if (!at) return K_WHERE_OUTSIDE;
        return mb && at->pad < mb->pad ? K_WHERE_BELOW : K_WHERE_ABOVE;
    }
    for (KBlock* b = k_blocks; b != mb; b = b->next) {
        const char* start = (const char*)(b + 1);
        if (q >= start && q < start + b->cap) return K_WHERE_ABOVE;
    }
    if (mb)
        for (KBlock* b = mb->next; b; b = b->next) {
            const char* start = (const char*)(b + 1);
            if (q >= start && q < start + b->cap) return K_WHERE_BELOW;
        }
    return K_WHERE_OUTSIDE;
}

/* The buffers frozen constants live in. A frozen value is immortal, so a
   walk that meets one has nothing to copy and nothing to size: without this
   every beat that carried a reference to a constant copied the constant into
   its carry buffer, and a constant frozen out of another constant's value was
   copied a second time, which broke the identity a knot's cycle returns to
   -- `x = [x]` reached through `err x` rendered as `[[<cycle>]]` on native
   where the interpreter, holding one list, says `[<cycle>]`. Registered by
   k_caf_freeze; a handful of ranges, walked only for a pointer in no arena
   block and no tenure block. */
static const char** k_frozen_lo = NULL;
static const char** k_frozen_hi = NULL;
static int k_frozen_n = 0;
static int k_frozen_cap = 0;

static void k_frozen_note(const char* lo, size_t n) {
    if (k_frozen_n == k_frozen_cap) {
        k_frozen_cap = k_frozen_cap ? 2 * k_frozen_cap : 16;
        k_frozen_lo = realloc(k_frozen_lo, sizeof(const char*) * (size_t)k_frozen_cap);
        k_frozen_hi = realloc(k_frozen_hi, sizeof(const char*) * (size_t)k_frozen_cap);
        if (!k_frozen_lo || !k_frozen_hi) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_frozen_lo[k_frozen_n] = lo;
    k_frozen_hi[k_frozen_n] = lo + n;
    k_frozen_n++;
}

/* The short tokens `k_b_utf8_slice_raw` shares, stored where no rewind
   reaches and asked about by address: a walk that meets one has nothing to
   copy. See `k_token_miss`. */
#define K_TOKEN_BITS 12
typedef struct { KStr s; char data[8]; } KToken;
KToken k_token_store[1 << K_TOKEN_BITS];
/* Whether a filled slot's bytes hold no quote, no backslash and nothing below
   a space, which is everything the JSON writer escapes. Decided once, when the
   slot fills, so `k_b_find2_below_raw` can answer "not found" for a shared
   token without scanning it. Both arrays have external linkage because the
   hot unit reads them; see `hot_source` in main.rs. */
unsigned char k_token_clean[1 << K_TOKEN_BITS];
static inline int k_token_holds(const void* p) {
    return (uintptr_t)((const char*)p - (const char*)k_token_store) < sizeof(k_token_store);
}

static __attribute__((noinline)) int k_frozen_holds(const void* p) {
    const char* q = (const char*)p;
    for (int i = 0; i < k_frozen_n; i++)
        if (q >= k_frozen_lo[i] && q < k_frozen_hi[i]) return 1;
    return 0;
}

/* The chunks k_alloc_const hands out from: a subset of the frozen ranges,
   kept apart because what lies in them is never written again. */
static const char** k_const_lo = NULL;
static const char** k_const_hi = NULL;
static int k_const_n = 0;
static int k_const_cap = 0;

static __attribute__((noinline)) int k_const_holds(const void* p) {
    const char* q = (const char*)p;
    for (int i = 0; i < k_const_n; i++)
        if (q >= k_const_lo[i] && q < k_const_hi[i]) return 1;
    return 0;
}

static int k_survives_x(const void* p, KMark* m) {
    if (!m) return k_survives(p, NULL) || k_token_holds(p) || (k_frozen_n && k_frozen_holds(p));
    int w = k_where(p, m);
    if (w == K_WHERE_BELOW) return 1;
    if (w != K_WHERE_OUTSIDE) return 0;
    return k_token_holds(p) || (k_ten_any && k_ten_holds_outside(p)) ||
           (k_frozen_n && k_frozen_holds(p));
}

/* Sorted-view caches filled during a beat point above the mark; a rewind
   frees the view while the map header — below the mark, inside data the
   loop legitimately threads — survives holding the stale pointer. Fills
   register here, and every rewind resets the caches it just freed. */
typedef struct { const void* key; void* val; unsigned long long gen; } KPtrSlot;
typedef struct { KPtrSlot* slots; size_t cap; unsigned long long gen; } KPtrMap;

/* A rewind frees only above its mark, so a list whose slots all lie below the
   OUTERMOST mark outlives every rewind and its verdict never changes. That is
   the only verdict worth keeping: keying a cache by a mark's address or by its
   arena position both hand back an answer computed for a different boundary,
   because stack slots are reused and a rewound arena returns to the position
   it started from. Lists only — in-place list growth demands
   k_born_this_beat, which puts the list above the mark where the scan never
   runs, while k_b_put_mut grows a map in place with no such guard. */
/* Below this the walk is cheaper than the hash that would spare it. The memo
   pays when one list is asked about again and again, which is the streaming
   shape: the carry holds the whole remaining document and every element asks
   the same question of it. A loop over many small lists asks a different
   question each time, so every lookup is a miss and the map is pure cost —
   measured at 39% of a simulator that builds one short list per iteration. */
#define K_ISV_MIN 64
static KPtrMap k_isv_list;
static size_t k_isv_live;
static const char* k_isv_base;

static KPtrMap k_copy_seen;   /* size pass: key visited, val unused */
static KPtrMap k_copy_map;    /* copy pass: old node -> its one copy */

static void k_ptrmap_begin(KPtrMap* t) {
    if (!t->slots) {
        t->cap = 1024;
        t->slots = calloc(t->cap, sizeof(KPtrSlot));
        if (!t->slots) { fputs("out of memory\n", stderr); exit(1); }
    }
    t->gen++;
}

/* Inlined at its six sites: the sizing walk asks the seen-map once a node,
   340,000 times on the run program, and the call around a hash, a mask and
   a compare was a third of the ask. */
static inline __attribute__((always_inline))
size_t k_ptrmap_probe(KPtrMap* t, const void* key) {
    size_t i = ((uintptr_t)key >> 4) * 0x9E3779B97F4A7C15ULL & (t->cap - 1);
    while (t->slots[i].gen == t->gen && t->slots[i].key != key)
        i = (i + 1) & (t->cap - 1);
    return i;
}

static void k_ptrmap_grow(KPtrMap* t) {
    KPtrSlot* old = t->slots;
    size_t old_cap = t->cap;
    t->cap *= 2;
    t->slots = calloc(t->cap, sizeof(KPtrSlot));
    if (!t->slots) { fputs("out of memory\n", stderr); exit(1); }
    for (size_t i = 0; i < old_cap; i++) {
        if (old[i].gen != t->gen) continue;
        size_t j = k_ptrmap_probe(t, old[i].key);
        t->slots[j] = old[i];
    }
    free(old);
}

static inline __attribute__((always_inline))
KPtrSlot* k_ptrmap_at(KPtrMap* t, const void* key, size_t* live) {
    if (*live * 10 >= t->cap * 7) { k_ptrmap_grow(t); }
    size_t i = k_ptrmap_probe(t, key);
    return &t->slots[i];
}

typedef struct KTenBlock { struct KTenBlock* next; size_t cap; size_t used; char* data; } KTenBlock;
static KTenBlock* k_ten_blocks[K_BEAT_MAX];
/* One bit per depth that holds a block. */
static unsigned long long k_ten_mask = 0;
static size_t k_ten_bytes[K_BEAT_MAX];
/* Past this a program stops tenuring and behaves as it did before, so no loop
   can grow without bound on storage that is only freed at the pop. */
#define K_TEN_CAP (64u << 20)
/* The first block, and the base the doubling below starts from. 64 KiB left
   the wide-array benchmark needing a second block for 128 KiB of tenured
   storage, so every membership miss walked two; 256 KiB holds it in one and
   takes k_ten_holds from 37.7 instructions an ask to 30.4, with the call count
   unchanged. What it costs is address space rather than pages — malloc mmaps
   a block this size and untouched pages never become resident — and the
   measured peak is 80 KiB on widebench. */
#define K_TEN_BLOCK (256u << 10)
static int k_ten_on = 0;

/* Is this pointer tenured, at this beat depth or any outer one? A value
   promoted by an outer loop is outside the arena for an inner one too.

   A walk of the blocks, which is only affordable because each block doubles
   the one before it: 64 MiB of tenured storage is eleven blocks, not the
   thousand that a fixed 64 KiB would make, and K_TEN_CAP means there is never
   a twelfth. That bound is what pays for deleting the hash set this used to
   be — a set that cost a probe here AND an insert on every single tenured
   allocation, where the walk costs neither.

   No cache in front of it. A one-entry memo of the block that answered last
   was built and measured: it takes 16,080 of widebench's 96,496 asks, and it
   costs the other 80,416 a load and two compares before they walk two blocks
   anyway. That is 634,925 instructions spent to save 16,080 short walks, so
   the memo went. */
static __attribute__((noinline)) int k_ten_holds(const void* p, KMark* m) {
    if (k_above_mark(p, m)) return 0;
    return k_ten_holds_outside(p);
}

/* The tenure half of k_ten_holds, for a caller that already knows the
   pointer is in no arena block. */
static __attribute__((noinline)) int k_ten_holds_outside(const void* p) {
    const char* q = (const char*)p;
    unsigned long long held = k_ten_mask;
    if (k_beat_depth < K_BEAT_MAX) held &= (1ull << k_beat_depth) - 1;
    for (; held; held &= held - 1)
        for (KTenBlock* b = k_ten_blocks[__builtin_ctzll(held)]; b; b = b->next)
            if (q >= b->data && q < b->data + b->used) return 1;
    return 0;
}

/* Is this pointer inside the buffer the previous iteration copied into? That
   is the whole test for "has lived a lap". */
/* The range the previous iteration copied into, hoisted. Everything this test
   needs but the pointer is the same for every node of one evacuation — the
   depth, the cap, the buffer — and the walk asks it per node, so the loop reads
   two globals and compares rather than indexing two arrays and dereferencing.
   Both are zero whenever tenuring is off, which is every program but the
   wide-array one. */
static const char* k_from_lo = NULL;
static const char* k_from_hi = NULL;

static int k_carry_holds(const void* p) {
    const char* q = (const char*)p;
    return q >= k_from_lo && q < k_from_hi;
}

/* A carry pointer can reach an arena node without the copy machinery ever
   seeing it: the program writes one in place, through `set`, a map insert or
   a list push, into a node that was already there. Those writes are the only
   such path, and the pop's copy-out cannot find what they leave behind by
   walking the result -- it prunes at a survivor whose immediate interior
   survives, and the pointer sits one level deeper. So that copy-out stopped
   pruning, and pruning is worth 4.97% of deepbench.

   The writes can say instead. `k_carry_written` latches the first time one of
   them stores a pointer into a live carry buffer, and the copy-out prunes
   while it is clear: before that write there is nothing for the deep walk to
   find, and after it the walk is exactly what shipped. The latch never
   clears, so this only ever turns the walk ON later than it was on before.

   Asking has to be nearly free, because seven of the hottest paths in the
   runtime ask it. One unsigned compare against a span answers three questions
   at once: a span of zero means no carry buffer is live OR the latch is
   already set, and a pointer outside the span is outside every buffer. What
   the span admits goes to `k_note_exact`, which walks the live pairs; the
   span is a bounding box and holds whatever the allocator put between them.
   Measured with the exact question asked, the benchmark corpus latches
   nowhere and the trend gate -- the program whose crash found this -- latches
   on 1,634 writes. */
static uintptr_t k_cbox_base = 0;
static size_t k_cbox_span = 0;
static int k_carry_written = 0;

/* The box is over the buffers that are LIVE, not over every one ever
   malloc'd, and it is recomputed at the two places the set changes: the
   sizing malloc and the cohort free. */
static void k_cbox_refresh(void) {
    const char* lo = NULL;
    const char* hi = NULL;
    for (long long d = 0; d < K_BEAT_MAX; d++) {
        KCarry* c = &k_carries[d];
        KCarryBuf* bufs[2] = { &c->from, &c->to };
        for (int i = 0; i < 2; i++) {
            if (!bufs[i]->data) continue;
            if (!lo || bufs[i]->data < lo) lo = bufs[i]->data;
            if (bufs[i]->data + bufs[i]->cap > hi) hi = bufs[i]->data + bufs[i]->cap;
        }
    }
    k_cbox_base = (uintptr_t)lo;
    k_cbox_span = k_carry_written ? 0 : (size_t)(hi - lo);
}

static void k_note_exact(KValue written) {
    const char* q = (const char*)(intptr_t)written.payload;
    for (long long d = 0; d < K_BEAT_MAX; d++) {
        KCarryBuf* f = &k_carries[d].from;
        KCarryBuf* t = &k_carries[d].to;
        if ((f->data && q >= f->data && q < f->data + f->cap)
            || (t->data && q >= t->data && q < t->data + t->cap)) {
            k_carry_written = 1;
            k_cbox_span = 0;
            return;
        }
    }
}

static inline __attribute__((always_inline))
void k_note_if_carried(KValue written) {
    if (!k_is_heap(written.tag)) return;
    if ((uintptr_t)written.payload - k_cbox_base < k_cbox_span)
        k_note_exact(written);
}

static void k_from_window(int on) {
    k_from_lo = NULL;
    k_from_hi = NULL;
    if (!on || k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return;
    if (k_ten_bytes[k_beat_depth - 1] > K_TEN_CAP) return;
    KCarryBuf* b = &k_carries[k_beat_depth - 1].from;
    if (!b->data) return;
    k_from_lo = b->data;
    k_from_hi = b->data + b->cap;
}

/* Each block is twice the one before it. That is what keeps the list short
   enough for k_ten_holds to walk: K_TEN_CAP is 64 MiB, so doubling from 64
   KiB runs out of licence after eleven blocks. The tail of the last block is
   address space rather than pages -- malloc mmaps anything this size and
   untouched pages are never resident -- so the peak this costs is not memory.
   Opened out of line and preserve_most: a block opens fourteen times on the
   run program against 74,551 carves, and the carve's callers kept a frame
   for the two mallocs. */
static __attribute__((noinline, cold, preserve_most)) KTenBlock* k_ten_block_open(long long d, KTenBlock* b, size_t n) {
    size_t grow = b ? b->cap * 2 : (size_t)K_TEN_BLOCK;
    size_t cap = n > grow ? n : grow;
    KTenBlock* nb = malloc(sizeof(KTenBlock));
    if (!nb) { fputs("out of memory\n", stderr); exit(1); }
    nb->data = malloc(cap);
    if (!nb->data) { fputs("out of memory\n", stderr); exit(1); }
    nb->cap = cap;
    nb->used = 0;
    nb->next = k_ten_blocks[d];
    k_ten_blocks[d] = nb;
    k_ten_mask |= 1ull << d;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_ten_blocks++;
    return nb;
}

static void* k_ten_alloc(size_t n) {
    long long d = k_beat_depth - 1;
    KTenBlock* b = k_ten_blocks[d];
    if (!b && d > 0) {
        /* A beat that has no block yet opens its tenure in the block the
           depth outside already holds, when that block has room. An inner
           loop whose result is heap hands its block up at every pop, and
           runbench's did forty-nine times a phase: forty-nine 256 KiB blocks
           in the outer depth for 78 KB of tenured bytes, every one walked by
           every ask. Bytes carved here are accounted to the outer depth and
           freed with it, which is where a handed-up block's bytes went
           anyway; the licence check is the outer depth's too. */
        KTenBlock* pb = k_ten_blocks[d - 1];
        if (pb && pb->cap - pb->used >= n && k_ten_bytes[d - 1] <= K_TEN_CAP) {
            void* out = pb->data + pb->used;
            pb->used += n;
            k_ten_bytes[d - 1] += n;
            k_ten_any = 1;
            return out;
        }
    }
    if (!b || b->cap - b->used < n) b = k_ten_block_open(d, b, n);
    void* out = b->data + b->used;
    b->used += n;
    k_ten_bytes[d] += n;
    k_ten_any = 1;
    return out;
}

/* A tenure block holds copies the evacuation made because the value had lived
   a lap, and `k_survives_x` answers yes for a pointer into one. That answer is
   what lets the walk prune: a survivor whose immediate interior survives is
   shared rather than copied, and the walk stops there. For the arena the prune
   is sound, because arena allocation is monotonic and a survivor can only point
   at storage older than itself. Tenure storage is younger than the survivors
   that come to hold it, so the prune hides it — an arena record can carry a
   tenured string with no arena pointer anywhere on the path to say so.
   `k_beat_pop` then freed the block while that record was still reachable, and
   the read segfaulted. The trend gate hit it whenever a counter in the first
   golden it reads had moved: two frames of `k_copy_size` and a KStr in a
   munmap'd page.

   A heap result keeps the region alive — `k_beat_pop` does not rewind for one —
   so the blocks a survivor may still point into are handed to the depth outside
   rather than freed. They go at the first pop that does rewind, which is where
   everything the beat allocated goes back. `k_ten_bytes` travels with them, so
   `K_TEN_CAP` still bounds what one depth may hold.

   One case is narrower rather than closed: a node BELOW that mark, repaired to
   hold a tenured pointer, outlives the rewind that frees the block. The place
   to close it is `k_repaired_settle`, which exists to move repaired slots into
   the arena and does not move the ones that landed in tenure, because
   `k_survives_x` answers yes for those too.

   That is REACHED, and by exactly one program in the tree:
   `tests/golden/mem/a_repaired_node_below_the_mark_holds_tenure.kso`. Four
   earlier attempts missed it because the two halves are disjoint across the
   benchmark corpus — widebench, indexbench and scanbench tenure and never
   repair; encodebench and pendbench repair and never tenure — so no shape
   already in the tree does both. Building the carried list inside the bind
   and the accumulator outside it does.

   Reaching it is harmless, and until 2026-09-22 this comment said the hand up
   was why: the beat's result is a heap value, so the block goes to the depth
   outside instead of being freed and outlives every read of the repaired node.
   The carry's deep copy is what makes it harmless. `k_beat_pop_slow` runs the
   copy first and hands up after, and poisoning the depth's live tenure bytes
   at three points one step apart says which step matters — before the copy the
   fixture dies reading a length out of 0xAB, after the copy and before the
   three migrates it is correct, and at the hand up it is correct. The copy
   reads the tenured bytes out; after it nothing below the mark points into
   tenure, which is why replacing the hand up with a release leaves this
   program clean under AddressSanitizer.

   The hand up is load-bearing elsewhere — an inner beat that shares the outer
   depth's block, which is a different fixture — and `ten_handups` is what
   makes the difference visible. `ten_blocks` and `ten_frees` cannot see it:
   the hand up moves only WHERE the free happens, so the block is claimed once
   and given back once either way. This comment's last two lines used to claim
   the fixture below turned red on a change that stopped handing them up. It
   did not, for a fortnight, and all sixty-seven .mem goldens agreed with it.
   `scripts/ratchet/mutations/a_tenure_block_freed_where_it_was_handed_up.sh`
   makes the claim a thing CI checks rather than a thing written here. */
static void k_ten_hand_up(long long d) {
    if (!k_ten_blocks[d] || d == 0) return;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_ten_handups++;
    KTenBlock* tail = k_ten_blocks[d];
    while (tail->next) tail = tail->next;
    tail->next = k_ten_blocks[d - 1];
    k_ten_blocks[d - 1] = k_ten_blocks[d];
    k_ten_bytes[d - 1] += k_ten_bytes[d];
    k_ten_blocks[d] = NULL;
    k_ten_bytes[d] = 0;
    k_ten_mask = (k_ten_mask & ~(1ull << d)) | (1ull << (d - 1));
}

static void k_ten_release(long long d) {
    KTenBlock* head = k_ten_blocks[d];
    /* `k_ten_any` summarises whether ANY of the sixty-four depths holds a
       block, and recomputing it walks all sixty-four. Only this depth can
       have changed here, so a depth that held nothing leaves the summary
       exactly as it was. Every beat pop used to pay the walk. */
    if (!head) return;
    for (KTenBlock* b = head; b; ) {
        KTenBlock* next = b->next;
        free(b->data);
        free(b);
        b = next;
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_ten_frees++;
    }
    k_ten_blocks[d] = NULL;
    k_ten_bytes[d] = 0;
    k_ten_mask &= ~(1ull << d);
    k_ten_any = k_ten_mask != 0;
}

static size_t k_copy_seen_live;
static size_t k_copy_map_live;

/* mark visited; returns 1 when already seen this generation */
static int k_copy_seen_check(const void* p) {
    KPtrSlot* s = k_ptrmap_at(&k_copy_seen, p, &k_copy_seen_live);
    if (s->gen == k_copy_seen.gen) return 1;
    s->gen = k_copy_seen.gen;
    s->key = p;
    s->val = NULL;
    k_copy_seen_live++;
    return 0;
}

/* deep: the walk may not prune at a survivor whose immediate interior
   survives. Sharing a whole subtree is sound only while nothing inside it
   points at storage about to be retired, and the copy-out at a carried pop
   retires the depth's carry pair. One level down is all k_interior_survives
   can see, so a node two levels down holding a carry pointer was left for
   the caller's next stage to repair -- which was the next chain step until
   a step could leave.
   Set at that one call site, and set there from `k_carry_written`: the only
   way such a pointer reaches such a node is an in-place write, and the seven
   write sites latch that flag when they make one. Until they do there is
   nothing below for the walk to find, and pruning is worth 4.97% of
   deepbench. */
typedef struct {
    KCarryBuf* buf; KMark* mark; int to_arena; int in_ten; int deep;
} KCopy;

static void* k_copy_alloc(KCopy* cp, size_t n) {
    n = (n + 15) & ~(size_t)15;
    /* What the copying strategy costs, at the one point every evacuated byte
       passes through. A refcounting runtime pays none of it and pays per
       reference instead, so this is the side of that trade beats can measure.
       The arena path is counted here and in allocs both: the two ask different
       questions, one what was allocated and one why. */
    if (K_COUNTING) k_stat_evac_bytes += (long long)n;
    if (K_COUNTING) k_stat_evac_allocs++;
    if (cp->to_arena) return k_alloc(n);
    if (cp->in_ten) return k_ten_alloc(n);
    void* p = cp->buf->data + cp->buf->used;
    cp->buf->used += n;
    return p;
}

static size_t k_copy_size(KValue v, KMark* m);

/* A node below the mark survives the rewind; the storage it points at does
   not necessarily, and sharing the node shares whatever is left of it. That
   happens whenever a value taken out of a carry buffer is built into a fresh
   node — the node lands in the arena, its field still points at the buffer,
   and the buffer is retired two carries later. So a node is only shareable
   when its own interior is a survivor too; otherwise it is copied, and the
   copy walks the field that left. Only the immediate interior is asked
   about, which keeps the walk pruning at everything that has not left. */
static int k_slots_survive(const KValue* slots, long long n, KMark* m) {
    for (long long i = 0; i < n; i++) {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_survive_slots++;
        /* the cell survives, but what it captured may not — only the copy
           walk can answer that, so a thunk slot is never shareable as-is */
        if (slots[i].tag == K_THUNK) return 0;
        if (!k_is_heap(slots[i].tag)) continue;
        if (!k_survives_x((const void*)(intptr_t)slots[i].payload, m)) return 0;
    }
    return 1;
}

static int k_interior_survives(KValue v, const void* p, KMark* m) {
    switch (v.tag) {
        /* A builder's storage is malloc'd precisely so a rewind cannot reach
           it, and a positive cap is what says so. Copying one would strip the
           cap and the next append would find a plain string where it left a
           builder. */
        case K_STR: {
            KStr* st = (KStr*)p;
            return st->cap > 0 || k_survives_x(st->data, m);
        }
        case K_BIG:
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            return k_bytes_malloced(b) || k_survives_x(b->data, m);
        }
        case K_LIST: {
            KList* l = (KList*)p;
            if (!k_survives_x(l->items, m)) return 0;
            if (k_beat_depth > 0 && l->len >= K_ISV_MIN) {
                KMark* outer = &k_beat_stack[0];
                /* The table starts empty and so does `k_isv_base`, and an
                   outermost mark taken before the arena held anything has a
                   NULL position too. Comparing the two alone left the table
                   unopened, and its first lookup grew zero slots to zero and
                   probed with a mask of all ones. */
                if (!k_isv_list.slots || k_isv_base != outer->ptr) {
                    k_ptrmap_begin(&k_isv_list);
                    k_isv_live = 0;
                    k_isv_base = outer->ptr;
                }
                KPtrSlot* memo = k_ptrmap_at(&k_isv_list, p, &k_isv_live);
                if (memo->gen == k_isv_list.gen && memo->key == p)
                    return memo->val != NULL ? 1 : k_slots_survive(l->items, l->len, m);
                int deep = k_survives_x(l->items, outer)
                        && k_slots_survive(l->items, l->len, outer);
                k_isv_live++;
                memo->gen = k_isv_list.gen;
                memo->key = p;
                memo->val = deep ? (void*)1 : NULL;
                if (deep) return 1;
            }
            return k_slots_survive(l->items, l->len, m);
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            return k_survives_x(mp->pairs, m) && k_slots_survive(mp->pairs, 2 * mp->len, m);
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            return k_survives_x(r->fields, m) && k_slots_survive(r->fields, r->nfields, m);
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            return k_survives_x(cl->env, m) && k_slots_survive((KValue*)cl->env, cl->ncaps, m);
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            KValue pair[2] = { d->x, d->y };
            return k_slots_survive(pair, 2, m);
        }
        default: return 1;
    }
}

/* Shareable: the node and its interior both outlive the rewind. */
static int k_shareable(KValue v, const void* p, KMark* m) {
    return k_survives_x(p, m) && k_interior_survives(v, p, m);
}

/* The cohort guard sizes a survivor only until the verdict is settled: a
   walk past the budget already means "keep", and the seen-map the walk
   feeds would otherwise grow with the whole survivor. Zero disables the
   budget (the carry's own full-size pass). */
static size_t k_copy_size_budget = 0;
static size_t k_copy_size_spent = 0;

static int k_size_in_ten = 0;

static size_t k_copy_size_ptr(const void* p, size_t n, KMark* m) {
    (void)p;
    size_t rounded = (n + 15) & ~(size_t)15;
    k_copy_size_spent += rounded;
    if (k_size_in_ten) return 0;
    return rounded;
}

static size_t k_repair_size(KValue v, const void* p, KMark* m);

/* `k_copy_size` returns zero for an immediate and for nothing else without
   looking at it, so a caller walking a container can skip the call entirely.
   deepbench folds over lists of ints, and the call it used to make per element
   existed only to return zero. */
static inline int k_worth_sizing(KValue v) {
    return k_is_heap(v.tag) || v.tag == K_THUNK;
}

static size_t k_copy_size(KValue v, KMark* m) {
    /* A thunk cell is malloc'd and survives every rewind, but what it holds
       — captured args, or a forced result — may live in the arena about to
       go. The cell itself is never copied; its slots are sized like any
       other survivor's interior. */
    if (v.tag == K_THUNK) {
        KThunk* t = (KThunk*)(intptr_t)v.payload;
        if (k_copy_seen_check(t)) return 0;
        size_t n = 0;
        if (t->forced) n += k_copy_size(t->result, m);
        for (int i = 0; i < t->argc; i++) n += k_copy_size(t->args[i], m);
        return n;
    }
    if (!k_is_heap(v.tag)) return 0;
    if (k_carry_holds((const void*)(intptr_t)v.payload)) k_size_in_ten = 1;
    if (k_copy_size_budget && k_copy_size_spent > k_copy_size_budget) return 0;
    const void* p = (const void*)(intptr_t)v.payload;
    if (k_survives_x(p, m)) {
        if (k_interior_survives(v, p, m)) return 0;
        if (k_const_n && k_const_holds(p)) return 0;
        if (k_copy_seen_check(p)) return 0;
        return k_repair_size(v, p, m);
    }
    if (k_copy_seen_check(p)) return 0;
    size_t n = 0;
    switch (v.tag) {
        case K_STR: {
            KStr* s = (KStr*)p;
            n += k_copy_size_ptr(s, sizeof(KStr), m);
            /* Storage that follows its header lies where the header does,
               and the header was just found not to survive; only a string
               whose bytes live elsewhere -- a slice's, a builder's -- needs
               the walk asked again of its data. */
            if (s->data == (char*)(s + 1) || !k_survives_x(s->data, m))
                n += k_copy_size_ptr(s->data, (size_t)s->len + 1, m);
            break;
        }
        case K_BIG:
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            n += k_copy_size_ptr(b, sizeof(KBytes), m);
            if (b->data == (const unsigned char*)(b + 1) || !k_survives_x(b->data, m))
                n += k_copy_size_ptr(b->data, (size_t)b->len, m);
            break;
        }
        case K_LIST: {
            KList* l = (KList*)p;
            n += k_copy_size_ptr(l, sizeof(KList), m);
            n += k_copy_size_ptr(l->items, sizeof(KBuf) + sizeof(KValue) * (size_t)(l->len ? l->len : 1), m);
            for (long long i = 0; i < l->len; i++)
                if (k_worth_sizing(l->items[i])) n += k_copy_size(l->items[i], m);
            break;
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            n += k_copy_size_ptr(mp, sizeof(KMap), m);
            n += k_copy_size_ptr(mp->pairs, sizeof(KBuf) + sizeof(KValue) * (size_t)(2 * (mp->len ? mp->len : 1)), m);
            for (long long i = 0; i < 2 * mp->len; i++)
                if (k_worth_sizing(mp->pairs[i])) n += k_copy_size(mp->pairs[i], m);
            break;
        }
        case K_SUB: {
            KSub* sb = (KSub*)p;
            n += k_copy_size_ptr(sb, sizeof(KSub), m);
            if (k_worth_sizing(sb->inner)) n += k_copy_size(sb->inner, m);
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            n += k_copy_size_ptr(r, sizeof(KRec), m);
            n += k_copy_size_ptr(r->fields, sizeof(KValue) * (size_t)(r->nfields ? r->nfields : 1), m);
            for (long long i = 0; i < r->nfields; i++)
                if (k_worth_sizing(r->fields[i])) n += k_copy_size(r->fields[i], m);
            break;
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            n += k_copy_size_ptr(cl, sizeof(KClosure), m);
            n += k_copy_size_ptr(cl->env, sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1), m);
            /* A capture or a description's slot is an int or a string as
               often as a pointer, and the chain step sizes its continuation
               every lap: 176,113 of the 510,001 slots the walk visited on
               the run program were immediates, each paying a call whose
               first test sent it back. The list arm has asked first since
               deepbench; the closure, description and subtype arms ask
               now. */
            for (long long i = 0; i < cl->ncaps; i++)
                if (k_worth_sizing(((KValue*)cl->env)[i]))
                    n += k_copy_size(((KValue*)cl->env)[i], m);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            n += k_copy_size_ptr(d, sizeof(KDesc), m);
            if (k_worth_sizing(d->x)) n += k_copy_size(d->x, m);
            if (k_worth_sizing(d->y)) n += k_copy_size(d->y, m);
            break;
        }
        case K_ERR: {
            KErrBox* e = (KErrBox*)p;
            n += k_copy_size_ptr(e, sizeof(KErrBox), m);
            n += k_copy_size(e->reason, m);
            for (KHop* h = e->hops; h && !k_survives_x(h, m); h = h->prev)
                n += k_copy_size_ptr(h, sizeof(KHop), m);
            break;
        }
        default: break;
    }
    return n;
}

/* A survivor whose interior has left: the node itself outlives the rewind and
   only some of its slots point at storage that does not. Copying the node
   would answer that, and it is what a rewind used to do — but a copy lands in
   the carry buffer, which is not the arena, so the node never survives again
   and every later carry re-copies the whole of it. That is the quadratic:
   linear carries, each walking a structure that grows with the input.

   Repairing instead evacuates only the slots that left and writes them back,
   leaving the node where it is. It is what a copying collector does with a
   forwarding pointer, and it is sound for the same reason: anybody else
   holding this node held the same doomed pointer, and now holds a live one. */
static size_t k_repair_size(KValue v, const void* p, KMark* m) {
    size_t n = 0;
    switch (v.tag) {
        case K_STR: {
            KStr* st = (KStr*)p;
            if (!k_survives_x(st->data, m)) n += k_copy_size_ptr(st->data, (size_t)st->len + 1, m);
            break;
        }
        case K_BIG:
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            if (!k_survives_x(b->data, m)) n += k_copy_size_ptr(b->data, (size_t)b->len, m);
            break;
        }
        case K_LIST: {
            KList* l = (KList*)p;
            if (!k_survives_x(l->items, m))
                n += k_copy_size_ptr(l->items, sizeof(KBuf) + sizeof(KValue) * (size_t)(l->len ? l->len : 1), m);
            for (long long i = 0; i < l->len; i++)
                if (k_worth_sizing(l->items[i])) n += k_copy_size(l->items[i], m);
            break;
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            if (!k_survives_x(mp->pairs, m))
                n += k_copy_size_ptr(mp->pairs, sizeof(KBuf) + sizeof(KValue) * (size_t)(2 * (mp->len ? mp->len : 1)), m);
            for (long long i = 0; i < 2 * mp->len; i++)
                if (k_worth_sizing(mp->pairs[i])) n += k_copy_size(mp->pairs[i], m);
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            if (!k_survives_x(r->fields, m))
                n += k_copy_size_ptr(r->fields, sizeof(KValue) * (size_t)(r->nfields ? r->nfields : 1), m);
            for (long long i = 0; i < r->nfields; i++)
                if (k_worth_sizing(r->fields[i])) n += k_copy_size(r->fields[i], m);
            break;
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            if (!k_survives_x(cl->env, m))
                n += k_copy_size_ptr(cl->env, sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1), m);
            for (long long i = 0; i < cl->ncaps; i++) n += k_copy_size(((KValue*)cl->env)[i], m);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            n += k_copy_size(d->x, m);
            n += k_copy_size(d->y, m);
            break;
        }
        case K_SUB: {
            KSub* sb = (KSub*)p;
            n += k_copy_size(sb->inner, m);
            break;
        }
        default: break;
    }
    return n;
}

static void k_copy_map_put(const void* p, void* copy) {
    KPtrSlot* slot = k_ptrmap_at(&k_copy_map, p, &k_copy_map_live);
    if (slot->gen != k_copy_map.gen) k_copy_map_live++;
    slot->gen = k_copy_map.gen;
    slot->key = p;
    slot->val = copy;
}

static void k_repair_interior(KValue v, void* p, KCopy* cp);
static void k_repaired_settle(KMark* m);

/* Containers repaired during one carry, so the pass after the rewind can find
   them again. A repair leaves the node in the arena and its evacuated slots in
   the carry buffer, which is malloc'd — so the node would fail the same check
   at the next carry, and repairing it again is the quadratic in another shape.
   The second pass moves those slots into the arena and moves the mark up over
   them, which is what makes the repair stick: the survivor region absorbs what
   a survivor still points at. */
static KValue* k_repaired = NULL;
static long long k_repaired_n = 0, k_repaired_cap = 0;

static void k_repaired_note(KValue v) {
    if (k_repaired_n == k_repaired_cap) {
        k_repaired_cap = k_repaired_cap ? k_repaired_cap * 2 : 64;
        k_repaired = realloc(k_repaired, sizeof(KValue) * (size_t)k_repaired_cap);
        if (!k_repaired) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_repaired[k_repaired_n++] = v;
}

/* Nodes a repair wrote into that lie below an enclosing loop's mark. A pop
   copies its result out of the carry pair into the arena at the frontier, and
   a node the walk meets that survives but points at storage the pop retires is
   repaired in place. The frontier is in the enclosing loop's lap. When the
   repaired node is older than that loop's mark, it now points above the mark,
   and the enclosing loop's next stage rewinds the copy away. That stage's walk
   prunes at a survivor whose immediate interior survives, so it does not reach
   the node from the carried slots, and the node is left pointing at freed
   memory. The lox port's function calls did this: a record built before the
   evaluator's loop held a string in the outer loop's carry storage, an inner
   loop's pop copied the string to the frontier, and the evaluator's next stage
   gave it back.

   So the pop keeps the nodes it repaired below the enclosing mark here, and
   every stage revisits the ones below its own mark as extra roots before it
   rewinds. A stage copies what such a node points at into the carry and
   settles it back under its raised mark, which makes the node safe for that
   depth and not for the one outside it, so a node below the outer mark stays
   on the list for the outer stage. A rewind frees what lies above its mark,
   and the entries there go with it: while the list holds anything,
   `K_BUF_PEND` in `k_buf_dirty` sends every rewind to the slow path, which
   drops them. */
#define K_BUF_PEND 2
static KValue* k_pend = NULL;
static long long k_pend_n = 0, k_pend_cap = 0;

static void k_pend_push(KValue v) {
    if (k_pend_n == k_pend_cap) {
        k_pend_cap = k_pend_cap ? k_pend_cap * 2 : 64;
        k_pend = realloc(k_pend, sizeof(KValue) * (size_t)k_pend_cap);
        if (!k_pend) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_pend[k_pend_n++] = v;
    k_buf_dirty |= K_BUF_PEND;
}

static int k_pend_below(KValue v, KMark* m) {
    return k_where((const void*)(intptr_t)v.payload, m) == K_WHERE_BELOW;
}

/* A rewind to the outermost mark clears the list. What lies above that mark
   is freed by the rewind, and no stage outside it will ever revisit what lies
   below. */
static void k_pend_drop_above(KMark* m) {
    if (m == &k_beat_stack[0]) { k_pend_n = 0; return; }
    long long w = 0;
    for (long long i = 0; i < k_pend_n; i++)
        if (k_pend_below(k_pend[i], m)) k_pend[w++] = k_pend[i];
    k_pend_n = w;
}

/* Keep the nodes this walk repaired that the rewind of depth `d - 1` could
   still strand: the ones below its mark. */
static void k_pend_keep_repaired(long long d) {
    if (d <= 0 || d > K_BEAT_MAX) return;
    KMark* outer = &k_beat_stack[d - 1];
    for (long long i = 0; i < k_repaired_n; i++)
        if (k_pend_below(k_repaired[i], outer)) k_pend_push(k_repaired[i]);
}

/* Tenuring is sticky, and it is never unset within one evacuation. A promoted
   node's fresh descendants must land beside it — leave one in the pair and the
   pair overwrites it two laps later while the tenured parent still points at
   it — and the cheap way to guarantee that is to stop asking. Everything the
   walk reaches after the first lived-a-lap node is tenured too, which costs a
   little storage and saves a save and a restore at every node of the hottest
   recursion in the runtime. Cleared where the evacuation starts. */
static KValue k_deep_copy(KValue v, KCopy* cp) {
    /* The malloc'd cell keeps its identity — every holder resolves through
       the same thunk — and only its slots evacuate, written back in place. */
    if (v.tag == K_THUNK) {
        KThunk* t = (KThunk*)(intptr_t)v.payload;
        KPtrSlot* slot = k_ptrmap_at(&k_copy_map, t, &k_copy_map_live);
        if (slot->gen == k_copy_map.gen && slot->key == t) return v;
        k_copy_map_put(t, t);
        if (t->forced) t->result = k_deep_copy(t->result, cp);
        for (int i = 0; i < t->argc; i++) t->args[i] = k_deep_copy(t->args[i], cp);
        return v;
    }
    if (!k_is_heap(v.tag)) return v;
    if (k_carry_holds((const void*)(intptr_t)v.payload)) cp->in_ten = 1;
    void* p = (void*)(intptr_t)v.payload;
    if (k_survives_x(p, cp->mark)) {
        if (!cp->deep && k_interior_survives(v, p, cp->mark)) return v;
        /* A constant from k_alloc_const is never written after it is built,
           so what it points at is as permanent as it is, and repairing it
           would write into a constant -- a marker record's NULL field array
           among them. A frozen CAF is not asked here: a proven in-place push
           can move a frozen list's items out of frozen storage, and that list
           needs the repair. */
        if (k_const_n && k_const_holds(p)) return v;
        KPtrSlot* slot = k_ptrmap_at(&k_copy_map, p, &k_copy_map_live);
        if (slot->gen == k_copy_map.gen && slot->key == p) {
            if (K_COUNTING) k_stat_carry_dedup++;
            return v;
        }
        k_copy_map_put(p, p);
        k_repaired_note(v);
        k_repair_interior(v, p, cp);
        return v;
    }
    {
        /* A node copied into the pair before the walk began tenuring is not a
           copy a tenured parent may share. The tenured parent survives the
           next stage, whose walk prunes at it, and the pair is overwritten the
           stage after; so the parent copies the node again, into tenure, and
           the map hands that copy to whatever asks next. The first copy stays
           where its first holder put it, and the next stage walks that holder. */
        KPtrSlot* slot = k_ptrmap_at(&k_copy_map, p, &k_copy_map_live);
        if (slot->gen == k_copy_map.gen && slot->key == p
            && !(cp->in_ten && cp->buf && (char*)slot->val >= cp->buf->data
                 && (char*)slot->val < cp->buf->data + cp->buf->cap)) {
            if (K_COUNTING) k_stat_carry_dedup++;
            KValue out = v;
            out.payload = k_ptr(slot->val);
            return out;
        }
    }
    KValue out = v;
    switch (v.tag) {
        case K_STR: {
            KStr* s = (KStr*)p;
            KStr* ns = k_copy_alloc(cp, sizeof(KStr));
            k_copy_map_put(p, ns);
            ns->len = s->len;
            /* The copy is the same characters, so a count already paid for
               carries over. Dropping it made every evacuation re-walk the
               string, which is the quadratic the count exists to remove. A
               builder's positive cap does not travel: the copy owns no room. */
            ns->cap = s->cap < 0 ? s->cap : 0;
            if (k_survives_x(s->data, cp->mark)) {
                ns->data = s->data;
            } else {
                ns->data = k_copy_alloc(cp, (size_t)s->len + 1);
                memcpy(ns->data, s->data, (size_t)s->len);
                ns->data[s->len] = 0;
            }
            out.payload = k_ptr(ns);
            break;
        }
        case K_BIG:
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            KBytes* nb = k_copy_alloc(cp, sizeof(KBytes));
            k_copy_map_put(p, nb);
            nb->len = b->len;
            nb->cap = 0;
            if (k_survives_x(b->data, cp->mark)) {
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
            k_copy_map_put(p, nl);
            KBuf* buf = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)(l->len ? l->len : 1));
            k_buf_set_cap(buf, l->len ? l->len : 1, 0);
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
            k_copy_map_put(p, nm);
            KBuf* buf = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)(2 * (mp->len ? mp->len : 1)));
            k_buf_set_cap(buf, 2 * (mp->len ? mp->len : 1), 0);
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
        case K_SUB: {
            KSub* sb = (KSub*)p;
            KSub* nb = k_copy_alloc(cp, sizeof(KSub));
            k_copy_map_put(p, nb);
            nb->type_id = sb->type_id;
            nb->inner = k_deep_copy(sb->inner, cp);
            out.payload = k_ptr(nb);
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            KRec* nr = k_copy_alloc(cp, sizeof(KRec));
            k_copy_map_put(p, nr);
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
            k_copy_map_put(p, nc);
            KValue* env = k_copy_alloc(cp, sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1));
            for (long long i = 0; i < cl->ncaps; i++) env[i] = k_deep_copy(((KValue*)cl->env)[i], cp);
            nc->fn = cl->fn;
            nc->env = env;
            nc->ncaps = cl->ncaps;
            nc->arity = cl->arity;
            nc->body = cl->body;
            out.payload = k_ptr(nc);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            KDesc* nd = k_copy_alloc(cp, sizeof(KDesc));
            k_copy_map_put(p, nd);
            nd->dtag = d->dtag;
            nd->x = k_deep_copy(d->x, cp);
            nd->y = k_deep_copy(d->y, cp);
            out.payload = k_ptr(nd);
            break;
        }
        case K_ERR: {
            KErrBox* e = (KErrBox*)p;
            KErrBox* ne = k_copy_alloc(cp, sizeof(KErrBox));
            k_copy_map_put(p, ne);
            ne->reason = k_deep_copy(e->reason, cp);
            ne->origin = e->origin;
            /* the copy allocator does not zero either, and an err that
               survives an evacuation keeps what it was told and what it
               wrapped */
            ne->cause = e->cause;
            ne->merged = e->merged;
            ne->hako = e->hako;
            KHop** tail = &ne->hops;
            KHop* h = e->hops;
            for (; h && !k_survives_x(h, cp->mark); h = h->prev) {
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

/* The repair itself. Every slot goes back through k_deep_copy, which answers
   immediately for one that is already safe — the point is not to skip the
   scan but to leave the node in the arena, so it is still a survivor at the
   next carry instead of a malloc'd copy that has to be walked again. */
static void k_repair_interior(KValue v, void* p, KCopy* cp) {
    switch (v.tag) {
        /* A builder's storage and a malloc'd byte buffer are out of a
           rewind's reach, which is why k_interior_survives passes them. A
           deep walk comes here without asking it, and copying the storage
           into the arena would turn a builder into a plain string that the
           next append copies again. */
        case K_STR: {
            KStr* st = (KStr*)p;
            if (st->cap > 0 || k_survives_x(st->data, cp->mark)) break;
            char* d = k_copy_alloc(cp, (size_t)st->len + 1);
            memcpy(d, st->data, (size_t)st->len);
            d[st->len] = 0;
            st->data = d;
            st->cap = st->cap < 0 ? st->cap : 0;
            break;
        }
        case K_BIG:
        case K_BYTES: {
            KBytes* b = (KBytes*)p;
            if (k_bytes_malloced(b) || k_survives_x(b->data, cp->mark)) break;
            unsigned char* d = k_copy_alloc(cp, (size_t)b->len);
            memcpy(d, b->data, (size_t)b->len);
            b->data = d;
            b->cap = 0;
            break;
        }
        case K_LIST: {
            KList* l = (KList*)p;
            if (!k_survives_x(l->items, cp->mark)) {
                long long cap = l->len ? l->len : 1;
                KBuf* nb = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
                k_buf_set_cap(nb, cap, 0);
                nb->used = l->len;
                memcpy(nb + 1, l->items, sizeof(KValue) * (size_t)l->len);
                l->items = (KValue*)(nb + 1);
            }
            for (long long i = 0; i < l->len; i++) l->items[i] = k_deep_copy(l->items[i], cp);
            break;
        }
        case K_MAP: {
            KMap* mp = (KMap*)p;
            if (!k_survives_x(mp->pairs, cp->mark)) {
                long long cap = 2 * (mp->len ? mp->len : 1);
                KBuf* nb = k_copy_alloc(cp, sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
                k_buf_set_cap(nb, cap, 0);
                nb->used = 2 * mp->len;
                memcpy(nb + 1, mp->pairs, sizeof(KValue) * (size_t)(2 * mp->len));
                int aliased = mp->sorted == mp->pairs;
                mp->pairs = (KValue*)(nb + 1);
                if (mp->sorted && !aliased) k_map_view_free(mp->sorted);
                mp->sorted = NULL;
                mp->sorted_len = 0;
            }
            /* A sorted view is a second copy of the pairs, held outside the
               walk's reach. When the walk moves a key or a value, the view
               still names the old one, and a lookup that searches the view
               reads storage the next stage at this depth writes over. */
            int moved = 0;
            for (long long i = 0; i < 2 * mp->len; i++) {
                KValue nv = k_deep_copy(mp->pairs[i], cp);
                moved |= nv.payload != mp->pairs[i].payload;
                mp->pairs[i] = nv;
            }
            if (moved && mp->sorted && mp->sorted != mp->pairs) {
                k_view_free(mp->sorted);
                mp->sorted = NULL;
                mp->sorted_len = 0;
            }
            break;
        }
        case K_REC: {
            KRec* r = (KRec*)p;
            if (!k_survives_x(r->fields, cp->mark)) {
                size_t n = sizeof(KValue) * (size_t)(r->nfields ? r->nfields : 1);
                KValue* nf = k_copy_alloc(cp, n);
                memcpy(nf, r->fields, n);
                r->fields = nf;
            }
            for (long long i = 0; i < r->nfields; i++) r->fields[i] = k_deep_copy(r->fields[i], cp);
            break;
        }
        case K_CLOSURE: {
            KClosure* cl = (KClosure*)p;
            /* A closure over nothing holds no arena pointer, so there is
               nothing here to evacuate and the header must not be written:
               the emitter builds these as link-time constants. */
            if (cl->ncaps == 0) break;
            if (!k_survives_x(cl->env, cp->mark)) {
                size_t n = sizeof(KValue) * (size_t)(cl->ncaps ? cl->ncaps : 1);
                KValue* ne = k_copy_alloc(cp, n);
                memcpy(ne, cl->env, n);
                cl->env = ne;
            }
            for (long long i = 0; i < cl->ncaps; i++)
                ((KValue*)cl->env)[i] = k_deep_copy(((KValue*)cl->env)[i], cp);
            break;
        }
        case K_DESC: {
            KDesc* d = (KDesc*)p;
            d->x = k_deep_copy(d->x, cp);
            d->y = k_deep_copy(d->y, cp);
            break;
        }
        case K_SUB: {
            KSub* sb = (KSub*)p;
            sb->inner = k_deep_copy(sb->inner, cp);
            break;
        }
        default: break;
    }
}

/* The second half of a repair. The rewind has happened, so the arena frontier
   is back at the mark and what a repaired node points at is in the carry
   buffer. Copy it into the arena and raise the mark over it: those bytes are
   reachable from a node that already survives, so they are live by definition,
   and being below the mark is what lets the node be shared at the next carry
   instead of repaired again. */
static void k_repaired_settle(KMark* m) {
    if (!k_repaired_n) return;
    KCopy back = { NULL, m, 1 };
    k_ptrmap_begin(&k_copy_map);
    k_copy_map_live = 0;
    for (long long i = 0; i < k_repaired_n; i++) {
        KValue v = k_repaired[i];
        k_repair_interior(v, (void*)(intptr_t)v.payload, &back);
    }
    k_repaired_n = 0;
    m->ptr = k_arena;
    m->left = k_arena_left;
    m->block = k_blocks;
    m->bytes = k_live_block_bytes;
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
static int k_memo_outlives(KValue result) {
    if (!k_is_heap(result.tag)) return 1;
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return 1;
    KMark* inner = &k_beat_stack[k_beat_depth - 1];
    return k_survives((const void*)(intptr_t)result.payload, inner);
}

/* The pop's work, for a pop that has some. Kept out of line so the pop that
   has none -- a heap result with nothing carried, nothing registered and no
   tenure block at its depth, which is 507,678 of runbench's 507,685 -- pays
   its four tests and returns without the six callee-saved pushes the copy
   and the migrates below need. The same split the rewind made. */
static KBuf* k_buf_of(KValue* items);

/* Whether `p` lies in depth d's carry pair. */
static int k_carry_at_holds(long long d, const void* p) {
    const char* q = (const char*)p;
    KCarryBuf* f = &k_carries[d].from;
    KCarryBuf* t = &k_carries[d].to;
    return (f->data && q >= f->data && q < f->data + f->cap)
        || (t->data && q >= t->data && q < t->data + t->cap);
}

/* A map or list header in a carry buffer is born this beat by the arena's
   reckoning, since the buffer is not arena, so its view or its permanent
   storage registers at this depth. A pop that copies its result out of the
   carry pair leaves every such header dead: the pair is overwritten by the
   next loop at this depth or freed with it. Handing those entries up, as a
   kept region's are, left the parent flushing headers that no longer held
   anything, and the diff port died freeing a pointer into the middle of an
   arena block. They are released here instead, while the headers are still
   intact, and everything else hands up as the migrates below would hand it:
   one pass over each registry, where a release pass followed by the migrate
   cost deepbench 1,040,014 instructions. */
static void k_viewreg_migrate_carried(int d) {
    if (k_viewreg_n[d] == 0) return;
    k_reg_any_at(d) &= ~K_REG_VIEW;
    for (long long i = 0; i < k_viewreg_n[d]; i++) {
        KMap* m = k_viewreg[d][i];
        if (!k_carry_at_holds(d, m)) {
            if (d > 0) k_viewreg_push(d - 1, m);
            continue;
        }
        if (m->sorted) {
            if (m->sorted != m->pairs) k_view_free(m->sorted);
            m->sorted = NULL;
            m->sorted_len = 0;
        }
    }
    k_viewreg_n[d] = 0;
}

static void k_permreg_migrate_carried(int d) {
    if (!k_permreg_any || k_permreg_n[d] == 0) return;
    k_reg_any_at(d) &= ~K_REG_PERM;
    for (long long i = 0; i < k_permreg_n[d]; i++) {
        KValue** slot = k_permreg[d][i];
        if (!k_carry_at_holds(d, slot)) {
            if (d > 0) k_permreg_push(d - 1, slot);
            continue;
        }
        if (!*slot) continue;
        KBuf* b = k_buf_of(*slot);
        if (!k_buf_malloced(b)) continue;
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_bytes_freed++;
        k_perm_live -= (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)k_buf_cap(b));
        free(b);
        *slot = NULL;
    }
    k_permreg_n[d] = 0;
}

static __attribute__((noinline)) KValue k_beat_pop_slow(KValue r, long long d,
                                                        int rewound) {
    KCarry* c = &k_carries[d];
    if (rewound) {
        k_beat_rewind(&k_beat_stack[d]);
    } else {
        if (c->used_flag) {
            /* The walk has no mark, so it counts every arena node as a
               survivor and stops at one whose interior survives too. That
               is sound while everything the result holds in the carry pair
               sits under a node the walk reaches first. A lap that skipped
               its stage breaks it: the lap builds fresh nodes in the arena
               over values still in the pair, the walk stops at the fresh
               node, and the next loop entered at this depth stages into
               the same pair and writes over what the result still holds.
               So a pop after a skipped lap walks the whole value. */
            KCopy cp = { NULL, NULL, 1, 0, k_carry_written || c->skipped };
            k_ptrmap_begin(&k_copy_map);
            k_copy_map_live = 0;
            k_repaired_n = 0;
            r = k_deep_copy(r, &cp);
            k_pend_keep_repaired(d);
            k_repaired_n = 0;
            k_chunkreg_migrate((int)d);
            k_viewreg_migrate_carried((int)d);
            k_permreg_migrate_carried((int)d);
        } else {
            k_chunkreg_migrate((int)d);
            k_viewreg_migrate((int)d);
            k_permreg_migrate((int)d);
        }
    }
    if (rewound) k_ten_release(d);
    else k_ten_hand_up(d);
    c->used_flag = 0;
    c->skipped = 0;
    return r;
}

KValue k_beat_pop(KValue r) {
    if (k_beat_depth > 0) {
        k_beat_set_depth(k_beat_depth - 1);
        long long d = k_beat_depth;
        if (d < K_BEAT_MAX) {
            int rewound = !k_is_heap(r.tag) && r.tag != K_THUNK;
            /* k_reg_any_at(d) summarises the three registries: every add sets
               its bit and every migrate or flush clears it, and the chunk
               spill count travels with the chunk bit. An empty migrate wrote
               three zeros over zeros. */
            if (!rewound && !k_carries[d].used_flag && !k_reg_any_at(d)
                && !k_ten_blocks[d]) {
                return r;
            }
            return k_beat_pop_slow(r, d, rewound);
        }
    }
    return r;
}

/* The pop of a region (beat::region_sites): a mark taken before a call's
   arguments, so the call and everything its arguments built lie above it.
   When the result cannot reach that storage -- a scalar, or a bytes or string
   header from before the mark -- the region is garbage whole and goes back.
   Anything else is the ordinary pop: a fresh structure lives in the region, so
   the region is handed up to the enclosing frame rather than freed. */
static __attribute__((noinline)) KValue k_region_pop_slow(KValue r) {
    if (k_beat_depth > 0) {
        k_beat_set_depth(k_beat_depth - 1);
        long long d = k_beat_depth;
        if (d < K_BEAT_MAX) {
            int outside = (!k_is_heap(r.tag) && r.tag != K_THUNK)
                || ((r.tag == K_BYTES || r.tag == K_STR)
                    && !k_above_mark((const void*)(intptr_t)r.payload, &k_beat_stack[d]));
            if (outside && !k_carries[d].used_flag) {
                k_beat_rewind(&k_beat_stack[d]);
                k_ten_release(d);
                return r;
            }
            if (!outside && !k_carries[d].used_flag && !k_reg_any_at(d) && !k_ten_blocks[d]) {
                return r;
            }
            return k_beat_pop_slow(r, d, outside);
        }
    }
    return r;
}

/* The common case, and two stores: the region stayed in its mark's block,
   touched no registry or tenure block, and the result lies outside the range
   it allocated. Every other case is a tail call to the full pop, so this path
   saves no registers. On the run program every one of 248,490 region pops
   takes it. */
KValue k_region_pop(KValue r) {
    int d = k_beat_depth - 1;
    if ((unsigned)d < (unsigned)K_BEAT_MAX) {
        KMark* m = &k_beat_stack[d];
        if (!(k_buf_dirty | m->reg_any) && k_blocks == m->block && !k_ten_blocks[d]) {
            /* No carry test: a carry is staged only at a carry beat's own
               depth, and a loop inside the region pushes its mark above
               this one. Bytes and strings are asked first, being what a
               region's call answers. */
            int outside = ((r.tag == K_BYTES || r.tag == K_STR)
                           && (uintptr_t)r.payload - (uintptr_t)m->ptr
                                  >= (uintptr_t)k_arena - (uintptr_t)m->ptr)
                || (!k_is_heap(r.tag) && r.tag != K_THUNK);
            if (outside) {
                if (m < k_seek_under) k_seek_str = NULL;
                k_arena = m->ptr;
                k_arena_left = m->left;
                k_beat_set_depth(d);
                return r;
            }
        }
    }
    return k_region_pop_slow(r);
}

void k_carry_reset(void);
void k_carry_stage(KValue v);
KValue k_carry_take(long long i);
void k_beat_iter_carry(void);

/* A construction cohort ends here: the call built a value, and everything
   else it allocated is garbage the caller will never see. When enough
   blocks accumulated to be worth the dance, the result is evacuated
   through the carry machinery — sized against the mark, copied into the
   side buffer sharing everything below the mark, the segment rewound, the
   survivor copied back — and the garbage goes with the rewind. A small
   segment just drops the mark: the bookkeeping stays, the reclaim was not
   worth a copy. The threshold is block-granular, deterministic, and
   priced by the one-shot welfare term. */
KValue k_cohort_pop(KValue r) {
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) {
        if (k_beat_depth > 0) k_beat_set_depth(k_beat_depth - 1);
        return r;
    }
    KMark* m = &k_beat_stack[k_beat_depth - 1];
    long long grown = k_live_block_bytes - m->bytes;
    if (grown < (long long)(1 << 19)) {
        k_beat_set_depth(k_beat_depth - 1);
        k_chunkreg_migrate(k_beat_depth);
        k_viewreg_migrate(k_beat_depth);
        k_permreg_migrate(k_beat_depth);
        return r;
    }
    if (!k_is_heap(r.tag) && r.tag != K_THUNK) {
        k_beat_set_depth(k_beat_depth - 1);
        k_beat_rewind(m);
        k_spare_release(2);
        if (K_COUNTING) k_stat_cohort_frees++;
        return r;
    }
    /* the survivor-ratio guard: the threshold above counts what grew, not
       what died. When the result is most of the growth — utf8 of a large
       buffer, a decode that is nearly all live tree — the dance would copy
       the value to reclaim almost nothing. Size the copy first and keep
       the region when it exceeds half the reclaim. */
    k_ptrmap_begin(&k_copy_seen);
    k_copy_seen_live = 0;
    size_t cap = (size_t)(4 << 19);
    size_t half = (size_t)(grown / 2);
    k_copy_size_budget = cap < half ? cap : half;
    k_copy_size_spent = 0;
    size_t survivor = k_copy_size(r, m);
    k_copy_size_budget = 0;
    /* two refusals: a copy that exceeds half the reclaim buys too little,
       and a survivor much larger than the threshold scale makes the carry
       buffers themselves the peak — the dance transiently holds the copy
       twice on top of the garbage it frees */
    if ((long long)(2 * survivor) > grown || survivor > cap) {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_cohort_kept++;
        k_beat_set_depth(k_beat_depth - 1);
        k_chunkreg_migrate(k_beat_depth);
        k_viewreg_migrate(k_beat_depth);
        k_permreg_migrate(k_beat_depth);
        return r;
    }
    k_carry_reset();
    k_carry_stage(r);
    k_beat_iter_carry();
    if (K_COUNTING) k_stat_beat_iters--; /* the dance is a free, not a loop iteration */
    if (K_COUNTING) k_stat_cohort_frees++;
    r = k_carry_take(0);
    r = k_beat_pop(r);
    /* the carry pair is sized for a loop that will stage again; a cohort
       stages exactly once, and for a large evacuated result the pair holds
       megabytes nothing will reuse */
    if (k_beat_depth >= 0 && k_beat_depth < K_BEAT_MAX) {
        KCarry* done = &k_carries[k_beat_depth];
        free(done->from.data);
        free(done->to.data);
        done->from = (KCarryBuf){ NULL, 0, 0 };
        done->to = (KCarryBuf){ NULL, 0, 0 };
        k_cbox_refresh();
    }
    k_spare_release(2);
    return r;
}

void k_carry_reset(void) {
    k_carry_n = 0;
    memset(k_carry_kept, 0, sizeof(k_carry_kept));
}

void k_carry_stage(KValue v) {
    if (k_carry_n < K_CARRY_MAX) k_carry_slots[k_carry_n] = v;
    k_carry_n++;
}

KValue k_carry_take(long long i) { return k_carry_slots[i]; }

/* Stage a slot the compiler proved is this cycle's own builder. */
/* Stage a slot the compiler proved is this cycle's own builder, and move its
   header off the arena if the rewind about to happen would reclaim it. A seed is
   an arena bump like every other allocation, which is what seeds want; the one
   carried by identity is the exception, and this is the only place that knows
   which one that is. It happens at most once per builder — a promoted header
   survives the mark from then on. */
void k_carry_stage_kept(KValue v) {
    if (k_carry_n < K_CARRY_MAX) k_carry_kept[k_carry_n] = 1;
    if (v.tag == K_STR && k_beat_depth > 0 && k_beat_depth <= K_BEAT_MAX) {
        KStr* h = (KStr*)(intptr_t)v.payload;
        KMark* m = &k_beat_stack[k_beat_depth - 1];
        if (!k_survives(h, m)) {
            KStr* out = malloc(sizeof(KStr));
            if (!out) { fputs("out of memory\n", stderr); exit(1); }
            *out = *h;
            v.payload = k_ptr(out);
        }
    }
    k_carry_stage(v);
}


/* The stage's half of the list above: walk the nodes below this stage's mark
   as roots, so the ones that point above it are repaired into the carry, and
   keep the old entries the outer rewind could still strand. An entry this walk
   repaired is left off here, because `k_pend_keep_repaired` adds it back from
   the repaired list. */
static void k_pend_revisit(KMark* m, KCopy* cp) {
    long long d = k_beat_depth - 1;
    KMark* outer = d > 0 ? &k_beat_stack[d - 1] : NULL;
    long long n = k_pend_n;
    for (long long i = 0; i < n; i++)
        if (k_pend_below(k_pend[i], m)) k_deep_copy(k_pend[i], cp);
    long long w = 0;
    for (long long i = 0; i < n; i++) {
        const void* p = (const void*)(intptr_t)k_pend[i].payload;
        if (!outer || !k_pend_below(k_pend[i], outer)) continue;
        KPtrSlot* slot = k_ptrmap_at(&k_copy_map, p, &k_copy_map_live);
        if (slot->gen == k_copy_map.gen && slot->key == p) continue;
        k_pend[w++] = k_pend[i];
    }
    k_pend_n = w;
}

void k_beat_iter_carry(void) {
    if (K_COUNTING) k_stat_beat_iters++;
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return;
    for (long long i = 0; i < k_carry_n; i++) {
        long long tag = k_carry_slots[i].tag;
        if (tag == 4 || tag == 5) return; /* failure: the callee propagates it */
    }
    KMark* m = &k_beat_stack[k_beat_depth - 1];
    KCarry* c = &k_carries[k_beat_depth - 1];
    size_t need = 0;
    k_ten_on = 1;
    k_from_window(1);
    k_size_in_ten = 0;
    k_ptrmap_begin(&k_copy_seen);
    k_copy_seen_live = 0;
    for (long long i = 0; i < k_carry_n; i++)
        if (!k_carry_kept[i]) need += k_copy_size(k_carry_slots[i], m);
    for (long long i = 0; i < k_pend_n; i++)
        if (k_pend_below(k_pend[i], m)) need += k_copy_size(k_pend[i], m);
    if (c->to.cap < need) {
        free(c->to.data);
        c->to.data = malloc(need ? need : 16);
        if (!c->to.data) { fputs("out of memory\n", stderr); exit(1); }
        c->to.cap = need ? need : 16;
        k_cbox_refresh();
    }
    c->to.used = 0;
    KCopy cp = { &c->to, m, 0, 0 };
    k_size_in_ten = 0;
    k_ptrmap_begin(&k_copy_map);
    k_copy_map_live = 0;
    k_repaired_n = 0;
    for (long long i = 0; i < k_carry_n; i++)
        if (!k_carry_kept[i]) k_carry_slots[i] = k_deep_copy(k_carry_slots[i], &cp);
    if (k_pend_n) k_pend_revisit(m, &cp);
    k_pend_keep_repaired(k_beat_depth - 1);
    k_beat_rewind(m);
    k_repaired_settle(m);
    KCarryBuf swap = c->from;
    c->from = c->to;
    c->to = swap;
    c->used_flag = 1;
    c->skipped = 0;
    k_ten_on = 0;
    k_from_window(0);
}

/* A compiled loop's carry, which waits until the lap has left something worth
   reclaiming. `k_beat_iter_carry` sizes every carried slot, copies it into the
   carry pair and rewinds, and a loop whose laps are small paid that on every
   one: mal's evaluator crosses a carrying edge about fifty times for each call
   it runs, and copying the machine state at each of them made the loop ten
   times slower than letting the arena grow. Here the rewind waits until the
   arena has drifted a quarter megabyte past where the last stage left it, in
   the same block, with nothing registered at the depth. That is the bind
   chain's drift test, used for the same reason. Skipping is always sound: the
   carried values stay where they are, and the next stage copies whatever is
   still reachable. What the test bounds is the garbage held in the meantime.

   The registries are asked because their storage is malloc'd, so the block
   test cannot see it grow; skipping past a registered buffer held 49 KB more
   permanent storage at the run program's peak. The view registry is not
   asked. A view is never larger than its map, and its map is either part of
   the drift or still live, so the views a skipped lap holds are bounded by
   what the test already bounds. Asking it made every edge of a cycle that
   sorts a map stage at once: 3,200,304 bytes copied where the lap rule copied
   6,224, and mal ran fifteen times slower. The lap rule's figure is 6,128
   since constants stopped being copied. A stale position left by an
   earlier loop at the same depth only makes the first lap stage early, and the
   cohort and the chain still call `k_beat_iter_carry` directly. */
/* How far the arena has moved since a stage left it at `at_arena` in the block
   `at_blocks`: the rest of that block, every block opened since, and what the
   newest holds. Counting the newest at its full size made a stage of every
   block opened, and a block opens on the first pass after a stage that leaves
   the mark near a block's end. A block the chain no longer holds was rewound
   past, which is as far as a lap can go. */
static size_t k_drift_since(const KCarry* kc) {
    size_t n = (size_t)(k_arena - (const char*)(k_blocks + 1));
    for (const KBlock* b = k_blocks->next; b; b = b->next) {
        if ((const void*)b == kc->at_blocks)
            return n + (size_t)((const char*)(b + 1) + b->cap - kc->at_arena);
        n += b->cap;
    }
    return (size_t)-1;
}

/* The wait grows with what the last stage copied. Once a loop's live state
   outgrows tenure (K_TEN_CAP), every stage copies all of it, and a fixed
   quarter megabyte of drift made that copy once per quarter megabyte of
   garbage: the lox port's F18 loop ran 130,000 passes in 2.7 s and 160,000 in
   99 s, the evacuations rising from 5.8 million to 139 million. Waiting for
   twice the copy keeps the copying proportional to the garbage it reclaims,
   which is the rule a copying collector sizes its heap by. A loop whose stages
   copy little waits the quarter megabyte it always did, and only such a loop
   still stages when the arena opens a block. */
void k_beat_lap_carry(void) {
    long long d = k_beat_depth - 1;
    if (d < 0 || d >= K_BEAT_MAX) { k_beat_iter_carry(); return; }
    KCarry* kc = &k_carries[d];
    size_t wait = (size_t)1 << 18;
    if (kc->used_flag && 2 * kc->from.used > wait) wait = 2 * kc->from.used;
    if (kc->at_arena && !(k_beat_stack[d].reg_any & ~K_REG_VIEW)
        && (kc->at_blocks == (const void*)k_blocks
                ? (size_t)(k_arena - kc->at_arena) < wait
                : wait > ((size_t)1 << 18) && k_drift_since(kc) < wait)) {
        if (K_COUNTING) k_stat_beat_iters++;
        kc->skipped = 1;
        return;
    }
    k_beat_iter_carry();
    kc->at_arena = k_arena;
    kc->at_blocks = (const void*)k_blocks;
}

/* A permanent object: malloc'd, so it lives outside the beat arena and
   survives every rewind. Interned single-char strings and zero-field marker
   records are cached and reused across beats — an arena rewind moves the
   bump pointer, so caching arena storage and reusing it after a pop hands
   back reclaimed, since-reused memory. Permanent storage is the only cache
   that is sound across beats. */
static __attribute__((noinline, preserve_most)) void* k_alloc_perm(size_t n) {
    if (K_COUNTING) k_stat_perm_allocs++;
    void* p = malloc(n);
    if (!p) { fputs("out of memory\n", stderr); exit(1); }
    return p;
}

/* Permanent storage the copy walk knows is permanent. A literal, an interned
   character and a marker record are built once and handed out for the rest of
   the run, but malloc'd storage lies in no arena block, and the walk takes an
   unregistered pointer for one it has to rescue. Copying a constant into a
   carry is only waste; the harm is the repair, which leaves a surviving node
   where it is and copies what it points at -- so a map built before a loop,
   whose keys were literals, came back from the loop's first stage with its
   keys moved into the loop's region, and the next rewind of that region freed
   them under a map that was still live. Constants come from chunks registered
   with the frozen ranges instead, which the walk already treats as immortal.
   Bounded by the program text and the caches: a bignum moved to permanent
   storage is the run's own data, so it stays on k_alloc_perm and out of the
   ranges every outside pointer is checked against. */
/* Chunks start at 4 KiB and double up to 64 KiB. A program with few
   constants then holds one small chunk, which in the browser is the
   difference between staying inside the pages it already had and taking one
   more. */
#define K_CONST_FIRST (4 * 1024)
static char* k_const_at = NULL;
static size_t k_const_left = 0;

static void k_const_chunk(size_t size) {
    char* chunk = malloc(size);
    if (!chunk) { fputs("out of memory\n", stderr); exit(1); }
    k_frozen_note(chunk, size);
    if (k_const_n == k_const_cap) {
        k_const_cap = k_const_cap ? 2 * k_const_cap : 8;
        k_const_lo = realloc(k_const_lo, sizeof(const char*) * (size_t)k_const_cap);
        k_const_hi = realloc(k_const_hi, sizeof(const char*) * (size_t)k_const_cap);
        if (!k_const_lo || !k_const_hi) { fputs("out of memory\n", stderr); exit(1); }
    }
    k_const_lo[k_const_n] = chunk;
    k_const_hi[k_const_n] = chunk + size;
    k_const_n++;
    k_const_at = chunk;
    k_const_left = size;
}
static __attribute__((noinline, preserve_most)) void* k_alloc_const(size_t n) {
    if (K_COUNTING) k_stat_perm_allocs++;
    n = (n + 15) & ~(size_t)15;
    if (n > k_const_left) {
        size_t step = K_CONST_FIRST << (k_const_n < 4 ? k_const_n : 4);
        k_const_chunk(n > step ? n : step);
    }
    void* p = k_const_at;
    k_const_at += n;
    k_const_left -= n;
    return p;
}
/* Freeze a constant so it is built once instead of per call. A zero mark
   makes k_survives answer no for everything, which is what forces a full
   copy out of the arena rather than the sharing the carry path wants;
   permanent storage is then the only cache a rewind cannot invalidate. The
   copy lands at exact capacity, which is also what keeps k_b_push_mut from
   mutating a shared constant in place — it writes through only when a list
   has room to spare. */
KValue k_caf_freeze(KValue v) {
    if (!k_is_heap(v.tag)) return v;
    KMark none = { NULL, NULL, 0 };
    k_ptrmap_begin(&k_copy_seen);
    k_copy_seen_live = 0;
    size_t need = k_copy_size(v, &none);
    KCarryBuf* buf = k_alloc_perm(sizeof(KCarryBuf));
    buf->data = malloc(need ? need : 16);
    if (!buf->data) { fputs("out of memory\n", stderr); exit(1); }
    buf->cap = need ? need : 16;
    buf->used = 0;
    k_frozen_note(buf->data, buf->cap);
    KCopy cp = { buf, &none, 0, 0 };
    k_ptrmap_begin(&k_copy_map);
    k_copy_map_live = 0;
    return k_deep_copy(v, &cp);
}

/* A constant whose builder answered the very cell it was seeded with has
   computed nothing: freezing it would tie the knot to itself and every later
   demand would chase the loop rather than find a value. The blackhole is the
   answer, and a name sitting in a constructor's slot never reaches here —
   that one answers the record it helped build. */
KValue k_caf_complete(KValue built, KValue seeded) {
    if (built.tag == K_THUNK && seeded.tag == K_THUNK && built.payload == seeded.payload) {
        k_die("a lazy binding demands its own value");
    }
    KValue frozen = k_caf_freeze(built);
    /* A field that stored this cell mid-flight holds the SEED, and the frozen
       value is a copy of what the builder made — so storing the copy in the
       global is not enough, and the field would chase a blackhole nobody ever
       answers. Answering the seed itself is what ties the knot: every holder
       of the cell, copied or not, resolves through the same thunk. */
    if (seeded.tag == K_THUNK) {
        KThunk* t = (KThunk*)seeded.payload;
        t->result = frozen;
        t->forced = K_MEMO_KEPT;
    }
    return frozen;
}


/* Every tag whose payload is a pointer. `K_SUB` was absent until 2026-08-30,
   and `k_cohort_pop` reads this to decide whether a beat's result has to be
   carried out of the arena before the rewind: a returned subtype was taken for
   a scalar, the arena went back to the mark under it, and the caller got a
   dangling `KSub*`. The value read back as whatever the arena was reused for,
   which on the fixtures below is a pointer rendered as a double. `K_THUNK` is
   spelled out at that call site rather than here because a thunk is a promise
   and not a value; every other pointer tag belongs in this list. */
/* Every tag whose payload is a pointer — K_ERR K_STR K_REC K_DESC K_LIST
   K_MAP K_CLOSURE K_BYTES K_SUB — as bit positions in the enum above. `tag`
   is masked to the enum's width so the shift is defined for any input; every
   tag a value can carry is already in range.

   `K_SUB` was absent until 2026-08-30 and its payload is a `KSub*`, so a
   returned subtype was taken for a scalar by `k_cohort_pop` and by
   `k_slots_survive` alike: the arena went back to the mark under it and the
   caller kept a dangling pointer.

   A mask rather than the switch this used to be, because the switch is inlined
   into `k_slots_survive` and through it into `k_copy_size`, which is 36% of
   deepbench. A tenth `case` cost that benchmark 49,528,233 instructions —
   6.14%, all of it inside `k_copy_size`, on both hosts to the instruction —
   on a program that never makes a subtype: `k_survives_x` and `k_ptrmap_at`
   are byte-identical across the change, so it is the same walk compiled
   differently. The mask gives 6,149,160 of that back. Two other shapes were
   measured and were worse: a mask carrying a bounds branch reads 878,869,219,
   and giving `k_slots_survive` its own copy of the switch reads exactly what
   the shared switch does. */
static int k_is_heap(long long tag) {
    /* bit 17 is K_BIG, whose limbs sit in the arena like bytes do */
    return (int)((0x2AFE0U >> (tag & 31)) & 1U);
}

/* stdout is written through stdio's buffer and stderr is not. Text a program
   printed reached a shared pipe after a later error, and a program killed
   while it waited lost all of it, where the interpreter had already written
   both. So the buffer is emptied before anything goes to stderr, before the
   program waits on input, a connection or the clock, and before it starts a
   child that shares the stream. */
static void k_out_settle(void) { fflush(stdout); }

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
    /* every runtime diagnostic opens with this color, so it is where the
       program's own output is put ahead of the error */
    k_out_settle();
    switch (k_color_mode()) {
        case 2: return "\x1b[38;2;240;58;0m";
        case 1: return "\x1b[38;5;202m";
        default: return "";
    }
}

static const char* k_c_dim(void) { return k_color_mode() ? "\x1b[2m" : ""; }
static const char* k_c_off(void) { return k_color_mode() ? "\x1b[0m" : ""; }

/* The interpreter is the oracle and it names what it was handed. A reader
   told only what was wanted has to guess, and the two engines have to agree
   anyway, so a type complaint carries the value that caused it. */
__attribute__((noreturn, noinline)) static void k_die_value(const char* msg, KValue v);
__attribute__((noreturn, noinline)) static void k_die_got(const char* msg, KValue v);

static long long k_itoa(char* buf, long long v);

__attribute__((noreturn, noinline)) void k_die_arity(long long want, long long got) {
    char w[24], g[24];
    k_itoa(w, want);
    k_itoa(g, got);
    fprintf(stderr, "%serror[runtime]:%s this function takes %s argument(s), got %s\n",
            k_c_err(), k_c_off(), w, g);
    exit(1);
}

__attribute__((noreturn, noinline)) void k_die_overload(const char* name) {
    fprintf(stderr, "%serror[runtime]:%s no overload of `%s` matches these arguments\n",
            k_c_err(), k_c_off(), name);
    exit(1);
}

__attribute__((noreturn, noinline)) void k_die(const char* msg) {
    fprintf(stderr, "%serror[runtime]:%s %s\n", k_c_err(), k_c_off(), msg);
    exit(1);
}

/* A function value called with a count it does not take. A group names the
   arms that did not match; a builtin takes exactly one count and says both,
   which is the sentence the interpreter prints. */
__attribute__((noreturn, noinline)) static void k_die_ref_arity(KFnref* r, long long got) {
    if (!r->builtin) k_die_overload(r->name);
    char w[24], g[24], said[128];
    k_itoa(w, r->arity);
    k_itoa(g, got);
    snprintf(said, sizeof said, "`%s` takes %s argument(s), got %s", r->name, w, g);
    k_die(said);
}

/* Hand-rolled lld formatting: the vfprintf machinery showed up hot in
   the encode profile, and a digit loop beats it several times over. */
/* Two digits a time, and the length first — shared by `k_itoa` below and by
   `render_ryu`'s digit extraction further down. Each used to do one 64-bit
   division per digit into a scratch buffer and then reverse it; a pair table
   halves the divisions and writes straight into place, which needs the length
   before the first digit. The ladder climbs from one digit rather than down
   from twenty because the values that reach it are small: a json document's
   floats are short, and so are the integers beside them.

   Nineteen digits is a long long's most, and twenty a uint64_t's, so every
   rung is reachable from `k_itoa` even though `ryu_d2d` never passes more than
   seventeen. The extraction walks DOWN from the length it is given, and a
   length one short would write at a negative index rather than truncate. */
static const char RYU_DIGITS[201] =
    "0001020304050607080910111213141516171819"
    "2021222324252627282930313233343536373839"
    "4041424344454647484950515253545556575859"
    "6061626364656667686970717273747576777879"
    "8081828384858687888990919293949596979899";

static inline int ryu_declen(uint64_t v) {
    if (v < 10ULL) return 1;
    if (v < 100ULL) return 2;
    if (v < 1000ULL) return 3;
    if (v < 10000ULL) return 4;
    if (v < 100000ULL) return 5;
    if (v < 1000000ULL) return 6;
    if (v < 10000000ULL) return 7;
    if (v < 100000000ULL) return 8;
    if (v < 1000000000ULL) return 9;
    if (v < 10000000000ULL) return 10;
    if (v < 100000000000ULL) return 11;
    if (v < 1000000000000ULL) return 12;
    if (v < 10000000000000ULL) return 13;
    if (v < 100000000000000ULL) return 14;
    if (v < 1000000000000000ULL) return 15;
    if (v < 10000000000000000ULL) return 16;
    if (v < 100000000000000000ULL) return 17;
    if (v < 1000000000000000000ULL) return 18;
    if (v < 10000000000000000000ULL) return 19;
    return 20;
}

/* Returns the number of digits written, not counting the terminator, so a
   caller that turns the buffer into a string does not walk it again to find
   out how long it is. `k_render_at` did exactly that on every number it
   rendered: k_str is k_str_n behind a strlen, and the length was already here.

   Digits come out in pairs, straight into place. This used to fill a scratch
   buffer least-significant digit first, one division apiece, and then copy it
   back reversed — two passes over the digits and twice the divisions. */
static long long k_itoa(char* buf, long long v) {
    char* w = buf;
    if (v < 0) *w++ = '-';
    uint64_t u = v < 0 ? (uint64_t)(-(v + 1)) + 1 : (uint64_t)v;
    char* end = w + ryu_declen(u);
    char* o = end;
    while (u >= 100) {
        uint32_t c = (uint32_t)(u % 100);
        u /= 100;
        o -= 2;
        o[0] = RYU_DIGITS[c * 2];
        o[1] = RYU_DIGITS[c * 2 + 1];
    }
    if (u >= 10) {
        o -= 2;
        o[0] = RYU_DIGITS[u * 2];
        o[1] = RYU_DIGITS[u * 2 + 1];
    } else {
        *--o = (char)('0' + (uint32_t)u);
    }
    *end = 0;
    return (long long)(end - buf);
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

/* glibc's memcpy spends a dozen instructions deciding how to move bytes
   before it moves any, which is most of the cost when the run is shorter
   than a couple of words. Two overlapping loads and two overlapping stores
   touch only bytes inside [s, s+n) and [d, d+n), so no caller needs slack
   at either end for this to be safe. */
/* A copy long enough to be libc's, out of line and preserve_most so a caller
   whose short copies are inline words keeps no frame for the long case. */
static __attribute__((noinline, cold, preserve_most)) void k_copy_cold(void* d, const void* s, size_t n) {
    memcpy(d, s, n);
}

static inline __attribute__((always_inline)) void k_copy_short(char* d, const char* s, long long n) {
    if (n >= 16) { k_copy_cold(d, s, (size_t)n); return; }
    if (n >= 8) {
        uint64_t a, b;
        __builtin_memcpy(&a, s, 8);
        __builtin_memcpy(&b, s + n - 8, 8);
        __builtin_memcpy(d, &a, 8);
        __builtin_memcpy(d + n - 8, &b, 8);
    } else if (n >= 4) {
        uint32_t a, b;
        __builtin_memcpy(&a, s, 4);
        __builtin_memcpy(&b, s + n - 4, 4);
        __builtin_memcpy(d, &a, 4);
        __builtin_memcpy(d + n - 4, &b, 4);
    } else if (n >= 2) {
        uint16_t a, b;
        __builtin_memcpy(&a, s, 2);
        __builtin_memcpy(&b, s + n - 2, 2);
        __builtin_memcpy(d, &a, 2);
        __builtin_memcpy(d + n - 2, &b, 2);
    } else if (n == 1) {
        d[0] = s[0];
    }
}

KValue k_int(long long i) { KValue v; v.tag = K_INT; v.payload = i; return v; }
KValue k_bool(long long b) { KValue v; v.tag = b ? K_TRUE : K_FALSE; v.payload = 0; return v; }
KValue k_none(void) { KValue v; v.tag = K_NONE; v.payload = 0; return v; }

