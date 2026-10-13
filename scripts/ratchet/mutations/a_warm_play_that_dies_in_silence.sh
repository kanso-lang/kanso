#!/bin/sh
# A warm play runs its binary without compiling the file, and words a death by
# signal from the signal alone. This mutation words it with nothing. The
# binary names its own stack overflow before the signal reaches the play, so
# the mutation also stops it watching its stack, and a warm play whose
# program is ended by a signal ends with an empty line.
#
# a_warm_play_ended_by_a_signal_names_it reads nothing on the second play. A
# program that runs out of stack now says so itself, so the stack test no
# longer reaches this wording.
set -e
line='                return execute(&played, ended_by_signal);'
watch='static void k_watch_the_stack(void) {'
grep -qxF "$line" src/main.rs || {
  echo "the warm play's explanation changed shape; rewrite this" >&2
  exit 1
}
[ "$(grep -cxF "$watch" src/runtime.c)" -eq 1 ] || {
  echo "the runtime's stack watch changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^                return execute(&played, ended_by_signal);$|                return execute(\&played, \|_\| String::new());|' src/main.rs
sed -i 's|^static void k_watch_the_stack(void) {$|static void k_watch_the_stack(void) { return;|' src/runtime.c
grep -qxF '                return execute(&played, |_| String::new());' src/main.rs
grep -qxF 'static void k_watch_the_stack(void) { return;' src/runtime.c
