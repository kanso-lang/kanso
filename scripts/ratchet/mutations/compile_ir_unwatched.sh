#!/bin/sh
# The shape this vein exists to see, and the one no other gate can: work the
# front end does that costs instructions and nothing else. Point inference's
# maps back at std's hasher and the compiler retires millions more
# instructions per check, while compile_allocs, the fixpoint rounds and the
# expression visits stay where they were.
set -e
sed -i.bak 's|^use crate::hash::Map as HashMap;$|use std::collections::HashMap;|' src/infer.rs
rm -f src/infer.rs.bak
grep -q '^use std::collections::HashMap;' src/infer.rs
