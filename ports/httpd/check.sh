#!/bin/sh
# The port's whole check: the unit tests, then every fixture scenario run
# three ways -- on the interpreter, as a dev-tier binary and as a release
# binary -- with each transcript compared against the expected file and
# against the other two engines.
#
# Every scenario starts its own server inside the process, on a port picked
# here at random from a high range (the program falls back to one the
# operating system chooses if that port is taken), and ends by closing the
# listener and checking that nothing answers on the port any more.
#
#   KANSO=/path/to/kanso sh check.sh          run everything
#   sh check.sh --update                      rewrite the expected files from
#                                             the interpreter's transcripts
set -u
# The Date header reads the clock; pinning it is what lets a transcript be
# compared byte for byte.
KANSO_NOW=1760059200000
export KANSO_NOW
KANSO=${KANSO:-/tmp/claude-0/kanso-main/kanso}
here=$(cd "$(dirname "$0")" && pwd)
cd "$here"
update=${1:-}
fail=0
out=$(mktemp -d)
trap 'rm -rf "$out"' EXIT

echo "== unit tests"
for module in http router todo static client; do
  if ! "$KANSO" test "$module" > "$out/test.txt" 2>&1; then
    cat "$out/test.txt"
    echo "FAIL: unit tests in $module"
    fail=1
  else
    tail -n 1 "$out/test.txt" | sed "s/^/$module: /"
  fi
done

echo "== build"
# `kanso build` writes its binary into the current directory, so each tier
# is built in its own scratch directory and nothing is left behind here.
mkdir -p "$out/dev" "$out/release"
(cd "$out/dev" && "$KANSO" build "$here/main.kso" > /dev/null) || {
  echo "FAIL: dev build"; exit 1; }
(cd "$out/release" && "$KANSO" build "$here/main.kso" --release > /dev/null) || {
  echo "FAIL: release build"; exit 1; }
dev="$out/dev/main"
release="$out/release/main"

port() {
  # A high port, different on every run, from the shell's own generator.
  echo $(( 40000 + $(od -An -N2 -tu2 /dev/urandom | tr -d ' ') % 20000 ))
}

echo "== fixtures"
for scn in fixtures/*.scn; do
  name=$(basename "$scn" .scn)
  want="fixtures/$name.out"
  timeout 120 "$KANSO" run main.kso --interp -- e2e "$scn" --port "$(port)" \
    > "$out/$name.interp" 2>&1
  timeout 120 "$dev" e2e "$scn" --port "$(port)" \
    > "$out/$name.dev" 2>&1
  timeout 120 "$release" e2e "$scn" --port "$(port)" \
    > "$out/$name.release" 2>&1
  if [ "$update" = "--update" ]; then
    cp "$out/$name.interp" "$want"
  fi
  ok=1
  for engine in interp dev release; do
    if ! cmp -s "$want" "$out/$name.$engine"; then
      echo "FAIL: $name on $engine differs from $want"
      diff "$want" "$out/$name.$engine" | head -20
      ok=0
    fi
  done
  if [ $ok = 1 ]; then
    echo "ok: $name ($(wc -l < "$want") lines, three engines agree)"
  else
    fail=1
  fi
done

# The standalone server, as a user starts it: in the background, driven by
# curl, then killed. The transcript has the port replaced by PORT, and the
# run fails if anything still answers on the port after the kill.
echo "== standalone server"
serve_once() {
  p=$(port)
  log="$out/serve.log"
  : > "$log"
  "$@" serve --port "$p" --root public > "$log" 2>&1 &
  pid=$!
  tries=0
  until grep -q "serving" "$log" || [ $tries -ge 100 ]; do
    sleep 0.1; tries=$((tries + 1))
  done
  {
    curl -s -H Host:localhost "http://127.0.0.1:$p/todos"; echo
    curl -s -H Host:localhost -H Content-Type:application/json \
      -d '{"title":"from the shell"}' "http://127.0.0.1:$p/todos"; echo
    curl -s -o /dev/null -w '%{http_code} %{size_download}\n' \
      "http://127.0.0.1:$p/"
  } > "$out/serve.curl"
  sleep 0.2
  kill "$pid" 2>/dev/null
  wait "$pid" 2>/dev/null
  if nc -z 127.0.0.1 "$p" 2>/dev/null; then
    echo "!! port still answers after the server was killed"
  fi
  sed "s/$p/PORT/g" "$log"
  cat "$out/serve.curl"
}
serve_once "$KANSO" run main.kso --interp -- > "$out/serve.interp"
serve_once "$dev" > "$out/serve.dev"
serve_once "$release" > "$out/serve.release"
if [ "$update" = "--update" ]; then
  cp "$out/serve.interp" fixtures/serve.out
fi
ok=1
for engine in interp dev release; do
  if ! cmp -s fixtures/serve.out "$out/serve.$engine"; then
    echo "FAIL: standalone server on $engine differs from fixtures/serve.out"
    diff fixtures/serve.out "$out/serve.$engine" | head -20
    ok=0
  fi
done
if [ $ok = 1 ]; then
  echo "ok: serve ($(wc -l < fixtures/serve.out) lines, three engines agree)"
else
  fail=1
fi

if [ $fail = 0 ]; then
  echo "== all green"
else
  echo "== FAILED"
fi
exit $fail
