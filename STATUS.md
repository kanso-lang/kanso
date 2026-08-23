# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

**Does an undemanded knot count as a thunk allocation?** The engines disagree
and a fixture cannot pin the shape until this is settled. `x = [x]` that
nothing demands reads `thunk_allocs=1, thunk_live_exit=1` on native and
`thunk_allocs=0` on the oracle; forces and evals agree at zero on both. Native
freezes every knotted constant in `k_caf_init` before main, because the
alternative is a branch and a store on every read of a frozen constant and that
sits in the hottest dispatcher. Four ways out, and the cost of each is known:
defer the cell on native and pay that branch; give knot cells their own counter,
which is additive and moves 146 `.mem` files, four cost goldens, the emitted
golden, the ch10 sample and the siblings' veins; retire `thunk_allocs` from the
engine-shared set, which narrows what the differential law covers; or leave it
and record that no fixture pins an undemanded knot. Filed 2026-08-20 as a
finding, unresolved since.

**Is a bare list of small ints bytes?** (task #2). Six functions answer
differently on the two engines, and in four of them the ORACLE answers where
native refuses — `text/append ["a"] "x"` is `["a" 120]` on the interpreter and
a refusal natively — so a program written against the oracle runs and the same
program compiled dies. The cause is one representation: the interpreter has no
distinct bytes value, so any list goes down the bytes path. Either native
widens, and a list and bytes become interchangeable, or the interpreter gains a
real bytes value and every place it builds or reads them moves. The measured
table is in pending-gavels. Nothing can be pinned until this is ruled.

**The compile-memory band has been hiding main's own drift, and correcting it
costs the floor.** `bench/compile_memory_golden.txt` holds peak bytes at
871,649 and CI asserts only that reality is within two per cent of it — 17,432
bytes of slack to absorb a documented host divergence of 56. Main measures
872,025 on this box, three runs identical; this branch measures 872,035, and on
the runner 872,061 twice. Ten of those bytes are the branch's and 376 are
main's, accrued green. It matters because welfare reads that row as the current
value of the compile-memory term rather than measuring it, so the term has been
scored against a figure the compiler left behind and the floor was ratcheted to
84.85 while it was. Put 872,061 in the file and `scripts/welfare` exits 1,
though the printed score still reads 84.85. Three options priced in the log for
2026-08-23: `--set` the floor on a fall, which `--set` has never been used for;
pay the 376 bytes back out of the front end; or tighten the band to something
near the divergence it documents, which looks right either way now that a gate
can refuse off the reference host instead of widening to tolerate it. Nothing
changed pending your call.

**Does `>>` accumulate failures the way a parallel group does, or stop at the
first one?** (task #141). Both channels are settled: the value channel answers
one value, and failures merge associatively so the fractal you worried about
never gets built — that is the applicative/monad split, with `.` as the monad.
The open fork is measured: a *build* failure on either side merges, but a
run-time *effect* failure on the left stops the right dead. So one operator
behaves two ways depending on when the failure lands. Say the wall orders
effects and unordered failures cannot preempt, or run the right anyway and
merge. It collides with #105 wanting the right side lazy.

## In flight

**Branch `claude/go-to-town-m0dicm` is pull request #987, open, twenty-four
commits, waiting on review.** Main requires one, so I cannot land it.

The largest piece is the 2026-08-17 gavel built: `std/os` split out of
`std/io`, fourteen names moved and three left behind, across 69 `.kso` files
and 332 call sites. `stdin`, `write` and `write_err` stay in `io` — the
boundary case, answered by the language committee rather than sent back, since
Go's standard streams are files in `os` and the writing is done from `fmt`, and
kanso has neither. A program written before the split is now told where the
name went rather than that the name is unknown.

**Merge order: #987 first, then bump `.kanso-version` on kanso-lang/kq#78.**
kq's branch is red on purpose — it builds against a pinned compiler that has no
`std/os`. kanso's own `kq specs` job is green against that branch. vse and
kanso-json use none of the moved names.

Also in it: the accumulator rewrite reads an operand it can prove, closing an
engine disagreement on `n * fact (n - 1)` past ten thousand frames and gaining
a differential of its own. Two gates that had gone blind — page_drift read
`0/3` for days while the page fell twenty-two entries behind, because a shallow
fetch two steps above it truncated the history it reads, and the pages build
served whatever `docs/kanso.wasm` was committed rather than the engine it was
built from. The chain-depth spec measures with the kernel rather than with
time(1), which a container does not have. `to_int` and `to_float` name every
kind they take. `kanso build myapp` from the directory above it says what
happened instead of handing over the linker's complaint. The last `.py` file is
a kanso program. A release workflow exists and fires on a tag, which nobody has
pushed.

**Two goldens now name the host that measured them**, because I read one on the
wrong host and pasted this container's numbers over the runner's. Retired
instruction counts belong to the runner's glibc — 2.39-0ubuntu8.7 here against
2.39-0ubuntu8.8 there is about four hundred instructions before main, more than
most of what that vein exists to catch — and `.text` sizes belong to the clang
that emitted them. Each golden carries a `measured-on` line,
`scripts/gates/measured_on.sh` reads it before the expensive part of either
gate, and off the reference host it refuses without printing a number to copy.
Ratchet rows `instructions_host_unpinned` and `text_host_unpinned`.

**The repo has no python left in it, and a gate says so** (task #55, closed
too early, and re-opened by the regression below). Both harnesses that drove
headless chrome are kanso, CI runs the kanso ones, and `scripts/stale_a_panel`
— the last `.py` file, which the book gate ran on every build — is a kanso
program as of 2026-08-23, checked byte-identical against the python it
replaces on a real chapter.

The six heredocs in `scripts/ratchet/mutations/*.sh` that used to shell out to
the other language are POSIX awk, producing byte-identical mutated sources and
identical exits. They
are not kanso, and the bootstrap is why: each edits a compiler source file
before anything in that worktree is built, and the `target/` the worktree
links to is shared across rows, so the binary sitting there is whatever the
previous row's mutated source produced. A tool that damages the compiler
cannot be written in the language that compiler compiles — but it can be
written in awk, which needs no build at all.

- `scripts/site_smoke` makes four visits, one per page the site promises — the
  landing sample, the playground, a book chapter and the chart. Each probe was
  watched red before it was trusted.
- `scripts/browser_differential_run` compiles every corpus program in the tab
  and requires byte-identical status and output against the native engine,
  excusing a disagreement only where tests/golden/wasm_gaps.txt records what the
  wasm engine answers instead. Last run here, 2026-08-23: 317 programs, 310
  agree, 7 known gaps, 0 disagree.

Porting them found four defects the suite could not see. Two were in the
library: a server could not hand its report back, and a connection that said
nothing killed it — which is what a browser's speculative preconnection is.
Two were in the runtime: a repair raised three of a beat mark's four fields and
let the arena hand out memory past the end of a block (#823, caught by glibc on
linux and invisible on macOS), and a program could only ever open sixty-three
sockets and processes because the guard counted takes rather than asking
whether a slot was free (#825).

Python then crept back within days of that port — #854's mutation heredocs,
#862's panel staler — and a dead bench/kq_race.sh predated it, racing an
apps/kq this repo no longer holds. All three are gone: the staler is kanso,
the heredocs are awk, the racer is deleted. `scripts/gates/python_free.sh`
now fails CI on any tracked .py file or python invocation, with a ratchet
row proving it turns red.

## Recently ruled by Clay

- **2026-08-05** — two errs in one operation merge, both engines. Shipped.

- **2026-08-05** — the chart draws from deterministic counters; wall clock is
  only for kq's table against jq, where a third party's cost cannot be counted.
- **2026-08-05** — the wider decode field (rust, go) is a demonstration re-sat at
  releases, not a CI gate. It lives under the CI board, dated.
- **2026-08-04** — `kanso play` is the relaxed single file: runs, never builds,
  stdlib imports only.
- **2026-08-04** — the stdlib apes Go. `std/net/http` carries Go's name and shape.

## Standing

Everything else is on the task list, which is the source of truth for what is
in flight. A decision that is Clay's gets `owner: clay`, a `CLAY'S CALL` prefix,
a push notification, and the top of this file.
