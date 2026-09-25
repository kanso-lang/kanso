#!/bin/sh
# An index of the character after the cursor's steps one character on from
# the cursor. This mutation closes the step, so every index goes through the
# general seek, which still resumes from the cursor and still answers the
# same character. Nothing a program prints changes and `seek_resumes` holds;
# the run program's `seek_steps` falls from 689,999 to 0.
set -e
anchor='        if (s == k_seek_str \&\& want == k_seek_char + 1 \&\& s->cap != 0 \&\& k_seek_byte < s->len) {'
grep -q '^        if (s == k_seek_str && want == k_seek_char + 1 && s->cap != 0 && k_seek_byte < s->len) {$' src/runtime.c || {
  echo "the index's cursor step changed shape; rewrite this" >&2
  exit 1
}
sed -i "s|^$anchor\$|        if (0) {|" src/runtime.c
grep -q '^        if (0) {$' src/runtime.c
