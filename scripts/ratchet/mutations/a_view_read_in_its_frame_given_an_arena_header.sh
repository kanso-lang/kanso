#!/bin/sh
# `framed_views` finds the byte views a function only reads while its frame
# stands, and the emitter writes their headers into the frame instead of the
# arena. This mutation empties the set, so every view is allocated again. The
# bytes out are the same; the allocation counters are the witness, since the
# run program allocates one header per string it escapes.
set -e
target='        framed_views: framed_views(program, &forwarder_map(program)),'
n=$(grep -cF "$target" src/codegen.rs)
[ "$n" -eq 1 ] || { echo "the framed-view set changed shape ($n); rewrite this" >&2; exit 1; }
sed -i 's|^        framed_views: framed_views(program, &forwarder_map(program)),$|        framed_views: crate::hash::Set::default(),|' src/codegen.rs
[ "$(grep -cF '        framed_views: crate::hash::Set::default(),' src/codegen.rs)" -eq 1 ]
