static long long k_fork_kid(KValue cmd, KValue argv_list);
static KValue k_kill_kid(KValue handle);

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

KValue k_desc_args(void) { return k_mkdesc(2, k_none(), k_none()); }
/* The box built by hand (ruled 2026-09-15). Nothing about it is deferred, so
   it is built holding what it was handed, value or err, and running it hands
   that over. An err arrives as content, never as a failure to propagate. */
KValue k_settled(KValue v) { return k_mkdesc(31, v, k_none()); }
KValue k_b_effect(KValue v) { return k_settled(v); }
KValue k_desc_stdin(void) { return k_mkdesc(3, k_none(), k_none()); }
/* The next line of standard input, or done at its end (ruled 2026-10-13). */
KValue k_desc_read_line(void) { return k_mkdesc(32, k_none(), k_none()); }
KValue k_desc_now(void) { return k_mkdesc(16, k_none(), k_none()); }

KValue k_b_read_file(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("read_file takes a path string");
    return k_mkdesc(4, path, k_none());
}

/* The other reader, per the 2026-08-29 ruling: `read_file` is text and
   refuses bytes that are not, `read_bytes` hands the bytes back as they are.
   Rust ships fs::read beside fs::read_to_string and Go os.ReadFile beside an
   explicit conversion; two readers, each naming what it reads. */
KValue k_b_read_bytes(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("read_bytes takes a path string");
    return k_mkdesc(30, path, k_none());
}

/* argv is a kanso list of strings; the description carries it whole so the
   process is not started until the description runs. */
/* Sockets and running children live for the process. A program holds a
   handle — a small number counted from one, the same number the interpreter
   hands out, so a program that prints one reads the same on both engines. */
#define K_SOCKETS 64
static struct { int used; int fd; } k_sockets[K_SOCKETS];
static struct { int used; pid_t pid; int out; int err; int started;
                char* obuf; long long olen; long long ocap;
                char* ebuf; long long elen; long long ecap; } k_kids[K_SOCKETS];

/* A child that could not be executed and a child that ran and answered 127
   are different answers, and an exit status cannot tell them apart: 127 is
   what a shell reports for not-found, and it is also a status a program may
   choose. So the child keeps a pipe the exec closes for it — nothing arrives
   and the parent reads end-of-file — and writes its errno through when exec
   comes back instead. Bytes on that pipe are the only evidence that the
   command never started. */
static int k_exec_pipe(int fds[2]) {
    if (pipe(fds)) return -1;
    fcntl(fds[1], F_SETFD, fcntl(fds[1], F_GETFD, 0) | FD_CLOEXEC);
    return 0;
}

static void k_exec_failed(int wfd) {
    int why = errno;
    ssize_t wrote = write(wfd, &why, sizeof why);
    (void)wrote;
}

static int k_started(int rfd) {
    int why = 0;
    ssize_t got = read(rfd, &why, sizeof why);
    close(rfd);
    return got <= 0;
}
static long long k_handles = 0;

/* A handle names its slot by `h % K_SOCKETS`, so a new one has to land on a
   slot nobody is using. Counting how many have ever been taken is not the same
   question: it made a program's sixty-fourth socket or process fail however
   many it had already closed, and say "at once" about a table holding nothing.
   A server answering requests spends two of them a turn. */
static int k_socket_free(long long slot);
static int k_kid_free(long long slot);

static long long k_free_handle(int (*free_at)(long long)) {
    for (long long i = 1; i <= K_SOCKETS; i++) {
        long long cand = k_handles + i;
        if (free_at(cand % K_SOCKETS)) {
            k_handles = cand;
            return cand;
        }
    }
    return -1;
}

static int k_socket_free(long long slot) { return !k_sockets[slot].used; }

static long long k_hold_socket(int fd) {
    long long h = k_free_handle(k_socket_free);
    if (h < 0) k_die("too many sockets open at once");
    k_sockets[h % K_SOCKETS].used = 1;
    k_sockets[h % K_SOCKETS].fd = fd;
    return h;
}

static int k_socket_of(long long h) {
    if (h <= 0 || !k_sockets[h % K_SOCKETS].used) return -1;
    return k_sockets[h % K_SOCKETS].fd;
}

KValue k_b_listen(KValue port) {
    if (!k_not_failure(port)) return port;
    if (port.tag != K_INT) k_die("listen takes a port number");
    return k_mkdesc(20, port, k_none());
}

/* Port 0 asks the operating system for a free one, and this reads back what it
   gave. Without it a caller binds 0, reads the port from a listener it then
   drops, and hands the number to something that binds it again — anything that
   binds in that gap takes it. */
KValue k_b_net_port(KValue listener) {
    if (!k_not_failure(listener)) return listener;
    if (listener.tag != K_INT) k_die("port takes a listener");
    return k_mkdesc(28, listener, k_none());
}

KValue k_b_accept(KValue listener) {
    if (!k_not_failure(listener)) return listener;
    if (listener.tag != K_INT) k_die("accept takes a listener");
    return k_mkdesc(21, listener, k_none());
}

KValue k_b_net_read(KValue conn) {
    if (!k_not_failure(conn)) return conn;
    if (conn.tag != K_INT) k_die("net_read takes a connection");
    return k_mkdesc(22, conn, k_none());
}

KValue k_b_net_write(KValue conn, KValue text) {
    if (__builtin_expect(!k_not_failure(conn) || !k_not_failure(text), 0)) return k_failed2(conn, text);
    if (conn.tag != K_INT || (text.tag != K_STR && text.tag != K_BYTES))
        k_die("net_write takes a connection and a string");
    return k_mkdesc(23, conn, text);
}

KValue k_b_net_close(KValue handle) {
    if (!k_not_failure(handle)) return handle;
    if (handle.tag != K_INT) k_die("net_close takes a listener or a connection");
    return k_mkdesc(24, handle, k_none());
}

