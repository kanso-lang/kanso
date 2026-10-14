/* ---- slice 2: lists, maps, closures, builtins ---- */

/* Dead buffers from proven-unique growth, held by exact power-of-two
   capacity for the next collection that starts at that size. A donated
   buffer is arena storage that would otherwise idle until the run ends, so
   handing it to the next grower collapses the doubling trail from the sum
   of every intermediate size to the live sizes plus one shelf of spares.
   Rewinds flush the shelf: a rewound donation is reclaimed storage, and a
   free list must never outlive the arena region its entries sit in. */
#define K_BUF_CLASSES 12
static KBuf* k_buf_free[K_BUF_CLASSES];
static KBuf* k_buf_of(KValue* items);

static int k_buf_class(long long cap) {
    /* The classes are 4 << c for c below K_BUF_CLASSES, and anything else
       is -1. This was a doubling loop, up to eleven rounds to answer for a
       capacity of 8,192 and five for the decoder's arrays at 64; the trailing
       zeros say the same thing in one instruction. 1,431,562 asks a run on
       runbench, 5.6 rounds each. */
    if (cap < 4 || cap > (4LL << (K_BUF_CLASSES - 1)) || (cap & (cap - 1)))
        return -1;
    return __builtin_ctzll((unsigned long long)cap) - 2;
}

static void k_buf_donate(KValue* items) {
    KBuf* b = k_buf_of(items);
    /* a malloc'd buffer has no business on the arena's shelf */
    if (k_buf_malloced(b)) return;
    int c = k_buf_class(k_buf_cap(b));
    if (c < 0) return;
    b->used = (long long)(intptr_t)k_buf_free[c];
    k_buf_free[c] = b;
    k_buf_dirty = 1;
}

static void k_buf_flush(void) {
    if (!k_buf_dirty) return;
    memset(k_buf_free, 0, sizeof(k_buf_free));
    k_buf_dirty = k_pend_n ? K_BUF_PEND : 0;
}

/* Naming the value is the difference between "something here is wrong" and
   "you passed 5 where a function goes". The interpreter has always named it;
   native said half as much, which the differential law does not allow. */
__attribute__((noreturn, noinline)) static void k_die_not_callable(KValue f) {
    KValue shown = k_render(f, 0);
    KStr* s = k_as_str(shown);
    char said[256];
    snprintf(said, sizeof said, "`%.*s` is not callable", (int)s->len, s->data);
    k_die(said);
}

/* A buffer for a list that outlives the beat it is being appended in. The
   bytes builder has answered this question since the string accumulator
   landed: a header in the arena but below the current mark arrived before this
   iteration, so it is the loop's accumulator rather than one of its
   transients, and its storage must sit where a rewind cannot reach. The sign
   of `cap` records the regime, which is why k_buf_cap normalises it and
   k_buf_donate refuses a negative one to the arena's shelf. */
static __attribute__((noinline, cold, preserve_most)) KValue* k_buf_perm(long long cap) {
    KBuf* b = malloc(sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
    k_perm_live += (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
    if (k_perm_live > k_perm_peak) k_perm_peak = k_perm_live;
    if (!b) { fputs("out of memory\n", stderr); exit(1); }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
        if (K_COUNTING) k_stat_bytes_malloc++;
    }
    k_buf_set_cap(b, cap, 1);
    b->used = 0;
    return (KValue*)(b + 1);
}

/* An accumulator's buffer that has already left the arena, grown again: one
   realloc in place of k_buf_perm, a copy and k_buf_release, the step the byte
   builder's k_bytes_buf_regrow takes. The escape shape of the run program
   grows each of its 3,872 lists five times, and those grows cost 658
   instructions apiece. The old and new buffers are never both held, so the
   permanent peak counts the one. Counted as the malloc and the free it
   replaces, so the counters keep their meaning.

   A map's pairs could take the same step and do not. No program the beat
   analysis brackets threads a map through a beat, so that arm would have no
   golden to hold it. */
static __attribute__((noinline, cold, preserve_most)) KBuf* k_buf_perm_regrow(KBuf* ob,
                                                                             long long cap) {
    long long was = (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)k_buf_cap(ob));
    long long now = (long long)(sizeof(KBuf) + sizeof(KValue) * (size_t)cap);
    KBuf* b = realloc(ob, (size_t)now);
    if (!b) { fputs("out of memory\n", stderr); exit(1); }
    k_perm_live += now - was;
    if (k_perm_live > k_perm_peak) k_perm_peak = k_perm_live;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += now;
        if (K_COUNTING) k_stat_bytes_malloc++;
        if (K_COUNTING) k_stat_bytes_freed++;
    }
    k_buf_set_cap(b, cap, 1);
    return b;
}

/* Does this header predate the beat it is being appended in? Then it is the
   loop's accumulator, not one of its transients. */
static inline int k_outlives_beat(const void* p) {
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return 0;
    KMark* inner = &k_beat_stack[k_beat_depth - 1];
    /* The head block answers without a walk, as k_born_this_beat's does: a
       header there is in the live chain, and it predates the beat exactly
       when the mark sits in the same block above it. The decoder's arrays
       used to outgrow their literal's one slot on the second push, 214,000
       times a run, and every one walked the chain twice to learn the header
       was born a few bytes below the bump pointer; the literal opens with
       room for four now, and the grows that remain still ask here. */
    if (k_blocks) {
        const char* q = (const char*)p;
        if (q >= (const char*)(k_blocks + 1) && q < k_arena)
            return k_blocks == inner->block && q < (const char*)inner->ptr;
    }
    return k_survives(p, NULL) && k_survives(p, inner);
}

/* A bignum written in place into a list or map that predates the beat. The
   analysis licenses an int as an accumulator's element because a word holds
   no pointer, and a bignum does: it was built in the arena, after the mark,
   and the loop's rewind would free it under the accumulator. So it moves to
   permanent storage on the way in. Only a program whose ints outgrow a word
   reaches this, and what it keeps is the bignum's own bytes. */
static __attribute__((noinline, cold)) KValue k_big_perm(KValue v) {
    KBytes* b = (KBytes*)(intptr_t)v.payload;
    KBytes* r = k_alloc_perm(sizeof(KBytes) + (size_t)b->len);
    r->data = (const unsigned char*)(r + 1);
    r->cap = 0;
    r->len = b->len;
    memcpy((unsigned char*)(r + 1), b->data, (size_t)b->len);
    v.payload = k_ptr(r);
    return v;
}

static inline KValue k_big_kept(KValue v, const void* holder) {
    if (__builtin_expect(v.tag == K_BIG, 0) && k_outlives_beat(holder)) return k_big_perm(v);
    return v;
}

static KValue* k_buf(long long cap) {
    int c = k_buf_class(cap);
    if (c >= 0 && k_buf_free[c]) {
        KBuf* b = k_buf_free[c];
        k_buf_free[c] = (KBuf*)(intptr_t)b->used;
        b->used = 0;
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_buf_reuse++;
        return (KValue*)(b + 1);
    }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_buf += (long long)((sizeof(KBuf) + sizeof(KValue) * (size_t)cap + 15) & ~(size_t)15);
    KBuf* b = k_alloc(sizeof(KBuf) + sizeof(KValue) * cap);
    k_buf_set_cap(b, cap, 0);
    b->used = 0;
    return (KValue*)(b + 1);
}

