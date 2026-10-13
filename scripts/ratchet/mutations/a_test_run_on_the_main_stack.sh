#!/bin/sh
# kanso test runs its tests on a stack the size of the main thread's.
#
# The interpreter's depth limit is sized for the deep thread `run --interp`
# spawns. On an 8 MiB stack a test that recursed too far overflowed Rust's
# stack first, the process aborted, and no other test reported.
# a_test_that_runs_out_of_stack_fails_alone sees the abort.
set -e
f=src/main.rs
grep -q "fn run_tests_on_stack" src/main.rs
line='            .spawn_scoped(scope, || run_tests_on_stack(target, suites))'
[ "$(grep -cxF "$line" "$f")" -eq 1 ] || {
  echo "the deep test thread moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^            \.stack_size(1 << 30)$/{
N
s/^            \.stack_size(1 << 30)\n\(            \.spawn_scoped(scope, || run_tests_on_stack\)/            .stack_size(8 << 20)\n\1/
}' "$f"
grep -qxF '            .stack_size(8 << 20)' "$f"
