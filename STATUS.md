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

**Branch `claude/go-to-town-m0dicm`, pushed, no pull request opened.**
The accumulator rewrite reads an operand it can prove, which closed an engine
disagreement on `n + weigh (n - 1)` past ten thousand frames and gained a
differential of its own. The chain-depth spec measures with the kernel rather
than with time(1), which a container does not have. page_drift could not see a
truncated history and read `0/3` for days while the page fell twenty-two
entries behind; both the workflow and the gate are fixed. `to_int` and
`to_float` name every kind they take. The last `.py` file is a kanso program.
A release workflow exists and fires on a tag, which nobody has pushed.

**The repo has one python left in it, and it is there on purpose** (task #55,
closed too early). Both harnesses that drove headless chrome are kanso now, CI
runs the kanso ones, and `scripts/stale_a_panel` — the last `.py` file, which
the book gate ran on every build — is a kanso program as of 2026-08-23, checked
byte-identical against the python it replaces on a real chapter.

What stays python: the `python3 - <<'PY'` heredocs in six of
`scripts/ratchet/mutations/*.sh`. Each edits a compiler source file before
anything in that worktree is built, and the `target/` the worktree links to is
shared across rows — so the binary sitting there is whatever the previous row's
mutated source produced. A tool that damages the compiler cannot be written in
the language that compiler compiles. The bootstrap is the reason, and it is
written down here so nobody re-opens it as an oversight.

- `scripts/site_smoke` makes four visits, one per page the site promises — the
  landing sample, the playground, a book chapter and the chart. Each probe was
  watched red before it was trusted.
- `scripts/browser_differential_run` compiles every corpus program in the tab
  and requires byte-identical status and output against the native engine,
  excusing a disagreement only where tests/golden/wasm_gaps.txt records what the
  wasm engine answers instead. Last run here, 2026-08-23: 315 programs, 308
  agree, 7 known gaps, 0 disagree.

Porting them found four defects the suite could not see. Two were in the
library: a server could not hand its report back, and a connection that said
nothing killed it — which is what a browser's speculative preconnection is.
Two were in the runtime: a repair raised three of a beat mark's four fields and
let the arena hand out memory past the end of a block (#823, caught by glibc on
linux and invisible on macOS), and a program could only ever open sixty-three
sockets and processes because the guard counted takes rather than asking
whether a slot was free (#825).

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