static KBuf* k_buf_of(KValue* items) { return ((KBuf*)items) - 1; }

/* Called before the arena pointer is restored, so the owning headers are
   still readable. A field that no longer holds escaped storage — the list was
   copied out onto the arena, or a second registration of the same field got
   here first — is passed over. */
static void k_permreg_flush_held(int d) {
    for (long long i = 0; i < k_permreg_n[d]; i++) {
        KValue** slot = k_permreg[d][i];
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

/* The beat whose rewind reclaims this header, or -1 for a header older than
   every live mark. */
static __attribute__((noinline, cold, preserve_most)) void k_permreg_add(const void* header, KValue** slot) {
    for (int d = k_beat_depth - 1; d >= 0; d--) {
        if (d >= K_BEAT_MAX) continue;
        if (!k_survives(header, &k_beat_stack[d])) { k_permreg_push(d, slot); return; }
    }
}

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

/* The empty literal is the common one -- the decoder opens 272,000 of them a
   run -- and it is the accumulator a loop pushes into, so its buffer opens
   with room for four. With one slot the second push of every array went
   through k_b_push_grow, a hundred instructions to copy one element into the
   buffer the literal could have had; 225,621 of the run program's 252,499
   grows were that one. A one-slot buffer is also below the size classes the
   free list keeps, so it was garbage the moment it was outgrown, where a
   four-slot one is recycled. glibc's memcpy costs thirteen instructions to
   learn it has nothing to move, hence the inline copy; the same split k_rec
   makes. */
static KValue k_mklist(long long n, KValue* items) {
    KValue* buf = k_buf(n ? n : 4);
    if (n <= 4) {
        for (long long i = 0; i < n; i++) buf[i] = items[i];
    } else {
        k_copy_cold(buf, items, sizeof(KValue) * n);
    }
    return k_list_own(buf, n);
}

KValue k_list_lit(long long n, KValue* items) {
    return k_mklist(n, items);
}

/* The empty literals, which the emitter calls in place of k_list_lit and
   k_map_lit when there is nothing to copy. The decoder opens 272,349 arrays
   and 273,339 objects a run on runbench, and through the general path each
   asked for its size class at run time, took its buffer and its header in
   two bumps, and computed a copy of nothing. Here the class is a constant,
   and a buffer the free list does not have comes out of the same bump as the
   header, header first. A buffer outgrown later goes to the free list at the
   capacity its own header records, which is the buffer's extent and nothing
   more. */
/* An empty list opens with room for six. At four, the decoder's arrays of
   five and six elements each grew on the fifth: bench/large.json holds 893
   of them among its 2,752 arrays, which run one to six elements and one of
   160. Six is not a class the shelf keeps, so the header and the items come
   from one allocation every time, as an empty map's do. */
#define K_LIST_SEED 6
KValue k_list_empty(void) {
    size_t buf_bytes = (sizeof(KBuf) + sizeof(KValue) * K_LIST_SEED + 15) & ~(size_t)15;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_buf += (long long)buf_bytes;
    size_t head_bytes = ((sizeof(KList) + 15) & ~(size_t)15);
    unsigned char* whole = k_alloc(head_bytes + buf_bytes);
    KList* l = (KList*)whole;
    KBuf* b = (KBuf*)(whole + head_bytes);
    k_buf_set_cap(b, K_LIST_SEED, 0);
    b->used = 0;
    l->len = 0;
    l->items = (KValue*)(b + 1);
    KValue v; v.tag = K_LIST; v.payload = k_ptr(l); return v;
}

KValue k_closure(KValue (K_CLOSCC *fn)(void*, KValue), long long arity, long long ncaps, KValue* caps, void* body) {
    KClosure* c = k_alloc(sizeof(KClosure));
    KValue* env = k_alloc(sizeof(KValue) * (ncaps ? ncaps : 1));
    if (ncaps <= 8) {
        for (long long i = 0; i < ncaps; i++) env[i] = caps[i];
    } else {
        k_copy_cold(env, caps, sizeof(KValue) * ncaps);
    }
    c->fn = fn; c->env = env; c->ncaps = (int)ncaps; c->arity = (int)arity; c->body = body;
    KValue v; v.tag = K_CLOSURE; v.payload = k_ptr(c); return v;
}

/* A lambda that captures nothing is the same value every evaluation, the way
   a string literal is, so it builds once into a permanent slot instead of a
   closure and an empty environment per visit. `escape_able_2` in encodebench
   built 709,200 of them, two arena allocations each, to hand `list/fold` a
   closure that never differed.

   Permanent rather than arena storage for the reason k_str_lit gives: the
   slot outlives every beat, and a rewind cannot reach what malloc handed out.
   The environment is allocated even though ncaps is zero, because the carry
   path reads `cl->env` unconditionally when it sizes a copy. */
KValue k_closure_lit(KValue (K_CLOSCC *fn)(void*, KValue), long long arity, KValue* slot) {
    if (slot->tag != K_CLOSURE) {
        KClosure* c = k_alloc_const(sizeof(KClosure));
        KValue* env = k_alloc_const(sizeof(KValue));
        env->tag = K_INT; env->payload = 0;
        c->fn = fn; c->env = env; c->ncaps = 0; c->arity = (int)arity; c->body = 0;
        slot->tag = K_CLOSURE; slot->payload = k_ptr(c);
    }
    return *slot;
}

KValue k_fnref(void* described) {
    KValue v; v.tag = K_FNREF; v.payload = (long long)(intptr_t)described; return v;
}

KValue k_env_get(void* env, long long i) { return ((KValue*)env)[i]; }

/* A group handed out as a value is entered with its failing arguments, as a
   direct call is. The group's own guard propagates the leftmost failure and
   names the group in the trace, and an `(err _)` arm takes the failure it was
   written for. A group mentioned as a value has every parameter widened to
   any value (infer.rs), so its wrapper unboxes nothing and the guard sees
   every argument as the program passed it. The dispatchers below used to
   answer for the group before entering it, which merged two failures a
   direct call reports one of and never reached an err arm at all. A builtin
   handed out as a value merges every failing argument, as the builtin itself
   does. */
static int k_any_failure(int n, const KValue* xs) {
    for (int i = 0; i < n; i++) {
        if (!k_not_failure(xs[i])) return 1;
    }
    return 0;
}

/* `foo()` runs a value that was waiting to be called — what `&` leaves when it
   has supplied every argument an arm takes. No arguments arrive, so there is
   nothing to propagate but the callable itself. */
KValue k_call0(KValue f) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        if (c->arity < 0) return k_partial_run(c);
        if (c->arity != 0) k_die_arity(c->arity, 0);
        return ((KValue(K_CLOSCC *)(void*))c->fn)(c->env);
    }
    if (f.tag == K_FNREF) {
        KFnref* r = (KFnref*)(intptr_t)f.payload;
        if (r->arity != 0) k_die_ref_arity(r, 0);
        return ((KValue(*)(void))r->fn)();
    }
    k_die_not_callable(f);
    return k_none();
}

