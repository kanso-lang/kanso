

## 2026-10-10 — two values a carry handed back while they were still held

The lap carry (kanso#1827) let the toml port parse a comment holding a
control character and then lose the error it meant to report. The program
died with an unhandled err whose record had no type, born in `toml/fail`.
The interpreter printed `toml/toml_error 10 1 "control character 7 in
comment" "a = 1 # xy"`.

The bug is older than the lap carry. `end_at` reads the byte after a `#`,
calls `skip_comment`, and hands what comes back to `newline_at` in tail
position. The two form a loop that rewinds the arena on the edge back into
`newline_at`. The cursor crosses that rewind uncopied, because inference
counts a slot that holds an int or an err as one that may cross a rewind
the way a number does. On the lap that meets the control character,
`skip_comment` answers an err wrapping a `parse_failure` record, and both
were built in the lap's own storage. The rewind handed that storage back,
`newline_at` answered the err by adding a hop to it, and the hop was
allocated over the record. A hardware watchpoint on the record's type word
caught `k_err_hop` writing it. Main has the same hole: the fixture below
dies on main's compiler in the same way. toml reached it only once #1827
admitted its loop, which main refused because the loop's carried byte may
be bytes.

The carry path already had the answer. `k_beat_iter_carry` returns without
rewinding when a carried slot holds an err, since the callee only passes
the failure on. A plain rewind asked nothing of the arguments it lets
cross. Codegen now tests each argument whose set includes a failure, and
skips the rewind when one of them is an err. That applies on a carrying
edge too, for the arguments the carry does not take. The test is a tag
compare and a branch at each rewinding edge where such an argument exists.

`tests/golden/micro/an_err_built_in_a_lap_outlives_its_rewind.kso` is the
toml loop cut down to three functions. Main's compiler prints `unhandled err
... record born in f/skip`. With the test in place it prints `failed at 3:
stopped` on both engines and in a release build.

The toml port's `many-tables` document showed the second fault once the
first was fixed: the last header's path came back as the path of a key read
two lines later. A carry stage copies its slots in order, and the first time
the walk meets a node that has already been through one copy it switches to
tenured storage for the rest of the stage. A node copied into the carry pair
before that switch could then be shared by a node tenured after it, through
the map that makes a node shared twice copy once. The tenured parent
survived the next stage, the stage's walk stopped there because everything
the parent held directly survived too, and the stage after that wrote over
the pair the shared node lay in. Main has this one as well. When the map
answers with a copy in the pair and the walk is tenuring, the node is now
copied again into tenure and the map points at that copy from then on.

`tests/golden/micro/a_tenured_node_shares_nothing_with_the_carry.kso` builds
the shape directly: a loop carrying a fresh list, last lap's tree, and a new
tree node wrapping the list, with enough garbage per lap that every lap
stages. Main's compiler prints `[200]` in three places where `[400]`, `[600]`
and `[800]` belong.

Neither fix moves an allocation counter. The test before a rewind adds a
compare and a branch at each rewinding edge with an argument that may be an
err, which the emitted code shows: runbench gains 22 branches and 125 lines,
the decoder 4 branches and 15 lines, and seven other benchmarks between 2 and
13 branches.

CI's rows, measured with main's constructor change merged in. The code the
compiler writes grows with the new tests: emitted_branches lands on 1,105 and
emitted_lines on 10,070 for the decoder, emitted_other_branches on 16,143 and
emitted_other_lines on 144,731 across the other benchmarks, and the machine
code total, text, on 4,644,478 bytes, from 16 bytes more on scanbench to 368
on runbench. The work rows move with the change, and which branch costs what
has not been isolated: work_encodebench lands on 2,393,926,456, up 10,064,560
or 0.42%; work_runbench on 1,131,539,079, up 68,142; work_digestbench on
5,597,963, up 36,820; work_jsonbench on 860,549,995 and work_oneshot on
13,176,892, up 450 and 364; and livebench falls 259. Building costs a little
more: codegen_instructions_dev lands on 118,918,569, up 139, and release
codegen falls 8 to 453,994,110. In the browser, browser_compile_instructions
lands on 388,494,007, up 217,425, and browser_run_instructions on 33,508,472,
up 1,119. Both changes fix what programs print, so the floor follows them,
from 90.25040 to 90.24995.
