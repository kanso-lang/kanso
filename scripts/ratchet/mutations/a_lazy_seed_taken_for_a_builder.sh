#!/bin/sh
# The native engine's first join hands the adoption the seed after forcing it.
#
# A seed that arrived lazily was left as it came by the loop's entry, and what
# it forces to is a finished string rather than a builder. Handed the forced
# value, the adoption takes it for a builder and the append dies.
set -e
f=src/codegen.rs
grep -q "fn maybe_force" src/codegen.rs
grep -qF '"{adopted} = call %KValue @k_b_adopt(%KValue {raw}, %KValue {t})"' "$f" || {
  echo "the lazy seed handed to k_b_adopt moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's/"{adopted} = call %KValue @k_b_adopt(%KValue {raw}, %KValue {t})"/"{adopted} = call %KValue @k_b_adopt(%KValue {value}, %KValue {t})"/' "$f"
grep -qF '"{adopted} = call %KValue @k_b_adopt(%KValue {value}, %KValue {t})"' "$f"