KValue k_call1(KValue f, KValue a) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        if (c->arity < 0) return k_partial_apply(c, 1, &a);
        if (c->arity != 1) k_die_arity(c->arity, 1);
        if (!k_not_failure(a)) return a;
        return c->fn(c->env, a);
    }
    if (f.tag == K_FNREF) {
        KFnref* r = (KFnref*)(intptr_t)f.payload;
        if (r->arity != 1) k_die_ref_arity(r, 1);
        return ((KValue(*)(KValue))r->fn)(a);
    }
    k_die_not_callable(f);
    return k_none();
}

/* Calling a lambda value with more than one argument. The closure's fn pointer
   is stored generically; cast it to the arity the call site knows, which is
   only sound once the value agrees it takes that many — a cast through the
   wrong signature leaves the extra argument in a register the callee never
   reads, and the call answers as though it were never written.

   The ORDER is the interpreter's, and it took a fixture to get right. A
   failing callable answers first, then the callable is asked whether it is
   callable at all and whether its arity matches, and only then do failing
   arguments propagate. Testing the arguments first — which these did until
   2026-09-03 — makes a wrong-arity call answer with the failure and exit 0
   where the oracle dies, and a call on a value that is not callable do the
   same. A wrong-arity call is a broken program, and a value that happens to
   fail must not be what hides the break. `eval.rs` refuses the callable in
   `call` and the count in `call_closure`, both above its argument test;
   tests/golden/runtime/a_wrong_arity_call_carrying_a_failing_argument.kso is
   the program that reaches it. */
KValue k_call2(KValue f, KValue a, KValue b) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        if (c->arity < 0) { KValue fresh[2] = { a, b }; return k_partial_apply(c, 2, fresh); }
        if (c->arity != 2) k_die_arity(c->arity, 2);
        if (!k_not_failure(a) || !k_not_failure(b)) return k_both_or_either(a, b);
        return ((KValue(K_CLOSCC *)(void*, KValue, KValue))c->fn)(c->env, a, b);
    }
    if (f.tag == K_FNREF) {
        KFnref* r = (KFnref*)(intptr_t)f.payload;
        if (r->arity != 2) k_die_ref_arity(r, 2);
        if (r->builtin && (!k_not_failure(a) || !k_not_failure(b))) return k_both_or_either(a, b);
        return ((KValue(*)(KValue, KValue))r->fn)(a, b);
    }
    k_die_not_callable(f);
    return k_none();
}

KValue k_call3(KValue f, KValue a, KValue b, KValue c) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* cl = (KClosure*)(intptr_t)f.payload;
        if (cl->arity < 0) { KValue fresh[3] = { a, b, c }; return k_partial_apply(cl, 3, fresh); }
        if (cl->arity != 3) k_die_arity(cl->arity, 3);
        KValue xs[3] = { a, b, c };
        if (k_any_failure(3, xs)) return k_failed_of(3, xs);
        /* In words, not as three KValues. C hands a struct that no longer
           fits the argument registers to the stack whole, and the emitted
           body, which takes its arguments as words, reads the first half of
           that struct from the last register: env and two KValues fill five
           of six, so c arrived torn. A word is a word on both sides. */
        typedef long long W;
        return ((KValue(K_CLOSCC *)(void*, W, W, W, W, W, W))cl->fn)(
            cl->env, a.tag, a.payload, b.tag, b.payload, c.tag, c.payload);
    }
    if (f.tag == K_FNREF) {
        KFnref* r = (KFnref*)(intptr_t)f.payload;
        if (r->arity != 3) k_die_ref_arity(r, 3);
        KValue xs[3] = { a, b, c };
        if (r->builtin && k_any_failure(3, xs)) return k_failed_of(3, xs);
        return ((KValue(*)(KValue, KValue, KValue))r->fn)(a, b, c);
    }
    k_die_not_callable(f);
    return k_none();
}

KValue k_call4(KValue f, KValue a, KValue b, KValue c, KValue d) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* cl = (KClosure*)(intptr_t)f.payload;
        if (cl->arity < 0) { KValue fresh[4] = { a, b, c, d }; return k_partial_apply(cl, 4, fresh); }
        if (cl->arity != 4) k_die_arity(cl->arity, 4);
        KValue xs[4] = { a, b, c, d };
        if (k_any_failure(4, xs)) return k_failed_of(4, xs);
        typedef long long W;
        return ((KValue(K_CLOSCC *)(void*, W, W, W, W, W, W, W, W))cl->fn)(
            cl->env, a.tag, a.payload, b.tag, b.payload, c.tag, c.payload, d.tag, d.payload);
    }
    if (f.tag == K_FNREF) {
        KFnref* r = (KFnref*)(intptr_t)f.payload;
        if (r->arity != 4) k_die_ref_arity(r, 4);
        KValue xs[4] = { a, b, c, d };
        if (r->builtin && k_any_failure(4, xs)) return k_failed_of(4, xs);
        return ((KValue(*)(KValue, KValue, KValue, KValue))r->fn)(a, b, c, d);
    }
    k_die_not_callable(f);
    return k_none();
}

/* A partial over a VALUE: `&f 2` where `f` is a parameter, a lambda, a
   builtin or a bound name rather than a declared group. The emitter cannot
   lower it to a lambda, because a lambda fixes its parameter count where it
   is written and this one settles it when the arguments arrive — the
   interpreter's rule, which tests/partial.rs specifies. So it is a closure
   with no body: arity -1 marks it, and the environment holds the callee
   first and the arguments held so far after it, ncaps counting both. Every
   dispatcher above tests arity before it calls, so a body-less closure never
   reaches a call through the fast arm the emitter inlines; it arrives here.

   A new call gathers held and fresh arguments and asks the callee's arity: a
   closure's is its field, a fnref's its field, a partial's its callee's less
   what it holds, and a value that is not callable answers nothing, which is
   what the interpreter's arities_of answers, and the partial grows. Equal
   dispatches through k_callN, which orders the failure tests the way the
   oracle does; short grows; past it dies naming the arity, in the oracle's
   words. `()` on one runs it or says how many it is short by. */
static KValue k_partial_build(KValue callee, long long n, KValue* args) {
    KClosure* c = k_alloc(sizeof(KClosure));
    KValue* env = k_alloc(sizeof(KValue) * (1 + n));
    env[0] = callee;
    for (long long i = 0; i < n; i++) env[1 + i] = args[i];
    c->fn = 0; c->env = env; c->ncaps = (int)(1 + n); c->arity = -1; c->body = 0;
    KValue v; v.tag = K_CLOSURE; v.payload = k_ptr(c); return v;
}

/* `&f a b` evaluates the callee and then each argument, and the first
   failure among them is the answer, callee first — the interpreter's order
   in its App-with-Partial arm. A bare `&f` wraps whatever `f` holds. */
