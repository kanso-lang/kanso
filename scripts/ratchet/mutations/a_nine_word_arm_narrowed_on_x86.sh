#!/bin/sh
# A release build on x86-64 keeps `tailcc` on arms of up to nine argument
# words, so the JSON decoder's `obj_key_end` loop runs in one frame. This
# mutation narrows at eight, as arm64 must, and a 300,000-key object
# overflows the stack; the witness is a_big_object_decodes_in_a_release_build.
set -e
line='    false => 9,'
[ "$(grep -cxF "$line" src/main.rs)" -eq 1 ] || {
  echo "the x86-64 tailcc limit moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^    false => 9,$/    false => 8,/' src/main.rs
[ "$(grep -cxF '    false => 8,' src/main.rs)" -eq 1 ]
