#!/bin/sh
# The short name an import's record also answers to is looked up before the
# foreign test. This skips the lookup, so `cursor 1 xs` builds std/list's
# record without a word while `list/cursor 1 xs` is still refused.
set -e
t='        Some(origin) if named.arities.get(name).is_none() => origin,'
n=$(grep -cF "$t" src/check.rs)
[ "$n" -eq 1 ] || { echo "the short-name lookup changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        Some(origin) if named.arities.get(name).is_none() => origin,$/        Some(origin) if false => origin,/' src/check.rs
grep -qF '        Some(origin) if false => origin,' src/check.rs