KValue k_b_run(KValue cmd, KValue argv) {
    if (__builtin_expect(!k_not_failure(cmd) || !k_not_failure(argv), 0)) return k_failed2(cmd, argv);
    if (cmd.tag != K_STR) k_die("run takes a command string");
    if (cmd.tag == K_STR && argv.tag != K_LIST) k_die("run takes a list of argument strings");
    return k_mkdesc(17, cmd, argv);
}

KValue k_b_start(KValue cmd, KValue argv) {
    if (__builtin_expect(!k_not_failure(cmd) || !k_not_failure(argv), 0)) return k_failed2(cmd, argv);
    if (cmd.tag != K_STR) k_die("start takes a command string");
    if (cmd.tag == K_STR && argv.tag != K_LIST) k_die("start takes a list of argument strings");
    return k_mkdesc(26, cmd, argv);
}

KValue k_b_kill(KValue handle) {
    if (!k_not_failure(handle)) return handle;
    if (handle.tag != K_INT) k_die("kill takes a started process");
    return k_mkdesc(27, handle, k_none());
}

KValue k_b_write(KValue content) {
    if (!k_not_failure(content)) return content;
    if (content.tag != K_STR) k_die("write takes a string");
    return k_mkdesc(11, content, k_none());
}

KValue k_b_write_err(KValue content) {
    if (!k_not_failure(content)) return content;
    if (content.tag != K_STR) k_die("write_err takes a string");
    return k_mkdesc(12, content, k_none());
}

/* The harness's hole in infectiousness: this is the one builtin that must
   look at a failure rather than propagate it. */
KValue k_b_failed(KValue v) {
    return k_bool(!k_not_failure(v));
}

KValue k_b_env(KValue name) {
    if (!k_not_failure(name)) return name;
    if (name.tag != K_STR) k_die("env takes a string");
    return k_mkdesc(13, name, k_none());
}

KValue k_b_is_dir(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("is_dir takes a string");
    return k_mkdesc(18, path, k_none());
}

/* When a file last changed, in milliseconds since the epoch, or none for a
   file that is not there, which os/modified names file_not_found the way
   os/read_file names it (ruled 2026-10-13). */
KValue k_b_modified(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("modified takes a string");
    return k_mkdesc(33, path, k_none());
}

KValue k_b_exists(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("exists takes a string");
    return k_mkdesc(14, path, k_none());
}

KValue k_b_list_dir(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("list_dir takes a string");
    return k_mkdesc(15, path, k_none());
}

KValue k_b_make_dir(KValue path) {
    if (!k_not_failure(path)) return path;
    if (path.tag != K_STR) k_die("make_dir takes a path string");
    return k_mkdesc(19, path, k_none());
}

KValue k_b_write_file(KValue path, KValue content) {
    if (__builtin_expect(!k_not_failure(path) || !k_not_failure(content), 0)) return k_failed2(path, content);
    if (path.tag != K_STR || (content.tag != K_STR && content.tag != K_BYTES))
        k_die("write_file takes a path and content strings");
    return k_mkdesc(5, path, content);
}

/* Merge two failures: err + err becomes one err whose reason lists both
   reasons (origin-less: the merge has no single birthplace); a none adds
   nothing to an err; two nones stay none. Mirrors eval.rs exactly. */
/* What an err contributes to a merge: the reasons it already carries when it
   was itself merged, and otherwise itself, whatever shape its reason has. */
static long long k_reason_count(KValue e) {
    KErrBox* box = k_err_box(e);
    if (!box->merged) return 1;
    return k_as_list(box->reason)->len;
}

static long long k_spill_reasons(KValue e, KValue* into, long long at) {
    KErrBox* box = k_err_box(e);
    if (!box->merged) {
        into[at] = box->reason;
        return at + 1;
    }
    KList* held = k_as_list(box->reason);
    for (long long i = 0; i < held->len; i++) into[at + i] = held->items[i];
    return at + held->len;
}

