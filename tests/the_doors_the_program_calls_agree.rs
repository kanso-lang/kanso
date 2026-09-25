//! The runtime functions that take `preserve_none` are the ones the emitter
//! calls with it.
//!
//! src/runtime.c defines a few of the doors the program calls with
//! `K_DOORCC`, which is `preserve_none` wherever the host's clang takes it,
//! and the emitter writes `preserve_nonecc` on the declares and calls of the
//! names in `kanso::codegen::PRESERVE_NONE_DOORS`. A door on one side and not
//! the other is called with its arguments in the wrong registers, and the
//! program reads garbage rather than failing to build. So the two lists are
//! read and compared here.
//!
//! Watched red with `k_b_utf8` taken off the emitter's list: the runtime
//! defined four doors and the emitter named three.

use std::collections::BTreeSet;

#[test]
fn every_door_the_runtime_defines_is_one_the_emitter_calls() {
    let runtime = include_str!("../src/runtime.c");
    let defined: BTreeSet<&str> = runtime
        .lines()
        .filter_map(|l| l.strip_prefix("K_DOORCC KValue "))
        .filter(|l| l.trim_end().ends_with('{'))
        .filter_map(|l| l.split('(').next())
        .collect();
    let called: BTreeSet<&str> = kanso::codegen::PRESERVE_NONE_DOORS.into_iter().collect();
    assert_eq!(defined, called);
}
