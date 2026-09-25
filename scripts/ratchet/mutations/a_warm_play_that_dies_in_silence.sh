#!/bin/sh
# A warm play runs its binary without compiling the file, and compiles it only
# to word a death by signal. This mutation words it with nothing, so a warm
# play that runs out of stack ends with an empty line.
#
# a_warm_play_that_runs_out_of_stack_still_says_so reads nothing on the
# second play.
set -e
line='                    ended_by_signal(code, kanso::compile_play_file(&file, &source).ok().as_ref())'
grep -qxF "$line" src/main.rs || {
  echo "the warm play's explanation changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^                    ended_by_signal(code, kanso::compile_play_file(&file, &source).ok().as_ref())$|                    { let _ = code; String::new() }|' src/main.rs
grep -qxF '                    { let _ = code; String::new() }' src/main.rs
