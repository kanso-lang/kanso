#!/bin/sh
# An application takes its arguments off a shared stack as a vector of exactly
# their number. This mutation gives each vector the four slots growing one from
# nothing used to leave, and the front end's peak holds them again.
#
# compile_memory.sh reads compile_peak_bytes about 60,000 bytes over the
# golden.
set -e
line='        let args = ARGS.with(|stack| stack.borrow_mut().split_off(base));'
grep -qxF "$line" src/parser.rs || {
  echo "parse_app's hand-off changed shape; rewrite this" >&2
  exit 1
}
sed -i 's/^        let args = ARGS.with(|stack| stack.borrow_mut().split_off(base));$/        let mut args = ARGS.with(|stack| stack.borrow_mut().split_off(base));\n        args.reserve(4usize.saturating_sub(args.len()));/' src/parser.rs
grep -qF '        args.reserve(4usize.saturating_sub(args.len()));' src/parser.rs
