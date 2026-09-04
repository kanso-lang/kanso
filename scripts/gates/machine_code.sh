#!/bin/sh
# The counters say what a program ALLOCATES and the emitted golden says what
# the compiler WROTE. Neither sees the machine code, and that is the dimension
# that carried a measured 17% of the 7.6% decode regression: 54,708 bytes of
# .text to 79,192 over eleven days, with every allocation counter
# byte-identical the whole way. Padding an old build up to that size with
# functions it never calls costs 1.6% of decode time by itself.
set -e

# Whose bytes these are. .text is what the toolchain made of the source, so a
# different clang answers a different number for code nobody touched.
# A host the golden does not name still MEASURES on CI, so the job log carries
# the sitting the refusal tells a reader to copy. scripts/gates/host_gate.sh
# carries the reasons; 3 means measure, print, and refuse without comparing.
host=0
sh scripts/gates/host_gate.sh bench/text_golden.txt || host=$?
if [ "$host" -ne 0 ] && [ "$host" -ne 3 ]; then
  exit "$host"
fi

# Apple's size takes no --format, and the awk downstream then reads an empty
# section size for every benchmark, so the diff goes red on four blank values
# and says the machine code moved. A gate that cannot measure has to say that
# instead of reporting a change it did not see.
size --format=sysv ./jsonbench >/dev/null 2>&1 || {
  echo "::error::this gate reads section sizes with GNU size, which the"
  echo "::error::host running it does not have. The linux runner does."
  exit 1
}
# scanbench and digestbench joined the corpus after this gate was written and
# nobody extended the list, so the two newest benchmarks were the two this vein
# could not see. The bit twins landed on digestbench and moved its `.text` by
# 1,968 bytes with every row here byte-identical, which is the shape of move
# this file exists to catch.
for b in jsonbench encodebench oneshot basket widebench deepbench escapebench pendbench \
         indexbench scanbench digestbench readbench; do
  printf '%s text=%s\n' "$b" "$(size --format=sysv ./$b | awk '/^\.text/{print $2}')"
done > text.txt
# Measured, and on a host the golden does not name that is as far as this goes.
if [ "$host" -eq 3 ]; then
  echo "::error::this runner's sitting, to copy into bench/text_golden.txt"
  echo "::error::together with its measured-on line. NOTHING IS COMPARED:"
  sed 's/^/::error::    /' text.txt
  exit 1
fi

grep -v '^#' bench/text_golden.txt > text_want.txt
diff text_want.txt text.txt || {
  echo "::error::the machine code changed size. A rise is a"
  echo "::error::regression to explain and a fall is a win to bank —"
  echo "::error::say which in the PR and regenerate"
  echo "::error::bench/text_golden.txt. Allocation counters cannot"
  echo "::error::see this, and neither can the emitted-line count:"
  echo "::error::a fifth of those lines were dropped by the linker"
  echo "::error::anyway. 17% of the last regression lived here."
  exit 1
}
