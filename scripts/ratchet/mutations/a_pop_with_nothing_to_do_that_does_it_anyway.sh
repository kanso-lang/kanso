#!/bin/sh
# A beat pop whose result is a heap value migrates three registries and hands
# up the depth's tenure blocks. On 507,678 of runbench's 507,685 pops every
# one of those is empty, and the pop still paid the six callee-saved pushes
# the copy and the migrates need, plus three zero stores over zeros: 70
# instructions a pop. Since 2026-09-07 the pop tests the carry flag, the
# registry summary k_reg_any[d] and the tenure block first, and a pop with
# nothing to do returns from a frame that pushed nothing; the rest is out of
# line. This mutation sends every pop the long way again. No counter moves
# either way -- the tests are of state the migrates would have found empty --
# so the work vein is the witness: runbench 2,717,267,333 -> 2,692,922,207 on
# the container when the split landed.
set -e
grep -q '^            if (!rewound && !k_carries\[d\].used_flag && !k_reg_any\[d\]$' src/runtime.c || {
  echo "k_beat_pop's fast path changed shape; rewrite this" >&2
  exit 1
}
sed -i 's|^            if (!rewound && !k_carries\[d\].used_flag && !k_reg_any\[d\]$|            if (0 \&\& !rewound \&\& !k_carries[d].used_flag \&\& !k_reg_any[d]|' \
  src/runtime.c
grep -q '^            if (0 && !rewound && !k_carries\[d\].used_flag && !k_reg_any\[d\]$' src/runtime.c
