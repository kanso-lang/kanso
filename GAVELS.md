# the gavel stack

Ephemeral decision doc. Ruled items live only in the ledger; open items keep
thorough examples. Rule by letter: "T3, W2, B yes, C yield, …"

---

## ruled so far (the ledger)

| item | ruling | status |
| --- | --- | --- |
| A1 bare effect lines = unordered group, failures accumulate | ✅ "parallelism should be the default; you go out of your way to NOT have it" | **implemented** |
| A2 same-resource auto-ordering | ❌ "parallel lines are EXACTLY as if in two threads — the resource does not matter." Resource-dependent order would add inconsistency, more to memorize, exceptions to rules. One rule, no asterisks. | — |
| A3 lone `>>` line = wall; `>> expr` = fused singleton (identical meaning) | ✅ | **implemented** |
| A4 `<< labels` for DAG edges | ⏸ parked — walls suffice; true DAGs pay a false edge or decompose into functions; revive on real demand | — |
| A5 `&` retires from the surface (bitwise future orthogonal) | ✅ | **implemented** (teaching error; Join node stays internal) |
| R5 bindings precede the effect chain (the desugar hoists them; interleaving lied) | ✅ correctness, auto-adopted | **implemented** |
| T indent trap | ✅ resolved by killing `>>`-continuation lines entirely — `>>` never splices; `.` continuations unchanged | **implemented** |
| W fused walls | ✅ **fused closes**: `>> step` is a complete sequential step (Clay's space-saver for singleton stages); a bare line can't silently join it (teaching error); multi-member groups use the lone wall | **implemented** |
| X width canon | ✅ "the compiler fails if you do a needless multi-liner version or a one-liner version that's actually multiple lines." Lines cap at 80. Fits-on-one-line ⇒ MUST be one line (needless `.` continuation, needless multi-line chain, lone wall before a one-step stage: all errors). Doesn't-fit ⇒ one step per line, no partial chaining. Width alone decides. | **implemented** |

The live semantics, in one example — **bare lines are two threads; no order
exists unless you wrote one**:

```
main =
  print "steeping the sencha"    # ┐ unordered — like two threads; any
  print "warming the cups"       # ┘ interleaving is legal output
  >> print "serving"             # the wall: only runs after both settle

main =
  simmer_the_dashi_for_an_hour   # a fully sequential chain: each fused
  >> strain_out_the_katsuobushi  # `>> step` is one CLOSED sequential step
  >> season_and_serve_the_broth  # (a bare line can't silently join it)
```

(X: this block form is only legal because the inline chain would blow the
80 column cap — `main = a >> b >> c` that fits MUST be written inline.)

(Today's executor happens to run in program order — that's scheduling, not
semantics. Corpus/goldens may only pin programs whose output order is forced
by `>>`.)

---

## B. gather at consumption (open — yes/no + surface)

Does consuming description **labels as value arguments** mean "execute,
gather, reify the outcomes, dispatch"?

```
main =
  r = reserve_tatami
  w = order_wagashi
  confirmation r w        # <- the gavel

fn confirmation booking:tatami_success order:wagashi_success
  print "confirmed: see you at {time booking}"

fn confirmation booking order
  parts = [(failure_message booking) (failure_message order)]
  print (parts . filter (s -> 0 < length s) . join ", ")
```

Outcomes arrive **reified** — the success value, or an inert `failure`
record — never a live err, so the railway can't skip your arms and the
catch-all can say "gomen." A gather is a mini-boundary.

**Alternative surface** — an explicit word instead of implicit
execution-at-argument:

```
gather [reserve_tatami order_wagashi] . confirmation
```

Same primitive powers value-collecting fan-out (three random dice).

**My rec: adopt, implicit form.**

---

## C. `pure` / `yield` — the unit primitive (open — C1/C2 + name)

Today the fold-yield idiom dies:

```
fn step _ _ _ _ out none
  out . (_ -> store)      # BROKEN: out ends in a print, print yields none,
                          # the bind railway-skips the lambda
```

**C1 — a named primitive:**

```
fn step _ _ _ _ out none
  out >> yield store      # effects run, then the description yields store
```

**C2 — a plain value on `>>`'s right auto-lifts (Metz):**

```
fn step _ _ _ _ out none
  out >> store
```

Either unlocks the one-fold redux:

```
main = play 0 moves 1 logger (print "opens at 0") . resume

fn resume monday
  play monday tuesday_moves 1 logger (print "tuesday opens at {monday}")
```

**My rec: C1, named `yield`.**

---

## D. what does `print` yield? (open — D1/D2)

```
main = print "a" . (x -> print "got {x}")
# D1 (today, none):  prints "a", then NOTHING — none skips the continuation
# D2 (done marker):  prints "a", then "got done"
```

**D1** keeps `none` — but a *succeeded* effect yielding a failure-class
value is a footgun (the silent skip surprises everyone once). **D2** adds a
`done` zero-field marker: succeeded effects yield `done`; `none` stays
purely absence/failure.

**My rec: D2.**

---

## E. the err-arm restriction (open — yes/no)

*A live-err arm may only return a description or another failure — never an
ordinary value.*

```
fn describe (err reason)
  "gomen: {reason}"            # ILLEGAL under E: resurrects the happy track

fn close day (err reason)
  print "{day} failed"          # legal: handling = describing what's next

fn retag (err reason)
  err (wrapped reason)          # legal: failure to failure

fn failure_message (failure f)
  "problem: {reason f}"         # legal always: f is REIFIED data (from a
                                # gather/boundary), not a live err
```

**My rec: adopt.**

---

## F. labeled nameless patterns + type-constrained wildcard (open — yes/no)

*A pattern that binds no name must wear one.*

```
fn step _ _ _ _ out none          # today: what is none HERE?
fn step _ _ _ _ out action:none   # with F: the action slot, matching none

fn _value_for 34 cs p             # today (json byte table)
fn _value_for byte:34 cs p        # with F

fn failure_message _:tatami_success   # the type-constrained wildcard:
  ""                                  # match the type, bind nothing
```

Patterns that bind names (`n`, `(err reason)`, `(deposit n)`) stay bare.
Group headers stay parked behind this (your naming-freedom argument).

**My rec: adopt.**

---

## G. eta-reduction as canon (open — yes/no)

```
slots . filter (s -> failed s) . map (f -> message f)   # ILLEGAL under G
slots . filter failed . map message                      # the one rendering
```

A forwarding lambda is derivable clutter (annotation-doctrine logic). Bare
names as function values already compile. Adopting sweeps the corpus
(`fanout.kso`'s `map cities (c -> fetch_quote c)`).

**My rec: adopt.**

---

## H. the `random` effect (open — H1/H2)

`random n` = a description yielding 1..n, executor-owned. The gavel is the
determinism policy:

- **H1**: real entropy by default; `KANSO_SEED=42` pins it (goldens, the
  lattice, reproducible bug reports).
- **H2**: fixed seed by default; entropy is the opt-in.

```
main =
  rolls = gather [(random 6) (random 6) (random 6)]   # needs B
  print (rolls . map render_die . join " ")
```

**My rec: H1.**

---

## parked (on the record, no action)

- **`<<` labels (A4)** — walls cover staircases; revive on real DAG demand.
- **dot-absorbs-`>>`** — argued no: erases the visible then/bind split.
- **postfix index on `)`** — `(sort xs)[1]` stays illegal; bind-then-index.
- **group headers** — behind F.
- **`;` inline separator** — the borrow if inline groups are ever demanded.
- **`&` as bitwise** — orthogonal, someday.
- **`serve` / processes** — the executor-loop primitive; next design
  campaign (three investigations already terminate there).
