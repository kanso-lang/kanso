#!/bin/sh
# The description of a module says every declaration is in a file with no
# name, so a convention that orders code by its file's name falls back on
# whatever order the names themselves give. Every other program is unmoved.
set -e
grep -c 'sources.insert(MapKey::Str(bare.to_string().into()), text(&file));' src/derive.rs | grep -qx 1 || {
  echo "the description's sources line moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/sources.insert(MapKey::Str(bare.to_string().into()), text(&file));/sources.insert(MapKey::Str(bare.to_string().into()), text(""));/' src/derive.rs
grep -q 'sources.insert(MapKey::Str(bare.to_string().into()), text(""));' src/derive.rs
