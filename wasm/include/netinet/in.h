/* The page has no sockets; see sys/wait.h beside this file. */
#ifndef KANSO_WASM_IN_H
#define KANSO_WASM_IN_H
#include <sys/socket.h>
struct in_addr { uint32_t s_addr; };
struct sockaddr_in { sa_family_t sin_family; uint16_t sin_port; struct in_addr sin_addr; char sin_zero[8]; };
#define INADDR_LOOPBACK 0x7f000001u
static inline uint32_t htonl(uint32_t x) { return __builtin_bswap32(x); }
static inline uint16_t htons(uint16_t x) { return __builtin_bswap16(x); }
static inline uint16_t ntohs(uint16_t x) { return __builtin_bswap16(x); }
#endif
