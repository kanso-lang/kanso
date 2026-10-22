#!/bin/sh
# Five states of the JSON decoder skip a blank run out of line and read the
# byte after it with arms of their own. This mutation gives `array_delim` back
# its old blank arm, which called the state again, so the state loops. The
# output is the same; LLVM folds the decoder's states together differently,
# and the run benchmark reads 13.8M more instructions. The witness is the
# work vein.
set -e
line='  if blank (array_delim_past cs (skip_ws cs (p + 1)) acc) (array_bad c p)'
n=$(grep -cxF "$line" lib/json/value.kso)
[ "$n" -eq 1 ] || {
  echo "the comma state's blank arm moved ($n); this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  if blank (array_delim_past cs (skip_ws cs (p + 1)) acc) (array_bad c p)$/  if blank (array_delim cs cs[p + 1] (p + 1) acc) (array_bad c p)/' lib/json/value.kso
[ "$(grep -cxF '  if blank (array_delim cs cs[p + 1] (p + 1) acc) (array_bad c p)' lib/json/value.kso)" -eq 1 ]
