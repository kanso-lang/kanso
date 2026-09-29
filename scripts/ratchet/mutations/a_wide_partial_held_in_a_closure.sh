#!/bin/sh
# Hold a native partial over a declared group in a closure, as native did for
# a group of more than four parameters until 2026-09-29 and for one of more
# than eight until later that day. `k = &f5 0 0; k 0 0 bad` then answers the
# err before f5 runs, the trace drops f5, and the runtime corpus goes red.
set -e
old='        self.partial_lambda(name, supplied, span)?;'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        self.partial_lambda(name, supplied, span)?;$#        let lambda = self.partial_lambda(name, supplied, span)?;\n        return self.emit_expr(f, \&lambda);#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
