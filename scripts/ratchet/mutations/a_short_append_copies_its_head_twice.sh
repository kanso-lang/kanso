#!/bin/sh
# A string of sixteen bytes or fewer is appended by a pair of overlapping loads
# rather than a call into memcpy: the first eight bytes and the last eight,
# each read once and written once. The mutation writes the HEAD word at the
# tail position, which is the copy-paste this shape invites -- the two stores
# differ in one character.
#
# Nine to sixteen bytes come out wrong and everything else is untouched, which
# is why it needs the corpus rather than a length check: an object key of that
# size is ordinary, and the shorter appends the same site handles would hide it.
set -e
grep -qF 'store i64 %sw8b, ptr %sw8dp, align 1' src/codegen.rs || {
  echo "the overlapping-load append moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|store i64 %sw8b, ptr %sw8dp, align 1|store i64 %sw8a, ptr %sw8dp, align 1|' \
  src/codegen.rs
grep -qF 'store i64 %sw8a, ptr %sw8dp, align 1' src/codegen.rs
