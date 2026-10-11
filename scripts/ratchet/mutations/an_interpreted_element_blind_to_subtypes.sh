#!/bin/sh
# Stop the interpreter's `join` and `to_bytes` reading an element through a
# subtype, as they did not until 2026-09-28. Each then refuses a list of
# subtypes, and the micro fixture a_list_of_subtypes_joins stops where a
# compiled build prints.
set -e

old1='                        _ => match sub_base(item) {'
old2='                        other => match sub_base(other.clone()) {'
[ "$(grep -cxF "$old1" src/eval/builtins.rs)" -eq 1 ]
[ "$(grep -cxF "$old2" src/eval/builtins.rs)" -eq 1 ]
sed -i.bak -e 's#^                        _ => match sub_base(item) {$#                        _ => match item {#' \
  -e 's#^                        other => match sub_base(other.clone()) {$#                        other => match other.clone() {#' src/eval/builtins.rs
rm -f src/eval/builtins.rs.bak
! grep -qxF "$old1" src/eval/builtins.rs
! grep -qxF "$old2" src/eval/builtins.rs
