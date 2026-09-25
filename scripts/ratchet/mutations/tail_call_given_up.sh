#!/bin/sh
# The release profile stops keeping tailcc for any arm at all, which is what it
# did before the register cap was measured.
set -e
sed -i.bak 's/regs > widest/regs > 0/' src/main.rs
rm -f src/main.rs.bak
grep -q 'regs > 0' src/main.rs
