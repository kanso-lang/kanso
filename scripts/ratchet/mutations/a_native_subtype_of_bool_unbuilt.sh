#!/bin/sh
# Stop a native build knowing `bool` as a subtype's parent, as it did not
# until 2026-09-28. `type flag bool` is then an unknown type to both
# backends, and the micro fixture a_subtype_of_bool_is_a_condition does not
# build where the interpreter prints.
set -e
old='            "bool" => -100,'
[ "$(grep -cxF "$old" src/codegen/dispatchers.rs)" -eq 1 ]
sed -i.bak '/^            "bool" => -100,$/d' src/codegen/dispatchers.rs
rm -f src/codegen/dispatchers.rs.bak
! grep -qxF "$old" src/codegen/dispatchers.rs
