#!/bin/sh
# The tab lets the groups its walk left out go before it emits (2026-10-23).
# This mutation keeps them, as every other build does, and the browser
# compile peak is the witness.
set -e
target='    let written = emit_written_as(Given::Owned(program), convention, false, true)?;'
n=$(grep -cxF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the tab's emit changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^    let written = emit_written_as(Given::Owned(program), convention, false, true)?;$/    let written = emit_written_as(Given::Owned(program), convention, false, false)?;/' src/codegen.rs
grep -qF 'emit_written_as(Given::Owned(program), convention, false, false)?' src/codegen.rs
