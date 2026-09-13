#!/bin/sh
# check_box_where_value asks two questions of every name and every call head:
# is this name bound here, and does the table say it answers a box. Both are
# hash lookups on the spelling, and the binder set the first one reads is
# rebuilt per declaration by a second walk of the body. None of it can change
# the answer when no declaration in the whole program answers a box -- the
# table lookup is false for every entry by construction -- so the pass reads
# `any_boxed` once and skips all of it, leaving the chain, which the
# expression says on its own.
#
# Most programs never name an effect, so this is the ordinary path: it is
# 637,295 instructions on the module corpus and 1,308,849 on the entry corpus,
# 43% of what the whole pass costs. This mutation writes the question back by
# answering it `true`, and the compile rows are the witness.
set -e
grep -q '^    let any_boxed = returns.values().any(|(s, _)| boxed(\*s));$' src/check.rs || {
  echo "the box check's any_boxed hoist changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^    let any_boxed = returns.*$@    let any_boxed = true;@' src/check.rs
grep -q '^    let any_boxed = true;$' src/check.rs