KValue k_partial0(KValue f) { return k_partial_build(f, 0, 0); }
KValue k_partial1(KValue f, KValue a) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    KValue args[1] = { a };
    return k_partial_build(f, 1, args);
}
KValue k_partial2(KValue f, KValue a, KValue b) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    KValue args[2] = { a, b };
    return k_partial_build(f, 2, args);
}
KValue k_partial3(KValue f, KValue a, KValue b, KValue c) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (!k_not_failure(c)) return c;
    KValue args[3] = { a, b, c };
    return k_partial_build(f, 3, args);
}
KValue k_partial4(KValue f, KValue a, KValue b, KValue c, KValue d) {
    if (!k_not_failure(f)) return f;
    if (!k_not_failure(a)) return a;
    if (!k_not_failure(b)) return b;
    if (!k_not_failure(c)) return c;
    if (!k_not_failure(d)) return d;
    KValue args[4] = { a, b, c, d };
    return k_partial_build(f, 4, args);
}

/* The count a callee answers to, or -1 when it answers none. A partial
   answers its own callee's count less what it holds, as the interpreter's
   arities_of does, so `&m` over `m = &k 2` runs when one more arrives rather
   than growing. A value that is not callable answers none. */
static long long k_callee_arity(KValue callee) {
    if (callee.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)callee.payload;
        if (c->arity >= 0 || c->fn) return c->arity;
        long long inner = k_callee_arity(((KValue*)c->env)[0]);
        long long held = c->ncaps - 1;
        return inner >= held ? inner - held : -1;
    }
    if (callee.tag == K_FNREF) return ((KFnref*)(intptr_t)callee.payload)->arity;
    return -1;
}

/* A group value called with five or more arguments, which only a partial
   reaches. The order is k_call4's: the count, then the group, entered with
   whatever it was handed. Past eight the wrapper takes the arguments as
   one array, since no cast here can name every width. */
static KValue k_call_ref_wide(KFnref* r, long long n, KValue* a) {
    typedef KValue V;
    if (r->arity != n) k_die_ref_arity(r, n);
    if (r->builtin && k_any_failure((int)n, a)) return k_failed_of((int)n, a);
    switch (n) {
        case 5: return ((V(*)(V, V, V, V, V))r->fn)(a[0], a[1], a[2], a[3], a[4]);
        case 6: return ((V(*)(V, V, V, V, V, V))r->fn)(a[0], a[1], a[2], a[3], a[4], a[5]);
        case 7: return ((V(*)(V, V, V, V, V, V, V))r->fn)(a[0], a[1], a[2], a[3], a[4], a[5], a[6]);
        case 8: return ((V(*)(V, V, V, V, V, V, V, V))r->fn)(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
        default: return ((V(*)(V*))r->fn)(a);
    }
}

static KValue k_dispatch_n(KValue callee, long long n, KValue* args) {
    switch (n) {
        case 0: return k_call0(callee);
        case 1: return k_call1(callee, args[0]);
        case 2: return k_call2(callee, args[0], args[1]);
        case 3: return k_call3(callee, args[0], args[1], args[2]);
        case 4: return k_call4(callee, args[0], args[1], args[2], args[3]);
    }
    /* Past four the arguments come from a partial: a call site hands a value
       at most four, so a count this high is a partial's held arguments and
       its fresh ones together. A partial over a declared group of five or
       more is held over the group's value, which names the group when it
       turns down a failure, as k_call4 does. */
    if (!k_not_failure(callee)) return callee;
    if (callee.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)callee.payload;
        if (c->arity < 0) return k_partial_apply(c, n, args);
        k_die_arity(c->arity, n);
    }
    if (callee.tag == K_FNREF) return k_call_ref_wide((KFnref*)(intptr_t)callee.payload, n, args);
    k_die_not_callable(callee);
    return k_none();
}

static KValue k_partial_apply(KClosure* p, long long n, KValue* fresh) {
    KValue* env = (KValue*)p->env;
    KValue callee = env[0];
    long long held = p->ncaps - 1;
    long long total = held + n;
    KValue* all = k_alloc(sizeof(KValue) * (total ? total : 1));
    for (long long i = 0; i < held; i++) all[i] = env[1 + i];
    for (long long i = 0; i < n; i++) all[held + i] = fresh[i];
    long long arity = k_callee_arity(callee);
    if (arity >= 0 && total == arity) return k_dispatch_n(callee, total, all);
    if (arity >= 0 && total > arity) {
        char t[24], a[24], said[128];
        k_itoa(t, total);
        k_itoa(a, arity);
        snprintf(said, sizeof said, "no %s-argument arm of `<fn>` (arms take %s)", t, a);
        k_die(said);
    }
    return k_partial_build(callee, total, all);
}

/* `p()`: the call the partial was built for, or the count it is still
   short by — zero when it holds more than the callee takes, as the oracle
   says it. */
static KValue k_partial_run(KClosure* p) {
    KValue* env = (KValue*)p->env;
    long long held = p->ncaps - 1;
    long long arity = k_callee_arity(env[0]);
    if (arity >= 0 && held == arity) return k_dispatch_n(env[0], held, env + 1);
    k_die_arity(arity > held ? arity - held : 0, 0);
    return k_none();
}

static int k_key_cmp(KValue a, KValue b) {
    a = k_sub_base(a);
    b = k_sub_base(b);
    /* every int ranks with the ints, a bignum among them */
    if (K_IS_INT(a) && K_IS_INT(b)) return k_int_cmp(a, b);
    if (a.tag != b.tag) return K_IS_INT(a) ? -1 : K_IS_INT(b) ? 1 : (a.tag < b.tag ? -1 : 1);
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
/* How much the view holds is a property of the view, so it is stored with
   the buffer rather than in the map header. A header is allocated for every
   map a program builds and most are never read at all — a decode of the
   188 kb board allocates 828,000 of them and builds no view — so a field
   there is paid for by every map to serve the few that are read. One slot
   in front of the buffer is paid only by those few. */
static KValue* k_view_alloc(long long cap) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_view_allocs++;
    char* raw = malloc(sizeof(KValue) + sizeof(KValue) * 2 * (size_t)cap);
    if (!raw) { fputs("out of memory\n", stderr); exit(1); }
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_allocs++;
        k_stat_alloc_bytes += (long long)(sizeof(KValue) + sizeof(KValue) * 2 * (size_t)cap);
    }
    *(long long*)raw = cap;
    ((long long*)raw)[1] = 0;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        k_stat_held_live += (long long)(sizeof(KValue) + sizeof(KValue) * 2 * (size_t)cap);
        if (k_stat_held_live > k_stat_held_peak) k_stat_held_peak = k_stat_held_live;
    }
    return (KValue*)(raw + sizeof(KValue));
}

static long long k_view_cap(KValue* view) {
    return *(long long*)((char*)view - sizeof(KValue));
}

/* A view can carry a SIDE: keys put since the view was built, held sorted
   in a buffer of their own, so that a key landing in the middle of a large
   view costs a move within the side rather than within the view. Counting
   40,000 distinct keys spent 4.9 seconds moving the view one slot to the
   right at a time, where the interpreter took 0.2.

   The side lives in the second word of the view's header slot, which the
   capacity leaves unused, and the map says it has one by setting the top bit
   of its view pointer. So a header pays nothing for it, and read as a signed
   number the pointer answers both of a reader's questions at once: positive
   is a view with no side, zero is no view, negative is a view with a side.
   `keys` and `values` ask on every call, and the low bit cost them two
   instructions a call, 706,000 on the run program. A 64-bit address has its
   top bit clear, which the mark needs. A 32-bit one may not, so wasm32 marks
   the low bit and asks the two questions separately. In a side buffer the
   same word holds the side's length. */
