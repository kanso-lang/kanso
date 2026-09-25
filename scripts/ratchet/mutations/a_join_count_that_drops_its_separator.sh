#!/bin/sh
# The joined string's count is the pieces' counts plus the separator's once
# between each pair. This mutation leaves the separator out, so
# a_walk_by_index_steps_from_its_cursor reads `spaced: 9` where the string
# holds eleven characters.
set -e
grep -q '^        chars = (-(long long)ss->cap - 1) \* (l->len ? l->len - 1 : 0);$' src/runtime.c || {
  echo "the join's separator count changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^        chars = (-(long long)ss->cap - 1) \* (l->len ? l->len - 1 : 0);$|        chars = 0;|' src/runtime.c
grep -q '^        chars = 0;$' src/runtime.c
