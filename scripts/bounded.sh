#!/bin/sh
# What a program started by a test or a ratchet gate may allocate: 6 GiB.
#
# A compiler that is wrong can build a binary that allocates without end, and
# the kernel only ends it once the machine is out of memory, which ends CI's
# runner with it and leaves a log nobody can read. Under this limit the
# allocation fails at 6 GiB and the test that started it fails by name.
#
# A program that runs away has written to nearly all of its limit by the time
# it is refused, so the limit is also what one such program costs the
# machine, and it is no larger than the tests need. The corpus's programs
# want megabytes. The test that sets the number is
# `an_answer_above_two_gibibytes_reads_back` in tests/wasm_engine.rs, which
# grows a wasm memory past two gibibytes inside the test process: it fails
# under 4 GiB and passes under 5.
#
# Two things use it. Cargo runs every test binary through it on Linux
# (`runner` in .cargo/config.toml), so the test and whatever it starts inherit
# the limit. The ratchet sources it before each command it runs.
#
# The limit is on data (`-d`): the heap and every private writable mapping,
# which is what an allocator asks for. A limit on address space (`-v`) stops
# the same program, and also stops headless Chromium from starting, because a
# browser reserves far more address space than it writes to, and two of the
# ratchet's gates run one.
#
# It is the soft limit, the one the kernel enforces, so a spec that has to see
# a program started without it can lift it again. Linux since 4.7 counts
# mappings under it; a system that does not carries on unbounded.
ulimit -S -d 6291456 2>/dev/null
[ "$#" -eq 0 ] || exec "$@"
