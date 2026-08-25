# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-08-23 — the gate that watches the page could not see the history

The page-drift check counts log entries written since docs/compiler.html last
changed and fails past a budget of three. It reported `0/3` on every pull
request while the page fell twenty-two entries behind, and the reason is one
line in the workflow, two steps above it:

    git fetch origin main --depth 1

The job checks out with `fetch-depth: 0`, so the history was there; that fetch
truncates it for every step after it. In a truncated history the shallow
boundary commit looks like the one that created every file, so
`git log -1 -- docs/compiler.html` answers with the tip, and the diff from the
tip to HEAD contains no log entries, so the count is zero.

MEASURED, on main's tip in a fresh clone of this repository:

    full history            the gate fails, 22 entries ahead of the page
    after `--depth 1`       `page drift 0/3`, exit 0

Two fixes, because either alone leaves the class open. The workflow's two
fetches drop `--depth 1` — nothing in that job wanted a shallow one, and the
checkout had already fetched everything. And the gate now asks
`git rev-parse --is-shallow-repository` first and refuses to answer on a
truncated clone, watched refusing before it was trusted. A gate that cannot see
must not report success, which is the same rule as never trusting a spec you
have not seen fail.

This is what the ratchet exists to catch and could not: its rows prove a gate's
own script goes red on a defect in the tree, and this defect was in the
workflow around the script. The gate's script was fine. The clone it read was
not.

Found while checking why PR #985 has sat red for two days. That one is a
different story and a real one: with its goldens regenerated — the decoder's
emitted lines 11,593 to 11,603, the front end's rounds on lib/json 28 to 30,
visits 23,224 to 23,345, peak 871,649 to 878,422 bytes — welfare reads 84.69
against a floor of 84.85. It is not a stale golden. By the weights as written
that change costs the project 0.16 points, and the branch owes either the
reason it is worth it or the compile cost back.

## 2026-08-23 — a sweep of the refusals, one divergence re-found and two messages fixed

The `stale_a_panel` port died with `split takes two strings`, which named
neither what it got nor where. That message is one of about twenty runtime
refusals written separately for each engine, and the coverage ratchet does not
reach them: `scripts/diagnostic_coverage` scans `Diagnostic::new(` literals,
which are the check-time diagnostics, and a `RuntimeError` is not one.

So the twelve most reachable were driven on both engines with a wrong argument
hidden behind a call, since a literal is refused before the program runs.
Eleven agree word for word — split, chars, bytes, join, slice, push, length,
char_code, from_code, utf8, and to_int on an int, which answers rather than
refusing.

MEASURED, the twelfth:

    text/to_float ["a"]
    native:  to_float takes a string, bytes, or number, not ["a"]
    oracle:  error[endpoint]: unhandled err reached the entry:
             "bytes are not a number"

Two engines, two answers, and not only the wording: native refuses at runtime
where the oracle answers an err VALUE, which is a different channel. The cause
is structural and already known — the interpreter has no distinct bytes value,
so a list goes through `bytes_to_str`, and native's `K_BYTES` tag refuses
anything that was never bytes. It was measured on 2026-08-02 and recorded, and
the record went into the archive where nothing on a live list mentioned it
again. It is in pending-gavels now, under also-open, with both ways out and
what each costs.

The sweep also showed why nothing caught it. `scripts/diagnostic_differential`
already drives every std function with a wrong argument on both engines, and
its one wrong value is a record — chosen because a record is wrong for
everything and cannot be a literal. A record reaches the same refusal on both
engines. Only a list reaches the one that differs, and a list is a legitimate
argument to enough of the surface that probing with one needs care: `list/cycle`
would not return. Widening the probe waits on the ruling that makes to_float's
answer knowable.

SHIPPED from the sweep, because it needed no ruling: `to_int` and `to_float`
both accept the bytes a file read hands back, and neither said so — "to_int
takes a string", "to_float takes a string or int". Both now name every kind
they take and what arrived instead, the way `length` has all along, in both
engines and byte-identical. Two fixtures pin them, watched red first. The
arguments are a float and a `none`, which render without a module name, so the
direct run and the run through an import say the same words and one golden
covers both.

The machine-code gate priced it: rendering the refused value costs 48 bytes of
`.text` in four of the eight benchmarks and nothing in the other four, where the
linker had already dropped the path. `bench/text_golden.txt` is regenerated on
that reading. Every allocation counter is flat, both compile goldens are
byte-identical, and welfare holds at 84.85.

## 2026-08-23 — the bytes fork is six functions, and four of them answer

The entry above records `text/to_float ["a"]` diverging and cites the archive's
prediction that a ruling would settle `append`, `find2`, `find2_below` and
`utf8` at the same time. Those four are measured now, driven with a list on
both engines, and the prediction was right and understated.

    text/append ["a"] "x"           native refuses    oracle: ["a" 120]
    text/append [65 66] "x"         native refuses    oracle: [65 66 120]
    text/find2 [65 66] 1 65 66      native refuses    oracle: 1
    text/find2_below [65 66] ...    native refuses    oracle: 1
    text/utf8 ["a"]                 native: an err    oracle: a refusal
    text/to_float ["a"]             native refuses    oracle: an err

Four of the six are worse than a wording difference. The oracle ANSWERS where
native refuses, so a program written against the oracle runs and the same
program compiled dies — which is the differential law's hardest case, and it
points the wrong way: the oracle is meant to be the engine that can express
whatever native runs, and here it is the one that accepts more.

`text/utf8 [65 66]` agrees, answering `AB` on both, because a list of small
ints is bytes to each of them. What the two disagree about is everything a
list can be that bytes cannot.

Nothing is fixed here, and nothing can be until the fork is ruled: either a
bare list of small ints IS bytes, and native widens, or the interpreter gains a
real bytes value. The table is in pending-gavels with both costs, and it is on
the task list as Clay's.

## 2026-08-23 — the accumulator rewrite gets a differential of its own

Widening TRMC's license this morning meant trusting a reassociation nothing was
checking. The pass rewrites `n * fact (n - 1)` into a tail-calling helper
threading an accumulator, and a reassociation bug does not fail — it answers a
number. No counter can see it: the rewrite changes the shape a recursion runs
in and not one allocation.

So the shapes are written twice. `f` is the plain form the rewrite reaches;
`g` is the same arithmetic with the leftover operand passed through a function,
which the license refuses to read through, so that group descends the way it
always did. Twenty shapes — two operators over five operands over two base
values — at four depths each, and the two forms must answer identically on both
engines. It runs in 0.6 seconds, which is why it sits with the other
differentials rather than in a nightly.

The last line reads the instrument rather than the compiler: a sum twenty
thousand deep, taken from the INTERPRETER's output, because that engine refuses
unlicensed recursion at ten thousand frames and native would answer it either
way on the operating system's stack. Delete the pass and that line dies.

Watched red twice before it was trusted. With the identity for `*` changed from
1 to 2, every product shape answers double and the gate names the lines. With
the pass returning before it looks at anything, the deep sum stops answering on
the interpreter and the gate says the comparison proves nothing.

The depths are per operator, and the reason is native's own limit rather than
the rewrite's: an int is arbitrary precision in the spec and an int64 in a
native build, so a product thirty-seven deep overflows there while the
interpreter answers. Every value the gate asks for stays inside int64 on both
engines, which is what lets one output be compared against the other. The first
run of the gate found that boundary by falling over it.

`accumulator_rewrite_deleted` is the row that proves it, and it uses awk rather
than the python heredoc its neighbours use — a line inserted after a matched
line is one awk expression, and the six python mutations stay python for the
bootstrapping reason recorded in STATUS.md rather than for the editing.

## 2026-08-23 — the differential guards the license, not just the rewrite

The gate from the entry above compares shapes the license accepts. Three more
were added that it must keep refusing: a float operand over an integer base,
`0.1`, `n * 0.1` and `n / 2`. Today neither copy is rewritten and they agree
trivially, which is the point — the guard is against a future widening.

Watched biting. With `int_arithmetic` widened by one line to accept a float
literal, the gate goes red and names the line:

    1.5000000000000002 1.5

Five terms of `n * 0.1` summed one way and the other. That is the license's
entire argument in one line of output, and it is now checked rather than
asserted.

The first attempt at these guards proved nothing, and the reason is worth
keeping. Their base arms answered `0.0`, and `classify` requires an integer
literal base before it looks at anything else — so the groups were refused for
their base rather than their operand, and widening the operand rule left them
refused. A guard that cannot fail is the failure this repo already has a rule
about; the bases are integers now, and the widening was run again to watch the
gate go red for the reason intended.

## 2026-08-23 — the os package, built

Gavel of 2026-08-17: the stdlib apes Go, `os` takes the filesystem, the
environment, the arguments and the processes, `io` keeps the abstract read and
write surface, `MkdirAll` goes to `os`, and any boundary case Go does not
answer goes to the language committee and never back to Clay. It has sat
unbuilt on the also-open list since.

Moved: `exit_status`, `process`, `args`, `env`, `exit`, `exists`, `is_dir`,
`list_dir`, `make_dir`, `read_file`, `run`, `start`, `kill`, `write_file`.
Stayed: `stdin`, `write`, `write_err`.

That last line is the boundary case, and this is the committee answering it.
Go's standard streams are files in `os` and the writing is done from `fmt`;
kanso has neither files nor a `fmt`, so what would move is three verbs and what
would be left is a module named for a surface with nothing behind it. They stay
in `io`, which is also what the gavel's own words ask for — "io keeps the
abstract read/write surface" describes a module that still has one.

The sweep: 69 `.kso` files rewritten, 332 call sites moved, 262 left; seven
Rust tests carrying kanso programs in string literals; the `include_str!` table
in `src/lib.rs`; and two places that knew the type's name — `deliberate_exit`
in `src/main.rs` and `k_exit_status` in `src/runtime.c` both matched
`io/exit_status` and now match `os/exit_status`, which is what keeps `os/exit 2`
an exit rather than an unhandled err.

Four things the sweep got wrong and the gates caught, each worth naming:

  - A blanket re-sort of import blocks sorted `tests/golden/errors/import_order`,
    the fixture whose whole job is to be out of order. The error corpus said so
    on the next run.
  - The imports in `.rs` string literals needed the same treatment as the files,
    and a raw string is easier to rewrite than an escaped one.
  - `bench/make_jsonbench` writes a program, and the program it writes carried
    `import "std/io"` with nothing left in it that says `io/`. The decoder's
    checksum gate reads that program.
  - `scripts/effects_differential` emits one fixed import header for every
    probe, and its own comment says a probe carries exactly the imports it uses,
    because an unused import is an error and an error compares equal on both
    engines — which reads as agreement. Eight probes went that way. The header
    is read off the probe's body now.

kq speaks the moved names and is the gating downstream job. Its branch is named
`claude/go-to-town-m0dicm`, the same as this one, which is how
`.github/clone-sibling.sh` checks the two together, and against this compiler
its unit tests, twelve jq goldens, three cost goldens, scale gate and
published-numbers stamp are all green. vse and kanso-json use none of the moved
names and need no branch.

Every counter gate is flat, both compile goldens byte-identical, welfare at
84.85, the book's panels regenerated, and the browser differential reads 317
programs with 0 disagreements.

## 2026-08-23 — the names that moved say where they went

A program written before the split says `io/read_file` and is told `unknown
name`, which is true and useless: the name is right and the module moved. The
refusal now names the destination and what stayed:

    error[name]: unknown name `io/read_file` — it moved to `os/read_file`,
    and `std/io` keeps the reading and writing

Fourteen names, matched only under the `io/` prefix, so every other unknown
name reads exactly as it did. Pinned at
`tests/golden/errors/a_name_that_moved_to_os`, and the message before the
change is on the record two entries up — the plain `unknown name` was what
this repo's own migration met first.

## 2026-08-23 — the perf check this branch owed, against main's own compiler

Every change carries a perf check, and this branch moved the compiler three
times — the accumulator rewrite, two refusal messages, and the library split.
The counter gates say the decoder's emitted IR is byte-identical and both
compile goldens held, but neither watches a clock, and the machine-code gate
did move: the decoder's `.text` grew 48 bytes because `to_float`'s refusal
renders the value it refused.

So main's compiler was built beside this one and the two were interleaved on
one box. That is a relative measurement and nothing else: this container has
been compiling all day, and the published figures are a sitting on a quiet
machine, so nothing here can re-sit them. What it can say is whether this
branch moved anything.

    front end, `check lib/json`, same input both sides, 12 rounds
      main   21.6 ms best, 21.6–26.2 spread
      here   21.2 ms best, 21.2–26.7 spread

    decode, jsonbench, 6 rounds
      main   332 ms best, 332–379 spread
      here   336 ms best, 336–403 spread

