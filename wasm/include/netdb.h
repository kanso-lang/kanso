/* The page has no sockets, so no names to look up; see sys/wait.h beside
   this file. */
#ifndef KANSO_WASM_NETDB_H
#define KANSO_WASM_NETDB_H
#include <sys/socket.h>
struct addrinfo {
    int ai_flags;
    int ai_family;
    int ai_socktype;
    int ai_protocol;
    socklen_t ai_addrlen;
    struct sockaddr* ai_addr;
    char* ai_canonname;
    struct addrinfo* ai_next;
};
int getaddrinfo(const char* node, const char* service, const struct addrinfo* hints,
                struct addrinfo** found);
void freeaddrinfo(struct addrinfo* found);
#endif
