#!/bin/sh
# A new diagnostic is added to the compiler and no golden pins its text.
#
# That is the whole rule this gate holds: a message with no golden can be
# reworded, weakened or lost with nothing going red, and the error corpus grew
# by whoever remembered — which is how forty-one of them ended up unpinned at
# once. Adding a diagnostic means adding a golden, and the excused list in
# tests/golden/unpinned_diagnostics.txt only shrinks.
#
# Appended as a function nobody calls, so the tree still builds and every other
# gate stays green: what is being tested is whether the SCAN finds an unpinned
# message, not whether the compiler can raise it. `Diagnostic::new(` is what
# the scan keys on, and the message is literal text so it has something stable
# to match.
set -e
grep -qF 'Diagnostic::new(' src/lexer.rs || {
  echo "the diagnostic constructor moved; this mutation needs rewriting" >&2
  exit 1
}
cat >> src/lexer.rs <<'RUST'

#[allow(dead_code)]
fn unpinned_bait() -> Diagnostic {
    Diagnostic::new(
        "formatting",
        "this diagnostic has no golden and the corpus has never seen it".to_string(),
        Span { line: 1, col: 1 },
    )
}
RUST
grep -qF 'this diagnostic has no golden' src/lexer.rs
