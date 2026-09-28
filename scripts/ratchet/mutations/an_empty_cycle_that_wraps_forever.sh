#!/bin/sh
# Take away the empty cycle's end, as lib/list had it before 2026-09-28. Its
# `next` wraps to the first position and asks again forever, and the spec
# an_empty_cycle_ends finds no answer from either engine inside its deadline.
set -e
grep -q '^  return done if length source == 0$' lib/list/list.kso
sed -i.bak '/^  return done if length source == 0$/d' lib/list/list.kso
rm -f lib/list/list.kso.bak
! grep -q '^  return done if length source == 0$' lib/list/list.kso