#if UINTPTR_MAX > 0xffffffffu
#define K_VIEW_SIDE ((uintptr_t)1 << 63)
#else
#define K_VIEW_SIDE ((uintptr_t)1)
#endif
static inline long long* k_view_word(KValue* view) {
    return (long long*)((char*)view - sizeof(KValue)) + 1;
}
static inline KValue* k_view_untagged(KValue* view) {
    return (KValue*)((uintptr_t)view & ~K_VIEW_SIDE);
}
static inline KValue* k_view_side(KValue* main) {
    return (KValue*)(intptr_t)*k_view_word(main);
}

static void k_view_free(KValue* view) {
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) {
        if (K_COUNTING) k_stat_view_frees++;
        long long cap = k_view_cap(view);
        k_stat_held_live -= (long long)(sizeof(KValue) + sizeof(KValue) * 2 * (size_t)cap);
    }
    free((char*)view - sizeof(KValue));
}

/* A map's own view, with the side it keeps, empty or not. Never the pairs:
   a view that is the pairs has no header of its own to keep a side in, and
   every caller asks first. */
static void k_map_view_free(KValue* view) {
    KValue* main = k_view_untagged(view);
    KValue* side = k_view_side(main);
    if (side) k_view_free(side);
    k_view_free(main);
}

/* The build runs once per map and `entries` asks 248,490 times a run on the
   run program, so the build saves what it touches and the ask keeps no
   frame; k_map_sorted then inlines into its callers. */
/* Fold the side into the view: two sorted runs with no key in common, since
   a key goes to the side only when neither holds it. The merge runs from the
   back, so a view with room for both takes it where it stands; one without
   grows to twice what it will hold, as the end-append below does, and a map
   taking keys in no order allocates a view about as often as one taking
   them in order. */
static void k_map_view_merge(KMap* m) {
    KValue* main = k_view_untagged(m->sorted);
    KValue* side = k_view_side(main);
    long long n = m->sorted_len, sl = *k_view_word(side);
    KValue* out = main;
    if (n + sl > k_view_cap(main)) {
        out = k_view_alloc((n + sl) * 2);
        memcpy(out, main, sizeof(KValue) * 2 * (size_t)n);
    }
    long long i = n - 1, j = sl - 1, w = n + sl - 1;
    while (j >= 0) {
        if (i >= 0 && k_key_cmp(out[i * 2], side[j * 2]) > 0) {
            out[w * 2] = out[i * 2];
            out[w * 2 + 1] = out[i * 2 + 1];
            i--;
        } else {
            out[w * 2] = side[j * 2];
            out[w * 2 + 1] = side[j * 2 + 1];
            j--;
        }
        w--;
    }
    /* the side stays, emptied, for the next key out of order */
    *k_view_word(side) = 0;
    if (out != main) {
        k_view_free(main);
        *k_view_word(out) = (long long)(intptr_t)side;
    }
    m->sorted = out;
    m->sorted_len = n + sl;
}

static __attribute__((noinline, preserve_most)) void k_map_sort_build(KMap* m) {
    if ((uintptr_t)m->sorted & K_VIEW_SIDE) {
        k_map_view_merge(m);
        return;
    }
    {
        long long n = m->len;
        /* PAIRS ALREADY IN ORDER ARE THEIR OWN VIEW. A decoded object whose
           keys arrived ascending with none repeated has nothing to sort and
           nothing to drop, and a copy of its pairs is the whole of what the
           view would be: the run program's doc held 728,040 bytes of such
           copies for as long as it lived. The view then points at the pairs.
           Nothing writes through a view but the in-place put, which keeps the
           alias only while the keys stay ascending and drops it otherwise,
           and the growth path, which moves it with the pairs; every free of a
           view asks first whether it is one. Pairs are frontier-shared, but a
           map that shares them sees only its own prefix, which no append
           changes. */
        long long a = 1;
        while (a < n && k_key_cmp(m->pairs[(a - 1) * 2], m->pairs[a * 2]) < 0) a++;
        if (a >= n) {
            m->sorted = m->pairs;
            m->sorted_len = n;
            return;
        }
        /* The view lives outside the arena for the same reason a builder's
           buffer does: a below-mark map header must never hold a pointer a
           rewind can free. That retires the registry sweep each rewind paid
           to null exactly such pointers — with nothing arena-backed to
           dangle, nothing registers and the sweep walks an empty table. A
           transient map's view leaks with the map; the trade is recorded in
           the log beside the numbers. */
        KValue* out = k_view_alloc(n ? n : 1);
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
        /* Born this beat means the header dies at the rewind, which is the
           only moment anything knows this view has become garbage. */
        k_viewreg_own(m);
    }
}

/* No view yet, or one with a side to fold in: either way the build runs. */
static inline int k_view_stale(KValue* view) {
    uintptr_t p = (uintptr_t)view;
#if UINTPTR_MAX > 0xffffffffu
    return (intptr_t)p <= 0;
#else
    return !p || (p & K_VIEW_SIDE);
#endif
}

static inline KValue* k_map_sorted(KMap* m, long long* out_len) {
    if (__builtin_expect(k_view_stale(m->sorted), 0)) k_map_sort_build(m);
    if (out_len) *out_len = m->sorted_len;
    return m->sorted;
}

KValue k_map_lit(long long n, KValue* flat_pairs) {
    /* literal keys arrive sorted and unique from the parser; k_map_sorted
       still recomputes on first read, cheaply (already sorted, no dups). */
    KMap* m = k_alloc(sizeof(KMap));
    /* An empty literal is a seed, not a value: nothing writes `{}` except to
       put into it, and at two slots the second put already had to grow. It
       starts at the floor the grow path itself uses, so a small object is
       built without a reallocation. A literal with keys in it is a finished
       value and gets exactly the room it needs. */
    m->pairs = k_buf(n ? 2 * n : 8);
    /* The empty literal is the common one -- the decoder opens 273,000 of
       them a run -- and glibc's memcpy costs thirteen instructions to learn
       it has nothing to move. The same split k_rec and k_mklist make. */
    if (n <= 2) {
        for (long long i = 0; i < 2 * n; i++) m->pairs[i] = flat_pairs[i];
    } else {
        k_copy_cold(m->pairs, flat_pairs, sizeof(KValue) * 2 * n);
    }
    k_buf_of(m->pairs)->used = 2 * n;
    m->len = n;
    m->sorted = NULL;
    m->sorted_len = 0;
    KValue mv; mv.tag = K_MAP; mv.payload = k_ptr(m); return mv;
}

/* `{}`: k_list_empty says why this is not k_map_lit(0).

   It opens with room for five pairs. At four, every fifth key grew the map:
   the decoder's objects run one to five keys, and each five-key object of
   bench/large.json paid a grow, a copy and a donation, 56,430 times a run at
   about 190 instructions each. Ten slots is not a class the shelf keeps, so
   the header and the pairs come from one allocation every time; a map that
   outgrows them moves to sixteen slots, which is. */
