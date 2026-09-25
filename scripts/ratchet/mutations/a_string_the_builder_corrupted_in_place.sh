#!/bin/sh
# A byte builder that writes the right LENGTH and the wrong bytes.
#
# This row's job runs kq — a real program, a jq clone with its own JSON — and
# the reason it gates is written in ci.yml: kq once caught a compiler bug that
# nineteen suites, ninety-eight differential programs, three cost veins and
# every memory pin agreed was not there. An in-place string concat printed 267
# nul bytes at exactly the right length, so every count still matched.
#
# So the mutation is that shape, at that place. `k_b_append_into`'s fast path
# is the in-place append: the bytes go into the accumulator's own spare
# capacity and the header's length is written where it sits. Zeroing the first
# byte of every multi-byte append there keeps every length and every counter
# right and makes the contents wrong.
#
# kq leans on this: `text/append` appears seventy times in its query sources,
# and its own counters read append_fast=242,226 on one query.
#
# THE ANCHOR IS THE MEMCPY, AND IT USED TO BE A LINE OFFSET. The first version
# matched `k_stat_append_fast++;`, skipped one line and appended after it. That
# worked until a comment was written above the copy, at which point the skipped
# line became the comment's opening and the injected statement landed INSIDE
# `/* ... */`. It was not code. The compiler built clean, kq's suite ran green,
# and the script's own grep found its text in the file and reported success, so
# the row went on claiming a proof it was not making. `scripts/ratchet -- prove`
# reported it BLIND, nightly, and nothing read that.
#
# Substituting a statement cannot land in a comment: the anchor IS the code
# being replaced. The grep below fails loudly if the copy is ever rewritten.
#
# WHAT THIS DOES NOT CLAIM. The same mutation is caught by `specs` too — three
# golden tests fail under it. It proves this gate runs and reddens, not that kq
# sees what the others miss. A mutation only kq catches would be a better row
# and is not this one; the historical bug took an incident to find.
set -e
anchor='else memcpy((unsigned char*)a->data + a->len, src, (size_t)n);'
grep -qF "$anchor" src/runtime.c || {
  echo "the in-place append's copy moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|else memcpy((unsigned char\*)a->data + a->len, src, (size_t)n);|else { memcpy((unsigned char*)a->data + a->len, src, (size_t)n); ((unsigned char*)a->data)[a->len] = 0; }|' \
  src/runtime.c
grep -qF 'memcpy((unsigned char*)a->data + a->len, src, (size_t)n); ((unsigned char*)a->data)[a->len] = 0;' \
  src/runtime.c
