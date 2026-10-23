#!/bin/sh
# A numbered name is UTF-8 by construction, and reading one no longer checks
# it (2026-10-23). This mutation checks it on every read again, and the
# browser compile row is the witness.
set -e
f=src/codegen.rs
target='        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }'
n=$(grep -cxF "$target" $f)
[ "$n" -eq 1 ] || { echo "Numbered's read changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^        unsafe { std::str::from_utf8_unchecked(&self.buf\[..self.len as usize\]) }$/        std::str::from_utf8(\&self.buf[..self.len as usize]).unwrap_or_default()/' $f
grep -qxF '        std::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or_default()' $f
