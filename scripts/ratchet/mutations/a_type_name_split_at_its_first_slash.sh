#!/bin/sh
# A TYPE NAME INSIDE A SHELL IS SEVERAL NAMES: `<g/cell>effect` holds one
# qualified name and one bare one. The import check scans the runs of name
# characters and marks the qualified ones; this mutation hands it the whole
# spelling instead, which is what it did until 2026-09-16 — `<g/cell>effect`
# then splits at its first slash and answers `<g`, a qualifier no import
# matches. The qualified_yield module is refused twice for a program that
# compiles, so its spec goes red.
set -e
grep -q "^        for part in name.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '/')) {$" src/lib.rs || {
  echo "the qualifier scan changed shape; this needs rewriting" >&2
  exit 1
}
sed -i "s@^        for part in name.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '/')) {\$@        for part in [name] {@" src/lib.rs
grep -q '^        for part in \[name\] {$' src/lib.rs
