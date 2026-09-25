#!/bin/sh
# A play file that reads a module from disk is not keyed by its own text,
# because the module can change while the file does not. This mutation stops
# the loader saying so, and the file keeps running the module as it was.
#
# a_module_read_from_disk_changes_what_the_same_play_file_runs reads "a!"
# where it wants "a?".
set -e
line='        LOADED_ELSEWHERE.with(|c| c.set(true));'
test "$(grep -cxF "$line" src/lib.rs)" = 1 || {
  echo "the loader's note for a module on disk changed shape; rewrite this" >&2
  exit 1
}
grep -vxF "$line" src/lib.rs > src/lib.rs.new
mv src/lib.rs.new src/lib.rs
test "$(grep -cxF "$line" src/lib.rs)" = 0
