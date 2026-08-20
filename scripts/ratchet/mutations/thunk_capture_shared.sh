#!/bin/sh
# The sizing walk stops seeing inside a thunk, which is the pre-#972 world:
# the malloc'd cell survives the rewind while its captured args do not, and
# the survivor copy neither counts nor moves what the thunk holds. The micro
# golden a_thunk_capture_survives_the_cohort reads reclaimed memory and goes
# red. The mutation removes the walk rather than editing the golden because
# the golden is what an edit would prove nothing about.
set -e
python3 - <<'PY'
path = "src/runtime.c"
source = open(path).read()
a = source.find("    if (v.tag == K_THUNK) {\n        KThunk* t = (KThunk*)(intptr_t)v.payload;\n        if (k_copy_seen_check(t)) return 0;")
assert a >= 0, "the k_copy_size thunk walk moved; this mutation needs rewriting"
b = source.find("    if (!k_is_heap(v.tag)) return 0;", a)
assert b > a, "the k_copy_size shape moved; this mutation needs rewriting"
open(path, "w").write(source[:a] + source[b:])
PY
grep -q 'if (k_copy_seen_check(t)) return 0;' src/runtime.c && exit 1
exit 0