#define K_MAP_SEED 10
KValue k_map_empty(void) {
    size_t buf_bytes = (sizeof(KBuf) + sizeof(KValue) * K_MAP_SEED + 15) & ~(size_t)15;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_buf += (long long)buf_bytes;
    size_t head_bytes = ((sizeof(KMap) + 15) & ~(size_t)15);
    unsigned char* whole = k_alloc(head_bytes + buf_bytes);
    KMap* m = (KMap*)whole;
    KBuf* b = (KBuf*)(whole + head_bytes);
    k_buf_set_cap(b, K_MAP_SEED, 0);
    b->used = 0;
    m->pairs = (KValue*)(b + 1);
    m->len = 0;
    m->sorted = NULL;
    m->sorted_len = 0;
    KValue mv; mv.tag = K_MAP; mv.payload = k_ptr(m); return mv;
}

/* Only an int or a string keys a map, which is what the interpreter has
   always said. Native used to take anything, so a float made a key that
   nothing could look up by equality — `1 == 1.0` is false — while sorting
   next to its integer twin. */
static void k_check_map_key(KValue key) {
    if (K_IS_INT(key) || key.tag == K_STR) return;
    KValue shown = k_render(key, 1);
    KStr* s = k_as_str(shown);
    char said[256];
    snprintf(said, sizeof said, "%.*s is not usable as a map key", (int)s->len, s->data);
    k_die(said);
}

KValue k_b_put(KValue mv, KValue key, KValue val);

/* The same put at a site the linearity analysis proved unique: the map is
   moved into this call, so its header can carry the new length instead of a
   fresh one being allocated. The sorted view is per-header state and the
   header now describes different contents, so it resets; put never builds
   one, and a later read rebuilds it. */
/// A key already present is written where it sits.
///
/// Storage otherwise grew with the number of writes rather than the number
/// of keys: a counting loop over a hundred keys held ten thousand pairs
/// after ten thousand puts, deduped only when something read the map. That
/// is unbounded growth for a bounded key set. Replacing in place also keeps
/// the sorted view alive — its value slot is patched too, so the read that
/// follows does not rebuild it.
/* The value slot for `key` in a sorted run of n pairs, or NULL. */
static KValue* k_view_search(KValue* run, long long n, KValue key) {
    long long lo = 0, hi = n - 1;
    while (lo <= hi) {
        long long mid = lo + (hi - lo) / 2;
        int c = k_key_cmp(run[mid * 2], key);
        if (c == 0) return &run[mid * 2 + 1];
        if (c < 0) {
            lo = mid + 1;
        } else {
            hi = mid - 1;
        }
    }
    return NULL;
}

/* The value slot for `key` in a map whose view is built, searching the view
   and then its side, so a read between two puts merges nothing. */
static KValue* k_map_find(KMap* m, KValue key) {
    if (!((uintptr_t)m->sorted & K_VIEW_SIDE)) return k_view_search(m->sorted, m->sorted_len, key);
    KValue* main = k_view_untagged(m->sorted);
    KValue* hit = k_view_search(main, m->sorted_len, key);
    if (hit) return hit;
    KValue* side = k_view_side(main);
    return k_view_search(side, *k_view_word(side), key);
}

static int k_map_replace(KMap* m, KValue key, KValue val) {
    /* Only when a view is already built. Without one there is no cheap way
       to know whether the key is present, and appending is correct anyway
       (the view keeps the last write) — so a write-only loop pays nothing
       and never scans for a key that is not there. */
    if (!m->sorted) {
        return 0;
    }
    /* the view is sorted, so absence costs a binary search rather than a
       walk: a loop writing keys it has never written pays nothing here */
    KValue* slot = k_map_find(m, key);
    if (!slot) {
        return 0;
    }
    *slot = val;
    /* the pairs log holds the same key, last write winning, so the newest
       occurrence is the one a rebuild would keep */
    for (long long i = m->len - 1; i >= 0; i--) {
        if (k_key_cmp(m->pairs[i * 2], key) == 0) {
            m->pairs[i * 2 + 1] = val;
            k_note_if_carried(val);
            return 1;
        }
    }
    return 1;
}

/// A key not yet present is inserted into the view rather than discarding it.
///
/// Discarding meant the next read rebuilt the whole thing: a loop inserting
/// ten thousand keys sorted ten thousand times and malloc'd a fresh view on
/// each, which is where its time and its peak both went. Insertion keeps the
/// order the view already has, so the sort never runs again and the buffer is
/// grown by doubling instead of reallocated per read.
/* Where `key` goes in a sorted run of n pairs: the first slot whose key is
   greater. */
