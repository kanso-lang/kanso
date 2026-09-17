#!/bin/sh
# The maps parse back in the row.
#
# Each compile gate reads its row as `kanso::main` inclusive out of the
# callgrind profile, and the one thing that anchor exists to leave out is
# Rust's startup: `std::rt::lang_start_internal` calling `pthread_getattr_np`,
# which parses /proc/self/maps and costs whatever the binary's section layout
# makes the file. A 64 KiB `.bss` addition that no execution reaches moved the
# whole-process count by 2,128 with the compiler's frame identical to the
# instruction. Clay ruled that term external state on 2026-09-15, and the
# spec pins the anchor in all three gates.
#
# The mutation is the shorter read: callgrind's own `summary:` line, which is
# the whole process, parse and loader included. One gate is enough — the spec
# reads all three and refuses on the first.
set -e
gate=scripts/gates/compile_instructions.sh
if ! grep -q "awk '/kanso::main/ && !seen" "$gate"; then
  echo "expected the kanso::main anchor in $gate; the read moved and this" >&2
  echo "mutation needs rewriting" >&2
  exit 1
fi
# The anchored read is two lines: the callgrind_annotate pipe and the awk.
# Replace the pair with a read of the profile's summary line.
sed -i "/^own=\$(callgrind_annotate --inclusive=yes --threshold=100 \/tmp\/cg.compile/{N;s|.*|own=\$(grep -o '^summary: [0-9]*' /tmp/cg.compile \| tr -dc 0-9)|}" "$gate"
# THERE ARE TWO ANCHORED READS SINCE 2026-09-16, and the assertion below counts
# them both. The gate counts a second time on the same binary when the first
# reading disagrees with the golden, so that a job can say for itself whether
# the binary is stable or counted two numbers; that second read is anchored the
# same way and the mutation has to move it too, or the "anchor is gone" check
# fires on a line the first sed never reached. The mutation's subject is where
# the row is read FROM, so both reads move to the summary line together.
sed -i "/^again=\$(callgrind_annotate --inclusive=yes --threshold=100 \/tmp\/cg.compile2/{N;s|.*|again=\$(grep -o '^summary: [0-9]*' /tmp/cg.compile2 \| tr -dc 0-9)|}" "$gate"
if grep -q "awk '/kanso::main/ && !seen" "$gate"; then
  echo "wanted the anchor gone from $gate, and it is still there" >&2
  exit 1
fi
if ! grep -q "^own=\$(grep -o '^summary: \[0-9\]\*' /tmp/cg.compile" "$gate"; then
  echo "the summary read did not land in $gate" >&2
  exit 1
fi
if ! grep -q "^again=\$(grep -o '^summary: \[0-9\]\*' /tmp/cg.compile2" "$gate"; then
  echo "the second summary read did not land in $gate" >&2
  exit 1
fi
