// The runtime every compiled program links, in the order its parts are
// compiled. It is one translation unit: each part sees what the parts above it
// declared, and none of them is a header.
//
// clang compiles this file by path, and the includes resolve beside it. The
// compiler embeds the runtime instead, so build.rs joins the same parts, in
// this order, into the one text `kanso::RUNTIME_C` holds. That text is what
// the object cache digests and what the release build's hot unit is cut from.
// build.rs refuses any line here that is not a comment, a blank line or a
// part, so the two ways in compile the same thing.
#include "runtime/core.c"
#include "runtime/values.c"
#include "runtime/effects.c"
#include "runtime/collections.c"
#include "runtime/text.c"
#include "runtime/numbers.c"
#include "runtime/main.c"
