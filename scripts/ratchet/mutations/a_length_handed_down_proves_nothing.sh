#!/bin/sh
# A loop that hands its bound down as a list's length proves its reads in
# range with `i > n` alone: `handed_lengths` finds that `n` is `length xs` for
# the whole arm, and the arm starts with that fact. This mutation drops the
# fact, so the guard says nothing about `xs[i]` again, and the micro corpus
# reads a_length_handed_down_bounds_the_read refused where it printed.
set -e
grep -q '^            ctx.facts.extend(lengths\[i\].iter().map(|&(n, xs)| Fact::NamedLength(n, xs)));$' src/infer.rs || {
  echo "the handed-down length fact changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^            ctx.facts.extend(lengths\[i\].iter().map(|&(n, xs)| Fact::NamedLength(n, xs)));$@            let _ = \&lengths;@' src/infer.rs
grep -q '^            let _ = &lengths;$' src/infer.rs