static KValue k_accumulate_failures(KValue l, KValue r) {
    if (l.tag == K_ERR && r.tag == K_ERR) {
        long long n = k_reason_count(l) + k_reason_count(r);
        KValue* items = k_buf(n);
        long long at = k_spill_reasons(l, items, 0);
        k_spill_reasons(r, items, at);
        KValue merged = k_err(k_list_own(items, n), NULL);
        k_err_box(merged)->merged = 1;
        return merged;
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
    if (a.tag != K_DESC || b.tag != K_DESC) k_die("a group joins descriptions");
    return k_mkdesc(7, a, b);
}

KValue k_maybe_bind(KValue piped, KValue closure) {
    if (piped.tag == K_DESC) return k_mkdesc(6, piped, closure);
    return k_call1(closure, piped);
}

/* k_call1 without its argument guard, for the two callers that have already
   decided about the argument themselves.

   `rescue` is one: k_call1 refuses to hand a failure to a closure — a failure
   propagates instead of entering the body, which is what threads failures
   through a chain for free — and rescue is the step that must get past it. A
   group needs no help; dispatch already routes an err to the arm written for
   it.

   `bind` is the other, and it is there for speed rather than meaning. The
   chain loop tests the yielded value for failure before deciding whether the
   callback runs at all — that test IS the difference between the worded steps
   — so k_call1's copy of it is redundant work on the hottest path in the
   language. Measured: dropping it takes deepbench back under its pre-gavel
   count. */
KValue k_call_decided(KValue f, KValue a) {
    if (!k_not_failure(f)) return f;
    if (f.tag == K_CLOSURE) {
        KClosure* c = (KClosure*)(intptr_t)f.payload;
        if (c->arity < 0) return k_call1(f, a);
        if (c->arity != 1) k_die_arity(c->arity, 1);
        return c->fn(c->env, a);
    }
    return k_call1(f, a);
}

/* What a worded chain step answers once its subject has settled — the whole
   difference between the words of the 2026-08-26 gavel. `bind` (dtag 6) reads
   the value channel and lets a failure past untouched; `rescue` (dtag 29)
   reads the failure channel and lets a value past. The interpreter's
   `worded_step` is the oracle for both. */
KValue k_worded_step(long long dtag, KValue yielded, KValue callee) {
    int failed = !k_not_failure(yielded);
    if (dtag == 29) return failed ? k_call_decided(callee, yielded) : yielded;
    return failed ? yielded : k_call_decided(callee, yielded);
}

/* An effect defers and becomes a chain node; a value that has already settled
   decides here, which is what makes a word mean the same thing called
   prefix-style outside a chain. */
KValue k_b_bind(KValue subject, KValue callback) {
    if (subject.tag == K_DESC) return k_mkdesc(6, subject, callback);
    return k_worded_step(6, subject, callback);
}

/* `rescue` and `annotate` are both a wrapper around their callback on the one
   description shape, so neither needs a description of its own — which
   matters, because both are written at a site and a description's two slots
   are already the subject and the callback. The runtime builds the wrapper
   here instead: a closure over the callback and the site, standing where the
   user's callback would.

   `rescue` needs the site for its licence. The 2026-08-29 gavel's rescue is
   foreign-only, and the 2026-09-15 ruling moved that test from the arm to
   the word: a rescue written in the package that raised the failure hands it
   on without entering the callback. The compare is the raise's package
   against the site's, both the first field of an origin literal, and a
   failure nobody can be held responsible for — merged, or raised by a host
   with no frame — carries no package and passes.

   `annotate` needs it because the err it builds is a raise, and a raise
   records where it happened.

   The site rides as an int payload. It is a static literal the collector
   never owns and never traces, and a raw pointer in a payload is already how
   `k_fnref` carries what it carries. */
static int k_own_failure(KValue failure, const char* site) {
    const char* raiser = k_err_box(failure)->hako;
    return raiser && strcmp(raiser, site) == 0;
}

static K_CLOSCC KValue k_rescue_wrap(void* env, KValue failure) {
    KValue callback = k_env_get(env, 0);
    KValue site = k_env_get(env, 1);
    if (k_own_failure(failure, (const char*)(intptr_t)site.payload)) return failure;
    return k_call_decided(callback, failure);
}

static K_CLOSCC KValue k_annotate_wrap(void* env, KValue failure) {
    KValue callback = k_env_get(env, 0);
    KValue site = k_env_get(env, 1);
    return k_b_wrap_err(k_call_decided(callback, failure), failure,
                        (const char*)(intptr_t)site.payload);
}

static KValue k_sited_word(KValue subject, KValue callback, const char* origin,
                           K_CLOSCC KValue (*wrap)(void*, KValue)) {
    KValue site; site.tag = K_INT; site.payload = (long long)(intptr_t)origin;
    KValue caps[2]; caps[0] = callback; caps[1] = site;
    KValue wrapped = k_closure(wrap, 1, 2, caps, 0);
    if (subject.tag == K_DESC) return k_mkdesc(29, subject, wrapped);
    return k_worded_step(29, subject, wrapped);
}

KValue k_b_rescue(KValue subject, KValue callback, const char* origin) {
    return k_sited_word(subject, callback, origin, k_rescue_wrap);
}

KValue k_b_annotate(KValue subject, KValue callback, const char* origin) {
    return k_sited_word(subject, callback, origin, k_annotate_wrap);
}

/* The work-ahead pool (#1856): the compiler defines it, forcing each
   constant it lifted out of the work that follows a park. It runs once, at the
   first bare sleep. */
void k_ahead_pool(void);

/* One pooled constant: `force` reads it, which builds its cell. A death on the
   way jumps back here. The cells table pairs each constant's cell with its
   ready flag; a cell still holding its builder's blackhole was being built
   when the work died, and clearing its flag leaves it unbuilt. Answers
   whether the pool may go on. */
int k_ahead_try(void (*force)(void), void** cells, long long n) {
#ifdef __wasm__
    (void)force; (void)cells; (void)n;
    return 0;
#else
    int depth = k_beat_depth;
    fflush(stderr);
    int err = dup(2);
    int quiet = open("/dev/null", O_WRONLY);
    if (err >= 0 && quiet >= 0) dup2(quiet, 2);
    int ok = 1;
    k_ahead_pooling = 1;
    if (sigsetjmp(k_ahead_out, 1) == 0) {
        force();
        if (K_COUNTING) k_stat_ahead_forced++;
    } else {
        ok = 0;
        if (K_COUNTING) k_stat_ahead_dropped++;
        for (long long i = 0; i < n; i++) {
            KValue* cell = (KValue*)cells[2 * i];
            char* ready = (char*)cells[2 * i + 1];
            if (*ready && cell->tag == K_THUNK
                && ((KThunk*)cell->payload)->site == K_SITE_BLACKHOLE
                && ((KThunk*)cell->payload)->forced == K_MEMO_NONE) {
                *ready = 0;
            }
        }
        k_beat_set_depth(depth);
    }
    k_ahead_pooling = 0;
    fflush(stderr);
    if (err >= 0) { dup2(err, 2); close(err); }
    if (quiet >= 0) close(quiet);
    return ok;
#endif
}
static long long k_ahead_now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long long)ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
}

KValue k_desc_sleep(KValue ms) {
    if (!k_not_failure(ms)) return ms;
    /* a count past a word reads as none at all, as the interpreter reads it */
    if (ms.tag == K_BIG) ms = k_int(0);
    if (ms.tag != K_INT) k_die_got("sleep takes milliseconds (an int)", ms);
    return k_mkdesc(8, ms, k_none());
}

