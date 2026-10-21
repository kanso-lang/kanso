#!/bin/sh
# A map's key goes through the untyped escape again.
#
# `key_onto` takes the key as a string and hands it to `append_key`, which
# writes the quotes, the escaped key and the colon in one claim. This writes
# it the old way, the opening quote, `escape_onto` on the bare `k` of the
# entry pattern and the closing pair, so the output is the same and every key
# pays for three claims and an untyped escape. The instruction rows of every
# program that encodes a map move.
set -e
f=lib/json/json.kso
line='  encode_onto (key_onto acc k) v'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the encoder's key path moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  encode_onto (key_onto acc k) v$/  encode_onto (text\/append (escape_onto (text\/append acc 34) k) "\\":") v/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
