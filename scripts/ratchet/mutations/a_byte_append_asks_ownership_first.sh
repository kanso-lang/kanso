#!/bin/sh
# A builder's in-place byte append asks for room before it reads the buffer's
# header, because a borrowed view has no room and the room test turns it away
# on its own. This mutation puts the separate test for a borrowed view back in
# front of the room test in `k_b_append_mut_int`, the twin the json encoder's
# separators go through. Same answers, two more instructions an append, and the
# work vein reads encodebench and runbench heavier.
set -e
f=src/codegen/declares.rs
head='define internal %KValue @k_b_append_mut_int(%KValue %acc, %KValue %x, ptr %site) alwaysinline {'
[ "$(grep -cxF "$head" src/codegen/declares.rs)" -eq 1 ] || {
  echo "the int append twin moved; this mutation needs rewriting" >&2
  exit 1
}
awk -v head="$head" '
  $0 == head { inside = 1 }
  inside && $0 == "  %len1 = add i64 %len, 1" && !done {
    print "  %owned = icmp ne i64 %cap, 0"
    print "  br i1 %owned, label %bown, label %slow"
    print "bown:"
    done = 1
  }
  inside && $0 == "}" { inside = 0 }
  { print }
  END { if (!done) exit 1 }
' "$f" > "$f.mut"
mv "$f.mut" "$f"
grep -qxF "bown:" "$f"
