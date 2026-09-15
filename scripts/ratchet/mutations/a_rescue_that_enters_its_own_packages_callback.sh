#!/bin/sh
# The foreign-only rescue licence lives at the word since 2026-09-15: a
# `.?` written in the package that raised the failure hands it on without
# entering its callback. This deletes the interpreter's licence arm, so a
# rescue enters its callback on every failure, own or foreign. The micro
# fixture a_rescue_hands_on_its_own_packages_failure prints "claimed mine"
# under it where the golden says "still failed, mine", and the runtime
# fixture a_rescue_on_its_own_boxed_failure_reaches_the_endpoint prints
# instead of reaching the endpoint. The interpreter is the oracle, so the
# mutation lands there; native and wasm keep their licence and the wasm
# walk disagrees with the interpreter for the same reason.
set -e
grep -qF 'Word::Rescue if own_failure(&cause, raised) => Ok(yielded),' src/eval.rs
sed -i '/Word::Rescue if own_failure(&cause, raised) => Ok(yielded),/d' src/eval.rs
! grep -qF 'Word::Rescue if own_failure(&cause, raised)' src/eval.rs
