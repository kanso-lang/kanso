#!/bin/sh
# Writes the expected file for every case in divergences.txt from the port's
# own output, since rg cannot be the reference for those. Read the diff
# before committing it: this accepts whatever the port prints.
set -eu
K=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
(cd "$work" && "$K" build "$here/main.kso" >/dev/null)
sh "$here/fixtures/make_tree.sh" "$work/tree"
grep -v '^#' "$here/fixtures/divergences.txt" | cut -f1 | while read -r name; do
  [ -n "$name" ] || continue
  args=$(grep "^$name	" "$here/fixtures/cases.txt" | cut -f2)
  where=.
  case "$args" in @*) where=${args%% *}; where=${where#@}; args=${args#* } ;; esac
  (
    cd "$work/$where"
    eval "set -- $args"
    set +e
    "$work/main" "$@" >"$work/stdout" 2>"$work/stderr" </dev/null
    echo $? >"$work/code"
  )
  { cat "$work/stdout"; echo "[stderr]"; cat "$work/stderr"
    echo "[exit $(cat "$work/code")]"; } >"$here/fixtures/expected/$name.out"
  echo "accepted $name"
done
