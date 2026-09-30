#!/bin/sh
# An import's type twin keeps the short name a function of the module took.
#
# Every import's pub names also exist under their short names. A module that
# declares a function by one of those names has to reach its function, so the
# loader drops the type's short-named twin there and sends a pattern naming the
# type to the declaration the twin was cloned from. With no twin dropped, the
# function and the twin share a spelling, `sorted 1` builds std/list's record,
# and the micro fixture prints it.
set -e
f=src/lib.rs
grep -q "let beaten_twins: crate::hash::Map<String, String> = dep" src/lib.rs
line='        .filter(|t| t.synthetic && !ast::has_slash(&t.name) && own_bare.contains(t.name.as_str()))'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the twin filter moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        \.filter(|t| t\.synthetic \&\& !ast::has_slash(&t\.name) \&\& own_bare\.contains(t\.name\.as_str()))$/        .filter(|_| false)/' "$f"
grep -qxF '        .filter(|_| false)' "$f"
