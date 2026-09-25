#!/bin/sh
# A conjunction of byte compares over one bytes value and one int is read as a
# single range test and plain compares. This mutation turns the fusion off, so
# every read pays its own overflow check, bounds test and none merge again.
# The answers are the same; the work vein is the witness.
set -e
line='        let Some((x, p, reads)) = byte_run(args) else { return Ok(None) };'
[ "$(grep -cxF "$line" src/codegen.rs)" -eq 1 ] || {
  echo "the byte run's match moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/^        let Some((x, p, reads)) = byte_run(args) else { return Ok(None) };$/        let Some((x, p, reads)) = byte_run(args).filter(|_| false) else { return Ok(None) };/' src/codegen.rs
if grep -qxF "$line" src/codegen.rs; then exit 1; fi
