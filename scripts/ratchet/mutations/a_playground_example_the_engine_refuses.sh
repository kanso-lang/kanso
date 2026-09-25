#!/bin/sh
# A playground example that no longer runs.
#
# The examples are the language's shop window, and tests/playground.rs reads
# them out of docs/play.js rather than from a list of its own, so a new example
# cannot ship without coverage. That also makes them a corpus nothing else
# reads: the golden corpus under tests/golden is a different set of programs,
# and `specs` never opens play.js.
#
# The `hello` example is the first thing a visitor runs. Pointing it at a name
# nothing declares is refused by the interpreter, which is the oracle here, so
# `every_playground_example_runs_on_the_interpreter` fails on it.
set -e
f=docs/play.js
grep -qF '  hello: `print "hello, kanso"' "$f" || {
  echo "the hello example moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^  hello: `print "hello, kanso"$|  hello: `print "hello, {nobody 0}"|' "$f"
grep -qF 'print "hello, {nobody 0}"' "$f"