Both differences sit inside the per-run spread, and the decode difference is
1.2% against the 3% this log already records for randomised layout on one
tree. No published number moves. The 48 bytes are real and pinned in
`bench/text_golden.txt`; they do not show up in the time.

The front-end measurement is the interesting one, because the library split
adds a module for the resolver to find whenever a program imports `std/os`.
It costs nothing measurable on a program that imports it, which is what the
byte-identical compile goldens already implied and this confirms with a clock.

## 2026-08-23 — the site served whatever copy of the engine was committed

`docs/kanso.wasm` is what the playground runs, and the pages workflow shipped
the committed file. It builds the compiler, but only to run the fingerprinter;
nothing rebuilt the engine. So a merge that changed the compiler without a
hand-rebuild published a playground older than the page describing it, and no
gate could see it: the specs rebuild the artifact *before* they test it, which
proves this commit's source has a third engine and says nothing about the file
the site serves.

It is easy to be wrong about, which is the argument. This branch tripped the
spec's own staleness guard three times in one day — twice after a
formatting-only edit, because panic messages carry line numbers and moving them
moves the bytes.

The pages build rebuilds the engine before jekyll copies `docs` now, so the
site serves the compiler it was built from. The committed copy stays for a
checkout to run and for the browser differential to load, and the specs go on
rebuilding it, which is the check that the source has a third engine at all.

## 2026-08-23 — `kanso build myapp` from the directory above it

