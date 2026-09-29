#!/bin/sh
# The native engine's first join asks about a seed after forcing it.
#
# A seed that arrived lazily was left as it came by the loop's entry, and what
# it forces to is a finished string rather than a builder. Asked about after
# the force, the join takes it for a builder and the append dies.
set -e
f=src/codegen.rs
grep -q "fn maybe_force" src/codegen.rs
grep -qF '                            match joins_builder && i == 0 && f.set_of(&raw) & !STR != 0 {' "$f" || {
  echo "the lazy seed question at the first join moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^                            match joins_builder \&\& i == 0 \&\& f.set_of(&raw) \& !STR != 0 {$/                            match joins_builder \&\& i == 0 \&\& f.set_of(\&value) \& !STR != 0 {/' "$f"
grep -qF '                            match joins_builder && i == 0 && f.set_of(&value) & !STR != 0 {' "$f"
