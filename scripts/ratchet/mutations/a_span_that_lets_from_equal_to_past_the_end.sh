#!/bin/sh
# A span's first compare drops the minus one.
#
# `from - 1 <u to` is `1 <= from <= to` because a from below one wraps past
# every to. Written `from <u to`, a one-element span is refused and a span
# starting at zero is let through, and the slice reads the byte before its
# container.
set -e
f=src/runtime.c
grep -q 'k_span_in(long long from' src/runtime.c
line='    return (unsigned long long)from - 1 < (unsigned long long)to &&'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the span test moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    return (unsigned long long)from - 1 < (unsigned long long)to \&\&$/    return (unsigned long long)from < (unsigned long long)to \&\&/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
