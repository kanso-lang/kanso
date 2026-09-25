#!/bin/sh
# The sizing walk stops seeing inside a thunk, which is the pre-#972 world:
# the malloc'd cell survives the rewind while its captured args do not, and
# the survivor copy neither counts nor moves what the thunk holds. The micro
# golden a_thunk_capture_survives_the_cohort reads reclaimed memory and goes
# red. The mutation removes the walk rather than editing the golden because
# the golden is what an edit would prove nothing about.
set -e
A1='    if (v.tag == K_THUNK) {' \
A2='        KThunk* t = (KThunk*)(intptr_t)v.payload;' \
A3='        if (k_copy_seen_check(t)) return 0;' \
B='    if (!k_is_heap(v.tag)) return 0;' \
awk '
  state == 0 && $0 == ENVIRON["A1"] { one = $0; state = 1; next }
  state == 1 && $0 == ENVIRON["A2"] { two = $0; state = 2; next }
  state == 1 { print one; print; state = 0; next }
  state == 2 && $0 == ENVIRON["A3"] { state = 3; next }
  state == 2 { print one; print two; print; state = 0; next }
  state == 3 && $0 == ENVIRON["B"] { state = 4; print; next }
  state == 3 { next }
  { print }
  END { if (state != 4) exit 3 }
' src/runtime.c > src/runtime.c.mut || {
  rm -f src/runtime.c.mut
  echo "the k_copy_size thunk walk moved; this mutation needs rewriting" >&2
  exit 1
}
mv src/runtime.c.mut src/runtime.c
grep -q 'if (k_copy_seen_check(t)) return 0;' src/runtime.c && exit 1
exit 0