KValue k_desc_random(KValue n) {
    if (!k_not_failure(n)) return n;
    if (n.tag == K_BIG) n = k_int(0);
    if (n.tag != K_INT) k_die_got("random takes a bound (an int)", n);
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
   Bind, so `sleep` may sit anywhere and suspension needs no coroutine.
   Mirrors eval.rs's Step exactly. */
typedef struct { int blocked; unsigned long long ms; KValue cont; KValue value; } KStep;

static KValue k_schedule(KDesc* join);
static KStep k_step(KDesc* d);

static int k_argc_global = 0;
static char** k_argv_global = NULL;

static KValue k_exec(KDesc* d) {
    switch (d->dtag) {
        case 31: return d->x;
        case 0: {
            KStr* s = k_as_str(d->x);
            fwrite(s->data, 1, s->len, stdout);
            fputc('\n', stdout);
            return k_done();
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
            k_out_settle();
            while ((got = fread(data + len, 1, cap - len, stdin)) > 0) {
                len += got;
                if (len == cap) { cap *= 2; data = realloc(data, cap); }
            }
            KValue out = k_str_n(data, (long long)len);
            free(data);
            return out;
        }
        case 4: {
            /* The text, or none, which os/read_file names: the file being
               absent is DATA, ruled 2026-09-03, and everything else about
               reading it is still a failure. errno separates them — ENOENT
               (and ENOTDIR, a path whose parent is a file, which is the same
               "it is not there") answers none, and any other reason the open
               failed is an err. The interpreter's read_file_text makes the
               identical split on ErrorKind::NotFound.

               `none` and not a two-element list, which this was for a day:
               the list cost the caller the string's type, because inference
               gives a list index the top set, and the loop analyses read that
               set. jsonbench's decode loop stopped rewinding. */
            KStr* p = k_as_str(d->x);
            FILE* fh = fopen(k_cstr(p), "rb");
            if (!fh) {
                if (errno == ENOENT || errno == ENOTDIR) {
                    return k_none();
                }
                return k_err(k_concat(k_concat(k_str("cannot read "), d->x),
                                      k_str(": unreadable")), NULL);
            }
            fseek(fh, 0, SEEK_END);
            long size = ftell(fh);
            fseek(fh, 0, SEEK_SET);
            char* data = malloc(size + 1);
            size_t got = fread(data, 1, size, fh);
            fclose(fh);
            /* A file that is there, readable and not text is a failure on
               every engine, worded as the interpreter's read_file_text words
               it. Ruled 2026-08-29: read_file is text; the bytes are
               read_bytes's to hand back. Until 2026-09-09 native read any
               file and printed its bytes back unchanged, the one divergence
               the differential law let stand only for a refusal. */
            long long chars;
            if (k_utf8_bad(data, (long long)got, NULL, &chars).tag == K_ERR) {
                free(data);
                return k_err(k_concat(k_concat(k_str("cannot read "), d->x),
                                      k_str(": the bytes are not text")), NULL);
            }
            KValue out = k_str_n(data, (long long)got);
            k_str_seed_count(k_as_str(out), chars);
            free(data);
            return out;
        }
        case 30: {
            /* The same three answers as case 4 -- the bytes, none for a file
               that is not there, an err for anything else -- with no test
               of what the bytes are. They go straight into the arena, the
               way to_bytes builds a value, so nothing is copied twice. */
            KStr* p = k_as_str(d->x);
            FILE* fh = fopen(k_cstr(p), "rb");
            if (!fh) {
                if (errno == ENOENT || errno == ENOTDIR) {
                    return k_none();
                }
                return k_err(k_concat(k_concat(k_str("cannot read "), d->x),
                                      k_str(": unreadable")), NULL);
            }
            fseek(fh, 0, SEEK_END);
            long size = ftell(fh);
            fseek(fh, 0, SEEK_SET);
            unsigned char* data = (unsigned char*)k_alloc((size_t)(size > 0 ? size : 1));
            size_t got = fread(data, 1, (size_t)size, fh);
            fclose(fh);
            return k_bytes_view(data, (long long)got);
        }
        case 20: {
            int fd = socket(AF_INET, SOCK_STREAM, 0);
            if (fd < 0) return k_err(k_str("cannot open a socket"), NULL);
            int yes = 1;
            setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));
            struct sockaddr_in addr;
            memset(&addr, 0, sizeof(addr));
            addr.sin_family = AF_INET;
            addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
            addr.sin_port = htons((unsigned short)d->x.payload);
            if (bind(fd, (struct sockaddr*)&addr, sizeof(addr)) || listen(fd, 16)) {
                close(fd);
                return k_err(k_str("cannot listen on that port"), NULL);
            }
            return k_int(k_hold_socket(fd));
        }
        case 21: {
            /* Only reached outside a parallel group, where nothing else could
               ever connect; k_step yields instead of arriving here.

               The handle is checked first because this arm used to answer
               "nothing connected" whatever it was given — a true statement
               about a listener nobody has dialled, said about a value that is
               not a listener at all. The interpreter refused it; native
               described the world instead of the argument. The string is the
               one k_step already uses for the same fault, which is worth
               keeping identical: clang folds the two returns together, and a
               distinct wording costs 128 bytes of .text in every binary. */
            if (k_socket_of(d->x.payload) < 0)
                return k_err(k_str("that is not a listener"), NULL);
            return k_err(k_str("nothing connected"), NULL);
        }
        case 22: {
            int fd = k_socket_of(d->x.payload);
            if (fd < 0) return k_err(k_str("that is not a connection"), NULL);
            char buf[65536];
            k_out_settle();
            ssize_t got = read(fd, buf, sizeof(buf));
            if (got < 0) return k_err(k_str("cannot read"), NULL);
            return k_str_n(buf, (long long)got);
        }
        case 23: {
            int fd = k_socket_of(d->x.payload);
            if (fd < 0) return k_err(k_str("that is not a connection"), NULL);
            const char* tdata;
            long long tlen;
            if (d->y.tag == K_BYTES) {
                KBytes* b = k_as_bytes(d->y);
                tdata = (const char*)b->data;
                tlen = b->len;
            } else {
                KStr* t = k_as_str(d->y);
                tdata = t->data;
                tlen = t->len;
            }
            long long sent = 0;
            while (sent < tlen) {
                ssize_t n = write(fd, tdata + sent, (size_t)(tlen - sent));
                if (n <= 0) return k_err(k_str("cannot write"), NULL);
                sent += n;
            }
            return k_done();
        }
        case 24: {
            long long h = d->x.payload;
            int fd = k_socket_of(h);
            if (fd < 0) return k_err(k_str("that is not an open socket"), NULL);
            close(fd);
            k_sockets[h % K_SOCKETS].used = 0;
            return k_done();
        }
        case 19: {
            KStr* p = k_as_str(d->x);
            char* work = malloc(p->len + 1);
            if (!work) { fputs("out of memory\n", stderr); exit(1); }
            memcpy(work, p->data, p->len);
            work[p->len] = 0;
            /* Make each parent in turn. EEXIST is the ordinary case for a
               generator run twice, so only a real failure stops the walk. */
            for (size_t i = 1; i <= p->len; i++) {
                if (work[i] != '/' && work[i] != 0) continue;
                char held = work[i];
                work[i] = 0;
                if (mkdir(work, 0777) != 0 && errno != EEXIST) {
                    free(work);
                    return k_err(k_concat(k_str("cannot make "), d->x), NULL);
                }
                work[i] = held;
            }
            free(work);
            return k_done();
        }
        case 5: {
            KStr* p = k_as_str(d->x);
            FILE* fh = fopen(k_cstr(p), "wb");
            if (!fh) {
                return k_err(k_concat(k_str("cannot write "), d->x), NULL);
            }
            /* text or bytes: what read_bytes handed back writes back as it
               was, which is how the site's wasm module is digested */
            if (d->y.tag == K_BYTES) {
                KBytes* b = k_as_bytes(d->y);
                fwrite(b->data, 1, (size_t)b->len, fh);
            } else {
                KStr* c = k_as_str(d->y);
                fwrite(c->data, 1, c->len, fh);
            }
            fclose(fh);
            return k_done();
        }
        case 7: {
            /* no order between the sides, and both always run — a failure on
               one never abandons the other; failures accumulate. each side is
               a beat: its yield is discarded on success, so only a failing
               side keeps its region. */
            return k_schedule(d);
        }
        case 8: {
            /* a bare sleep executed outside a group: pause for real. The first
               one runs the work-ahead pool first (#1856), and the time the pool
               took comes off the wait, so the work hides inside it. Which work
               runs is decided by the compiler and never by the clock; only
               the length of the physical wait reads it. */
            long long ms = d->x.tag == K_INT ? d->x.payload : 0;
            k_out_settle();
#ifndef __wasm__
            /* The tab's runtime is linked apart from the program and its
               sleep is the host's, so it has no pool to run. */
            if (!k_ahead_ran) {
                k_ahead_ran = 1;
                long long before = k_ahead_now_ms();
                k_ahead_pool();
                ms -= k_ahead_now_ms() - before;
            }
#endif
            if (ms > 0) usleep((useconds_t)(ms * 1000));
            return k_done();
        }
        case 9: {
            long long n = d->x.tag == K_INT ? d->x.payload : 0;
            return k_int(k_rng_below(n));
        }
        case 10: {
            return k_done();
        }
        case 11: {
            KStr* s = k_as_str(d->x);
            fwrite(s->data, 1, s->len, stdout);
            return k_done();
        }
        case 12: {
            KStr* s = k_as_str(d->x);
            k_out_settle();
            fwrite(s->data, 1, s->len, stderr);
            return k_done();
        }
        case 13: {
            const char* found = getenv(k_cstr(k_as_str(d->x)));
            return found ? k_str(found) : k_none();
        }
        case 26: {
            long long h = k_fork_kid(d->x, d->y);
            if (h < 0) return k_err(k_concat(k_str("cannot start "), d->x), NULL);
            return k_int(h);
        }
        case 28: {
            long long h = d->x.payload;
            int fd = k_socket_of(h);
            if (fd < 0) return k_err(k_str("that is not an open socket"), NULL);
            struct sockaddr_in addr;
            socklen_t len = sizeof addr;
            if (getsockname(fd, (struct sockaddr*)&addr, &len) < 0)
                return k_err(k_str("cannot read the port of that socket"), NULL);
            return k_int(ntohs(addr.sin_port));
        }
        case 27: return k_kill_kid(d->x);
        case 17: {
            /* Two pipes rather than popen, because both streams are wanted and
               popen gives one. A non-zero status is an answer the caller reads,
               so only a process that never starts is a failure. */
            KList* args = k_as_list(d->y);
            long long argc = args->len;
            char** argv = malloc(sizeof(char*) * (size_t)(argc + 2));
            argv[0] = k_cstr(k_as_str(d->x));
            for (long long i = 0; i < argc; i++) {
                KValue item = args->items[i];
                if (item.tag != K_STR) k_die("run takes a list of argument strings");
                argv[i + 1] = k_cstr(k_as_str(item));
            }
            argv[argc + 1] = NULL;
            int outp[2], errp[2], gonep[2];
            if (pipe(outp) || pipe(errp) || k_exec_pipe(gonep)) {
                free(argv);
                return k_err(k_concat(k_str("cannot start "), d->x), NULL);
            }
            k_out_settle();
            pid_t kid = fork();
            if (kid < 0) {
                free(argv);
                close(outp[0]); close(outp[1]); close(errp[0]); close(errp[1]);
                close(gonep[0]); close(gonep[1]);
                return k_err(k_concat(k_str("cannot start "), d->x), NULL);
            }
            if (kid == 0) {
                dup2(outp[1], 1);
                dup2(errp[1], 2);
                close(outp[0]); close(outp[1]); close(errp[0]); close(errp[1]);
                close(gonep[0]);
                execvp(argv[0], argv);
                k_exec_failed(gonep[1]);
                _exit(127);
            }
            free(argv);
            close(outp[1]);
            close(errp[1]);
            close(gonep[1]);
            int started = k_started(gonep[0]);
            char* buf[2] = {NULL, NULL};
            long long len[2] = {0, 0}, cap[2] = {0, 0};
            int fds[2] = {outp[0], errp[0]};
            for (int s = 0; s < 2; s++) {
                cap[s] = 4096;
                buf[s] = malloc((size_t)cap[s]);
                for (;;) {
                    if (len[s] == cap[s]) {
                        cap[s] *= 2;
                        buf[s] = realloc(buf[s], (size_t)cap[s]);
                    }
                    ssize_t got = read(fds[s], buf[s] + len[s], (size_t)(cap[s] - len[s]));
                    if (got <= 0) break;
                    len[s] += got;
                }
                close(fds[s]);
            }
            int status = 0;
            waitpid(kid, &status, 0);
            long long code = WIFEXITED(status) ? WEXITSTATUS(status) : 128;
            if (!started) {
                free(buf[0]);
                free(buf[1]);
                return k_err(k_concat(k_str("cannot start "), d->x), NULL);
            }
            KValue triple[3];
            triple[0] = k_int(code);
            triple[1] = k_str_n(buf[0], len[0]);
            triple[2] = k_str_n(buf[1], len[1]);
            free(buf[0]);
            free(buf[1]);
            return k_mklist(3, triple);
        }
        case 16: {
            const char* pinned = getenv("KANSO_NOW");
            if (pinned) return k_int(atoll(pinned));
            struct timespec ts;
            clock_gettime(CLOCK_REALTIME, &ts);
            return k_int((long long)ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
        }
        case 14: {
            struct stat st;
            return k_bool(stat(k_cstr(k_as_str(d->x)), &st) == 0);
        }
        case 18: {
            struct stat st;
            int seen = stat(k_cstr(k_as_str(d->x)), &st) == 0;
            return k_bool(seen && S_ISDIR(st.st_mode));
        }
        case 33: {
            /* The same split case 4 makes: ENOENT and ENOTDIR are the file
               not being there, which is data, and any other reason is an
               err worded as reading words it. */
            struct stat st;
            if (stat(k_cstr(k_as_str(d->x)), &st) != 0) {
                if (errno == ENOENT || errno == ENOTDIR) return k_none();
                return k_err(k_concat(k_concat(k_str("cannot read "), d->x),
                                      k_str(": unreadable")), NULL);
            }
#ifdef __APPLE__
            struct timespec at = st.st_mtimespec;
#else
            struct timespec at = st.st_mtim;
#endif
            /* a time before the epoch has a negative second and a positive
               nanosecond, so this rounds down, as the interpreter does */
            return k_int((long long)at.tv_sec * 1000 + at.tv_nsec / 1000000);
        }
        case 32: {
            /* One line, its \n and a \r before that dropped. stdout goes out
               first, so a prompt reaches a person before the program waits on
               them. A stream that fails has ended, as it has for case 3. A
               last line with no ending is a line; the read after it is done. */
            k_out_settle();
            size_t cap = 128, len = 0;
            char* data = malloc(cap);
            int c = EOF;
            while ((c = getc(stdin)) != EOF) {
                if (len == cap) { cap *= 2; data = realloc(data, cap); }
                data[len++] = (char)c;
                if (c == '\n') break;
            }
            if (len == 0) {
                free(data);
                return k_done();
            }
            if (data[len - 1] == '\n') {
                len--;
                if (len > 0 && data[len - 1] == '\r') len--;
            }
            long long chars;
            if (k_utf8_bad(data, (long long)len, NULL, &chars).tag == K_ERR) {
                free(data);
                return k_err(k_str("cannot read standard input: the bytes are not text"), NULL);
            }
            KValue out = k_str_n(data, (long long)len);
            k_str_seed_count(k_as_str(out), chars);
            free(data);
            return out;
        }
        case 15: {
            DIR* dh = opendir(k_cstr(k_as_str(d->x)));
            if (!dh) {
                return k_err(k_concat(k_concat(k_str("cannot list "), d->x),
                                      k_str(": no such directory or unreadable")), NULL);
            }
            long long cap = 16, n = 0;
            KValue* items = malloc(sizeof(KValue) * cap);
            struct dirent* e;
            while ((e = readdir(dh))) {
                if (!strcmp(e->d_name, ".") || !strcmp(e->d_name, "..")) continue;
                if (n == cap) { cap *= 2; items = realloc(items, sizeof(KValue) * cap); }
                items[n++] = k_str(e->d_name);
            }
            closedir(dh);
            /* sorted, so what a program prints never depends on the order a
               filesystem happened to hand the names back */
            for (long long i = 1; i < n; i++) {
                KValue key = items[i];
                long long j = i - 1;
                while (j >= 0 && strcmp(k_cstr(k_as_str(items[j])), k_cstr(k_as_str(key))) > 0) {
                    items[j + 1] = items[j];
                    j--;
                }
                items[j + 1] = key;
            }
            /* k_mklist copies into an arena buffer; k_list_own would demand
               that this array already be one */
            KValue out = k_mklist(n, items);
            free(items);
            return out;
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
            /* A chain's first step always sizes and stages: its depth's
               last stage, if any, was another chain's. */
            k_carries[k_beat_depth - 1].at_arena = NULL;
            KValue cur;
            cur.tag = K_DESC;
            cur.payload = k_ptr(d);
            for (;;) {
                KDesc* dd = k_as_desc(cur);
                if (dd->dtag != 6 && dd->dtag != 29) {
                    return k_beat_pop(k_exec(dd));
                }
                KValue yielded = k_exec(k_as_desc(dd->x));
                KValue next = k_worded_step(dd->dtag, yielded, dd->y);
                if (next.tag != K_DESC) {
                    return k_beat_pop(next);
                }
                /* A step whose region has not drifted far since the last
                   stage leaves its value where it is, and it decides that
                   before sizing anything. The walk used to run every step,
                   and the leave was taken only for a value it measured at
                   4 KB or under: deepbench's 384,000 steps each walked a
                   closure over a short list at a thousand instructions to
                   learn what the previous step had learnt, 64% of the
                   program. What the size test protected against was a
                   large value re-walked every step; a step that does not
                   walk cannot re-walk. The drift test alone keeps the mark
                   inside its block and the garbage under a quarter
                   megabyte past the last staged top, and the stage that
                   ends a drift sizes and copies whatever the steps since
                   left, small or large: the same choices as before, made
                   once a drift instead of once a step. */
                KCarry* kc = &k_carries[k_beat_depth - 1];
                if (kc->at_blocks == (const void*)k_blocks
                    && kc->at_arena
                    && (size_t)(k_arena - kc->at_arena) < (size_t)(1 << 18)) {
                    kc->skipped = 1;
                    cur = next;
                    continue;
                }
                /* a large survivor — a continuation holding a decoded
                   document, say — would ride the carry pair on every step
                   of the chain, and even one evacuation doubles it while
                   the step's region is still live. Size it first (the
                   budgeted pass from the cohort guard): past the
                   threshold, skip the evacuation and floor the region
                   under it — the survivors sit below a fresh mark, every
                   later step shares them, and the ping-pong resumes for
                   the small per-step junk. The step's garbage joins the
                   floor, the same trade the cohort's keep makes. */
                k_ptrmap_begin(&k_copy_seen);
                k_copy_seen_live = 0;
                k_copy_size_budget = (size_t)(1 << 18);
                k_copy_size_spent = 0;
                size_t nsz = k_copy_size(next, &k_beat_stack[k_beat_depth - 1]);
                k_copy_size_budget = 0;
                if (nsz > (size_t)(1 << 18)) {
                    k_beat_set_depth(k_beat_depth - 1);
                    k_chunkreg_migrate(k_beat_depth);
                    k_viewreg_migrate(k_beat_depth);
                    k_permreg_migrate(k_beat_depth);
                    k_beat_push();
                    k_carries[k_beat_depth - 1].at_arena = NULL;
                    cur = next;
                } else {
                    /* The drift test above already said no, so a value that
                       is not floored is staged: copied out through the
                       carry pair, the region rewound to the mark, and the
                       next drift measured from here. The mark advances
                       whenever the copy walk repairs an interior, a rewind
                       can only return the arena to the mark, and a mark
                       that crosses a block leaves everything behind it
                       stranded, which is what the drift test's block
                       comparison guards. */
                    k_carry_reset();
                    k_carry_stage(next);
                    k_beat_iter_carry();
                    cur = k_carry_take(0);
                    kc->at_arena = k_arena;
                    kc->at_blocks = (const void*)k_blocks;
                }
            }
        }
    }
}

