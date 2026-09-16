#!/bin/sh
# The 2026-09-16 ruling: `!` is the programmer's word that the read is in
# range, and the checker drops the miss from `xs[i]!`. This puts the miss
# back as an err in the answer set, the reading before the ruling, so a `!`
# declaration whose only failure is a strict index answers a failure the
# checker can see again and a_bang_name_whose_only_failure_is_a_promise
# compiles clean where its golden holds the naming refusal.
set -e
grep -qF '                true => 0,' src/infer.rs
sed -i 's/^                true => 0,$/                true => ERR,/' src/infer.rs
grep -qF '                true => ERR,' src/infer.rs
