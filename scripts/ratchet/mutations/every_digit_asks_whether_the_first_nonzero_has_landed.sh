#!/bin/sh
# k_b_to_float counts significant digits, and since 2026-09-15 it skips the
# leading zeros before the digit loops so both loops count unconditionally.
# Before that every digit re-asked `if (w) digits++` -- a predicate that is
# monotone in the digit position, false until the first nonzero digit lands
# and true forever after -- at three instructions a digit.
#
# This puts the question back on every digit and takes the two skips out.
# The (w, q) handed to eisel-lemire is identical either way, so nothing but
# the work vein can see this, which is exactly what the row asserts.
set -e
grep -q "while (p < stop && \*p == '0') { any = 1; p++; }" src/runtime.c || {
  echo "the leading-zero skip changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -e "/^        while (p < stop && \*p == '0') { any = 1; p++; }$/d" \
    -e "/^            if (w == 0) {$/,/^            }$/d" \
    -e "s/(unsigned long long)(\*p - '0'); digits++; }/(unsigned long long)(*p - '0'); if (w) digits++; }/" \
    -e "s/^                    digits++;$/                    if (w) digits++;/" \
    src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
grep -q "if (w) digits++; }" src/runtime.c
if grep -q "if (w == 0) {" src/runtime.c; then
  echo "the fraction skip survived the mutation" >&2
  exit 1
fi
