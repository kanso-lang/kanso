extern KValue k_user_main(void);

static void k_report_trace(KErrBox* box) {
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
    if (box->cause) {
        KStr* cs = k_as_str(k_render(box->cause->reason, 1));
        fprintf(stderr, "%s  caused by: ", k_c_dim());
        fwrite(cs->data, 1, cs->len, stderr);
        fprintf(stderr, "%s\n", k_c_off());
        k_report_trace(box->cause);
    }
}

/* A deliberate exit is an err whose reason is `os/exit_status`. The endpoint
   reads its code rather than reporting it: the program did not fail to say
   what it meant, it said it. Returns -1 when the err is an ordinary one. */
static int k_exit_status(KValue e) {
    KValue reason = k_err_inner(e);
    if (reason.tag != K_REC) return -1;
    KRec* r = k_as_rec(reason);
    /* the module chain qualifies at whatever depth the import graph built:
       os/exit_status directly, hako/os/exit_status one hop in */
    const char* xty = k_type_name(r->type_id);
    size_t xn = strlen(xty);
    if (strcmp(xty, "os/exit_status") &&
        (xn < 15 || strcmp(xty + xn - 15, "/os/exit_status")))
        return -1;
    /* an exit_status carrying something that is not a status is not a program
       saying what it meant — it is one that went wrong computing the code, and
       the reader is owed that rather than a silent 1 */
    if (r->nfields < 1 || r->fields[0].tag != K_INT) return -1;
    long long code = r->fields[0].payload;
    return (int)(code & 0xff);
}

static void k_report_err(KValue e, const char* reached) {
    KValue r = k_render(k_err_inner(e), 1);
    /* The reason is written by its length: a reason can hold a NUL, as
       `text/to_int (text/from_code 0)`'s does, and %s stopped there. */
    KStr* why = k_as_str(r);
    fprintf(stderr, "%serror[endpoint]:%s unhandled err reached %s: ",
            k_c_err(), k_c_off(), reached);
    fwrite(why->data, 1, why->len, stderr);
    fputc('\n', stderr);
    k_report_trace(k_err_box(e));
}

void k_caf_init(void);

#ifndef __wasm__
#include <sys/resource.h>
/* A recursion deeper than the stack ends in a fault on the guard page below
   it. `kanso run` explains a child's segmentation fault as running out of
   stack, but a binary run directly died with the bare signal, and the lox port
   is run that way. The handler runs on a stack of its own, since the program's
   is the one that ran out, and answers only for a fault inside the main
   stack's reach: anything else goes back to the default and faults again. */
static char* k_stack_top;
static char* k_stack_floor;
static void k_on_segv(int sig, siginfo_t* info, void* context) {
    (void)context;
    char* at = (char*)info->si_addr;
    if (k_stack_floor && at >= k_stack_floor && at <= k_stack_top) {
        if (k_ahead_pooling) siglongjmp(k_ahead_out, 1);
        fflush(stdout);
        fprintf(stderr,
                "%serror[runtime]:%s the program ran out of stack: recursion went "
                "deeper than the stack holds\n",
                k_c_err(), k_c_off());
        _exit(1);
    }
    signal(sig, SIG_DFL);
}
static void k_watch_the_stack(void) {
    struct rlimit limit;
    if (getrlimit(RLIMIT_STACK, &limit) != 0 || limit.rlim_cur == RLIM_INFINITY) return;
    static char spare[1 << 16];
    stack_t alt;
    alt.ss_sp = spare;
    alt.ss_size = sizeof spare;
    alt.ss_flags = 0;
    if (sigaltstack(&alt, 0) != 0) return;
    k_stack_top = (char*)__builtin_frame_address(0);
    /* the guard sits just below the limit; a page or two of slack past it
       covers a frame that jumped the guard */
    k_stack_floor = k_stack_top - limit.rlim_cur - (1 << 20);
    struct sigaction on;
    memset(&on, 0, sizeof on);
    on.sa_sigaction = k_on_segv;
    on.sa_flags = SA_SIGINFO | SA_ONSTACK;
    sigemptyset(&on.sa_mask);
    sigaction(SIGSEGV, &on, 0);
}
#endif

int main(int argc, char** argv) {
    k_argc_global = argc;
#ifndef __wasm__
    k_watch_the_stack();
#endif
    k_argv_global = argv;
    /* constants are built once, before any user code runs, so reading one is
       a load rather than a check */
    k_caf_init();
    if (k_stats_on) {
        atexit(k_stats_dump);
        /* Counters print at exit, so a program that has to be killed reports
           nothing at all — which is how a run that takes minutes ends up
           telling you less than one that takes seconds. SIGUSR1 asks for them
           now. fprintf is not async-signal-safe and this is a diagnostic
           handler, not a production path: it is armed only when the counters
           are, and the alternative was two investigations that produced a
           zero-byte file. */
        signal(SIGUSR1, k_stats_on_signal);
    }
    KValue v = k_user_main();
    if (v.tag == K_DESC) {
        KValue y = k_exec(k_as_desc(v));
        if (y.tag == K_ERR) {
            int status = k_exit_status(y);
            if (status >= 0) return status;
            k_report_err(y, "the executor");
            return 1;
        }
        return 0;
    }
    if (v.tag == K_ERR) {
        int status = k_exit_status(v);
        if (status >= 0) return status;
        k_report_err(v, "the entry");
        return 1;
    }
    if (v.tag == K_NONE) {
        fprintf(stderr, "%serror[endpoint]:%s unhandled none reached the entry\n", k_c_err(), k_c_off());
        return 1;
    }
    return 0;
}
