# The work-ahead engine — work the red lights

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

**v1 is not speculative at all** (ratified 2026-07-22): the pool holds
only **proven-demand work** — the strictness analyzer's "certain" set.
Early execution of work that must happen is risk-free by construction;
no evaluation can ever be wasted. Pool ordering is **program order**
(earliest binding whose inputs are ready) — the statement index is
already computed, deterministic, and free; no demand-graph walking.

Demand propagates backward through the dependency graph: an input a
proven computation definitely touches is itself proven, transitively —
so most inputs join the certain set outright and rung 1 covers them.

The true-speculation tiers are severed into an optional v2, admitted
only when rung 1 runs dry, as a priority ladder (ratified 2026-07-22):

1. **Proven, inputs ready** — always first. No policy beyond program
   order.
2. **Gates of proven work** — conditional inputs of certain
   computations (a branch inside the proven dependent decides whether
   the input is touched). A gamble priced by that one branch alone —
   the dependent is definitely coming — and finishing one can unblock
   rung 1, so the pool re-sorts as evaluation proceeds.
3. **Free gambles** — unproven thunks with no proven dependent. Only
   when rungs 1–2 are empty, priced by the cost model.

Certain work vastly outnumbers conditional work in real programs (the
json decoder is 100% certain), so v1 — rung 1 alone — should carry
most of the win.

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
