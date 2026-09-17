# The cost every kanso process pays whatever it is compiling.
#
# WHY THIS EXISTS. The three compile-instruction rows -- module, entry and
# library -- have read two values thirteen apart across sittings of one commit,
# and no property printed in the job -- the runner`s CPU model, the binary`s
# sha256, its .text size, the glibc and rustc versions -- predicted which. The
# fact that broke it open is that all THREE rows move by exactly thirteen at
# once. A 36.9-million-instruction module compile and a 132.0-million-
# instruction library compile cannot both lose precisely thirteen instructions
# of the work they have in common if that work is the compiling. So the
# thirteen is a per-process constant, and this instrument prints it.
#
# HOW. Give it the profiles of two or more different workloads from the same
# process shape. A frame whose SELF cost is identical in every one of them did
# not scale with the input, which is the definition being used here: it is part
# of what the process pays to exist. The set is derived from the profiles
# rather than named in a list, so a frame that appears or disappears is
# reported rather than silently missed.
#
# Compare the `per_process_floor=` line between two jobs. If it moves by
# thirteen, the per-frame listing above it names the frame that moved, and the
# hunt is over. If it holds while a compile row moves, the thirteen is not
# here after all and this file says so honestly.
#
#   sh scripts/gates/per_process_floor.sh /tmp/cg.compile /tmp/cg.entry /tmp/cg.library
set -e
[ $# -ge 2 ] || { echo "need two or more profiles" >&2; exit 2; }
here=$(dirname "$0")
tmp=${TMPDIR:-/tmp}/ppf.$$
trap 'rm -rf "$tmp"' EXIT INT TERM
mkdir -p "$tmp"
n=0
for p in "$@"; do
  n=$((n + 1))
  sh "$here/callgrind_self.sh" "$p" | sort > "$tmp/$n"
done
# A frame is in the floor when it appears in every profile with one value. awk
# holds the first reading, drops the frame the moment a later profile disagrees
# or omits it, and counts how many profiles each survivor was seen in.
awk -F'\t' '
  { seen[$1]++; if (!($1 in first)) first[$1] = $2; else if (first[$1] != $2) bad[$1] = 1 }
  END {
    for (f in first)
      if (!(f in bad) && seen[f] == N) printf "%d\t%s\n", first[f], f
  }
' N="$n" "$tmp"/* | sort -rn > "$tmp/floor"
echo "=== the cost every process pays, whatever it compiles"
while IFS="$(printf '\t')" read -r cost name; do
  printf '  %12s  %s\n' "$cost" "$name"
done < "$tmp/floor"
printf 'per_process_floor_frames=%s\n' "$(wc -l < "$tmp/floor" | tr -d ' ')"
printf 'per_process_floor=%s\n' "$(awk -F'\t' '{ s += $1 } END { printf "%d", s }' "$tmp/floor")"
