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
# kq leans on this: `text/append` appears seventy times in its query sources.
# Under the mutation its suite dies with `invalid utf-8`, born in text/utf8.
#
# WHAT THIS DOES NOT CLAIM. The same mutation is caught by `specs` too — three
# golden tests fail under it. It proves this gate runs and reddens, not that kq
# sees what the others miss. A mutation only kq catches would be a better row
# and is not this one; the historical bug took an incident to find.
set -e
grep -qF 'k_stat_append_fast++;' src/runtime.c || {
  echo "the in-place append moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/k_stat_append_fast++;/{n;a\
            if (n > 1) ((unsigned char*)a->data)[a->len] = 0;
}' src/runtime.c
grep -qF 'if (n > 1) ((unsigned char*)a->data)[a->len] = 0;' src/runtime.c
