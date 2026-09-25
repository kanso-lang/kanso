#!/bin/sh
# A block is taken back from the spare list only at the size that was asked
# for. This mutation takes the first spare block at least that large, so a
# half-megabyte request can be handed a 786,448-byte block the index phase
# freed, and the live chain counts the whole of it.
#
# runbench's arena_peak_bytes reads 3,932,192 against 3,670,032.
set -e
line='    while (*link && (*link)->cap != need) link = &(*link)->next;'
grep -qF "$line" src/runtime.c || {
  echo "the spare list's search changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    while (\*link \&\& (\*link)->cap != need) link = \&(\*link)->next;$|    while (*link \&\& (*link)->cap < need) link = \&(*link)->next;|' src/runtime.c
grep -qF '    while (*link && (*link)->cap < need) link = &(*link)->next;' src/runtime.c
