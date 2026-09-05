#!/bin/sh
# Every counter gate at once, and the nine goldens they read.
#
# There are nine of them and each is its own file, so following the rule in
# CLAUDE.md -- "counters changed, regenerate every vein in the same PR" --
# meant remembering nine filenames and which golden each one writes. A branch
# that changed how closures are built regenerated the .mem vein, the four code
# goldens and the wasm blob and missed all nine of these; CI found them, one
# round late, and the round cost eight minutes for something a container can
# answer in twenty seconds.
#
# It does NOT stop at the first divergence. Each vein is an independent
# dimension of the same run, and the CI job that owns them says why: stopping
# early once kept the instruction counts from being measured at all for two
# runs, and the decoder was 12.2% slower the whole time with nothing able to
# say so. The summary at the end names every vein that moved.
#
#   sh scripts/gates/all_counters.sh           read-only, one summary
#   sh scripts/gates/all_counters.sh --write   regenerate each golden that moved
#
# --write preserves each golden's comment header. Two of them have one --
# cost_golden_wide.txt and cost_golden_pend.txt -- and a plain `cp` of the
# measured file over the golden strips it, which is a trap worth holding here
# rather than remembering.
set -e
write=0
[ "$1" = "--write" ] && write=1

sh scripts/gates/build_benchmarks.sh >/dev/null

# vein:program:golden
veins="decode:jsonbench:bench/cost_golden.txt
encode:encodebench:bench/cost_golden_encode.txt
oneshot:oneshot:bench/cost_golden_oneshot.txt
basket:basket:bench/cost_golden_basket.txt
wide:widebench:bench/cost_golden_wide.txt
pend:pendbench:bench/cost_golden_pend.txt
escape:escapebench:bench/cost_golden_escape.txt
digest:digestbench:bench/cost_golden_digest.txt
read:readbench:bench/cost_golden_read.txt
scan:scanbench:bench/cost_golden_scan.txt
live:livebench:bench/cost_golden_live.txt"

moved=""
for row in $veins; do
  vein=${row%%:*}
  rest=${row#*:}
  prog=${rest%%:*}
  golden=${rest#*:}
  got=$(mktemp)
  want=$(mktemp)
  KANSO_COUNTERS=1 "./$prog" 2>"$got" >/dev/null
  grep -v '^#' "$golden" > "$want"
  if diff -q "$want" "$got" >/dev/null; then
    rm -f "$got" "$want"
    continue
  fi
  moved="$moved $vein"
  echo "=== $vein ($golden)"
  diff "$want" "$got" || true
  if [ "$write" -eq 1 ]; then
    # keep the header: replace the data rows in place, line for line
    awk -v got="$got" '
      /^#/ || /^[[:space:]]*$/ { print; next }
      { if ((getline line < got) > 0) print line; else print }
    ' "$golden" > "$golden.new" && mv "$golden.new" "$golden"
    echo "--- rewrote $golden"
  fi
  rm -f "$got" "$want"
done

if [ -z "$moved" ]; then
  echo "counters: all eleven veins agree with their goldens"
  exit 0
fi
echo "counters moved:$moved"
[ "$write" -eq 1 ] && exit 0
echo "run with --write to regenerate them, then say why in design/compiler-log.md"
exit 1
