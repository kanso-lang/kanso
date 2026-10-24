#!/bin/sh
# Every lambda keeps `tailcc` in a module that defines `k_tcall{n}`, because
# the twin jumps to a lambda through a closure's pointer and the text shows no
# edge. This mutation stops collecting the lambdas, and the narrowing's spec
# `kept_beside_the_twin` sees one lose the convention.
set -e
target='                    lambdas.extend(current.clone());'
n=$(grep -cxF "$target" src/codegen/narrow.rs)
[ "$n" -eq 1 ] || { echo "the lambda collection changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's/^                    lambdas.extend(current.clone());$/                    lambdas.extend(None::<String>);/' src/codegen/narrow.rs
grep -qF 'lambdas.extend(None::<String>);' src/codegen/narrow.rs
