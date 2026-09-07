#!/bin/sh
# The remainder and the quotient of two integers the inference has proved are
# one instruction each since 2026-09-07, with zero and minus one sent to the
# runtime. Before that every `%` and `/` was a call: runbench made 2,565,677
# remainder calls. This turns the fast path off, so every one is a call
# again; the emitted code gains a call line for each site and loses its
# `srem` and `sdiv`, and the emitted goldens read it.
set -e
grep -qF '        if (op == "%" || op == "/") && pure_int {' src/codegen.rs || {
  echo "the remainder fast path changed shape; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|        if (op == "%" \|\| op == "/") \&\& pure_int {|        if (op == "%" \|\| op == "/") \&\& pure_int \&\& f.set_of(a) != INT {|' \
  src/codegen.rs
grep -qF 'pure_int && f.set_of(a) != INT {' src/codegen.rs
