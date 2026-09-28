#!/bin/sh
# Write the runtime's C source straight onto the shared path, as every build
# did before 2026-09-28.
#
# A build that compiles the runtime cold truncates the file under another
# build's clang, which dies reading it, and the cold-runtime race in
# tests/concurrent_build.rs answers with clang's crash instead of the program.
set -e
sed -i.bak 's|^    std::fs::rename(&c_staging, &c_path)?;$|    std::fs::write(\&c_path, source)?;|' src/main.rs
rm -f src/main.rs.bak
grep -q '^    std::fs::write(&c_path, source)?;$' src/main.rs
