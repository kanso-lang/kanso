#!/bin/sh
# Copy a list's text so far into a fresh string after every element, the
# allocation the renderer made before 2026-09-28 by joining each element onto
# everything before it.
#
# The mem fixture a_long_list_renders_in_one_pass allocates its list's text
# once per element again, and alloc_bytes reads millions where it pins
# hundreds of thousands.
set -e
sed -i.bak 's|^                k_render_into(b, l->items\[i\], 1);$|&\n                (void)k_str_n(b->p, b->len);|' src/runtime.c
rm -f src/runtime.c.bak
grep -q '^                (void)k_str_n(b->p, b->len);$' src/runtime.c
