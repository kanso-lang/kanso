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
            int outside = ((!k_is_heap(r.tag) && r.tag != K_THUNK)
                || ((r.tag == K_BYTES || r.tag == K_STR)
                    && !k_above_mark((const void*)(intptr_t)r.payload, &k_beat_stack[d])))
                && !(k_pend_n && k_pend_holds_below(&k_beat_stack[d]));
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
    if (!k_is_heap(r.tag) && r.tag != K_THUNK && !(k_pend_n && k_pend_holds_below(m))) {
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

/* Out of line. Since a plain rewind can turn into a stage, LLVM inlined this
   into `k_beat_rewind_slow` and `k_cohort_pop` as well, and every binary grew
   5,664 bytes for a copy and a rewind that gain nothing from being inlined. */
__attribute__((noinline)) void k_beat_iter_carry(void) {
    if (K_COUNTING) k_stat_beat_iters++;
    if (k_beat_depth <= 0 || k_beat_depth > K_BEAT_MAX) return;
    for (long long i = 0; i < k_carry_n; i++) {
        long long tag = k_carry_slots[i].tag;
        if (tag == 4 || tag == 5) return; /* failure: the callee propagates it */
    }
    KMark* m = &k_beat_stack[k_beat_depth - 1];
    KCarry* c = &k_carries[k_beat_depth - 1];
    size_t need = 0;
    k_pend_staging = 1;
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
    c->last_copy = c->to.used;
    KCarryBuf swap = c->from;
    c->from = c->to;
    c->to = swap;
    c->used_flag = 1;
    c->skipped = 0;
    k_ten_on = 0;
    k_from_window(0);
    k_pend_staging = 0;
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
   since constants stopped being copied. The cohort and the chain still call
   `k_beat_iter_carry` directly. */
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
   still stages when the arena opens a block.
   The size is the depth's, not the loop's. lox's statement loop re-enters
   itself from its own lap, so each entry is a fresh loop carrying the same
   tree, and a size forgotten at entry made each of them stage every quarter
   megabyte and copy the whole tree. */
void k_beat_lap_carry(void) {
    long long d = k_beat_depth - 1;
    if (d < 0 || d >= K_BEAT_MAX) { k_beat_iter_carry(); return; }
    KCarry* kc = &k_carries[d];
    size_t wait = (size_t)1 << 18;
    if (2 * kc->last_copy > wait) wait = 2 * kc->last_copy;
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