static long long k_view_place(KValue* run, long long n, KValue key) {
    long long lo = 0, hi = n;
    while (lo < hi) {
        long long mid = lo + (hi - lo) / 2;
        if (k_key_cmp(run[mid * 2], key) < 0) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    return lo;
}

/* A view this small takes the key in place: moving at most 64 pairs costs
   less than keeping a second run to search. */
#define K_VIEW_DIRECT 64

static void k_map_view_insert_built(KMap* m, KValue key, KValue val) {
    KValue* main = k_view_untagged(m->sorted);
    long long n = m->sorted_len;
    int sided = ((uintptr_t)m->sorted & K_VIEW_SIDE) != 0;
    /* a key past the view's last goes on its end, whatever its size, so a
       map filled in ascending order never starts a side */
    int last = n == 0 || k_key_cmp(main[(n - 1) * 2], key) < 0;
    if (!sided && (last || n < K_VIEW_DIRECT)) {
        if (n + 1 > k_view_cap(main)) {
            KValue* grown = k_view_alloc(n * 2 + 1);
            memcpy(grown, main, sizeof(KValue) * 2 * (size_t)n);
            *k_view_word(grown) = *k_view_word(main);
            k_view_free(main);
            main = grown;
            m->sorted = grown;
        }
        long long at = last ? n : k_view_place(main, n, key);
        memmove(&main[(at + 1) * 2], &main[at * 2], sizeof(KValue) * 2 * (size_t)(n - at));
        main[at * 2] = key;
        main[at * 2 + 1] = val;
        m->sorted_len++;
        return;
    }
    KValue* side;
    long long sl;
    side = k_view_side(main);
    if (sided) {
        sl = *k_view_word(side);
    } else {
        if (!side) {
            side = k_view_alloc(16);
            *k_view_word(main) = (long long)(intptr_t)side;
        }
        sl = 0;
        m->sorted = (KValue*)((uintptr_t)main | K_VIEW_SIDE);
    }
    if (sl + 1 > k_view_cap(side)) {
        KValue* grown = k_view_alloc(sl * 2);
        memcpy(grown, side, sizeof(KValue) * 2 * (size_t)sl);
        k_view_free(side);
        side = grown;
        *k_view_word(main) = (long long)(intptr_t)side;
    }
    long long at = k_view_place(side, sl, key);
    memmove(&side[(at + 1) * 2], &side[at * 2], sizeof(KValue) * 2 * (size_t)(sl - at));
    side[at * 2] = key;
    side[at * 2 + 1] = val;
    sl++;
    *k_view_word(side) = sl;
    /* past the square root of the view, a side costs more to insert into
       and to search than one merge of the two costs spread over its keys */
    if (sl * sl > n) k_map_view_merge(m);
}

/* A map with no view has nothing to insert into, and that is every map a
   decode builds: 1,254,150 writes on the decode benchmark, not one of them
   into a view. The absent-view test belongs at the write, not behind a call
   the write always makes. */
static inline void k_map_view_insert(KMap* m, KValue key, KValue val) {
    if (!m->sorted) return;
    if (m->sorted == m->pairs) {
        /* the pair is already appended at len - 1: still in order, the view
           is one longer and nothing is copied */
        if (m->sorted_len == 0 || k_key_cmp(m->pairs[(m->len - 2) * 2], key) < 0) {
            m->sorted_len = m->len;
            return;
        }
        /* out of order, the view becomes a copy of the ordered prefix, at the
           room a view built on the first read would have grown to by now --
           1, 3, 7, 15, the insert's own series -- and the key goes in as it
           would have. Sized any other way the series overshoots: dropping the
           view and letting the next read build one at its exact size left a
           map taking 800 keys in descending order holding 73,696 bytes of view
           where it had held 49,120. */
        long long room = 1;
        while (room < m->sorted_len) room = room * 2 + 1;
        KValue* own = k_view_alloc(room);
        memcpy(own, m->sorted, sizeof(KValue) * 2 * (size_t)m->sorted_len);
        m->sorted = own;
        k_viewreg_own(m);
    }
    k_map_view_insert_built(m, key, val);
}

KValue k_b_put_mut(KValue mv, KValue key, KValue val) {
    if (__builtin_expect(!k_not_failure(mv) || !k_not_failure(key) || !k_not_failure(val), 0)) return k_failed3(mv, key, val);
    if (mv.tag != K_MAP) k_die("put takes a map, a key, and a value");
    k_check_map_key(key);
    KMap* m = k_as_map(mv);
    val = k_big_kept(val, m);
    if (k_map_replace(m, key, val)) {
        return mv;
    }
    KBuf* buf = k_buf_of(m->pairs);
    if (buf->used == m->len * 2 && m->len * 2 + 2 <= k_buf_cap(buf)) {
        m->pairs[m->len * 2] = key;
        m->pairs[m->len * 2 + 1] = val;
        k_note_if_carried(key);
        k_note_if_carried(val);
        buf->used += 2;
        m->len++;
        if (K_COUNTING) k_stat_put_mut_fast++;
        k_map_view_insert(m, key, val);
        return mv;
    }
    /* growth at a proven-unique site: the map keeps its header, the pairs
       move to a bigger buffer, and the outgrown one goes to the shelf */
    {
        if (K_COUNTING) k_stat_put_mut_grow++;
        long long need = 2 * (m->len + 1);
        long long cap = 8;
        while (cap < need) cap <<= 1;
        /* An accumulator's pairs go outside the arena for the same reason a
           list's items do: the loop's rewind reclaims the iteration's garbage
           without reaching what the iteration was building. */
        int perm = k_outlives_beat(m);
        KValue* np = perm ? k_buf_perm(cap) : k_buf(cap);
        memcpy(np, m->pairs, sizeof(KValue) * 2 * m->len);
        np[m->len * 2] = key;
        np[m->len * 2 + 1] = val;
        k_buf_of(np)->used = m->len * 2 + 2;
        if (m->sorted == m->pairs) m->sorted = np;
        if (k_buf_malloced(k_buf_of(m->pairs))) {
            free(k_buf_of(m->pairs));
        } else {
            k_buf_donate(m->pairs);
        }
        m->pairs = np;
        m->len++;
        if (perm) k_permreg_add(m, &m->pairs);
        k_map_view_insert(m, key, val);
        return mv;
    }
}

KValue k_b_put(KValue mv, KValue key, KValue val) {
    if (__builtin_expect(!k_not_failure(mv) || !k_not_failure(key) || !k_not_failure(val), 0)) return k_failed3(mv, key, val);
    if (mv.tag != K_MAP) k_die("put takes a map, a key, and a value");
    k_check_map_key(key);
    KMap* m = k_as_map(mv);
    KBuf* buf = k_buf_of(m->pairs);
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_sh_map += (long long)sizeof(KMap);
    KMap* out = k_alloc(sizeof(KMap));
    KValue ov; ov.tag = K_MAP; ov.payload = k_ptr(out);
    out->sorted = NULL;
    out->sorted_len = 0;
    if (buf->used == m->len * 2 && m->len * 2 + 2 <= k_buf_cap(buf)) {
        /* frontier-owned: claim the next pair slot in place (O(1)), leaving
           the key unsorted and any duplicate to be resolved on read */
        m->pairs[m->len * 2] = key;
        m->pairs[m->len * 2 + 1] = val;
        k_note_if_carried(key);
        k_note_if_carried(val);
        buf->used += 2;
        out->len = m->len + 1;
        out->pairs = m->pairs;
        return ov;
    }
    long long need = 2 * (m->len + 1);
    long long cap = 4;
    while (cap < need) cap <<= 1;
    KValue* np = k_buf(cap);
    memcpy(np, m->pairs, sizeof(KValue) * 2 * m->len);
    np[m->len * 2] = key;
    np[m->len * 2 + 1] = val;
    k_buf_of(np)->used = m->len * 2 + 2;
    out->len = m->len + 1;
    out->pairs = np;
    return ov;
}

/* The map without the key (ruled 2026-10-13, kanso#1914), built from the
   sorted, deduplicated view so a key written twice goes in one step. A key
   that is not there answers an equal map. The answer is always a fresh map:
   an in-place put on it must never reach the map it was made from. */
KValue k_b_delete(KValue mv, KValue key) {
    if (__builtin_expect(!k_not_failure(mv) || !k_not_failure(key), 0)) return k_failed2(mv, key);
    if (mv.tag != K_MAP) k_die("delete takes a map and a key");
    k_check_map_key(key);
    long long n;
    KValue* s = k_map_sorted(k_as_map(mv), &n);
    KValue* kept = k_buf(n ? 2 * n : 2);
    long long w = 0;
    for (long long i = 0; i < n; i++) {
        if (k_key_cmp(s[i * 2], key) == 0) continue;
        kept[w * 2] = s[i * 2];
        kept[w * 2 + 1] = s[i * 2 + 1];
        w++;
    }
    return k_map_lit(w, kept);
}

/* A failing field is the rare arm of `entries`, and k_rec is a plain call
   that would cost the hot arm its frame. */
static __attribute__((noinline, cold, preserve_most)) KValue k_rec_cold(long long t, long long n, KValue* f) {
    return k_rec(t, n, f);
}

K_DOORCC KValue k_b_entries(KValue mv) {
    if (!k_not_failure(mv)) return mv;
    if (mv.tag != K_MAP) k_die("entries takes a map");
    KMap* m = k_as_map(mv);
    long long n;
    KValue* s = k_map_sorted(m, &n);
    /* One arena block for all n records rather than one bump apiece. They were
       already landing next to each other -- k_alloc only bumps a pointer -- so
       this changes where the arithmetic happens and not where the bytes go.
       encodebench builds 3,344,400 of these a run over 1,104,400 calls, so
       two bumps in every three were bookkeeping for a block the one before it
       had already reserved. The pair goes on the stack either way: k_rec and
       the write below both copy into the storage that follows the header and
       keep no reference to the arguments.

       The item buffer and the list header join the same block, after the
       records, unless the free list holds a buffer of exactly this size. Each
       is a bump the block could have made, and the run program calls this
       248,490 times for three pairs apiece. A buffer outgrown later is handed
       to the free list at its own capacity, which the header records, so the
       records behind it are never handed out with it. */
    long long cap = n ? n : 1;
    size_t slot = (sizeof(KRec) + sizeof(KValue) * 2 + 15) & ~(size_t)15;
    size_t records = slot * (size_t)n;
    size_t buf_bytes = (sizeof(KBuf) + sizeof(KValue) * (size_t)cap + 15) & ~(size_t)15;
    size_t list_bytes = (sizeof(KList) + 15) & ~(size_t)15;
    int c = k_buf_class(cap);
    KValue* items;
    unsigned char* block;
    KList* l;
    if (c >= 0 && k_buf_free[c]) {
        items = k_buf(cap);
        block = (unsigned char*)k_alloc(records + list_bytes);
        l = (KList*)(block + records);
    } else {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
            k_stat_sh_buf += (long long)buf_bytes;
        unsigned char* whole = (unsigned char*)k_alloc(records + buf_bytes + list_bytes);
        block = whole;
        KBuf* b = (KBuf*)(whole + records);
        k_buf_set_cap(b, cap, 0);
        b->used = 0;
        items = (KValue*)(b + 1);
        l = (KList*)(whole + records + buf_bytes);
    }
    for (long long i = 0; i < n; i++) {
        KValue key = s[i * 2];
        KValue val = s[i * 2 + 1];
        if (__builtin_expect(!k_not_failure(key) || !k_not_failure(val), 0)) {
            /* A failing field makes a record that is the failure, not a
               record: the slot reserved for it above goes unused, and the
               merge stays where every other construction site finds it. */
            KValue fields[2];
            fields[0] = key;
            fields[1] = val;
            items[i] = k_rec_cold(0, 2, fields);
            continue;
        }
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0)) k_stat_sh_rec += (long long)slot;
        KRec* r = (KRec*)(block + slot * (size_t)i);
        r->type_id = 0;
        r->nfields = 2;
        r->fields = (KValue*)(r + 1);
        r->fields[0] = key;
        r->fields[1] = val;
        KValue rv;
        rv.tag = K_REC;
        rv.payload = k_ptr(r);
        items[i] = rv;
    }
    l->len = n;
    l->items = items;
    k_buf_of(items)->used = n;
    KValue out; out.tag = K_LIST; out.payload = k_ptr(l); return out;
}

