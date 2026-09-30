#!/bin/sh
# Forget the positions an arm dropped for an unbuilt type was reading. A cell
# handed to such a position goes on unforced, and the runtime corpus fixture
# names `found_in` and `next` where every engine should name `find`.
set -e
old='        }) || self.unread_positions.iter().any(|(g, n, p)| g == callee && *n == arity && *p == i)'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        }) || self.unread_positions.iter().any(|(g, n, p)| g == callee \&\& \*n == arity \&\& \*p == i)$#        }) || self.unread_positions.iter().any(|(g, n, p)| g == callee \&\& *n == arity \&\& *p == i \&\& false)#' src/codegen.rs
rm -f src/codegen.rs.bak
grep -qF '*p == i && false)' src/codegen.rs