/* Drain what a child has written so far. A pipe nobody reads fills, and a
   child blocked on a full pipe never exits, so every poll takes what is
   there. */
static void k_drain(int fd, char** buf, long long* len, long long* cap) {
    for (;;) {
        if (*len + 4096 > *cap) {
            *cap = *cap ? *cap * 2 : 8192;
            *buf = realloc(*buf, (size_t)*cap);
        }
        ssize_t got = read(fd, *buf + *len, (size_t)(*cap - *len));
        if (got <= 0) return;
        *len += got;
    }
}

/* Forks a child with both pipes registered and non-blocking, answering the
   handle a later await or kill names. -1 is the fork or the pipes failing. */
static int k_kid_free(long long slot) { return !k_kids[slot].used; }

static long long k_fork_kid(KValue cmd, KValue argv_list) {
    KList* args = k_as_list(argv_list);
    long long argc = args->len;
    char** argv = malloc(sizeof(char*) * (size_t)(argc + 2));
    argv[0] = k_cstr(k_as_str(cmd));
    for (long long i = 0; i < argc; i++) {
        KValue item = args->items[i];
        if (item.tag != K_STR) k_die("run takes a list of argument strings");
        argv[i + 1] = k_cstr(k_as_str(item));
    }
    argv[argc + 1] = NULL;
    int outp[2], errp[2], gonep[2];
    if (pipe(outp) || pipe(errp) || k_exec_pipe(gonep)) {
        free(argv);
        return -1;
    }
    k_out_settle();
    pid_t kid = fork();
    if (kid < 0) {
        free(argv);
        close(outp[0]); close(outp[1]); close(errp[0]); close(errp[1]);
        close(gonep[0]); close(gonep[1]);
        return -1;
    }
    if (kid == 0) {
        dup2(outp[1], 1);
        dup2(errp[1], 2);
        close(outp[0]); close(outp[1]); close(errp[0]); close(errp[1]);
        close(gonep[0]);
        execvp(argv[0], argv);
        k_exec_failed(gonep[1]);
        _exit(127);
    }
    free(argv);
    close(outp[1]);
    close(errp[1]);
    close(gonep[1]);
    int started = k_started(gonep[0]);
    fcntl(outp[0], F_SETFL, fcntl(outp[0], F_GETFL, 0) | O_NONBLOCK);
    fcntl(errp[0], F_SETFL, fcntl(errp[0], F_GETFL, 0) | O_NONBLOCK);
    long long h = k_free_handle(k_kid_free);
    if (h < 0) k_die("too many processes at once");
    long long slot = h % K_SOCKETS;
    k_kids[slot].used = 1;
    k_kids[slot].started = started;
    k_kids[slot].pid = kid;
    k_kids[slot].out = outp[0];
    k_kids[slot].err = errp[0];
    k_kids[slot].obuf = NULL; k_kids[slot].olen = 0; k_kids[slot].ocap = 0;
    k_kids[slot].ebuf = NULL; k_kids[slot].elen = 0; k_kids[slot].ecap = 0;
    return h;
}

