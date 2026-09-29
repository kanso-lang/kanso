#!/bin/sh
# Hold a partial over a declared group in a closure, as native did until
# 2026-09-29. `k = &f1 0; k bad` then answers the err before f1 runs, the
# trace drops f1, and the runtime corpus goes red. The partial is lowered to
# the lambda over the group where it would have been held over the group's
# value.
set -e
old='        self.emit_partial_value(f, &Name::new(name), supplied, span)'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        self.emit_partial_value(f, \&Name::new(name), supplied, span)$#        let lambda = self.partial_lambda(name, supplied, span)?;\n        self.emit_expr(f, \&lambda)#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
