#!/bin/sh
# A warm play runs its binary without compiling the file, and words a death by
# signal from the signal alone. This mutation words it with nothing, so a warm
# play that runs out of stack ends with an empty line.
#
# a_warm_play_that_runs_out_of_stack_still_says_so reads nothing on the
# second play.
set -e
line='                return execute(&played, ended_by_signal);'
grep -qxF "$line" src/main.rs || {
  echo "the warm play's explanation changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^                return execute(&played, ended_by_signal);$|                return execute(\&played, \|_\| String::new());|' src/main.rs
grep -qxF '                return execute(&played, |_| String::new());' src/main.rs
