#!/bin/sh
# `collapse_diamonds` builds one key per declaration and one per type to drop a
# module reached by two import paths. The keys borrow the names; this makes them
# own copies, which is what the code did before kanso#1417 and costs a heap
# allocation apiece. Measured on the gate's box against the shipped binary:
# module 45,848,443 -> 46,082,589 (+234,146 / +0.5107%), entry 152,256,142 ->
# 153,116,387 (+860,245 / +0.5650%), library 152,591,027 -> 153,439,255
# (+848,228 / +0.5559%), compile_allocs 29,695 -> 30,210 (+515). Only the mask's
# element type and the two name reads move; the keep mask itself stays, so the
# row watches the borrow and not the shape around it.
set -e
grep -q '^        let mut fns: crate::hash::Set<(u32, &str, usize, u32, u32, bool)> =$' src/lib.rs || {
  echo "collapse_diamonds' fn key type changed shape; this needs rewriting" >&2
  exit 1
}
grep -q '^        let mut types: crate::hash::Set<(&str, u32, u32)> = crate::hash::Set::default();$' src/lib.rs || {
  echo "collapse_diamonds' type key changed shape; this needs rewriting" >&2
  exit 1
}
sed -i \
  -e 's/^        let mut fns: crate::hash::Set<(u32, &str, usize, u32, u32, bool)> =$/        let mut fns: crate::hash::Set<(u32, String, usize, u32, u32, bool)> =/' \
  -e 's/^                    f\.name\.as_str(),$/                    f.name.clone(),/' \
  -e 's/^        let mut types: crate::hash::Set<(&str, u32, u32)> = crate::hash::Set::default();$/        let mut types: crate::hash::Set<(String, u32, u32)> = crate::hash::Set::default();/' \
  -e 's/^            \.map(|t| types\.insert((t\.name\.as_str(), t\.span\.line, t\.span\.col)))$/            .map(|t| types.insert((t.name.clone(), t.span.line, t.span.col)))/' \
  src/lib.rs
grep -q '^                    f.name.clone(),$' src/lib.rs
grep -q '^            .map(|t| types.insert((t.name.clone(), t.span.line, t.span.col)))$' src/lib.rs
