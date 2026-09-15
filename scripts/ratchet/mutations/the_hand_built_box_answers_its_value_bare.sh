#!/bin/sh
# `effect v` is a box that settles to v when it runs. The interpreter is the
# oracle for that, and the micro corpus compares it with native on every
# fixture. This mutation makes the interpreter answer the value itself,
# unboxed, so a chain that starts from `effect 5` is handed a 5 where the
# words expect a box, and the two engines stop agreeing on
# the_box_built_by_hand.
set -e
grep -q '^            return Ok(Value::Desc(Rc::new(Desc::Settled(v))));$' src/eval.rs || {
  echo "the effect builtin's arm changed shape; this needs rewriting" >&2
  exit 1
}
sed -i 's@^            return Ok(Value::Desc(Rc::new(Desc::Settled(v))));$@            return Ok(v);@' src/eval.rs
grep -q '^            return Ok(v);$' src/eval.rs
