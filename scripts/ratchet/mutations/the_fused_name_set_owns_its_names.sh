#!/bin/sh
# `fuse_enumerable` builds a set of std/list's short names, asks it `contains`
# once per declaration, and drops it. The names borrow; this makes the set own
# copies, a `String` apiece. Measured on the gate's box against the shipped
# binary: module 45,848,443 -> 45,953,024, entry 152,256,142 -> 152,595,147,
# library 152,591,027 -> 152,960,912, compile_allocs 29,695 -> 29,991.
set -e
grep -q '^    let std_names: crate::hash::Set<&str> = program$' src/lib.rs || {
  echo "fuse_enumerable's name set changed shape; this needs rewriting" >&2
  exit 1
}
grep -q '^        .map(|d| ast::split_qual(&d.name).map(|(_, s)| s).unwrap_or(&d.name))$' src/lib.rs || {
  echo "fuse_enumerable's short-name read changed shape; this needs rewriting" >&2
  exit 1
}
sed -i \
  -e 's/^    let std_names: crate::hash::Set<&str> = program$/    let std_names: crate::hash::Set<String> = program/' \
  -e 's/^        \.map(|d| ast::split_qual(\&d\.name)\.map(|(_, s)| s)\.unwrap_or(\&d\.name))$/        .map(|d| ast::split_qual(\&d.name).map(|(_, s)| s.to_string()).unwrap_or_else(|| d.name.clone()))/' \
  src/lib.rs
grep -q '^    let std_names: crate::hash::Set<String> = program$' src/lib.rs
