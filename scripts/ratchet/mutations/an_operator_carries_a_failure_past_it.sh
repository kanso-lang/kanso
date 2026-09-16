#!/bin/sh
# The 2026-09-16 ruling: a bare err arriving at an operator halts there with
# the report. This restores the railway at `+` on native, handing the failure
# past the operator the way every operator did until then, so the miss in
# a_miss_under_the_bang_halts_at_the_operator reaches the endpoint and the
# runtime corpus reads the endpoint's report where the golden holds the
# operator's.
set -e
grep -qF 'k_die_err_operand("+", a, b);' src/runtime.c
sed -i 's/k_die_err_operand("+", a, b);/return k_both_or_either(a, b);/' src/runtime.c
! grep -qF 'k_die_err_operand("+", a, b);' src/runtime.c
