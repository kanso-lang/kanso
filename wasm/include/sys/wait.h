/* The page has no processes. runtime.c's process arms compile against these
   declarations, and wasm/shim.c answers every call as a failure, which is
   what each arm already does when a spawn or a wait fails on a real host. */
#ifndef KANSO_WASM_WAIT_H
#define KANSO_WASM_WAIT_H
#include <sys/types.h>
#define WNOHANG 1
#define WIFEXITED(s) 0
#define WEXITSTATUS(s) 0
#define WIFSIGNALED(s) 0
#define WTERMSIG(s) 0
pid_t waitpid(pid_t pid, int* status, int options);
pid_t fork(void);
int pipe(int fds[2]);
int execvp(const char* file, char* const argv[]);
int kill(pid_t pid, int sig);
int dup2(int from, int to);
#endif
