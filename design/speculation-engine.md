# The speculation engine — work the red lights

Status: **design, head of the performance frontier. Not implemented.**
Clay's framing, ratified in dialog 2026-07-22: the fedex driver works on
his reports on his phone when he's stuck at red lights. The reports are
computations that must happen eventually; the red light is an IO stall;
nobody asks for a report before he starts one.

## The model

When a fiber blocks — sleep, read, any effect the scheduler parks — the
runtime does not idle. It pulls from a **pool of pending computations**
and evaluates them until the fiber wakes. Purity makes early evaluation
invisible: no effects can leak out of order, and the value computed
early is the value that would have been computed late.

Two populations feed the pool:

- **Proven-demand work** (the strictness analyzer's "certain" set):
  speculating it is risk-free — it was going to run regardless, so a
  stall-filled evaluation is a pure win. These enter the pool whenever
  their inputs are ready and a coarse-grain check passes.
- **Conditional thunks** (the demand analysis deferred them): gambles.
  The cost model prices them — likelihood of demand times cost saved —
  and wasted work is harmless (pure, arena-reclaimed) but not free
  (memory, cache), so admission is priced, not open.

## The three pieces

1. **Discovery.** Thunks live as cells inside structures; the scheduler
   needs a pool, not a heap walk. The defunctionalized-thunks build
   (past ICFP 2025 first-order laziness — closed world makes every
   thunk shape enumerable) gives each pending computation a site id and
   argument record; the pool holds (site, args) pairs. Shared
   machinery: build the two together.
2. **Grain.** Handing a multiply to the scheduler costs more than the
   multiply. The lazy cost gate already prices computations; the same
   model gates pool admission. Only coarse work rides.
3. **Determinism — the kanso-specific law.** Which computations run at
   which stall must be a function of the ROUTE, not of how long each
   light actually lasted. Speculation keys off logical scheduler state
   (the deterministic yield sequence), never wall-clock IO duration.
   Same program, same seed, same speculation transcript — the golden
   counters (thunk_evals and friends) stay byte-identical across runs
   and machines, and replay-on-demand survives. A speculative force
   that wall-clock happens to finish early or late changes NOTHING
   observable: values are pure, and the ledger is logical.

## Why this composes with strictness instead of fighting it

Demand-driven forcing has one trigger: a consumer touching the value —
a blocked fiber touches nothing, so laziness alone idles through every
stall. Strictness analysis keeps fine-grained values out of cells
entirely (the serde-beating economics); the demand proof doubles as
the marker for which speculations can never be wasted. The engine is
the third placement option the compiler owns for proven work: inline
(fine grain), hoisted (coarse, stall in the window), pooled (coarse,
stall discovered at runtime).

## Demo target

An IO-bound program whose wall-clock drops because the compiler worked
during the waits — same transcript, same values, earlier finish. The
"that's impossible" demo: run it with KANSO_COUNTERS and show the
speculation ledger identical across runs while wall time varies with
the disk.
