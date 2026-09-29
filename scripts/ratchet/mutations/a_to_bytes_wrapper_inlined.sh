#!/bin/sh
# Inline text/to_bytes into its caller again, as the compiler did until
# 2026-09-29. Its err is then born in the caller's module, a rescue there hands
# it on as its own, and the micro corpus goes red.
set -e
old='    "builtin_to_bytes",'
[ "$(grep -cxF "$old" src/inline.rs)" -eq 1 ]
sed -i.bak '/^    "builtin_to_bytes",$/d' src/inline.rs
rm -f src/inline.rs.bak
! grep -qxF "$old" src/inline.rs
