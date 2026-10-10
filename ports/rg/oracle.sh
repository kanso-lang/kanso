#!/bin/sh
# Runs every fixture through the real ripgrep and compares it with the
# expected files, which is how the expected files were made in the first
# place: `sh oracle.sh --write` regenerates them from rg, and a plain run
# reports where rg and the expected files disagree.
#
# Cases where the port deliberately differs from rg are listed in
# fixtures/divergences.txt, with the reason. Their expected files hold the
# port's output, so --write leaves them alone and a plain run reports them as
# known rather than failing.
#
# rg is run with --sort path (the port always sorts), --no-require-git (the
# port reads .gitignore without a repository) and --no-ignore-global (the
# port has no global gitignore).
set -u
here=$(cd "$(dirname "$0")" && pwd)
mode=${1:-compare}
command -v rg >/dev/null || { echo "oracle.sh: rg is not installed"; exit 1; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
sh "$here/fixtures/make_tree.sh" "$work/tree"
mkdir -p "$here/fixtures/expected"
bad=0
known=0
same=0
while IFS='	' read -r name args; do
  case "$name" in ''|'#'*) continue ;; esac
  want="$here/fixtures/expected/$name.out"
  got="$work/$name.rg"
  where=.
  case "$args" in @*) where=${args%% *}; where=${where#@}; args=${args#* } ;; esac
  (
    cd "$work/$where" || exit 1
    eval "set -- $args"
    rg --sort path --no-require-git --no-ignore-global "$@" \
      >"$work/stdout" 2>"$work/stderr" </dev/null
    echo $? >"$work/code"
  )
  { cat "$work/stdout"; echo "[stderr]"; cat "$work/stderr"
    echo "[exit $(cat "$work/code")]"; } >"$got"
  if grep -q "^$name	" "$here/fixtures/divergences.txt"; then
    known=$((known + 1))
    continue
  fi
  if [ "$mode" = --write ]; then
    cp "$got" "$want"
  elif ! cmp -s "$got" "$want"; then
    echo "DIFFERS from rg: $name"
    diff "$want" "$got" | head -20
    bad=$((bad + 1))
  else
    same=$((same + 1))
  fi
done <"$here/fixtures/cases.txt"
echo "oracle: $same agree with rg, $known known divergences, $bad differ"
[ "$bad" -eq 0 ]
