#!/bin/sh
# The same rule as a_diagnostic_arrives_without_a_golden, at the shortest
# message the scan will keep.
#
# The scan matches a diagnostic by its leading literal run — the text before
# the first interpolation — and a run has to be specific enough that finding
# it in the corpus means finding this message. That floor sat at twelve
# characters and hid three reachable diagnostics whose runs are ten:
# `function `, `constant ` and `the name `, each with its opening backtick.
# Nothing was red while they went unpinned, because the scan never reached
# their sites.
#
# So the floor is a number the gate depends on, and this holds it there. The
# bait's run is `unpinned ` plus a backtick, exactly ten characters: it goes
# unnoticed the moment the floor moves back up.
set -e
grep -qF 'Diagnostic::new(' src/lexer.rs || {
  echo "the diagnostic constructor moved; this mutation needs rewriting" >&2
  exit 1
}
cat >> src/lexer.rs <<'RUST'

#[allow(dead_code)]
fn unpinned_short_bait(what: &str) -> Diagnostic {
    Diagnostic::new(
        "formatting",
        format!("unpinned `{what}` at ten characters"),
        Span { line: 1, col: 1 },
    )
}
RUST
grep -qF 'unpinned `{what}`' src/lexer.rs
