#!/bin/sh
# The book rule, enforced: every code sample under docs/book/samples has a
# .kso (or directory) and a .out; run each and diff. A panel whose output
# drifts from the language fails here before it can lie on the page.
set -e
KANSO=./target/release/kanso
fail=0
for out in docs/book/samples/*/*.out; do
  base=${out%.out}
  if [ -d "$base" ]; then src="$base"; else src="$base.kso"; fi
  mode=run
  case "$base" in *_check) mode=check;; *_test) mode=test;; esac
  actual=$("$KANSO" "$mode" "$src" 2>&1) || true
  if [ "$actual" != "$(cat "$out")" ]; then
    echo "MISMATCH: $src"
    fail=1
  fi
done
[ "$fail" = 0 ] && echo "book samples: all outputs verified"
exit $fail