/* Ends a process the program is holding and reaps it, so a child that ignores
   its own exit budget does not outlive the run that started it. */
static KValue k_kill_kid(KValue handle) {
    long long slot = handle.payload % K_SOCKETS;
    if (!k_kids[slot].used) {
        return k_err(k_str("that is not a running process"), NULL);
    }
    int status = 0;
    kill(k_kids[slot].pid, SIGKILL);
    waitpid(k_kids[slot].pid, &status, 0);
    close(k_kids[slot].out);
    close(k_kids[slot].err);
    free(k_kids[slot].obuf);
    free(k_kids[slot].ebuf);
    k_kids[slot].used = 0;
    return k_done();
}

static KStep k_step(KDesc* d) {
    switch (d->dtag) {
        case 21: {
            /* A fiber waiting for a connection has nothing to do: it goes back
               in the queue and the group's other statements run, which is how
               anything ever connects to it. */
            int fd = k_socket_of(d->x.payload);
            if (fd < 0) {
                KStep s = {0, 0, k_none(), k_err(k_str("that is not a listener"), NULL)};
                return s;
            }
            int flags = fcntl(fd, F_GETFL, 0);
            fcntl(fd, F_SETFL, flags | O_NONBLOCK);
            int taken = accept(fd, NULL, NULL);
            if (taken < 0) {
                KStep s = {1, 1, k_mkdesc(21, d->x, d->y), k_none()};
                return s;
            }
            int tf = fcntl(taken, F_GETFL, 0);
            fcntl(taken, F_SETFL, tf & ~O_NONBLOCK);
            KStep s = {0, 0, k_none(), k_int(k_hold_socket(taken))};
            return s;
        }
        case 17: {
            /* Same shape for a process: start it, then wait by yielding, so
               the other statements of the group run while it does. */
            long long h = k_fork_kid(d->x, d->y);
            if (h < 0) {
                KStep s = {0, 0, k_none(),
                           k_err(k_concat(k_str("cannot start "), d->x), NULL)};
                return s;
            }
            KStep s = {1, 1, k_mkdesc(25, k_int(h), k_none()), k_none()};
            return s;
        }
        case 26: {
            long long h = k_fork_kid(d->x, d->y);
            if (h < 0) {
                KStep s = {0, 0, k_none(),
                           k_err(k_concat(k_str("cannot start "), d->x), NULL)};
                return s;
            }
            KStep s = {0, 0, k_none(), k_int(h)};
            return s;
        }
        case 27: {
            KStep s = {0, 0, k_none(), k_kill_kid(d->x)};
            return s;
        }
        case 25: {
            long long slot = d->x.payload % K_SOCKETS;
            if (!k_kids[slot].used) {
                KStep s = {0, 0, k_none(),
                           k_err(k_str("that is not a running process"), NULL)};
                return s;
            }
            k_drain(k_kids[slot].out, &k_kids[slot].obuf, &k_kids[slot].olen,
                    &k_kids[slot].ocap);
            k_drain(k_kids[slot].err, &k_kids[slot].ebuf, &k_kids[slot].elen,
                    &k_kids[slot].ecap);
            int status = 0;
            pid_t done = waitpid(k_kids[slot].pid, &status, WNOHANG);
            if (done == 0) {
                KStep s = {1, 1, k_mkdesc(25, d->x, d->y), k_none()};
                return s;
            }
            k_drain(k_kids[slot].out, &k_kids[slot].obuf, &k_kids[slot].olen,
                    &k_kids[slot].ocap);
            k_drain(k_kids[slot].err, &k_kids[slot].ebuf, &k_kids[slot].elen,
                    &k_kids[slot].ecap);
            close(k_kids[slot].out);
            close(k_kids[slot].err);
            k_kids[slot].used = 0;
            long long code = WIFEXITED(status) ? WEXITSTATUS(status) : 128;
            if (!k_kids[slot].started) {
                free(k_kids[slot].obuf);
                free(k_kids[slot].ebuf);
                KStep s = {0, 0, k_none(), k_err(k_str("cannot start that"), NULL)};
                return s;
            }
            KValue triple[3];
            triple[0] = k_int(code);
            triple[1] = k_str_n(k_kids[slot].obuf ? k_kids[slot].obuf : "",
                                k_kids[slot].olen);
            triple[2] = k_str_n(k_kids[slot].ebuf ? k_kids[slot].ebuf : "",
                                k_kids[slot].elen);
            free(k_kids[slot].obuf);
            free(k_kids[slot].ebuf);
            KStep s = {0, 0, k_none(), k_mklist(3, triple)};
            return s;
        }
        case 8: {
            long long ms = d->x.tag == K_INT ? d->x.payload : 0;
            KStep s = {1, (unsigned long long)(ms < 0 ? 0 : ms), k_desc_nil(), k_none()};
            return s;
        }
        case 6:
        case 29: {
            KStep in = k_step(k_as_desc(d->x));
            if (in.blocked) {
                KStep s = {1, in.ms, k_mkdesc(d->dtag, in.cont, d->y), k_none()};
                return s;
            }
            KValue next = k_worded_step(d->dtag, in.value, d->y);
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
            k_out_settle();
#ifndef __wasm__
            /* Every side is waiting: the group's first idle point runs the
               work-ahead pool (#1856), as a bare sleep does. The wall-credit
               below counts the pool's time against the wait. */
            if (!k_ahead_ran) {
                k_ahead_ran = 1;
                k_ahead_pool();
            }
#endif
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
long long k_truthy_bad(KValue v) {
    /* A subtype of bool is a condition like the bool it wraps; the inline
       test sees only the wrapper's tag, so it lands here. */
    if (v.tag == K_SUB) {
        KValue b = k_sub_base(v);
        if (b.tag == K_TRUE) return 1;
        if (b.tag == K_FALSE) return 0;
    }
    k_die_got("an if condition is true or false", v);
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
    return k_truthy_bad(v);
}

