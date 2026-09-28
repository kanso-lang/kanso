#!/bin/sh
# Hand the native backend each typeset's members as declared, as it had them
# until 2026-09-28. A member that is itself a typeset then has no test, and
# the micro fixture a_typeset_holds_a_typeset passes an int over.
set -e
old='        typesets: program.flat_typesets(),'
[ "$(grep -cxF "$old" src/codegen.rs)" -eq 1 ]
sed -i.bak 's#^        typesets: program.flat_typesets(),$#        typesets: program.types.iter().filter(|t| !t.members.is_empty()).map(|t| (t.name.clone(), t.members.clone())).collect(),#' src/codegen.rs
rm -f src/codegen.rs.bak
! grep -qxF "$old" src/codegen.rs
