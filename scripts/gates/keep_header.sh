#!/bin/sh
# Rewrite a golden's data rows from a measured file, keeping its header.
#
#   sh scripts/gates/keep_header.sh <golden> <got>     prints the new golden
#
# Comment and blank lines in the golden are printed as they are. Every other
# line is replaced by the next line of the measured file, and the measured
# file's remaining lines are appended after the golden's last row — a counter
# added to the runtime prints one more row than the golden holds, and until
# 2026-09-09 that row was dropped on the floor: the sweep reported `rewrote`,
# the file did not change, and CI found the missing row a round later.
golden=$1
got=$2
awk -v got="$got" '
  /^#/ || /^[[:space:]]*$/ { print; next }
  { if ((getline line < got) > 0) print line; else print }
  END { while ((getline line < got) > 0) print line }
' "$golden"
