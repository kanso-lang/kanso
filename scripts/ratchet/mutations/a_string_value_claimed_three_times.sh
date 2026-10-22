#!/bin/sh
# A string value is written by `append_string`, its quotes and its escaped
# bytes in one claim, inline for a clean shared token. This writes it the old
# way, the opening quote, `escape_onto` and the closing quote. The output is
# the same; the witness is the emitted gate, where every program that encodes
# a string brings back the escape's steps.
set -e
f=lib/json/json.kso
line='  builtin_append_string acc s'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the encoder's string path moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  builtin_append_string acc s$/  text\/append (escape_onto (text\/append acc 34) s) 34/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
