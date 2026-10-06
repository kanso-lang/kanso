/* What the page cannot do, answered as a failure.

   runtime.c compiles for wasm32 against the headers in wasm/include, which
   declare the process and socket calls its effect executor makes. The page
   has neither, so each call here fails the way it fails on a host that
   refused it, and the arm that made it reports the refusal as it already
   does there.

   Also the three compiler-rt helpers clang emits for 128-bit integer
   arithmetic, which no wasm32 compiler-rt on this toolchain provides. */
#include <errno.h>
#include <stdint.h>
#include <sys/types.h>

typedef int16_t k_unused_t;

pid_t waitpid(pid_t pid, int* status, int options) { (void)pid; (void)status; (void)options; errno = ENOSYS; return -1; }
pid_t fork(void) { errno = ENOSYS; return -1; }
int pipe(int fds[2]) { (void)fds; errno = ENOSYS; return -1; }
int execvp(const char* file, char* const argv[]) { (void)file; (void)argv; errno = ENOSYS; return -1; }
int kill(pid_t pid, int sig) { (void)pid; (void)sig; errno = ENOSYS; return -1; }
int dup2(int from, int to) { (void)from; (void)to; errno = ENOSYS; return -1; }

struct sockaddr;
typedef uint32_t socklen_t;
int socket(int d, int t, int p) { (void)d; (void)t; (void)p; errno = ENOSYS; return -1; }
int setsockopt(int fd, int l, int n, const void* v, socklen_t s) { (void)fd; (void)l; (void)n; (void)v; (void)s; errno = ENOSYS; return -1; }
int bind(int fd, const struct sockaddr* a, socklen_t s) { (void)fd; (void)a; (void)s; errno = ENOSYS; return -1; }
int listen(int fd, int b) { (void)fd; (void)b; errno = ENOSYS; return -1; }
int accept(int fd, struct sockaddr* a, socklen_t* s) { (void)fd; (void)a; (void)s; errno = ENOSYS; return -1; }
int getsockname(int fd, struct sockaddr* a, socklen_t* s) { (void)fd; (void)a; (void)s; errno = ENOSYS; return -1; }

/* 128-bit helpers, in 64-bit halves. */
typedef unsigned __int128 u128;
typedef __int128 i128;

u128 __multi3(u128 a, u128 b) {
    uint64_t al = (uint64_t)a, ah = (uint64_t)(a >> 64);
    uint64_t bl = (uint64_t)b, bh = (uint64_t)(b >> 64);
    uint64_t a0 = al & 0xffffffffu, a1 = al >> 32, b0 = bl & 0xffffffffu, b1 = bl >> 32;
    uint64_t p00 = a0 * b0, p01 = a0 * b1, p10 = a1 * b0, p11 = a1 * b1;
    uint64_t mid = (p00 >> 32) + (p01 & 0xffffffffu) + (p10 & 0xffffffffu);
    uint64_t lo = (p00 & 0xffffffffu) | (mid << 32);
    uint64_t hi = p11 + (p01 >> 32) + (p10 >> 32) + (mid >> 32);
    hi += al * bh + ah * bl;
    return ((u128)hi << 64) | lo;
}

u128 __lshrti3(u128 a, int b) {
    uint64_t lo = (uint64_t)a, hi = (uint64_t)(a >> 64);
    if (b == 0) return a;
    if (b >= 64) return (u128)(hi >> (b - 64));
    return ((u128)(hi >> b) << 64) | ((lo >> b) | (hi << (64 - b)));
}

/* Signed multiply with an overflow flag, which clang emits for checked
   128-bit products. No division: that would need __udivti3, which this
   toolchain does not have either. */
i128 __muloti4(i128 a, i128 b, int* overflow) {
    u128 x = a < 0 ? -(u128)a : (u128)a;
    u128 y = b < 0 ? -(u128)b : (u128)b;
    int neg = (a < 0) != (b < 0);
    *overflow = 0;
    if ((uint64_t)(y >> 64) != 0) { u128 t = x; x = y; y = t; }
    if ((uint64_t)(y >> 64) != 0) *overflow = 1;
    uint64_t yl = (uint64_t)y;
    u128 low = __multi3((u128)(uint64_t)x, (u128)yl);
    u128 high = __multi3((u128)(uint64_t)(x >> 64), (u128)yl);
    if ((uint64_t)(high >> 64) != 0) *overflow = 1;
    u128 shifted = high << 64;
    u128 p = low + shifted;
    if (p < low) *overflow = 1;
    u128 limit = neg ? ((u128)1 << 127) : (((u128)1 << 127) - 1);
    if (p > limit) *overflow = 1;
    return neg ? (i128)(-p) : (i128)p;
}
