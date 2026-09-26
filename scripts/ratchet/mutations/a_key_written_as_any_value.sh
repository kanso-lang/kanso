#!/bin/sh
# A map's key goes through the untyped escape again.
#
# `key_onto` takes the key as a string, so the emitter reaches escape_onto on
# a value it already knows the type of. Called on the bare `k` of the entry
# pattern, the output is the same and every key pays for not knowing, so the
# instruction rows of every program that encodes a map move.
set -e
f=lib/json/json.kso
line='  encode_onto (key_onto acc k) v'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the encoder's key path moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^  encode_onto (key_onto acc k) v$/  encode_onto (text\/append (escape_onto acc k) "\\":") v/' "$f"
if grep -qxF "$line" "$f"; then exit 1; fi
