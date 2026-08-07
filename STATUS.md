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

**Replace the last two python scripts** (task #55). `scripts/browser_differential.py`
and `scripts/site_smoke.py` both serve HTTP while driving headless chrome.

- Sockets: done, both engines. `listen`, `accept`, `net_read`, `net_write`,
  `net_close`, with `accept` and `run` as scheduling points so a server and its
  client can be adjacent statements. A program serves itself and prints the same
  bytes interpreted and compiled.
- `std/net/http`: done. Request and response are records, a handler is a plain
  function from one to the other, and the mux is arms on the path. A routed POST
  round-trips end to end.
- `io/start` and `io/kill`: done, both engines. Headless chrome ignores its own
  exit budget, so the port needed Go's `cmd.Start()` and `Process.Kill()`.
- Next: port the two scripts, which is what proves the surface is right. The
  first thing writing an http test found was `content-length` counting
  characters where the protocol counts bytes.

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
