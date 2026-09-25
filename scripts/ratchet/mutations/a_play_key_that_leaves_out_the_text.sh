#!/bin/sh
# A play file's binary is found by a key over its text. This mutation leaves
# the text out of the key, so a file edited between two plays runs the binary
# its first text built.
#
# an_edited_play_file_runs_what_it_now_says reads "first" where it wants
# "second".
set -e
line='    part(source.as_bytes());'
grep -qxF "$line" src/main.rs || {
  echo "the play key's text line changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^    part(source.as_bytes());$|    let _ = source;|' src/main.rs
grep -qxF '    let _ = source;' src/main.rs
