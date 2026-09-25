#!/bin/sh
# The check that refuses a private name at an import looks itself up under a
# key nothing declares, so it never fires: `pub` stops meaning anything across
# a module boundary and a program that must be REFUSED compiles instead.
#
# `pub` is the whole visibility system, and this is the one gate that runs
# whole modules on disk — a directory of files sharing one namespace, read
# from inside and from an importer. It asks two questions of every case, and
# this mutation is in the SHARED front end, so the engines go on agreeing
# perfectly. What moves is the verdict, which agreement alone cannot see, and
# this sweep is the only place that question is put to a real directory.
set -e
target='if let Some(false) = exports.get(name.as_str()) {'
grep -qF "$target" src/lib.rs || {
  echo "the import visibility check moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|if let Some(false) = exports.get(name.as_str()) {|if let Some(false) = exports.get("a name no module declares") {|' src/lib.rs
grep -qF 'exports.get("a name no module declares")' src/lib.rs
