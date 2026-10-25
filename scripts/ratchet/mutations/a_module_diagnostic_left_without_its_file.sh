#!/bin/sh
# Leave the diagnostics raised over a module's merged program without the file
# of the declaration they are about, as they were until 2026-09-29: the helper
# that places the walks' diagnostics does nothing, and the naming, tie,
# unused-value and constant-cycle refusals name no file. The tie refusal sits
# in src/check/arms.rs, which check.rs includes. Each then prints the module's
# name and no location, and the error corpus and tests/sibling_types.rs go
# red.
set -e
helper='            d.file = Some(std::sync::Arc::clone(file));'
[ "$(grep -cxF "$helper" src/check.rs)" -eq 1 ]
[ "$(grep -cxF '                .about(file),' src/check.rs)" -eq 3 ]
[ "$(grep -cxF '                        .about(&b.file),' src/check/arms.rs)" -eq 1 ]
[ "$(grep -cxF '                    .about(&decl.file),' src/check.rs)" -eq 1 ]
[ "$(grep -cxF '                        .about(&decl.file),' src/check.rs)" -eq 1 ]
sed -i.bak \
  -e 's#^            d.file = Some(std::sync::Arc::clone(file));$#            let _ = (\&d, file);#' \
  -e '/^                \.about(file),$/d' \
  -e '/^                    \.about(&decl\.file),$/d' \
  -e '/^                        \.about(&decl\.file),$/d' \
  src/check.rs
rm -f src/check.rs.bak
sed -i.bak -e '/^                        \.about(&b\.file),$/d' src/check/arms.rs
rm -f src/check/arms.rs.bak
! grep -qxF "$helper" src/check.rs
! grep -qxF '                .about(file),' src/check.rs
