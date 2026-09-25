#!/bin/sh
# A chain head that names a declaration the program made stops being asked what
# that declaration yields, and falls to the top set instead. That is the hole
# readbench exists for, in the shape it actually had: `os/read_file!` is a
# declared wrapper, and inference used to answer a chain's yield from a table
# keyed on the head's BARE name — `os/read_file` collided with the builtin's
# name and hit the table, `os/read_file!` did not. The document reaching the
# loop then has no type where the loop analysis reads one, and the loop runs on
# a grow-only arena.
#
# THE GROUP CONSULT IS WHAT READBENCH PINS, and it is not the per-declaration
# join in `call_yield`: dropping `| ctx.decl_yields[i]` there leaves this
# benchmark reading 1 block and 201 beats, because what `os/read_file!` returns
# already carries its type. The yield table earns its keep on the socket path
# instead, where `net/read c` is a bare builtin call with no declaration to ask.
# Both were measured before this was written, because the obvious edit proves
# nothing here and looked like it would.
#
# It edits the consult rather than the golden, because the golden is what an
# edit would prove nothing about. On this mutation readbench reads arena_blocks
# 41 against 1, arena_peak_bytes 42,991,616 against 1,048,576, and beat_iters 1
# against 201.
set -e
A='                if let Some(y) = group_yield(ctx, n.as_str(), args.len()) {'
B='                    return y;'
grep -qF "$A" src/infer.rs && grep -qF "$B" src/infer.rs || {
  echo "the group consult moved; this mutation needs rewriting" >&2
  exit 1
}
A="$A" B="$B" awk '
  $0 == ENVIRON["A"] { print "                if group_yield(ctx, n.as_str(), args.len()).is_some() {"; next }
  $0 == ENVIRON["B"] { print "                    return TOP & !FAIL;"; next }
  { print }
' src/infer.rs > src/infer.rs.mut
mv src/infer.rs.mut src/infer.rs
grep -qF 'group_yield(ctx, n.as_str(), args.len()).is_some()' src/infer.rs
