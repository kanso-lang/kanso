/* The seven calls the runtime makes into a program, for a runtime that is
   built once and handed programs at load time.

   Native links a program's module and the runtime into one binary, so the
   runtime calls `k_user_main` and the rest by name. A runtime built to wasm32
   ahead of time has no program to link against; it is instantiated first and
   each program second. So each hook is a slot here, exported by address, and a
   program's start function writes its own function's table index into each
   slot before anything runs. The runtime's calls are unchanged: they reach
   these definitions, which call through the slot. */
#include <stdint.h>

typedef struct { long long tag; long long payload; } KValue;

KValue (*k_hook_thunk_eval)(long long, KValue*);
const char* (*k_hook_type_name)(long long);
const char* (*k_hook_type_shown)(long long);
long long (*k_hook_type_field_count)(long long);
const char* (*k_hook_type_field_name)(long long, long long);
void (*k_hook_caf_init)(void);
KValue (*k_hook_user_main)(void);

KValue d_thunk_eval(long long site, KValue* args) { return k_hook_thunk_eval(site, args); }
const char* k_type_name(long long id) { return k_hook_type_name(id); }
const char* k_type_shown(long long id) { return k_hook_type_shown(id); }
long long k_type_field_count(long long id) { return k_hook_type_field_count(id); }
const char* k_type_field_name(long long id, long long i) { return k_hook_type_field_name(id, i); }
void k_caf_init(void) { k_hook_caf_init(); }
KValue k_user_main(void) { return k_hook_user_main(); }