/* `keys` and `values` are the two columns of `entries`, read off the same
   sorted view, so position i of each is the pair at position i of `entries`.
   Each is one list and no record, which is the point of them: a walk that
   wants both sides indexes two lists instead of building and taking apart a
   record per pair. A value may be a failure, the way an element of any list
   may be; a key never is.

   The buffer and the list header come from one allocation, the way
   `entries` takes its records, buffer and header, unless the free list holds
   a buffer of exactly this size. The encoder asks both of every map it
   writes, 248,490 maps a run on the run program, and two bumps where one
   would do was a tenth of what each call cost.

   They stay on the C convention where `entries` is a door. Measured both ways
   on the run program: as doors the run read 1,105,429,763, and on the C
   convention 1,102,980,253, because the encoder's walk keeps more live
   across the two calls than they save by being doors. */
/* Position i of one column of a map's sorted view, `side` 0 for the keys and
   1 for the values, into `items`. `keys`, `values` and the fused call that
   builds both all read through here, so the order they share is written once. */
static inline __attribute__((always_inline)) void k_column_into(KValue* items, KMap* m, KValue* s, long long n, int side) {
    (void)m;
    for (long long i = 0; i < n; i++) items[i] = s[i * 2 + side];
}

static inline __attribute__((always_inline)) KValue k_map_column(KValue mv, int side, const char* refusal) {
    if (!k_not_failure(mv)) return mv;
    if (mv.tag != K_MAP) k_die(refusal);
    KMap* m = k_as_map(mv);
    long long n;
    KValue* s = k_map_sorted(m, &n);
    long long cap = n ? n : 1;
    size_t buf_bytes = (sizeof(KBuf) + sizeof(KValue) * (size_t)cap + 15) & ~(size_t)15;
    size_t list_bytes = (sizeof(KList) + 15) & ~(size_t)15;
    int c = k_buf_class(cap);
    KValue* items;
    KList* l;
    if (c >= 0 && k_buf_free[c]) {
        items = k_buf(cap);
        l = (KList*)k_alloc(list_bytes);
    } else {
        if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
            k_stat_sh_buf += (long long)buf_bytes;
        unsigned char* whole = (unsigned char*)k_alloc(buf_bytes + list_bytes);
        KBuf* b = (KBuf*)whole;
        k_buf_set_cap(b, cap, 0);
        b->used = 0;
        items = (KValue*)(b + 1);
        l = (KList*)(whole + buf_bytes);
    }
    k_column_into(items, m, s, n, side);
    l->len = n;
    l->items = items;
    k_buf_of(items)->used = n;
    KValue out; out.tag = K_LIST; out.payload = k_ptr(l); return out;
}

KValue k_b_keys(KValue mv) { return k_map_column(mv, 0, "keys takes a map"); }

KValue k_b_values(KValue mv) { return k_map_column(mv, 1, "values takes a map"); }

/* `keys m` and `values m` asked side by side, as one call. The emitter writes
   this where a call's arguments name both columns of the same map, which is
   how the JSON encoder hands a map to its walk. Both lists come from one
   allocation, and the map is tested and its sorted view found once. The keys
   are the answer and the values go to `*values`. Each buffer carries its own
   capacity in its header, so either can be outgrown and handed to the free
   list without taking the other with it. */
KValue k_b_columns(KValue mv, KValue* values) {
    if (!k_not_failure(mv)) { *values = mv; return mv; }
    if (mv.tag != K_MAP) k_die("keys takes a map");
    KMap* m = k_as_map(mv);
    long long n;
    KValue* s = k_map_sorted(m, &n);
    long long cap = n ? n : 1;
    size_t buf_bytes = (sizeof(KBuf) + sizeof(KValue) * (size_t)cap + 15) & ~(size_t)15;
    size_t list_bytes = (sizeof(KList) + 15) & ~(size_t)15;
    size_t one = buf_bytes + list_bytes;
    if (__builtin_expect(K_COUNTING && k_stats_on > 0, 0))
        k_stat_sh_buf += 2 * (long long)buf_bytes;
    unsigned char* whole = (unsigned char*)k_alloc(2 * one);
    KList* lists[2];
    for (int side = 0; side < 2; side++) {
        unsigned char* at = whole + (size_t)side * one;
        KBuf* b = (KBuf*)at;
        k_buf_set_cap(b, cap, 0);
        b->used = n;
        KValue* items = (KValue*)(b + 1);
        k_column_into(items, m, s, n, side);
        KList* l = (KList*)(at + buf_bytes);
        l->len = n;
        l->items = items;
        lists[side] = l;
    }
    KValue v; v.tag = K_LIST; v.payload = k_ptr(lists[1]);
    *values = v;
    KValue k; k.tag = K_LIST; k.payload = k_ptr(lists[0]);
    return k;
}

