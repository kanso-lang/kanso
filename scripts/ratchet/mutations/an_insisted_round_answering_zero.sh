#!/bin/sh
# Answer 0 from `math/round!` for a non-finite float instead of a failure in
# the box. The rescue arm is then never reached, and the micro corpus goes red.
set -e
old='  err "round! takes a finite number, got {x}"'
[ "$(grep -cxF "$old" lib/math/math.kso)" -eq 1 ]
sed -i.bak 's#^  err "round! takes a finite number, got {x}"$#  0#' lib/math/math.kso
rm -f lib/math/math.kso.bak
! grep -qxF "$old" lib/math/math.kso
