# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

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

**The repo has no python in it** (task #55, closed). Both harnesses that drove
headless chrome are kanso now, and CI runs the kanso ones.

- `scripts/site_smoke` makes four visits, one per page the site promises — the
  landing sample, the playground, a book chapter and the chart. Each probe was
  watched red before it was trusted.
- `scripts/browser_differential_run` compiles all 287 corpus programs in the
  tab and requires byte-identical status and output against the native engine,
  excusing a disagreement only where tests/golden/wasm_gaps.txt records what the
  wasm engine answers instead. It says what the python said: 279 agree, 8 known
  gaps, 0 disagree.

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
now fails CI on any tracked .py file or python3 call, with a ratchet row
proving it turns red.

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
