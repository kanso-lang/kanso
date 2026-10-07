#!/bin/sh
# An arm whose pattern names a record type the program never builds is dropped
# before emission, with what only it calls. This mutation keeps every arm, so a
# program that maps once emits every lazy adapter std/list declares. The output
# is the same; the witness is an_arm_no_value_reaches_is_not_emitted, whose
# mapping program then defines next_skipped.
set -e
old='    let (program, unread_positions) = match without_unbuilt_arms(&program) {'
grep -qxF "$old" src/codegen.rs || {
  echo "the arm prune moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|^    let (program, unread_positions) = match without_unbuilt_arms(&program) {$|    let (program, unread_positions) = match None::<(Vec<bool>, UnreadPositions)> {|' src/codegen.rs
[ "$(grep -cF 'match None::<(Vec<bool>, UnreadPositions)> {' src/codegen.rs)" -eq 1 ]