Exercising the verbs after the library split — the point being that a stdlib
change can break a whole verb where no corpus looks — turned up one that has
nothing to do with the split. A build is named for its program (#984), so
`kanso build greeter` run beside `greeter/` wants to write a file where the
directory is:

    /usr/bin/ld: cannot open output file greeter: Is a directory
    clang: error: linker command failed with exit code 1
    error: clang failed on greeter.ll

Three lines that name neither the cause nor the way out, and the way out is one
line. The build refuses before it writes anything now, and says both:

    error: this build is named `greeter`, and a directory of that name is
    here — build it from inside (`cd greeter && kanso build .`), or build it
    from somewhere the name is free

The spec checks the refusal, that no `.ll` is left behind, that the linker's
words never reach the user — and that the route it recommends actually builds
and runs, which is what makes printing it worth anything. Watched red against
the old path first, where it read the linker's complaint instead.

Nothing in this repo trips it: the benchmarks build from the root, where
`jsonbench` names a file rather than a directory. It is the single-module
project — `myapp/` built from beside it — that always hit it.

## 2026-08-23 — three veins moved, and one of this morning's numbers was backwards

CI caught what a stale build hid here. The cost-goldens job went red three
pushes running on `emitted`, `machine code` and `work`, while all three passed
locally — because the benchmark binaries on this box predated the fix to
`bench/make_jsonbench`, whose generated entry now imports `std/os`. Rebuilt from
nothing, every one of them moves, and they move the same way:

    emitted lines   11,603 -> 11,588
    escapebench     49,650 -> 49,458 bytes of .text
    pendbench       73,458 -> 73,362
    jsonbench    2,860,478,794 -> 2,860,478,381 retired instructions
    deepbench      806,938,332 ->   806,934,626
    (encodebench, oneshot, basket, widebench, escapebench, pendbench alike)

Every number falls, and one cause explains all of them: a program that imports
`std/io` used to drag the filesystem, the environment, the arguments and the
processes in with it, and now pulls a module with three names in it. Less code
is emitted, less machine code is linked, and a few hundred fewer instructions
run before main. A fall is a win to bank, so all three goldens are regenerated
here. Welfare does not move — a few hundred instructions in three billion is far
below anything a saturating term can see — so there is nothing to `--set`.

CORRECTION to this morning's entry on PR #985, which recorded its emitted-line
move as 11,593 → 11,603, a rise. It is the other way round: the golden is
11,603 and that branch produces 11,593, which is a FALL of ten lines and a win
rather than a regression. What does not change is the finding that mattered:
with its goldens regenerated honestly, #985 reads welfare 84.69 against a floor
of 84.85, and the term that pays is compile cost — front-end rounds on lib/json
28 → 30, visits 23,224 → 23,345, peak 871,649 → 878,422 bytes. That branch owes
the reason it is worth it or the compile cost back, and the emitted-line
direction was never the argument.

The lesson for this box: a counter gate reads what is on disk, and what was on
disk was built before the change. `sh scripts/gates/build_benchmarks.sh` after
`rm -f` on the binaries is what makes a local green mean anything.

## 2026-08-23 (later) — the instructions vein belongs to the runner, and this box is not it

The entry above regenerated three veins together. Two of them were right and
one was measured in the wrong place, and CI said so on the next push: `emitted`
and `machine code` went green against the new numbers, `work` stayed red.

The two that held are real. Building main in a worktree on this same box
reproduces its emitted golden to the line — 11,603 — and this branch gives
11,588, so the fall is the branch's and not the box's. Its cause is visible in
the IR: `import "std/io"` carried 957 module-level globals into the decoder and
`import "std/os"` carries 945, with three blank lines between them. Defines,
calls and branches are identical either way.

Retired instructions do not follow, because a global nobody reads retires
nothing. What this box measured as a fall of about four hundred instructions
across all eight benchmarks is the box: glibc here is 2.39-0ubuntu8.7 and the
runner's is 2.39-0ubuntu8.8, one Ubuntu revision apart on the same upstream
release. That is worth about 400 instructions before main, and several thousand
where memcpy carries the work — widebench 993 and deepbench 3,680.

Measured where the vein lives, the branch's actual move is small:

    jsonbench     2,860,478,794   unmoved
    encodebench   9,727,148,124   unmoved
    oneshot          46,596,968   unmoved
    basket           57,400,154   unmoved
    widebench        84,816,701 ->    84,816,675   -26
    deepbench       806,938,332 ->   806,938,306   -26
    escapebench     258,568,120 ->   258,568,077   -43
    pendbench       988,706,663 ->   988,706,559  -104

So the two figures the entry above published for this vein are withdrawn.
jsonbench did not go to 2,860,478,381 and deepbench did not go to 806,934,626;
neither number moved at all. Welfare is unchanged either way, which is the one
claim that survives intact — a hundred instructions in a billion is far below
what a saturating term can resolve, and there is nothing to `--set`.

The file already said not to do this. Its header has warned since it was
written that a row must never be read against a number measured somewhere else,
and the warning did not stop me, because what I had in front of me was a diff
and a diff invites a paste. So the rule is checkable now: the golden names the
host that measured it in a `measured-on` line, and
`scripts/gates/instructions_host.sh` compares that line against the host it is
running on. `scripts/gates/instructions.sh` runs it first, so off the runner
the refusal costs milliseconds and prints no numbers at all, which is the
point — there is nothing to copy. On the runner nothing changes.

It is its own script rather than a block inside the big one so the ratchet can
prove it honestly. A row whose gate is `instructions.sh` would go red in a
scratch worktree whether or not the mutation landed, because there are no
benchmark binaries there to measure, and a row that is red either way is
evidence of nothing. The small gate needs the golden and `ldd`, so it is green
unmutated and red under `instructions_host_unpinned`, which moves the claim
rather than the box. Both directions were watched.

The machine-code vein has the same shape and got the same treatment, because
the trap is the class rather than the instance. Its rows are `.text` sizes,
which are what the toolchain made of the source, so they belong to the clang
that emitted them as surely as retired instructions belong to the glibc that
ran them. `bench/text_golden.txt` names `clang=18.1.3` and
`scripts/gates/machine_code.sh` checks it before measuring. Nothing has gone
wrong there — this box and the runner share a clang, which is why CI accepted
the `.text` numbers regenerated here — and the point is that nothing had to.

One script over both goldens, `scripts/gates/measured_on.sh`, reading whichever
facts the `measured-on` line names. The granularities differ and the difference
is deliberate: glibc carries its Ubuntu revision because two revisions of one
upstream release demonstrably moved the rows, and clang carries only the
upstream version, because what selects codegen is the release and nothing here
shows a package revision moving a byte. A fact pinned tighter than the evidence
reds the gate on changes that are not changes.

Valgrind's version is in neither, though it belongs in the instructions one on
the merits. Pinning it would make that check unprovable on a host with no
valgrind to ask, and the nightly ratchet runner is one. A valgrind bump moves
the whole vein at once the way any toolchain bump does, which the header
already covers.

A toolchain bump will trip this, and should: every row moves with the image, no
row has regressed, and the refusal says so and names both hosts.

## 2026-08-23 (later still) — the compile-memory band has been hiding main's own drift

Looking for other veins with the provenance problem turned up a different one
in `bench/compile_memory_golden.txt`. Its peak-bytes row is not diffed exactly.
CI asserts only that reality is within two per cent of it, and the header gives
the reason: peak bytes is measured by the compiler's own allocator and is a
property of the host, with linux and macos disagreeing by 56 bytes on the same
input.

Two per cent of 871,649 is 17,432 bytes. The divergence it was written to
absorb is 56. Everything between the two is drift nobody sees, and there is
drift in there now:

    golden                                     871,649
    main, measured here, three runs identical  872,025
    this branch, measured here                 872,035
    this branch, measured on the runner        872,061   (two runs identical)

The number is deterministic per host — three runs on this box give the same
digits, and two CI runs give the same digits. This branch is worth ten of those
bytes. The other 376 are main's, accumulated since whenever the golden was last
written, and every gate has been green the whole way.

That matters more than the bytes, because welfare reads `compile_peak_bytes`
out of this golden as the CURRENT value of a term rather than measuring it. So
the compile-memory term has been scored against a figure the compiler left
behind, and the floor was ratcheted to 84.85 while it was.

Correcting it costs the floor. With 872,061 in the file the score still prints
84.85 — the term moves from 0.167 to 0.168 points, below the second decimal —
and `scripts/welfare` exits 1, because the true value now sits under a floor
that was set against the stale reading.

**This is Clay's, and it is on the list.** The options, as I read them:

1. Regenerate the golden and `--set` the floor with the reason, which banks a
   fall rather than a rise. That is the one thing `--set` has never been used
   for, and the doctrine is explicit that moving the floor while leaving the
   weights alone declares the objective wrong without saying so. Against that:
   nothing here is a code change, and the floor was set against a misreading.
2. Pay the 376 bytes back out of the front end and regenerate at whatever it
   then reads, which keeps the floor honest and costs real work on a term the
   weights say matters least.
3. Tighten the band to something near the divergence it documents — say 128
   bytes rather than 17,432 — so the next drift is caught the week it happens.
   That is orthogonal to 1 and 2 and looks right regardless of them, and the
   `measured-on` line makes it affordable, since the gate can now refuse off
   the reference host instead of widening to tolerate it.

Nothing is changed here. This branch leaves the golden alone: its own ten bytes
are inside any reading of it, and the 376 predate it.
## 2026-08-23 — the python that crept back is out, and a gate watches now

The 2026-08-09 entry declared the repo python-free. Within three days it was
not: #854's ratchet mutations carried six `python3` heredocs and #862's write-
path gate added `scripts/stale_a_panel.py`, and a `bench/kq_race.sh` racing an
apps/kq this repo no longer holds had survived the original sweep entirely.
Nothing watched the claim, so nothing went red.

Three moves, each verified differentially against the python it replaces.

The panel staler is kanso (`scripts/stale_a_panel/`), byte-identical on ch04
against the python, loud on a missing title or marker, and watched both ways
through book_check: misname its panel and the check dies on `missing panel
title`; restore it and the write path rewrites both staled panels back.

The six mutation heredocs are POSIX awk in the same .sh files. Anchors travel
through ENVIRON so no byte is reinterpreted, replacement is first-occurrence
like the python's `replace(..., 1)`, and a missing anchor dies with the same
"moved; this mutation needs rewriting" message. awk rather than kanso because
a mutation runs in a fresh worktree before any build — a helper needing
target/release/kanso would make the harness depend on the binary it is about
to mutate. All six produce byte-identical mutated sources and identical exits
against the heredocs they replace.

kq_race.sh is deleted, not ported: the archive records apps/kq removed with
kanso-lang/kq as its sole home, and the script builds a path that is not
there.

The gate is `scripts/gates/python_free.sh`, a python-free CI job, and a
ratchet row: no tracked .py file, no python3 call outside design/'s history
and the one mutation whose job is to introduce one. Watched red three ways —
the stale racer before its deletion, a python3 line appended to book_check
(the row's mutation), a tracked creep.py — and green on the clean tree.
## 2026-08-23 — one ledger: pending decisions live in pending-gavels, and nowhere else

Clay ruled it in the developer chat, verbatim intent: "110% UNIFY those
into a clear single source of truth. compiler-log was supposed to just
be the history of actual decisions, whereas we need something like
'pending-gavels' for keeping track of anything that requires my personal
decision because it's about the 'UX' of the language, not the
implementation details."

What had happened: four surfaces claimed the same authority and
disagreed. STATUS.md carried a full-text "Waiting on Clay" section that
forked per branch — main's copy showed one item waiting, the working
branch's showed four. design/pending-gavels.md said "every decision
waiting on Clay, in one place" while filing list-as-bytes under "not
blocking" as STATUS.md called it blocking. GAVELS.md sat orphaned at the
root — nothing referenced it — holding nine July letters that never
closed, plus a ruled ledger duplicating the log. And sessions were
citing decisions to Clay by their own private task-list numbers ("#2
list-as-bytes"), which resolve nowhere outside the session that minted
them.

The unification, per the ruling:

- **design/pending-gavels.md is the ledger.** Charter at the top: UX
  forks only, entries leave on ruling (the file's own 2026-08-15
  precedent, drifted from since), STATUS.md indexes but never carries
  text, entries are cited by heading, edits ride promptly-merged PRs.
- **This log is history and nothing else.** Rulings land here; nothing
  pending lives here.
- **The ruled entries left the ledger** — gavels 1 (surface), 3+5, 6,
  8, 15, 16, 17, 18+19, 20b, 21, 22, 23, 24, fixed-length lists,
  the name of `[]T` — their rulings are above under their dates and
  their full text is in the file's git history.
- **GAVELS.md is deleted.** Its ruled ledger (A1–A5, R5, T, W, B, BB,
  X, nullary-BB) duplicates rulings recorded here and in the archive;
  its unclosed letters are triaged into the ledger: Y closed 2026-08-19
  (`>>` keeps its glyph), F parked 2026-08-19, H shipped as entropy-by-
  default with KANSO_SEED pinning, and C, D, G, Z, AA sit under a
  "stale — revalidate against the post-24 language" heading rather than
  pending, with Z marked presumed-declined by the 2026-08-15 err gavel.
  Full text in git history.
- **The four live decisions** — list-as-bytes (upgraded to blocking,
  which the measurements say it is), the undemanded knot, the
  compile-memory band, and `>>` under run-time effect failure — are the
  ledger's blocking section, carried over verbatim from the branch
  STATUS.md that held the freshest text.

CLAUDE.md's design-flow line now names the ledger; the vague "AND a
memory file" is gone.

## 2026-08-23 — gavel: an undemanded knot allocates nothing, on any engine

Clay ruled the first entry in the unified ledger, in the developer chat.
The principle: work defers until it is actually presented to IO — until
it can affect the real world — and eager evaluation exists only as a
resource-optimization heuristic inside that contract, never as a
semantic difference an engine may expose.

The shape that forced the question compiles clean and cannot be caught
at compile time (an unreferenced knot is already `error[unused]`; this
one is referenced in a dispatch arm the run never takes):

    x = [x]

    pub play = picked 1

    fn picked 1
      io/write "one\n"

    fn picked _
      io/write "{length (list/to_list x)}\n"

Native reads `thunk_allocs=1, thunk_live_exit=1` — `k_caf_init` builds
every knotted constant before main. The oracle reads zero. Reproduced
today on both engines before ruling.

The ruling: the oracle is right. A knotted constant defers like every
other binding; an undemanded knot allocates nothing, and `thunk_allocs`
stays in the engine-shared differential set counting demanded work
only. The disagreement closes by changing native — the startup freeze
goes — not by re-scoping the counter or splitting it.

On the hot-loop cost that motivated the freeze, Clay rejected the
premise that deferral means a perpetual conditional: "of course you
need that check. but it shouldn't really be a 'check' like a
conditional. instead, you just make a code change once it's evaluated.
… imagine you have a stored lambda. when you run it, it says 'compute
this expensive thing, then replace the existing lambda with a new one
that just returns this'. then the next call doesn't need to 'check'
anything. it just runs." That is update-in-place — the machinery the
runtime already uses for ordinary thunks — an indirection rewritten at
first evaluation, not a branch paid on every read. Implementation is
the implementer's; if measurement finds a real hot-loop regression
even in the update form, the number comes back to the ledger before
any freeze returns.

Unblocked: the fixture pinning an undemanded knot at zero on both
engines, and the .mem/golden regeneration that lands with the native
change. The entry leaves the ledger with this commit.

## 2026-08-23 — gavel: a list is never bytes, and acceptance is declared

Clay ruled the second ledger entry in the same sitting: "in general I
like consistency," with the committee heard on the counterarguments
before the gavel. The steelman for widening had three legs — bytes are
just small ints (the data-is-data lens), the language has no bytes
literal so `[104 105]` is the only spelling a user can write down, and
`text/utf8 [65 66]` answers "AB" on both engines today, so full
strictness would break the one case the engines agree on. The first leg
dies on the evidence: `["a" 120]` — the oracle's answer for
`text/append ["a"] "x"` — is not data-is-data, it is `bytes_to_str`
accepting whatever list arrives. The other two legs are real and became
the ruling's riders.

The ruling: the interpreter gains a real bytes value, and a list is
never ambiently bytes on any engine. The four cases where the oracle
answered become refusals matching native's, and the fixture family
(`append`, `find2`, `find2_below`, `utf8`, `to_float`) can finally pin.

Two riders, named with the gavel:

- **Acceptance is declared, not coerced.** Where a function genuinely
  wants a list of small ints as byte input — utf8 is the live case —
  that is a visible, per-function acceptance in the library, identical
  on both engines. Whether utf8 keeps its list acceptance is a library
  decision made in the migration, not an engine property. This is the
  typeset-acceptance idiom already pending under the AA entry, applied
  early.
- **The constructor ships in the same change.** A list→bytes function
  with a loud refusal on anything outside 0–255, so byte data stays
  writable now that the coercion is gone. `text/bytes` covers strings;
  this covers numbers.

The entry leaves the ledger with this commit. Unblocked: the six-case
fixture table, and the interpreter's bytes representation work, which
is the implementer's.

## 2026-08-23 — the startup freeze goes, and an undemanded knot builds nothing

Implementing the gavel above. `k_caf_init` used to seed every knotted
constant's cell and run every builder before main, so a knot the program never
demands was built anyway. Each constant now seeds and builds its own cell on
the first read, and `k_caf_init` is left holding only the math-id handshake.

The ready flag is set BEFORE the builder runs, which is the same discipline
`k_caf_init` had when it seeded every cell before running any builder: a
constant that mentions itself re-enters the reader and has to find the
blackhole rather than the zeroed global, which is an integer zero and reads as
one. That seeding is what keeps the cycle finite, and the demanded knot still
answers `1` on both engines.

Measured on the ruling's own program, with the counters the differential
shares:

    undemanded      oracle 0 allocs   native 1 -> 0
    demanded        oracle 0 allocs   native 1 -> 1

The ruled disagreement closes. It cost nothing and paid something: the one
fixture in the mem corpus whose numbers move,
`an_unasked_equality_stays_a_cell`, falls from six allocations to two and from
four evacuations to none, because a constant nobody asks for is no longer
built and so never has to be evacuated as a survivor. The freeze had been
buying eagerness nobody wanted and paying for it in the beat.

One branch, taken once per constant. Clay's preferred shape is update in place
— rewrite the indirection at first evaluation so later reads check nothing —
and it stays the better form if this ever costs anything measurable. Nothing
in this corpus says it does.

Across the benchmark veins the shape repeats. Three of the eight `.text` rows
rise by the branch — encodebench and widebench 16 bytes, escapebench 32, one
per knotted constant — and five do not move at all, jsonbench among them,
because the decoder links no knot and so the hottest path in the project is
untouched. Three counter goldens fall: widebench loses five permanent
allocations and six allocations outright, scanbench two permanent and two
evacuations, encodebench two and two. Constants that used to be built before
main are not built at all when nobody asks. Welfare holds at 84.85.

Sixty-four bytes of `.text` is what update in place would take back, and this
is the number it has to beat.

A CORRECTION about how this was measured, because it nearly became a false
report. `kanso run` compiles and runs; the oracle is `run --interp`. Measuring
the ruling's program with `run` twice and calling one of them the oracle
produced two identical rows and the conclusion that the gavel's premise did not
reproduce. The premise reproduces exactly. The lesson is the same one three
goldens now carry in a `measured-on` line, arriving this time through a verb
rather than a host: a number means nothing without the thing that produced it.

The price came back from the runner. Four rows of
`bench/instructions_golden.txt` rise, each by about what taking the ready-flag
branch costs: encodebench +17,931, deepbench +155,986, escapebench +5,993,
widebench +358. deepbench's is the largest and the smallest — 0.019% of 807
million — because it reads its knotted constants inside the hottest loop it
has, so the ready-flag test is paid once per read instead of once per program.
jsonbench, oneshot, basket and pendbench do not move at all. These numbers are
the runner's: the vein carries `measured-on glibc=2.39-0ubuntu8.8` and this
container is one revision off, so `scripts/gates/measured_on.sh` refuses to
hand over a diff here and CI's job log is the only place they can come from.

`encode_sh_buf` rises 96 bytes. A per-capacity histogram of freshly-allocated
buffers, split by beat depth, says what it is: exactly one more five-element
buffer, allocated inside the beat. Every other capacity class is
byte-identical, `buf_reuse` does not move, and no five-element buffer is allocated
before main under either codegen. Beside it on the same benchmark, two
permanent allocations and two evacuation copies go away — `perm_allocs` 12 to
10, `evac_allocs` 19 to 17, `evac_bytes` 624 to 576. Two constants that used
to be frozen into malloc'd storage before main, and copied there out of the
arena, are not frozen at all now; what is left is 96 bytes of arena the beat
reclaims on its next rewind, in place of two allocations that lived until exit
and forty-eight bytes of copying.

Left open, and going to the ledger as its own entry: the DEMANDED knot still
disagrees. Native reports `thunk_allocs=1` where the oracle reports `0`,
because the oracle's `knotted` builds its cell without touching the counter.
That predates this change and survives it, and which engine is right is a
question about what the counter counts rather than a defect to pick a side on.

## 2026-08-24 — bytes are a value on every engine

Implementing the bytes gavel. The interpreter had no bytes: `text/bytes`
answered a list of integers and every consumer ran whatever list arrived
through `bytes_to_str`, which is how `text/append ["a"] "x"` came to answer
`["a" 120]` where native refused. `Value::Bytes(Rc<Vec<u8>>)` exists now, and
the six rows the ledger measured agree word for word on both engines:

    text/append ["a"] "x"        append takes bytes and a string, bytes, or byte
    text/append [65 66] "x"      append takes bytes and a string, bytes, or byte
    text/find2 [65 66] 1 65 66   find2 takes bytes
    text/find2_below [65 66] …   find2_below takes bytes
    text/utf8 ["a"]              err "utf8 takes byte values (0-255)"
    text/to_float ["a"]          to_float takes a string, bytes, or number, not ["a"]

Four of those were the oracle ANSWERING where native refused, which is a
program that runs under the interpreter and dies compiled. The browser engine
comes along for free: `rt_builtin` calls the interpreter's `call_builtin`, so
one implementation serves two of the three engines.

The two riders shipped with it. utf8 keeps its list acceptance, spelled the
same on both engines — there is no bytes literal, `[104 105]` is the only
spelling a program can write down, and `text/utf8 [65 66]` was the one case
the engines already agreed on. And `text/to_bytes` is the constructor, loud
outside 0-255 rather than keeping the low byte: `text/bytes` covers strings,
this covers numbers. The one place the low byte is still taken is
`text/append`'s single-number form, because that is what the compiled engine
has always done (`x.payload & 0xff`) and matching it is the differential law.
Whether either engine should refuse there instead is a separate question.

`==` still crosses. Native has compared a byte string against a list of its
numbers since it was written (`k_bytes_eq_list`), so the interpreter does too;
making both refuse is a change to native's semantics that the gavel did not
order.

What it cost: `front_end_visits` on lib/json 23,224 -> 23,250, and the
decoder's emitted lines 11,588 -> 11,595. Both are the price of a public
function in std/text, which lib/json imports. Measured separately: a plain
`pub fn zzz x / x` in std/text costs 13 visits by itself, so roughly half of
the 26 is the function existing and half is the builtin call in its body. No
allocation counter moves, no `.text` row moves, and welfare's floor moved to
whatever it cost — which welfare's own header says is what happens to a change
that makes the engines agree.

The goldens: six refusal fixtures under tests/golden/runtime, each watched
red against the old oracle first — four of them ANSWERED there, which is the
bug — and two micro fixtures for the surface that works and for the
constructor's refusal. The error corpus moved three line numbers, and the book
three more, because std/text grew five lines above `to_int` and an err trace
names the line it was born on.

## 2026-08-24 — a type field wakes its readers, not the whole program

The bytes gavel cost lib/json 26 front-end visits, and looking at where they
were spent found a much larger number sitting beside them. `KANSO_PHASES=1`
on lib/json:

    round 1: 365 moved of 407 visited
    round 2:  45 moved of 407 visited
    round 3:  52 moved of 407 visited
    round 4:   7 moved of 407 visited

Four full sweeps of every function, the last one to let seven of them move.
The fixpoint has had dirty-tracking since it was written — a function's
returns wake its readers and nobody else — and one line was defeating it. When
a declared type's field set grew, inference set `all_dirty`, and the next round
walked the program. It had to: nothing recorded which functions could care.

They are static. `type_fields` is read in exactly one place, `bind_pattern`'s
`Pattern::Ctor` arm, so the functions that can be affected are the ones whose
patterns destructure that type — in the head or anywhere in the body. The index
is built once, before the first round, and a field growing now wakes those and
nothing else.

The rounds it saved were paid back by rounds it cost: information travels one
hop per round, and a round that walks forty functions carries it less far than
one that walks four hundred. So a change moves its readers in the CURRENT round
as well as the next. The sweep alternates direction, so about half of them are
still ahead of the cursor and take the new answer immediately; the rest are
behind it and are simply not walked again.

    lib/json          rounds 28 -> 40, visits 23,224 -> 17,786
    the module sample rounds  6 ->  6, visits  3,031 ->  2,403
    the five samples  visits    133 ->    115

`front_end_rounds` 28 -> 40 is the cost and it is real; 5,438 fewer expression
visits is what it buys, and the visit is what carries the work — a round is a
loop over a work list that is usually short now. Welfare weighs both and comes
out ahead: 84.85 to 84.87, banked.

The clock does not show it, and the entry would be dishonest without saying
so. Interleaved on this container, three runs each, `infer` reads 2.80-3.04 ms
on the branch against 2.82-4.06 ms on main — inference is about a fifth of a
15 ms front end and the spread here is wider than the effect. The visit count
is the instrument that can see it, which is the whole reason the compile
goldens count work rather than time.

The index costs memory to hold: `compile_peak_bytes` on lib/json reads 876,930
here against main's 872,035 on the same box, three runs identical each way.
That is inside the two per cent the gate allows and outside what welfare can
see, because welfare reads the golden's number rather than measuring — which is
the ledger entry that has been waiting on Clay since yesterday, and this change
adds 4,895 bytes to what it is hiding.

The answers do not move. Every engine, the error corpus, the diagnostics
differential and the browser differential are unchanged — the only goldens that
move are the ones that count the compiler's own work.

## 2026-08-24 — the highest-ranked idea on the memory board was priced against a number that is gone

`design/memory-frontier-research.md` has ranked copy-or-pin first since
2026-08-07, on a measurement: half of every allocation the one-shot shelf made
was the copy-out before a rewind — 63,967 evacuation allocations of 128,528,
1,991,456 bytes. Rechecked today, one-shot reads `evac_allocs=3`,
`evac_bytes=96`. #868 took it from 63,967 to 5 and #977 to 3. The measured half
the idea was going to delete had been deleted by something else, and the memo
did not know, because a status table records what an idea IS rather than
whether its premise still holds.

Where evacuation lives now, across the eight shelves: wide 264 allocations for
1,032,336 bytes, pending 2,658 for 498,976, scan 36 for 8,800, and everything
else under six hundred bytes. So the idea gets reposed rather than retired —
and the instrument that priced it the first time can price it again. The
evacuation path was instrumented to record each survivor's source address and
copied size.

Wide is four copies. Four nodes of 256,016 bytes — a 16,000-element list
buffer, 16 + 16 x 16000 — carry 99.2% of the megabyte. `bench/wide.json` is a
16,000-element list, so that is its top-level buffer evacuated as the streaming
loop's carried accumulator, once per rewind. Two of the four report the same
source address, which says only that the arena reused it — the addresses are
bump-allocated and a rewind hands the same bytes back. The other 260 survivors
are 8,272 bytes between them, median 32.

Pending is diffuse: 666 of 2,658 survivors are needed to reach 90% of half a
megabyte, nothing above four kilobytes, largest 3,216.

That is the answer the memo asked for and nobody had taken, and it is two
answers rather than one. A quarter-megabyte survivor occupies whole pages by
itself, so not copying it retains almost nothing — and it does not need general
page pinning, only a size threshold and storage that does not rewind. A
three-kilobyte survivor is threaded through the garbage, and pinning its page
keeps the garbage with it. The size distribution is the decision variable,
which is 5.2's survivor-ratio selection asked one level down.

Nothing is built. What changed is that the board now says what the shelves say.

## 2026-08-24 — the beat carry copies a loop-invariant capture, once per rewind

Chasing where widebench's megabyte of evacuation goes, from the reposed
copy-or-pin entry. `k_deep_copy` has exactly three entry points — `k_beat_pop`'s
result copy, `k_caf_freeze`, and `k_beat_iter_carry` — and tagging all three
says all 264 evacuations and all 1,032,336 bytes come from the beat carry. The
other two contribute nothing.

What the carry copies is one slot, unkept, tag 8: a `K_DESC`. widebench's
streaming loop hands on `io/write (...) . (_ -> stream_elems xs (i + 1))`, and
the 256,016 bytes are the `xs` that description captures — the 16,000-element
list, decoded inside `read_file`'s continuation and therefore ABOVE the outer
beat's mark. It is the same value every iteration and never rebuilt, and the
carry deep-copies it forward on every rewind regardless.

Reduced to a pair of fixtures in `tests/golden/mem`, identical but for where
the list is built:

    a_loop_invariant_capture_is_copied_every_rewind   24 allocs   32,672 bytes
    the_same_capture_built_below_the_mark_is_shared    6 allocs      192 bytes

Same loop, same 500-element list, 504 rewinds against 502. Build it as an
argument and it sits below the mark and is shared; build it inside the chain
and it travels forward whole, once per rewind. The difference is invisible in
the source shape, which is why it went unnoticed — and why the corpus needed
both halves rather than a description of one.

Two things this closes. `k_cohort_pop` has had a survivor-ratio guard since
#389 — it sizes the survivor and refuses a copy that exceeds half the reclaim,
or one above four times the block threshold — and `k_beat_iter_carry` has
nothing of the kind: it copies every unkept slot at any size. The board's 5.2,
"per-beat policy selection by survivor ratio", is therefore shipped for cohorts
and absent exactly where the remaining cost is. And the page-pinning framing
the board carried is the wrong shape for this: `k_carry_kept` already exists to
say a slot must not be copied, it means "this cycle's own builder" today, and a
loop-invariant capture is the other thing a carried slot can be.

Nothing is built. The fixtures pin the cost as it stands, and a change that
makes those two numbers converge is what they are for.

## 2026-08-24 — two front-end passes stop allocating per occurrence

A follow-on to the fixpoint entry earlier today, and much smaller: reading the
phase profile that change produced, two passes were doing avoidable work.

`prune_unused_getters` collected mentioned names into a `HashSet<String>`,
which is one String allocation per identifier OCCURRENCE — every mention in the
whole program, not every distinct name — plus a second for each qualified
read's bare half. The set borrows the program's names now. A keep mask carries
the answer across the end of the borrow so `retain` can have the program
mutably. Interleaved three times on kq's query library, the pass reads
0.84-0.89 ms against main's 1.11-1.19: about a quarter off, repeatable.

`canonicalize_bare_aliases` found each synthetic bare alias's qualified twin by
scanning every declaration in the program, with a `format!` inside the inner
loop — quadratic, and one String per pair examined. A synthetic alias and its
twin share a source position and an arity, so that tuple indexes them; the
index is built once and the needle formatted once per alias rather than once
per pair.

That one does NOT show up in the clock at these sizes: 0.57-0.65 ms against
0.52-0.76, which is noise on a loaded container. The change is asymptotic and
should be described as nothing else — lib/json has 407 declarations and kq's
query library is not much larger, so the quadratic has not had room to hurt
yet. It is worth removing before something does.

One idea from the same sweep was measured and declined. `load_dependencies` is
the second-largest phase at 2.13 ms, and its `visited` set is a cycle detector
rather than a cache — it removes each path when the module finishes, so a
diamond import would compile the shared module twice. There is no diamond to
exploit: `KANSO_PHASES=1` prints one `load` line per module on both lib/json
and kq's query library, four and six modules with no repeats, because the
stdlib modules these programs import do not import each other. Memoizing by
canonical path would buy nothing today. Recorded so it stays declined until a
program has the shape.

No GOLDEN counter moves — rounds, visits and `compile_peak_bytes` are
byte-identical and welfare holds at 84.87 — but a counter does, and finding it
corrects the sentence this entry originally carried. `kanso check` has printed
`compile_allocs` and `compile_alloc_bytes` since the counting allocator went in;
nothing pins them, so they were invisible. On lib/json, two runs each,
identical:

    compile_allocs       153,346 -> 148,073   (-5,273, -3.4%)
    compile_alloc_bytes  7,860,884 -> 7,942,065   (+81,181, +1.0%)

Five thousand fewer allocations and eighty-one thousand more bytes: the Strings
that went away were many and small, and the keep mask and the site index that
replaced them are few and larger. The count is what the 24% fell out of, and the
bytes are transient. Stating it as "no counter moves" would have been the
silence the trend gate exists to refuse, on a dimension no gate watches.

That the dimension is unwatched is its own finding. Both counters are
deterministic here across runs, and neither is in
`bench/compile_memory_golden.txt`, which pins rounds, visits and peak. Whether
they can be pinned depends on whether they agree across hosts the way rounds
and visits do and peak does not — that question, and the ratchet row a new gate
owes, are a change of their own rather than a rider on this one.

## 2026-08-24 — the dimension that watched nothing

The entry above corrected itself once: it said no counter moved, and one did.
`kanso check` has printed `compile_allocs` and `compile_alloc_bytes` since the
counting allocator went into main.rs, and no golden has ever held them. So the
front end has had an allocation-traffic dimension that nothing watched, and the
pass rewrite is what walked into it — a quarter off its time, and every gate in
the tree reporting nothing.

`bench/compile_allocs_golden.txt` holds the count now, with a gate beside the
compile-memory one. The proof that it earns its place is the ratchet's, and
it is worth stating because a new gate that duplicates an old one is cost with
no cover. Under `compile_allocs_unwatched` — the pass put back to owning the
program's names, one String per identifier occurrence:

    compile_rounds        40 ->        40
    compile_visits    17,786 ->    17,786
    compile_peak_bytes 876,930 ->   876,930
    compile memory     green
    welfare            green
    compile allocations  RED    148,073 -> 153,859

Rounds and visits cannot see it because the decision work is identical. Peak
cannot see it because the strings are transient — allocated, dropped, and gone
before the high-water mark is anywhere near them. That is three gates blind to
the same shape, and the fourth is the reason to add a fourth.

The allocator prints a second counter and the golden does not hold it. The
first version of this file pinned `compile_alloc_bytes` beside the count, and
the runner disagreed with the container by 26 bytes on a number near eight
million — too small for a toolchain and too exact for noise. Five directory
lengths, one tree, one binary:

    16 chars   7,942,065        29 chars   7,942,091
    17 chars   7,942,067        49 chars   7,942,131
    21 chars   7,942,075

which is 7,942,033 plus twice the length of the directory the compiler ran in,
at every point. Two allocations hold the absolute path, and
`/home/runner/work/kanso/kanso` is thirteen characters longer than
`/home/user/kanso`. A row like that pins the clone rather than the compiler,
and it would have reddened the first time somebody checked the repository out
somewhere else — under a name that said the front end's allocation traffic had
moved. It is dropped, with the measurement in the file so nobody puts it back.
What it was wanted for is held on both sides already: `compile_allocs` for the
traffic, `compile_peak_bytes` for the residency.

The count survives the same test. It reads 148,073 at all five lengths, and
148,073 under rustc 1.94.1 in the container and under 1.98.0 on the runner.

`compile_peak_bytes` carries the same term — 876,898 plus twice the length,
exact at all five — and that is a trap for the table above, which claims peak
does not move under the mutation. The first reading of it said peak moved 132
bytes, taken from a mutated build in a scratch worktree 66 characters deeper
than the tree it was compared against. Rerun at one path, mutated and clean,
peak is 876,930 both times: byte-identical, and the claim is measured rather
than argued from what transient means. Where that leaves
`bench/compile_memory_golden.txt` is a decision of Clay's and is in the ledger.

The rest of the veins were swept for the same shape and are clean. The four
runtime cost goldens produce byte-identical counter sets across a 38-character
change of working directory, and so do all 51 fixtures in `tests/golden/mem`.
Every counter that carries the path is a byte total on the compile side, which
is the only place the compiler's own allocations are what is being counted:
`compile_alloc_bytes` and `compile_peak_bytes`, and no others.
`compile_allocs`, `compile_rounds`, `compile_visits` and `compile_passes` sit
beside them and do not move.

The row is kept under a toolchain guard anyway, because a good part of the
count is the standard library's: a HashMap's growth schedule, a Vec's doubling,
a String's spare. So the file carries a `measured-on rustc=` line and
`scripts/gates/measured_on.sh` learned to read it, which makes three veins that
name their host and one script that reads all three. Upstream version only, no
point release — the same granularity clang gets, for the same reason: nothing
here shows a point release moving a count, and a fact pinned tighter than the
evidence reds the gate on changes that are not changes.

Two toolchains agreeing is evidence rather than a guarantee, and a regeneration
taken in the wrong place is silent, so the guard stays and the ratchet row that
proves it stays with it.

The row watched: green unmutated, red under `compile_allocs_unwatched` and
under `compile_allocs_host_unpinned`, and the ratchet's cover check still says
every CI job carries a mutation or a stated reason.

The guard earned itself on its first outing, which was not the plan but is the
best evidence it could have had. The golden went up carrying this container's
`rustc=1.94.1`; the runner is on 1.98.0, and the gate refused rather than
diffing — no numbers printed, both toolchains named. So the rows here are the
runner's, taken from a job log, and a container cannot produce them. That is
the same arrangement `bench/instructions_golden.txt` has had since a
container's glibc numbers were pasted over the runner's, and the same
bootstrap: name the host the vein will live on, let CI measure, copy the rows
out.

It follows that an image bump can red this gate. If it does, the row moved with
the toolchain and has not regressed; the response is a regeneration, the line
moved, and a sentence here — which is what the two veins beside it already ask
for.

## 2026-08-24 — the megabyte was the benchmark doing its job, and a correction

"The beat carry copies a loop-invariant capture" says, earlier today, that the
carry "copies every unkept slot on every rewind, at any size, with no guard",
and that is wrong. Writing the change that would have followed from it is what
found out.

The guard is in the bind-chain driver, which is what pushes the beat those
copies happen under. `k_exec`'s chain case sizes the staged continuation with
the cohort guard's budgeted pass and takes one of three paths: under 4 KB in a
region that has not drifted, leave it unstaged; over 256 KB, skip the
evacuation and floor the region under it; between, stage and copy. It landed
with the streaming-stdout work on 2026-07-27, where the 256 KB line was chosen
low enough to exempt kq's 13 MB decoded document and took kq's full print from
47.5 MB to 30.0.

widebench's carried continuation captures 256,016 bytes, which is 2,128 short
of that line. The cliff is sharp and reproducible — the same fixture at a range
of sizes, `evac_bytes` against the survivor:

    16,000 elements   256,016 bytes   1,028,512 evacuated
    16,380 elements   262,096 bytes       3,648 evacuated

So the proposed change was one constant: 256 KB down to 64 KB. It measures
extremely well. widebench's `evac_bytes` falls 1,032,336 to 7,488,
`alloc_bytes` by exactly one survivor copy, `beat_iters` 43 to 39; seven of the
eight shelves are byte-identical; `arena_peak_bytes` does not move on any of
them, nor on the reduced fixture at 128 KB or 192 KB survivors, where the
copying disappears for nothing.

**And it should not land.** Regenerating `bench/cost_golden_wide.txt` put its
header on screen, which has said since the shelf was written: "Sixteen thousand
elements, not twenty. The evacuation cost stops at 16,384 … so a fixture above
that line exercises none of it. This one sits below." The size is deliberate.
widebench exists to hold the staging path where a counter can see it, and
lowering the threshold would move the shelf out of the band it was built to
watch — hiding the cost rather than paying it.

What survives is a real question asked properly. The 4 KB-to-256 KB staging
band has never been measured against flooring on a program that is not a
benchmark; on the reduced fixture, flooring is free at 128 KB and 192 KB. That
is an argument for moving the line, and it has to be made on programs rather
than on the shelf that was placed to sit under it — with widebench's own size
moved in the same change, or the shelf loses its point either way.

Two smaller corrections while the file is open. `k_beat_iter_carry` is where
the copies are made, which the entry above measured correctly; the policy is at
its caller, which the entry did not look for. And the framing this came from —
copy-or-pin, page pinning, `k_carry_kept` — was aimed at a mechanism the tree
already has in a better form. The memo carries the same correction.

## 2026-08-24 — where the front end spends itself now

Two compile-side changes landed today — #993's reader index and #996's
borrowed names — and the plan carried forward from before them is stale. It
named lex at 1.53 ms and parse at 1.58 ms as what was left. Measured on
`kanso check lib/json`, five runs:

    check_merged   3.0-4.0 ms   30%
    infer          1.9-2.6 ms   19%
    parse          1.1-1.3 ms   11%
    lex            1.1-1.25 ms  10%

Lex and parse have each fallen by about a third and are no longer where to
look. `check_merged` is, and the number is its own work: `phase::watched`
computes `mine = whole - IN_CHILDREN`, so infer's time is subtracted rather
than counted twice.

It is 22 independent whole-program traversals. Instrumenting each one puts
the sum at 3.04 ms with no single pass to blame:

    literal_arguments 0.73   call_arities  0.40   effect_discarded    0.37
    call_shaped_list  0.21   wall_operands 0.18   none_in_collections 0.12
    build_blocks      0.12   if_arity      0.11   field_exists        0.11
    arm_ties          0.11   and twelve more below 0.10

So a fix here is fusion — one traversal running every per-expression check —
rather than a hot pass to rewrite. That is a real refactor and it is not
started.

`literal_arguments` earned a look on its own at twice the next largest, and
most of it is its own walk. `inline::aliases`, which it calls, is a fixpoint
that rebuilds a `HashMap<(String, usize), String>` every round and clones a
callee name to build each lookup key — the shape #996 fixed twice elsewhere —
and it has two callers. Priced at 0.30 ms across both, near three per cent of
the front end. Worth doing and small, recorded here so the size is known
before anybody starts.

One hypothesis killed on the way. `phase::watched` calls `env::var_os` on
every invocation, so 22 more calls per compile might have moved the counter
the day's other change had just pinned. It does not: 148,073 instrumented and
clean alike, because `var_os` on a missing variable answers None without
allocating. The per-pass instrumentation costs nothing when off, which is the
argument for keeping it rather than re-deriving it next time.

## 2026-08-24 — check_merged runs four times, and the library is checked twice

The entry above prices `check_merged` at about 3 ms and describes it as 22
traversals. It is 22 traversals run four times. One `kanso check lib/json`
reaches it once for each dependency and once for the merged program:

    std/list     158 declarations   1.33 ms
    std/text      26                0.19
    std/render     2                0.02
    merged       407                3.52

Those are wall figures, so they carry the inference each call makes inside
itself where the phase report subtracts it. The two agree: 5.06 ms less
infer's 1.9 is the 3.17 the report gives, to a hundredth of a millisecond.
Worth writing down because the apparent gap looked like a published number
being wrong and was nearly filed as a correction.

The merged pass's 407 declarations contain the dependencies' 186. So every
compile of every program checks the standard library on its own and then
checks it again inside the merge.

Whether the per-dependency pass can go is a question about diagnostics
rather than about time, and it is untested. `check_merged` exists because
some checks need the whole program — gavel 1b's construction check and
`check_call_arities` were moved there precisely because a per-file pass
cannot see an imported group's arms. What the per-dependency pass catches
that the merged pass would not is unproven in either direction, and the
error corpus is the only thing that can answer it.

If it can go, the saving is the 1.54 ms those three calls cost: near
fifteen per cent of a 10.5 ms front end, and five times what
`inline::aliases` is worth. That makes it the larger of the two leads
recorded today, and the one to settle first.

## 2026-08-24 — the per-dependency check is not redundant, and counting declarations never said it was

The entry above says `check_merged` runs four times on one compile, once for
each dependency and once for the merged program, and that the merged pass's
declarations contain the dependencies'. Both are true. Removing the
per-dependency call still loses a diagnostic.

A field read no record type declares, inside a library reached through
another library, goes unreported: `ok` where main answers `no record type has
a field ...`. It is a fixture now — `tests/golden/errors_module/`
`field_read_in_a_deep_library` — so the idea cannot come back quietly.

`desugar_field_reads` is the reason, established by ablation rather than
argued. It runs inside the dependency's own compile, before that compile
returns, and rewrites `n.nosuchfield` into a shape `check_field_exists`
cannot see. Guard all four of its call sites behind an environment variable
and the diagnostic comes back:

    desugar on     ok
    desugar off    error[name]: no record type has a field `nosuchfield`

Under main the dependency's own `check_merged` runs before that rewrite and
catches the fault there. Take the pass away and the fault is unreachable by
the time the entry merges.

So the declarations are present and are no longer checkable. A check that
reads a syntactic shape can only fire in the compile where that shape still
exists, and a count of declarations says nothing about whether it can. That
is the whole error: the case for removing the pass was that the same
declarations get walked twice, which is true and is not the question.

What it would have bought is real — rounds 40 to 28, visits 17,786 to 15,210,
allocs 148,073 to 130,747, welfare 84.87 to 85.75 — and is available only to
a change that moves the rewriting passes after the whole-program check, or
splits the checks into those that read syntax and those that do not. Neither
is started.

Four claims were made wrong and corrected on the way, and they are one habit:
evidence gathered under one configuration read as evidence about all of them.
The suite was green because the error corpus is 161 single-file fixtures and
none of them has a dependency. A comparison of two fixtures called them
identical while running on a branch that did not contain them, both binaries
answering `cannot read` and the digests agreeing. The first divergence was
read as a labelling problem and a fix was built for it, which was real work
at the wrong layer. The entry was said to get no whole-program check at all,
from instrumenting one of `check_merged`'s four call sites instead of the
function. Instrument the function. Run the ablation. Before generalising from
a green fixture, ask what would have to be true for it to pass while the
claim is false, and go build that.

## 2026-08-24 — the children were gathered into a vector nobody kept

`expr_children` handed every caller a fresh `Vec<&Expr>`. Thirty-seven call
sites ask it, and twenty-four of them are the same shape — `for child in
expr_children(e) { walk(child) }` — read once and dropped at the end of the
loop. Counting inside the function on one `kanso check lib/json`: 94,784
calls, of which 33,453 returned something and so allocated. The whole compile
makes 148,073 allocations.

The replacement is `for_each_child(e, |child| …)`, which hands the children
over one at a time, and `any_child` for the predicates that were writing
`expr_children(e).into_iter().any(..)`. Both sit on a private `walk_children`
taking `&mut dyn FnMut(&Expr) -> bool`, which stops as soon as the answer is
false — those predicates recurse into whole subtrees, so the short-circuit
`Iterator::any` gave them had to survive. Same nodes, same order, no pass
moved, and `front_end_rounds` 40 and `front_end_visits` 17,786 are identical
on both sides.

    compile_allocs        148,073 -> 91,185     -38.4%
    compile_alloc_bytes 7,942,065 -> 6,830,193  -14.0%
    compile_peak_bytes    876,930 -> 876,930     flat

Thirty-eight per cent where the instrumentation predicted twenty-three. The
counter recorded calls that allocated at least once, and several arms
allocate twice: `vec![cond, early]` followed by an `extend` of the guard's
rest reallocates, and a `.collect()` from a filtered iterator grows. 56,888
allocations removed over 33,453 allocating calls is 1.70 apiece, which is
what that shape predicts.

Wall clock, interleaved, twenty compiles a reading, on a container that had
been building all day: 16.5 ms down to 14.4 ms, about 13%. The figures on the
compiler page are a dated quiet-box sitting and are not re-sat from here.

Welfare reads 84.87 on both sides. It has no term for what compilation
allocates: its compile terms are rounds and visits, which count the work the
compiler decided to do, and `compile_peak_bytes`, which counts what it held.
A third of the traffic disappearing is invisible to all three. This is the
blind spot `bench/compile_allocs_golden.txt` was added for earlier today, and
that gate is now the only thing in the tree that can see this change at all.
Whether welfare should carry a traffic term is an argument about the weights,
and it is not made here.

## 2026-08-24 — a String per call expression, to look up a map

Timing the twenty-two checks in `check_merged` individually put one of them
at half the phase: `check_literal_arguments` 1.93 ms against a whole compile
of about 19 ms, where no other check reaches 0.5. Splitting it again put 0.53
ms in `inline::aliases` and 1.41 ms in the walk.

The walk asks, at every call expression, whether the callee is a std wrapper
over a builtin. `aliases` answers that from a `Map<(String, usize), String>`,
and a lookup needs a key — so the callee's name was cloned into a `String`,
twice, because `forwards` asked the same question a second line later. Two
more `String`s followed for the bare name and for stripping `builtin_`. Four
allocations per call expression to read a map.

A view keyed by the program's own names is built once per pass:

    let builtins: HashMap<(&str, usize), &str> =
        owned.iter().map(|((n, a), t)| ((n.as_str(), *a), t.as_str())).collect();

and the walk borrows through it. One lookup answers both questions and
nothing is allocated.

    compile_allocs   91,185 -> 87,824   -3.7%
    front_end_rounds     40 -> 40        flat
    front_end_visits 17,786 -> 17,786    flat

Wall clock does not move outside the noise of a loaded container, which is
the honest report: 3,361 allocations is about 840 call expressions reached
per pass, times four passes, times one saved allocation each after the
compiler folds the rest. The 1.41 ms is the traversal, not the malloc.

`tests/golden/errors/a_std_wrapper_does_not_hide_its_builtin` is the spec and
it was watched red first. Pin `forwards` to false — the one substitution that
would be wrong if `alias.is_some()` and `contains_key` disagreed — and the
fixture reports two diagnostics where it should report three, losing the one
for `split`, whose demand comes through a wrapper. Restore it and the corpus
is green.

## 2026-08-24 — the compiler was hashing against an attacker who was never coming

Callgrind on `kanso check lib/json`, after the child walk stopped building
vectors:

    14,622,313 (16.09%)  core::hash::sip::Hasher::write
    12,475,688 (13.73%)  core::hash::BuildHasher::hash_one

Twenty-nine point eight per cent of every instruction the front end retired
was the default hasher. `std` picks SipHash-1-3 with a per-process random key
because a server keying a map on a request header needs collisions to be
unpredictable. A compiler keying a map on the identifiers in a file it was
handed has no such adversary and pays for the protection at every lookup.

`src/hash.rs` is the multiply-rotate hash rustc has used for its own interner
since 2015, twenty lines of it, no dependency added. `Map` and `Set` alias
`std`'s containers over it, and every file in the compiler moved: check,
infer, lib, inline, advisory, beat, codegen, demand, dispatch, escape,
linear, provenance, trmc, wasm_backend, parser. `eval.rs` and `wasm_rt.rs`
keep `std`'s maps — those hold a running program's values rather than the
compiler's own bookkeeping.

    instructions   90,899,357 -> 67,160,895   -26.1%
    compile_allocs     87,824 -> 87,824        flat
    front_end_rounds       40 -> 40            flat
    front_end_visits   17,786 -> 17,786        flat
    compile_peak_bytes 876,930 -> 864,274      -12,656

The peak falls because `BuildHasherDefault` is zero-sized where `RandomState`
carries sixteen bytes of key, and every map in the compile is that much
smaller. It stays inside the compile-memory band, so no golden moves.

### The determinism, which was the real find

Instruction counts were being taken to check the change, and the baseline
would not sit still. Same binary, same sources, same directory, empty
environment:

    default hasher   90,704,760   90,676,800      len 80
                     90,667,959   90,708,405      len 115
    seedless hash    66,961,255 x3                len 80
                     66,966,868 x3                len 115

(These are lower than the pair above because they were taken with `env -i`
and from a copied tree; the comparison is within each block.)

Forty thousand instructions of spread between two runs of identical code,
because the random key changes the probe sequences and moves where the
rehashes land. The seedless hash gives the same digits three times.

So the compiler's instruction count was not a measurable quantity, and now it
is. That matters more than the 26%: every other vein in the tree pins an
exact number, and this dimension could not have had one. What the goldens
could see of this change is nothing — allocations identical, rounds and
visits identical, peak inside its band — which is the shape #998 was written
about, one dimension over. The vein that would see it is a separate change
and is written down as one.

Path length still moves the count, 160 instructions per character, visible
only once the run-to-run noise is gone. Any such gate has to compile from a
fixed-length directory rather than from the checkout, for the same reason
`compile_alloc_bytes` is absent from its own golden.

### Why iteration order was never at risk

`RandomState` reseeds every process, so map iteration order has always varied
run to run. The compiler's output is byte-deterministic and its goldens are
not flaky, which means nothing observable reads that order. A different hash
changes the order and can change nothing else. The full suite agrees: 161
flat error fixtures, the three directory cases, the diagnostics differential
and the browser differential are all green.

The ratchet's `compile_allocs_unwatched` mutation greps for
`std::collections::HashSet` in `src/lib.rs`, which this rename moves, so it
was rewritten against the new spelling and re-proved: applied, the counter
goes 87,824 to 93,610 and the gate reds. The other twenty-two mutations were
each applied to a fresh copy of the converted tree and all twenty-three still
take. `cover` runs per PR and only checks that every job has a row, so a
mutation that stopped matching would have gone unnoticed until the nightly
`prove`.

## 2026-08-24 — the compiler's own instruction count gets a vein

`bench/instructions_golden.txt` counts the benchmarks, which are the programs
the compiler produces. Nothing counted the compiler. The hasher change earlier
today took `kanso check lib/json` from 90.9 million retired instructions to
67.2 million and every gate in the tree reported nothing: `compile_allocs`
identical at 87,824, rounds identical at 40, visits identical at 17,786, and
`compile_peak_bytes` moving twelve thousand bytes inside a thirty-five
thousand byte band. A quarter of the work went away in silence, and a quarter
coming back would have been just as quiet.

`bench/compile_instructions_golden.txt` is that dimension, and
`scripts/gates/compile_instructions.sh` reads it.

Two things had to be true first, and only one of them was.

The count has to be repeatable, and under `std`'s hasher it was not: 90,704,760
on one run and 90,676,800 on the next, same binary and same sources, because
the per-process key moves the probe sequences and moves where the rehashes
land. Forty thousand instructions of spread with nothing behind it. That is
why this gate lands the day after the hasher and not before — the earlier
change was the precondition, not merely something the gate would have caught.

The count also moves with the length of the directory the compiler runs in,
about 160 instructions per character, because the absolute path is copied and
walked. That is the trap `compile_alloc_bytes` fell into, and the reason it is
absent from its own golden rather than pinned in it. So the gate does not
measure in the checkout: it copies `lib/` to a fixed path, compiles there, and
the number then agrees to the digit from clones at different depths — 80
characters and 115 characters both read 66,968,333 in the container where that
was tested.

The row itself is the runner's, 66,450,587, copied out of the first job log.
Half a per cent below the container's reading, which is the provenance line
earning its place on the gate's opening run: same sources and the same box,
a different rustc and a different libc.

The ratchet row is `compile_ir`, and its mutation points inference's maps back
at `std`'s hasher. That is the shape the vein exists for: instructions
66,968,333 to 72,245,921 in the container, with `compile_allocs`, the rounds
and the visits all byte-identical. A second row covers the provenance line, as the allocation and
machine-code veins each have.

## 2026-08-24 — the first thing the instruction vein caught

`inline::direct_aliases` decides whether a function is a rename over a
builtin. It resolved the target by stripping `builtin_` off the callee and
then rebuilding `format!("builtin_{target}")` — two `String` allocations per
candidate, on every pass of the fixpoint, for a round trip that cannot change
the string. Every value the map holds was written with the prefix on it, and
the other arm only fires when the callee already carries one, so both paths
came back to where they started. `counts` was rebuilt on each pass as well,
though the program does not change while the fixpoint runs.

    compile_allocs        87,824 -> 87,290       -534
    compile_instructions  66,450,587 -> 65,995,610   -0.68%
    front_end_rounds          40 -> 40             flat
    front_end_visits      17,786 -> 17,786         flat

Both rows are the runner's. Part of the instruction fall is a second edit CI
asked for: with the target a `&str` rather than a `&String`,
`BIRTHS_ERR.iter().any(|b| target == *b)` is `manual_contains` and clippy
refuses it — and the slice's own `contains` is the better codegen too, worth
another 75,373 instructions on the container reading. I had run clippy before
the cherry-pick and not after, so CI caught that rather than me.

`bench/emitted_golden.txt` is unmoved, and that is the evidence that matters
rather than the argument above: the same map drives
`inline_builtin_wrappers`, so a round trip that was not the identity would
have changed what the compiler wrote.

Half a per cent of the front end's instructions, and 534 allocations. Neither
number is large. What makes it worth an entry is that the vein landed six
hours ago and this is the first change it has seen — a move too small for the
wall clock on a loaded box, and invisible to rounds, visits and peak, which
all sat still.

## 2026-08-24 — the door analysis was copying names the program already held

The runner's profile, printed by the instruction gate on its opening run,
named a type outright:

    1,308,039 (1.98%)  HashMap<String, (), BuildHasherDefault<Fx>>::insert

A set of owned strings, hot enough to reach the top fifteen. Ablating the
door advisory behind an environment variable sized it: `compile_allocs`
87,824 to 85,118, so the pass was spending 2,706 allocations, and
`phase::watched` puts it at 0.41 ms of about 11.8.

Every name that analysis carries is a type the program declares. `name_types`
answered a declared type with `name.to_string()`, and the fixpoint's
`returns`, `env` and `tail` were `Set<String>` the whole way up — copies of
strings already in memory, keyed and hashed as copies. They are `Set<&str>`
now, borrowed from the program. `door_advisories` still returns owned
`Vec<String>` messages, so no lifetime escapes the pass.

    compile_allocs        87,290 -> 85,788           -1,502
    compile_instructions  65,995,610 -> 65,590,655   -0.61%
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat

That is 55% of what the pass spent. The rest is the fixpoint's own vectors
and the message strings, which are the work it exists to do.

`tests/advisory.rs` is the spec and it was watched red first. Make
`name_types` answer an empty set for a declared type and
`a_pub_fn_returning_a_foreign_type_with_no_accepting_op_is_advised` fails
while the other five stay green — the door path and only the door path.

The gate's own profile shows the swap where it happened. The owned-key line
falls from 1,308,039 to 1,106,146, and a borrowed-key line stands beside it
at 1,196,935:

    1,196,935 (1.82%)  HashMap<&str, (), Fx>::insert
    1,106,146 (1.69%)  HashMap<String, (), Fx>::insert

Those two cannot be added together and compared against the old single line.
The listing is thresholded at 90%, so a borrowed-key line may well have been
there before and sat under the cut; what a reader can take from it is which
monomorphisation the pass now uses, and nothing about totals. The total is
the row: 65,590,655.

Worth saying where this came from: nobody went looking for it. The gate added
yesterday prints its profile on every run, and the profile named the type on
the first red diff it produced. That is the second thing the instruction vein
has found in a day, and both times the finding was in output that already
existed rather than in a measurement somebody set out to take.

## 2026-08-24 — the demand pass owns keys made of names the program holds

`DemandInfo` answers two questions for codegen: is the bind at this statement
lazy, and may its cell be released with the frame. Both are sets keyed by
`(owning fn name, arity, statement index)`, and the name was a `String`.

That cost twice. Tallying the votes allocated a key per statement per
declaration — names the program was already holding — and every query
allocated another key to throw away:

    self.lazy_binds.contains(&(fn_name.to_string(), arity, stmt_index))

The sets are `Set<(&'a str, usize, usize)>` now, borrowed from the program.

    compile_allocs        85,788 -> 84,261       -1,527
    compile_instructions  65,590,655 -> 65,216,434  -374,221
    front_end_rounds          40 -> 40            flat
    front_end_visits      17,786 -> 17,786        flat

1,527 of the 2,813 the pass was spending, which is 54% — near enough the 55%
the door analysis gave up yesterday to suggest the shape has a characteristic
size. What stays is the fixpoint's own vectors and the discard map.

The instruction fall works out at 245 per allocation removed, against 270 for
the door analysis yesterday. Both are the same order as the malloc machinery
alone: `_int_malloc`, `malloc`, `_int_free` and `free` together are
15,367,938 of this run's 65,216,434, which over 84,261 allocations is 182
apiece before any name is copied. Rounds, visits and peak are flat, so the
allocation and instruction veins are the whole record of the change.

The query allocations go too, and no vein can see it: `is_lazy_bind` and
`is_releasable` are called only from codegen, which `kanso check` never runs.
They are gone all the same.

Two things about the borrow were settled by building rather than reasoning.
The first was whether `DemandInfo` could borrow at all, given that it outlives
its pass — all three owners already carry a lifetime and already borrow the
program, so `infer::Ctx<'a>`, `eval::Interp<'a>` and `codegen::Backend<'a>`
took it unchanged. The second looked worse: `HashSet::contains` wants
`K: Borrow<Q>`, and for a tuple key `Q` is the tuple exactly, so a
`Set<(&'a str, …)>` seemed to need an `&'a str` to probe with rather than the
shorter-lived `&str` the query signature offers. The plan was to restructure
as `Map<&'a str, Set<(usize, usize)>>` to avoid it. The plan was unnecessary:
`&str` is covariant and `HashSet` is covariant in its key, so the probe
compiles as written. One build settled what an afternoon of reasoning would
have got wrong in the safe direction.

`src/demand.rs`'s own two tests are the spec, and the mutation was watched:
tally the votes under a key whose name is `""` and
`discard_capable_argument_is_lazy` fails while
`scrutinized_binding_stays_strict` stays green, which is the lookup path and
only the lookup path.

## 2026-08-24 — the alias canonicaliser borrows the names it reasons about

`canonicalize_bare_aliases` decides which bare function names are clones of a
qualified original and rewrites them to the qualified spelling. Two things it
did cost allocations that had nothing to do with that decision.

The first was a needle. For every synthetic declaration it built
`format!("/{}", d.name)` and asked whether a twin's name ended with it, then
dropped the string. `strip_suffix` asks the same question without building
anything:

    let qualified = name
        .strip_suffix(d.name.as_str())
        .is_some_and(|qual| qual.ends_with('/'));

The second was the binder walk. `bound_in_pattern`, `bound_in_stmt` and
`bound_in_expr` collect every name a declaration binds — parameters, `x = ...`
bindings, lambda parameters, destructured fields — so that a name which is
ever locally bound is never rewritten. They collected them as `String`.
`check.rs` has carried a borrowed twin of the same walk at :1453 and :1463 for
as long as this one has owned its names, so the shape was already in the tree
and already proven. This walk was the outlier.

    compile_allocs        84,261 -> 82,848      -1,413
    front_end_rounds          40 -> 40           flat
    front_end_visits      17,786 -> 17,786       flat
    compile_peak_bytes   864,274 -> 864,274      flat

Measured against main before the demand pass landed, this same diff was worth
66,117,450 -> 65,586,233, a fall of 531,217. Measured against main after it
landed, the identical diff is worth 65,709,239 -> 65,364,674, a fall of
344,565. The allocation delta is -1,413 in both cases. Every count is exact:
the branch figure reads 65,364,674 on two consecutive runs in the fixed box,
which is what the seedless hash bought.

So the same 1,413 allocations are worth 376 instructions apiece on the fatter
tree and 244 on the leaner one. Allocation counts compose — 84,261 - 1,413 is
82,848 and that is what the counter reads — and instruction counts do not.
The malloc machinery's cost per call depends on the state of the free lists,
and a tree that already stopped making 1,527 allocations presents a cheaper
one. That is a caution about a habit this log has been building: three entries
now have quoted an instructions-per-allocation ratio (270 for the door
analysis, 245 for the demand pass), and the ratio is a property of the
measurement rather than of the change. Two changes each worth "250 per
allocation" do not add up to their sum.

The golden row was predicted before CI ran, by carrying the container's delta
across to the runner's baseline: 65,216,434 - 344,565 = 64,871,869. The runner
read 64,840,962, so the prediction was 30,907 high — the runner's own delta is
-375,472 against the container's -344,565. Allocations transferred exactly
(82,848 was predicted and 82,848 is what CI read, as it has every time), and
instructions did not. The toolchains differ, 1.98.0 on the runner against
1.94.1 here, and the malloc-shape argument above says the instruction cost of a
change depends on the tree it lands in. Predicting the allocation row is safe.
Predicting the instruction row saves nothing, because being 0.05% out costs the
same red cycle as not guessing at all.

`beat::tests::a_locally_bound_name_is_never_rewritten` is the spec, and the
mutation was watched. Make `bound_in_pattern` insert nothing for `Var` and
`Annotated` and it fails with `the local was rewritten: {"list/first", "xs"}`,
while the other 33 in that module stay green — the skip set loses the local
`xs`, and the bare alias is rewritten straight over it. That is the walk's
entire purpose and the only path that moves.

The ratchet needed a repair to survive this. Its `compile_allocs` mutation
rewrote `out.insert(name.as_str());` unscoped, meaning to hit `mentions_in_expr`
— the only place that line existed. `bound_in_pattern` now has a line spelled
exactly the same, and the sed rewrote both, which put a `String` into a
`Set<&str>` and made the mutation fail to compile. The substitution is scoped
to `mentions_in_expr` by address now, and it was checked the long way: apply
it, confirm the tree still builds, confirm `bound_in_pattern` was left alone,
and read the counter at 88,634 against a golden of 82,848.

The reason this could have gone unnoticed is written down in `ratchet.kso` as a
deliberate rule: "a build that breaks because of the mutation is the gate going
red early, and `prove` counts that as red." So a mutation that stops compiling
passes `prove` exactly as a working one does, and the row goes on asserting
that `compile_allocs` notices an owned-name pass when nothing has tested that
in months.

The rule is reasonable for a mutation whose defect is meant to break the build.
It is wrong for this vein, where the whole claim is that a COUNTER moves: a
tree that will not compile never reaches the counter, so the row proves
nothing. Any mutation keyed on a line of source rather than on a function is
one identical line away from that state, and by the rule above it will not
announce the change.

That is the ratchet's own design, so by the pending-gavels rule it is settled
here rather than sent up. `prove` now requires `setup` to succeed, and a row
whose mutated tree will not build reports UNBUILT instead of red. The split is
safe because the rows whose defect IS a build or lint failure — clippy bait,
misformatted source, a given-up tail call — carry `no_setup` and do their
building inside the GATE, where a failure still counts. Only the ten rows
carrying `release` as setup are affected, and every one of their gates reads a
counter or a golden.

The demonstration is the bug itself. With the unscoped sed restored, on the
same tree:

    old prove:  red      ... every row turned its gate red
    new prove:  UNBUILT  ... 1 rows proved nothing

and with the sed scoped, the new code reads red again and all eight rows pass.

Fixing that turned up a second thing immediately, which is the argument for
the change. Two rows came back UNBUILT on an unmutated tree, and the reason
was that `prove` symlinks each worktree's `target` at a shared
`/tmp/kanso-ratchet-target` that nothing creates. On a fresh machine the link
dangles, `cargo build` stops with "Not a directory", and every row carrying
`release` as setup was being counted red without its gate ever running. It
takes a `mkdir -p` to fix. How long it had been true is not something this log
can say — the old code could not distinguish that state from a working row,
which is the whole point.

One thing found on the way. The doc comment above `bound_in_pattern` had
collected two paragraphs belonging to other functions: one describing
`qualify` and one describing `canonicalize_types`, both of which sat
undocumented further down the file. They are back where they belong.

## 2026-08-24 — four maps that knew their size and did not say so

`reserve_rehash` was 2,402,471 instructions of the front end's 65.4 million,
which is a map growing from empty and copying itself at 3, 7, 14, 28 entries
and so on. Four of the maps that pay it are built by walking `program.fns`,
whose length is known before the loop starts: `groups` and `callee_first`'s
`by_name` in inference, `at_site` and `by_name` in the alias canonicaliser.
They are pre-sized now.

    compile_allocs        82,848 -> 82,776   -72
    front_end_rounds          40 -> 40        flat
    front_end_visits      17,786 -> 17,786    flat
    compile_peak_bytes   864,274 -> 864,274   flat

The instruction figure is the part worth keeping. Rehash work fell by 419,151,
from 2,402,471 to 1,983,320, and the total fell by 80,631 — a fifth of what
was saved. The rest went back out in the larger table: a capacity hint that
over-shoots buys a bigger allocation and a bigger region to clear.

Which suggested sizing each map to what it actually holds, so the table would
be no larger than natural growth would have made it. The sizes were measured
rather than guessed — printing `len()` at the end of each build over lib/json
and its dependencies gives 89/158, 22/26, 1/2, 240/407 and 186/326 for the two
name-keyed maps, so five eighths is above every sample; `at_site` runs at
1.00, 1.00, 1.00 and 0.81, so it wants the full count. Working through
hashbrown's bucket arithmetic, that lands each table on the same power of two
natural growth reaches, with none of the rehashes. It should have been strictly
better than the blanket hint.

It was worse. Sized to need reads 65,318,681 against the blanket 65,284,043,
so the careful version gives back 34,638 of the 80,631. The blanket hint's
extra room is not waste: a table at half load probes in fewer steps, and that
is worth more than the memset it costs. The arithmetic was right about bucket
counts and wrong about what bucket counts are for.

So the blanket `program.fns.len()` ships, and this is recorded mostly because
the reasoning was careful, checkable, and beaten by the measurement. There was
no way to reach it from the bucket arithmetic alone.

    compile_instructions  64,840,962 -> 64,771,091   -69,871

The runner's fall is 69,871 against the container's 80,631, and this time the
row was taken from CI rather than predicted, which is what the previous entry
concluded to do.

A third of `reserve_rehash` remains, in maps this change does not touch.

## 2026-08-25 — the allocation map, and one throwaway vector it found

Every allocation win so far was found by reading the instruction profile and
guessing which pass behind it was allocating. That guessing has been wrong at
least once today: `used_globals` looked like the obvious owner of the
986,615-instruction `Set<String>::insert` line, and instrumenting it showed 612
inserts, which cannot be it.

The per-phase allocation probe was built for this and declined, because it
costs the shipped compiler 92,873 instructions per compile to carry a counter
nobody outside a profiling run reads. It did not need to be in the compiler.
`valgrind --tool=dhat` records every allocation with its call stack and needs
no code at all — build with `RUSTFLAGS=-g` for symbols and read the JSON. It
agrees with the counter exactly: 82,788 blocks against `compile_allocs`
82,776, the difference being twelve allocations before the counting allocator
is installed.

Attributing each block to its innermost kanso frame gives the map the tree has
never had:

    8,649  10.4%  lexer::lex_line
    7,733   9.3%  infer::eval_expr
    5,768   7.0%  walk_children
    5,236   6.3%  hashbrown fallible_with_capacity
    3,956   4.8%  Tok::clone
    3,866   4.7%  infer::infer
    3,184   3.8%  lexer::lex
    2,726   3.3%  inline::rewrite
    2,407   2.9%  trmc::rewrite
    2,366   2.9%  provenance::Walk::expr
    2,318   2.8%  check::param_names
    2,239   2.7%  parser::P::parse_app

Lexing is 19.1% of the total across its three frames, which is the largest
single area and the one whose obvious fix was declined on 2026-08-24 for
reasons that still hold.

One row in that table needs reading carefully, and it is a caution about the
method rather than about the row. `walk_children` does not allocate — it was
given a callback in #1007 precisely so that it would not. Its 5,768 blocks are
`RawVec::grow_one` inside `for_each_child`'s closure, which is a CALLER's
vector growing while the walk runs. Four places in `check.rs` spell
`let mut stack = vec![e]` once per statement and then push children into it, so
each statement allocates a worklist and doubles it as it fills. The innermost
kanso frame is the honest thing for dhat to report and it is not the same as
the frame that owns the memory; where a callback separates the two, the stack
has to be read rather than the label.

`check::param_names` is not that. It answered `Vec<&str>` — already borrowed,
but a vector per call, and the `Ctor` arm built one for the fields and then a
second to put the `whole` name in front of it, so a nested pattern allocated
one per level for names nobody kept. That is the shape `expr_children` had
before it took a callback. It takes one now, with `first_param_name` beside it
for the single caller that wants only the first.

    compile_allocs        82,776 -> 80,458           -2,318
    compile_instructions  64,771,091 -> 64,175,885   -595,206
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat
    compile_peak_bytes   864,274 -> 864,274           flat

The fall is exactly what dhat attributed to that frame, which is the useful
part: the map predicts, rather than merely explains after the fact.

The spec is `tests/golden/errors/let_binding.kso` and it was watched red. Make
`for_each_param_name` hand back nothing for `Var` and `Annotated` and the
corpus reports ``a binding is ` = …` `` where it should say
``a binding is `x = …` ``, which is `first_param_name` and only that path.

Worth naming honestly: that mutation blinds both functions, and only the one
golden moved. The four callers that build a `bound` set — the shadow checks and
the literal walk — went on passing with every set empty. Their coverage is
thinner than it looks, and that is true of the code as it stood this morning
rather than anything this change did.

## 2026-08-25 — five worklists that were rebuilt for every statement

The entry above found that `walk_children`'s 5,768 blocks in the allocation
map belong to callers rather than to the walk. This is those callers.

Five places in `check.rs` drive an explicit worklist instead of recursing —
`let mut stack = vec![e]`, then pop and push the children. Three built that
vector once per STATEMENT and two once per DECLARATION, and in every case it
started empty and doubled its way up as the walk filled it. A vector per
statement, across every statement of every declaration, for a scratch list
that is empty again by the time the statement ends.

One vector per site now, cleared and reused.

    compile_allocs        80,458 -> 77,249           -3,209
    compile_instructions  64,175,885 -> 63,492,172   -683,713
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat
    compile_peak_bytes   864,274 -> 864,274           flat

dhat agrees on where it went: `walk_children` falls from 5,768 blocks to
4,237, and the rest of the difference is the `vec![e]` itself, which was
attributed to the calling function rather than to the closure.

The `clear()` is the part worth explaining, because the reuse does not need it
to be correct. Every one of those loops drains its stack — `while let Some(cur)
= stack.pop()` runs to empty, and none of the five has a `break`, `continue` or
`return` inside it, which was checked rather than assumed. So the stack is
already empty when the next statement starts.

What the `clear()` buys is that nothing has to keep being true. The mutation
was run: make one of those loops stop early, so it leaves items on the stack
for the next statement to inherit, and the entire golden error corpus stays
green. Ten golden tests, every diagnostic compared exactly, and not one of them
can see a walk that carries expressions from one statement into the next. An
invariant that load-bearing, with no spec under it, is not one to build a
performance change on top of; a `clear()` on an empty vector costs a store and
removes the question.

That the corpus cannot see this is the finding, and it is about the corpus
rather than about the worklists. It is recorded here rather than fixed because
the fixture that would catch it — a leak that changes which file a diagnostic
names — belongs to whoever next touches those checkers.

### The map was blurred, and sharpening it moved the headline

The map in the previous entry, and the one this entry started with, attributed
every block to the innermost kanso frame valgrind reported. On an optimised
build that frame is the ENCLOSING function: everything inlined into it collapses
under its name. The `walk_children` row was one symptom, caught by reading the
stacks. It was not the only one.

`valgrind --tool=dhat --read-inline-info=yes` resolves the inlined frames, and
the same 77,261 blocks sort quite differently:

    25,966  33.6%  String::clone
     5,885   7.6%  Vec::from_iter
     5,236   6.8%  hashbrown table allocation
     4,802   6.2%  infer::eval_expr
     3,707   4.8%  String::from_iter(&char)
     3,263   4.2%  Vec::clone
     3,078   4.0%  lexer::lex_line

A third of everything the front end allocates is copying a `String`. That did
not appear anywhere in the coarse map, because each clone was charged to
whichever pass had inlined it. `infer::eval_expr` shrank from 7,733 to 4,802
for the same reason, and its 3-to-5-byte blocks — which no `HashMap<&str, u16>`
clone could ever produce, since the smallest hashbrown table is over a hundred
bytes — are the ones that moved out.

Charging each `String::clone` to the first kanso frame beneath it gives the
list worth working from:

    3,807  Tok::clone                 1,628  inline::direct_aliases
    2,916  for_each_child closure     1,192  check::declared_names
    2,366  provenance::Walk::expr     1,046  trmc::rewrite
    2,292  check::check_file_shadow     919  demand::analyze
    2,105  compile_module_loaded        843  inline::rewrite
    1,829  provenance::analyze          788  fuse_enumerable

Provenance is the largest cluster outside the lexer at 4,195 across its two
frames. Nothing here is acted on yet.

The lesson is the same one the `walk_children` row taught, and it is worth
stating once: a profile that names functions is naming the frames the compiler
left behind, not the code that ran. Read the inline info, or read the stacks.

## 2026-08-25 — provenance borrows its group keys, and gets a spec at last

The sharpened map put `String::clone` at a third of the front end's
allocations, spread across a dozen passes. Provenance was the largest cluster
outside the lexer, 4,195 blocks across `Walk::expr` and `analyze`.

`type Group = (String, usize)` — a declaration's name and arity, with the name
owned. That is the shape the demand pass had before 2026-08-24, and it cost the
same way, only worse: this is a fixpoint that runs the whole program through up
to two hundred rounds, and every round rebuilt the key for every group it
touched. `self.returns.get(&(name.clone(), args.len()))` allocated a `String`
per lookup and dropped it.

`Group<'a>` is `(&'a str, usize)` now, `binds` is `HashMap<&'a str, Pkgs>`, and
`Provenance<'a>` carries the lifetime out to its one consumer in `main.rs`,
where the program outlives it. It compiled on the first attempt; the covariance
question that was settled by building on 2026-08-24 did not come up again.

    compile_allocs        77,249 -> 72,756           -4,493
    compile_instructions  63,492,172 -> 62,351,359   -1,140,813
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat
    compile_peak_bytes   864,274 -> 864,274           flat

That is 1.8% of the front end's work for one pass's keys, and 254 instructions
per allocation removed — the highest rate of the day, because a fixpoint pays
for its keys once per round rather than once.

The fall is larger than the 4,195 dhat charged to provenance, because the
candidate list's `group.clone()` and the vectors around it went with the keys.

### The pass had no spec, and now it does

The mutation was run before the change shipped, and it found nothing. Collapse
every group's name — `("", decl.params.len())` — so that provenance can no
longer tell one declaration from another of the same arity, and all six
advisories in `tests/advisory.rs` stay green, and lib/json's three licence
advisories come back byte-identical. The central key of a whole pass, and the
suite could not see it change.

`tests/golden/advisory/group_identity` is the fixture that can. `rescue` is pub,
so provenance seeds it — a published err parameter is assumed to see its own
package's failures, because its callers are not all in view. `quiet` is private
and uncalled, so nothing ever feeds it. The two differ only in their name. With
the key intact only `rescue` is advised; with the names collapsed `quiet`
inherits what `rescue` was fed and is advised for a rescue it never made, and
`a_group_is_told_apart_by_its_name_and_not_only_its_arity` goes red while the
other six stay green.

One thing worth recording about how that was found, because it nearly went the
other way. Restoring the source and re-running WITHOUT rebuilding produced a
reading that looked like a genuine result — both functions advised on correct
code — and the explanation for it was already half-written before a `grep` of
the source against the binary's behaviour showed the binary was still the
mutated one. The rule that saves this is cheap: after restoring a mutation,
rebuild before reading anything.

## 2026-08-25 — the log's older end moves to the archive, and the page catches up

This file's own header has said "the last forty entries" for as long as the
archive has existed, and it held eighty-two. 3,482 lines, read at the tail on
every session and in full by nobody, which is the cost the arrangement was
invented to stop paying.

The older forty-two are in `log/compiler-log-archive.md` now, unedited and in
order, which puts this file back at forty entries and 2,028 lines and the
archive at 673. The move was checked rather than trusted: the 713 entry
headings across the two files come back in identical order, and a multiset of
every line in both differs by exactly one blank line at the join.

`docs/compiler.html` moved with it, which is what `page_drift` was sitting at
3/3 to insist on. Three shipped changes had landed in this log since the page
was last edited and the numbers it published were three changes stale: the
front end allocates 72,756 times where the page said 82,776, and retires
62,351,359 instructions where it said 64,771,091. The page now also carries the
two things from this sweep that outlast the numbers — that dhat gives an
allocation map with no code in the compiler, and that its first map was blurred
because an optimised build charges inlined code to the enclosing frame.

## 2026-08-25 — the shadow checker's globals, and the arities beside them

`collect_globals` builds the set of every name a module puts in scope — the
ambient builtins, the nullary forms, every type and every declaration — and it
built that set out of `String`. `check_file_shadow` then extended it with a
clone of every extern global on top, and beside it keyed two more maps,
`fn_arities` and `type_arity`, on cloned declaration and type names. All of it
read once and dropped at the end of the call.

The set and both maps borrow from the program now, and `Resolver` holds
`&'a HashSet<&'a str>` where it held `&'a HashSet<String>`.

    compile_allocs        72,756 -> 70,356           -2,400
    compile_instructions  62,351,359 -> 61,349,546   -1,001,813
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat
    compile_peak_bytes   864,274 -> 864,274           flat

The gate's own profile shows the swap where it happened. The
`HashMap<String, ()>::insert` line that has sat in the top fifteen since the
seedless hash went in — 986,615 instructions — is gone from the listing
entirely, and the borrowed-key line beside it rises from 1,310,405 to
1,543,975. As with the door analysis on 2026-08-24, those two figures cannot be
added and compared against the old pair: the listing is thresholded at 90% and
what a reader can take from it is which monomorphisation the pass uses. The
total is the row.

`used_globals` stays owned, and that is not an oversight. It accumulates across
every dependency in the build and is read after each one's program has gone, so
its contents have to outlive the thing they came from. It is the one name set
here that cannot borrow.

This path is pinned, unlike the last two. Drop declaration names from the
globals set and thirty of `beat.rs`'s tests fail, the first of them with a wall
of `error[name]: unknown name` over std/text — every function in the program
becomes a name nothing declared. That is what a covered invariant looks like,
and it is worth putting beside the two found bare today: the difference is that
this set is what a user meets as a diagnostic, while a fixpoint's key and a
worklist's residue are internal and were tested as if they did not exist.

## 2026-08-25 — the extern name set stops being copied once per file

`declared_names` answered `HashSet<String>`, and the loop that checks each file
of a module used it twice over:

    let mut extern_globals = all_names.clone();
    for name in check::declared_names(program) {
        extern_globals.remove(&name);
    }

`all_names` holds every name the whole build declares. Cloning it once per file
is a `String` per name per file, and `declared_names` allocated another set of
its own on the next line so the copy could have names removed from it. The
result is read by the shadow check and dropped.

`declared_names` borrows now, and the extern set is built by filtering
references out of `all_names` rather than copying and subtracting:

    let extern_globals: crate::hash::Set<&str> = {
        let own = check::declared_names(program);
        all_names.iter().map(String::as_str).filter(|n| !own.contains(n)).collect()
    };

    compile_allocs        70,356 -> 67,948           -2,408
    compile_instructions  61,349,546 -> 60,277,520   -1,072,026
    front_end_rounds          40 -> 40                flat
    front_end_visits      17,786 -> 17,786            flat
    compile_peak_bytes   864,274 -> 864,274           flat

`all_names` itself stays owned, and the borrow checker is the reason rather
than a preference: the loop below it takes `&mut parsed`, so a set holding
references into those programs could not live across it. It is fed with an
explicit `String::from` now, which says at the call site that the copy is
deliberate.

Watched red: invert the filter, so the set holds a file's own names instead of
everything else, and six of `beat.rs`'s and the dispatch, escape and linear
suites go red together.

Two things about the method, both mine to own. The clippy that CI runs
(`--all-targets`, no `--release`) refused `declared_names<'a>` as a needless
lifetime; the elided form is what shipped, and running the CI form locally is
what caught it rather than CI. And a `cargo test --release` was started while
an earlier one was still running in the background, which put them on the same
target directory and produced a failing `ir_verifier` that reran green on its
own. A test result read out of a contended run is not a result. One suite at a
time, and read the exit code rather than grepping the stream.
