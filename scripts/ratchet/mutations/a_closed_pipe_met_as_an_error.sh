#!/bin/sh
# kanso leaves SIGPIPE ignored, as Rust sets it before `main`.
#
# The interpreter's `print` then meets a closed pipe as an error and panics
# with two backtraces, where a built binary stops quietly.
# a_closed_pipe_stops_every_engine_quietly sees the interpreter's stderr.
set -e
f=src/main.rs
grep -q "fn reader_may_close" src/main.rs
line='    reader_may_close();'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the SIGPIPE reset at start-up moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    reader_may_close();$/    let _ = reader_may_close as fn();/' "$f"
grep -qxF '    let _ = reader_may_close as fn();' "$f"
