#!/bin/sh
# The same rule again, on the fourth opener — and the one that hid longest.
#
# `source?` admitted a file only when its last three characters were `.rs`, so
# the scan had never read src/runtime.c at all. That file holds sixty-six
# distinct `k_die` texts, which is where a COMPILED BINARY gets its runtime
# messages, and on 2026-08-27 half of them were pinned by nothing.
#
# This holds the fourth arm there. The bait is a `k_die("` with a run past the
# ten-character floor and no golden anywhere. `k_b_sum` is the site because it
# is unreachable from a kanso program — the wrapper in lib/list is a fold — so
# the mutation cannot change what any fixture prints while it is applied.
set -e
grep -qF 'k_die("sum takes a list")' src/runtime.c || {
  echo "the sum refusal moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i.bak 's|if (lv.tag != K_LIST) k_die("sum takes a list");|if (lv.tag != K_LIST) k_die("this native refusal has no golden");|' src/runtime.c
rm -f src/runtime.c.bak
grep -qF 'this native refusal has no golden' src/runtime.c
