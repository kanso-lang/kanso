/* The page has no sockets; see sys/wait.h beside this file. */
#ifndef KANSO_WASM_SOCKET_H
#define KANSO_WASM_SOCKET_H
#include <stdint.h>
typedef uint32_t socklen_t;
typedef uint16_t sa_family_t;
struct sockaddr { sa_family_t sa_family; char sa_data[14]; };
#define AF_INET 2
#define SOCK_STREAM 1
#define SOL_SOCKET 1
#define SO_REUSEADDR 2
int socket(int domain, int type, int protocol);
int setsockopt(int fd, int level, int name, const void* value, socklen_t len);
int bind(int fd, const struct sockaddr* addr, socklen_t len);
int listen(int fd, int backlog);
int accept(int fd, struct sockaddr* addr, socklen_t* len);
int getsockname(int fd, struct sockaddr* addr, socklen_t* len);
#endif
