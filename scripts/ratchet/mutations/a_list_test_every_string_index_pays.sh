#!/bin/sh
# `at` reaches k_b_at for a string index and for a list index, and since
# 2026-09-14 the string arm is asked first: `length s[i]` over text is the
# index this runtime meets most -- 690,000 calls on runbench, all of them
# from tally_4 -- and the list test in front of it was two instructions
# every one of them paid to be told no. This mutation puts the list arm
# back in front. The bytes out are the same; the work vein is what sees
# it, and the whole vein is asked rather than runbench alone, because a
# list index now pays the two instructions the string index stopped
# paying.
set -e
str_arm='    if (container.tag == K_STR && index.tag == K_INT) {'
list_arm='    if (container.tag == K_LIST && index.tag == K_INT) {'
bytes_arm='    if (container.tag == K_BYTES && index.tag == K_INT) {'
# The guard names the file this patches, on its own line, the way every other
# runtime mutation does: the touched pass selects a row by reading the guard,
# and a row it selects whose script never greps the file is a row that would go
# on being selected after the code moved out from under it.
grep -q "container.tag == K_STR && index.tag == K_INT" src/runtime.c \
  || { echo "k_b_at has no string arm; rewrite this" >&2; exit 1; }
for line in "$str_arm" "$list_arm" "$bytes_arm"; do
  n=$(grep -cF "$line" src/runtime.c)
  [ "$n" -eq 1 ] || { echo "k_b_at changed shape ($n for '$line'); rewrite this" >&2; exit 1; }
done
# the string arm ends where the list arm begins, and the list arm where the
# bytes arm does, so the swap is two buffers emitted in the other order
awk -v s="$str_arm" -v l="$list_arm" -v b="$bytes_arm" '
  state == 0 && $0 == s { state = 1 }
  state == 1 && $0 == l { state = 2 }
  state == 2 && $0 == b {
    for (i = 1; i <= ln; i++) print listbuf[i]
    for (i = 1; i <= sn; i++) print strbuf[i]
    state = 3
  }
  state == 1 { strbuf[++sn] = $0; next }
  state == 2 { listbuf[++ln] = $0; next }
  { print }
' src/runtime.c > src/runtime.c.mut
mv src/runtime.c.mut src/runtime.c
# the list arm now stands in front of the string arm
awk -v s="$str_arm" -v l="$list_arm" '
  $0 == l { seen_list = NR }
  $0 == s { seen_str = NR }
  END {
    if (seen_list == 0 || seen_str == 0 || seen_list > seen_str) exit 1
  }
' src/runtime.c
