#!/bin/sh
# Native's first join dies on a seed that is not a builder yet.
#
# A builder loop's seed arrives as whatever its caller wrote: a value that is
# not a string, a string that was bound lazily, or a fold's literal. The first
# join into it finds no room and makes the builder there. Restoring the old
# refusal sends each of those to `a string builder was expected here`, and
# the micro corpus's builder and fold fixtures go red.
set -e
f=src/runtime/values.c
grep -q '^KValue k_concat_arr_mut(long long n, const KValue\* parts) {$' src/runtime/values.c
grep -qx '    if (acc->cap <= 0) {' "$f" || {
  echo "the first join's conversion moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    if (acc->cap <= 0) {$/    if (acc->cap <= 0) { k_die("a string builder was expected here");/' "$f"
grep -qF 'if (acc->cap <= 0) { k_die("a string builder was expected here");' "$f"
