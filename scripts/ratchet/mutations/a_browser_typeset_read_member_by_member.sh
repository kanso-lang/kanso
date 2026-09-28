#!/bin/sh
# Hand the browser's engine each typeset's members as declared, as it had
# them until 2026-09-28. A member that is itself a typeset then has no test,
# and the micro fixture a_typeset_holds_a_typeset passes an int over.
set -e
old='                    .flat_typesets()'
[ "$(grep -cxF "$old" src/wasm_backend.rs)" -eq 1 ]
sed -i.bak 's#^                    .flat_typesets()$#                    .types.iter().filter(|t| !t.members.is_empty()).map(|t| (t.name.clone(), t.members.clone())).collect::<crate::hash::Map<String, Vec<String>>>()#' src/wasm_backend.rs
rm -f src/wasm_backend.rs.bak
! grep -qxF "$old" src/wasm_backend.rs
