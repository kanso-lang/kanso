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
    /* The sizing walk counted what this copy may write, and the buffer was
       grown to that count. A copy that writes more has outrun the walk and
       would run off the end of the malloc'd buffer, a corruption nothing reads
       until much later, if at all. Refused here so it is seen where it happens. */
    if (__builtin_expect(cp->buf->used + n > cp->buf->cap, 0))
        k_die("a carry copied more than its sizing walk counted");
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
        case K_ERR:
        case K_ERRV: {
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

/* Whether the list holds a node below `m`. A loop whose result is a scalar
   rewinds its whole lap when it pops, on the reading that nothing older than
   its mark can point into the lap. A node on this list is the exception: a
   pop inside the loop repaired it to point at a copy at the frontier, which is
   in the lap. Rewinding there hands that copy back while the node still
   points at it, so such a pop keeps the lap instead and hands it up, as a pop
   with a heap result does, and the enclosing stage revisits the node. */
static __attribute__((noinline, cold)) int k_pend_holds_below(KMark* m) {
    for (long long i = 0; i < k_pend_n; i++)
        if (k_pend_below(k_pend[i], m)) return 1;
    return 0;
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
            /* the copy's header starts with no view, so it takes no dead
               pairs with it */
            if (k_map_has_gone(mp)) k_map_close_gone(mp);
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
        case K_ERR:
        case K_ERRV: {
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
        /* Copying a list's malloc'd buffer into the arena would drop the
           only pointer to it, and the registration that frees it would find
           an arena buffer in the field. Only its slots need the walk. */
        case K_LIST: {
            KList* l = (KList*)p;
            if (!k_survives_x(l->items, cp->mark) && !k_buf_malloced(k_buf_of(l->items))) {
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
            if (k_map_has_gone(mp)) k_map_close_gone(mp);
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

/* The pop's work, for a pop that has some. Kept out of line so the pop that
   has none -- a heap result with nothing carried, nothing registered and no
   tenure block at its depth, which is 507,678 of runbench's 507,685 -- pays
   its four tests and returns without the six callee-saved pushes the copy
   and the migrates below need. The same split the rewind made. */
static __attribute__((noinline)) KValue k_beat_pop_slow(KValue r, long long d,
                                                        int rewound) {
    KCarry* c = &k_carries[d];
    /* A scalar result rewinds the lap, unless the list of repaired nodes holds
       one below the mark: then the lap is handed up like a heap result's. The
       test lives here rather than in `k_beat_pop`, which every loop exit
       inlines, because a scalar pop always comes here. */
    if (rewound && k_pend_n && k_pend_holds_below(&k_beat_stack[d])) rewound = 0;
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

