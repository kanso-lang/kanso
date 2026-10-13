#!/bin/sh
# kanso leaves SIGPIPE ignored, as Rust sets it before `main`.
#
# The interpreter still stops by SIGPIPE, because its writes turn a broken
# pipe into the signal themselves. `kanso play` does not: a built binary
# behind it ends by SIGPIPE, the driver raises the same signal to end the same
# way, and an ignored signal leaves the driver running to report the end.
# a_closed_pipe_stops_every_engine_quietly sees the native run's status.
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
