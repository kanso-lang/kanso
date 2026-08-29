#!/bin/sh
# The utf-8 validator accepts a byte the independent reference refuses.
#
# k_utf8_bad opens with a scalar pass for strings shorter than one vector,
# where the wide classification is nearly all setup: walk while the bytes are
# ascii, and answer valid if that reaches the end. The bound is what makes it
# an ascii test. Moving it one value up admits 0x80, which is a continuation
# byte with nothing in front of it and not a character at all.
#
# The sweep is exhaustive over every string of three bytes or fewer, so it
# reaches this on the first length: `MISMATCH len=1 bytes=80 got=1 want=0`.
#
# Deliberately in the scalar prologue rather than in either vector body, so the
# mutation reads the same on x86 and on arm — the ratchet must not depend on
# which host it runs.
set -e
f=src/runtime.c
grep -qF 'while (j < len && (uint8_t)data[j] < 0x80) j++;' "$f" || {
  echo "the ascii prologue moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|while (j < len \&\& (uint8_t)data\[j\] < 0x80) j++;|while (j < len \&\& (uint8_t)data[j] <= 0x80) j++;|' "$f"
grep -qF 'while (j < len && (uint8_t)data[j] <= 0x80) j++;' "$f"
