#!/bin/sh
# A negative double loses its sign, in the C runtime only. `-0.1` reads as
# `0.1` on native and `-0.1` on the oracle.
#
# native_renders_a_float_wider covers the INTEGRAL fast path — the cast that
# handles whole numbers under 1e15 — and stops there. Everything with a
# fraction or a wide exponent goes through the shortest-round-trip branch
# instead, and until this row nothing broke it: seventy-three mutations and not
# one of them was a sign.
#
# The sign is where the branch is fragile, because the rendering does not carry
# it. render_ryu is handed the magnitude and writes into the buffer one byte
# past the sign the caller wrote, so the sign and the digits are placed by two
# different pieces of code that have to agree on an offset.
set -e
grep -qF "buf[0] = '-';" src/runtime.c || {
  echo "the negative render's sign moved; this mutation needs rewriting" >&2
  exit 1
}
grep -qF 'render_ryu(-d, buf + 1);' src/runtime.c || {
  echo "the negative render's destination moved; this mutation needs rewriting" >&2
  exit 1
}
# One edit is enough and it is the honest one: leave the caller writing the
# sign and send the rendering to buf instead of buf + 1, so the first digit
# lands on top of the minus. That is the exact off-by-one this arrangement
# risks, rather than a deletion no plausible edit would make.
sed -i 's|render_ryu(-d, buf + 1);|render_ryu(-d, buf);|' src/runtime.c
grep -qF 'render_ryu(-d, buf);' src/runtime.c
