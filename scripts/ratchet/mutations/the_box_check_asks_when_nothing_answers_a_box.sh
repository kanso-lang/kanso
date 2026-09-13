#!/bin/sh
# The box question asks two things of every name and every call head: is this
# name bound here, and does the table say it answers a box. Both are hash
# lookups on the spelling, and the binder set the first one reads is rebuilt
# per declaration. None of it can change the answer when no declaration in the
# whole program carries the description bit -- neither the table nor the tail
# walk behind it can answer yes -- so the question reads `any_boxed` once and
# skips all of it, leaving the chain, which the expression says on its own.
#
# Most programs never name an effect, so this is the ordinary path. This
# mutation writes the question back by answering it `true`, and the compile
# rows are the witness: +103,180 on the module corpus and +114,497 on the
# entry corpus, measured on this branch.
#
# THOSE NUMBERS FELL BY A FACTOR OF SIX WHEN THE CHECK WAS FOLDED. They read
# 637,295 and 1,308,849 while `check_box_where_value` was a pass of its own,
# because the skip then bought its whole traversal of every declaration and
# every node. The question now rides the descent `check_after_infer` was
# making anyway, so `any_boxed` buys only the per-node work and the walk is
# no longer the skip's to save. The mutation is the same and what it proves
# is smaller; it is stated here because a number carried forward unexamined
# is how this file goes stale.
set -e
grep -q '^    let any_boxed = groups.values().any(|(s, _, _)| s & DESC != 0);$' src/check.rs || {
  echo "the box check's any_boxed hoist changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^    let any_boxed = groups.*$@    let any_boxed = true;@' src/check.rs
grep -q '^    let any_boxed = true;$' src/check.rs
