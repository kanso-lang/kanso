#!/bin/sh
# A beat's rewind forgets the seek cursor only when the string it names sat
# above the mark, since those are the addresses the arena hands back. This
# mutation forgets it on every rewind, which is what the runtime did before
# 2026-09-23.
#
# Nothing a program prints changes, and no allocation moves: the cursor is a
# position cache. What changes is that a scan rewinding between positions
# walks its subject from the front at every one, and prose_check took more
# than five minutes where it had taken fifteen seconds.
# a_scan_keeps_its_place_in_the_text reads seek_resumes=276 against 408.
set -e
grep -q '^        if ((uintptr_t)k_seek_str - (uintptr_t)m->ptr < (uintptr_t)k_arena - (uintptr_t)m->ptr)$' src/runtime.c || {
  echo "the rewind's cursor test changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^        if ((uintptr_t)k_seek_str - (uintptr_t)m->ptr < (uintptr_t)k_arena - (uintptr_t)m->ptr)$|        if (1)|' src/runtime.c
grep -q '^        if (1)$' src/runtime.c
