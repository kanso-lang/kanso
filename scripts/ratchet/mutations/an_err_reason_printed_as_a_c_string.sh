#!/bin/sh
# Print an unhandled err's reason as a C string, as the runtime did before
# 2026-09-28. %s stops at the first NUL, and the runtime corpus fixture
# an_err_reason_holding_a_nul_is_printed_whole reads a cut reason.
set -e
grep -q '^    fwrite(why->data, 1, why->len, stderr);$' src/runtime.c
sed -i.bak 's|^    fwrite(why->data, 1, why->len, stderr);$|    fprintf(stderr, "%s", k_cstr(why));|' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^    fprintf(stderr, "%s", k_cstr(why));$' src/runtime.c
