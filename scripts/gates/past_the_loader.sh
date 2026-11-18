#!/bin/sh
# The work and codegen rows count each process from libc's entry, past the
# dynamic loader, and this gate holds the reason and checks that it holds.
#
# Before `main` the loader finds every shared library through
# /etc/ld.so.cache, a binary search whose cost is set by what the runner image
# installed. On 2026-11-18 two runner builds of one image, 20261111.370.1 and
# 20261111.127.1, put every work row 130 instructions apart, and the function
# tables differed in two functions and nowhere else: `_dl_cache_libcmp` and
# `_dl_load_cache_lookup`. The codegen rows, which count clang and the linker
# with a dozen libraries each, moved by 1,098 and 3,092. The compile rows,
# anchored at `kanso::main`, did not move. So `instructions.sh` and
# `codegen_instructions.sh` pass `--toggle-collect='__libc_start_main*'`, and
# what the loader does before it is not counted.
#
# The check: a program whose only difference between two runs is the loader's
# search path reads two numbers counted whole and one number counted from
# libc's entry. Then each gate is read for the flag on every callgrind it runs.
set -e
flag="--toggle-collect=__libc_start_main*"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
printf 'int main(void) { return 0; }\n' > "$work/p.c"
cc -O2 -o "$work/p" "$work/p.c"
count() {
  # $1: LD_LIBRARY_PATH or empty; the rest: valgrind flags
  lib=$1
  shift
  env -i ${lib:+LD_LIBRARY_PATH=$lib} valgrind --tool=callgrind "$@" \
    --callgrind-out-file="$work/cg" "$work/p" 2>&1 | sed -n 's/.*I   refs: *//p' | tr -dc 0-9
}
libdir=$(dirname "$(cc -print-file-name=libc.so.6)")
whole_a=$(count "" )
whole_b=$(count "$libdir")
past_a=$(count "" "$flag")
past_b=$(count "$libdir" "$flag")
echo "past the loader: whole $whole_a / $whole_b, from libc's entry $past_a / $past_b"
if [ -z "$past_a" ] || [ "$past_a" -eq 0 ]; then
  echo "::error::callgrind counted nothing from __libc_start_main, so the flag no longer names libc's entry"
  exit 1
fi
if [ "$whole_a" = "$whole_b" ]; then
  echo "::error::the loader's search path no longer moves a whole-process count, so this check shows nothing"
  exit 1
fi
if [ "$past_a" != "$past_b" ]; then
  echo "::error::counted from libc's entry, the program still reads $past_a and $past_b: the loader leaks into the row"
  exit 1
fi
for gate in scripts/gates/instructions.sh scripts/gates/codegen_instructions.sh; do
  runs=$(grep -c 'valgrind --tool=callgrind' "$gate")
  flagged=$(grep -A1 'valgrind --tool=callgrind' "$gate" | grep -c "toggle-collect='__libc_start_main\*'")
  if [ "$runs" -eq 0 ] || [ "$runs" != "$flagged" ]; then
    echo "::error::$gate runs callgrind $runs times and passes the flag $flagged times"
    exit 1
  fi
done
echo "past the loader: both gates count from libc's entry"
