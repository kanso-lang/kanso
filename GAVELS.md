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
| T indent trap | ✅ superseded by X: `>>` continuation lines are BACK (indent+2, mirroring `.`) as the wrap for over-wide statements. The trap is defanged structurally: a mis-indented wrap either hits a loud error (headers aren't wrappable; walls need groups) or lands on the same execution order | **implemented** |
| W fused walls | ✅ **fused closes**: `>> step` is a complete sequential step (Clay's space-saver for singleton stages); a bare line can't silently join it (teaching error); multi-member groups use the lone wall | **implemented** |
| B gather / failure reification | ❌ "an err is a true exception, so you can't handle it. it should short circuit except where it can't because of parallelism. if you want something that can be handled, don't use an exception." No gather, no freeze point, no reified-failure records in user code. An err short-circuits to the top; a parallel join is the sole concession (every member still runs; errs accumulate into one, which then short-circuits). Handleable outcomes are ordinary VALUES — dispatch already covers them, zero new machinery | — |
| X statement canon + width-as-rendering | ✅ v2 after Clay's correction: "width can break a line down into wrapped lines, but it doesn't turn one statement into multiple statements." STRUCTURE (width-free): a lone wall exists only for multi-member groups; a one-step stage ALWAYS fuses (`>> step`); fused closes; so `[a b] >> [c] >> d` renders `>> foo_c` / `>> foo_d`, never a lone wall over `foo_c`. WIDTH (rendering only): 80-col cap; an over-wide statement wraps onto `.`/`>>` continuation lines at indent+2, one step per line (no partial chaining); a wrap that would fit on one line is an error. Width never merges or splits statements. | **implemented** |

The live semantics, in one example — **bare lines are two threads; no order
exists unless you wrote one**:

```
main =
  print "steeping the sencha"    # ┐ unordered — like two threads; any
  print "warming the cups"       # ┘ interleaving is legal output
  >> print "serving"             # the wall: only runs after both settle

main =
  foo_a                          # a fully sequential chain: each fused
  >> foo_b                       # `>> step` is one CLOSED sequential step
  >> foo_c                       # (a bare line can't silently join it)
```

(X: this stage-statement structure is legal at any width — statements are
the programmer's semantic choice. Width only ever WRAPS one over-wide
statement onto `>>`/`.` continuation lines at indent+2, one step per line.)

(Today's executor happens to run in program order — that's scheduling, not
semantics. Corpus/goldens may only pin programs whose output order is forced
by `>>`.)

---

## B′. the value version of confirmation (consequence of B, no gavel)

B is dead: errs are true exceptions and never reify. The confirmation
pattern needs **no machinery at all** — a fallible-but-expected outcome
is an ordinary value, and dispatch was always the handler:

```
fn reserve_tatami
  ... (tatami_success time) or (tatami_full next_free) ...

main =
  r = reserve_tatami
  w = order_wagashi
  confirmation r w        # values flow; dispatch picks the arm

fn confirmation (tatami_success t) (wagashi_ready w)
  print "confirmed: see you at {t}"

fn confirmation (tatami_full next) _
  print "gomen — the tatami room opens next at {next}"
```

If the venue burns down mid-request, THAT is the err: it short-circuits
past all of this to the top, as it should.

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

## E. err arms in user code (open — E1/E2; B's ruling raised the stakes)

The original E was a restriction on what an err arm may return. B's
ruling ("you can't handle it") suggests something stronger:

**E1 — no `(err ...)` patterns in user code at all.** An err is opaque
in flight; only the runtime's outermost handler sees it. Dispatching on
err IS handling it.

**E2 — err arms exist but only pass it along** (return a description or
another err — never resurrect a value; the original E).

What's in the corpus today (would need sweeping under E1):

```
fn describe (err reason)
  "gomen: {reason}"              # examples/errors.kso + playground railway
                                 # demo — resurrection, illegal under BOTH

fn _string_ok p (err _)          # lib/json internals dispatch on their own
fn must (err reason)             # errs as railway plumbing — under E1 these
                                 # become ordinary values (Malformed-style
                                 # determinations), err reserved for defects
```

The json sweep is consistent with the settled doctrine ("a parser that
decides bytes are invalid *succeeded* — that's a value"), so E1 mostly
forces the lib to say what it already means.

**My rec: E1.**

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
  a = random 6            # bindings already fan out: no shared data, so
  b = random 6            # the compiler is free to run these in parallel
  c = random 6
  print ([a b c] . map render_die . join " ")
```

(B's death costs nothing here — value fan-out was always just bindings
plus data flow.)

**My rec: H1.**

---

## Y. the sequencing surface: `>>` vs a word (open — Y1/Y2/Y3)

Clay: "it's possible we want a different symbol, or to use a word like
`then` perhaps? i'm open minded." The surface appears in four positions:

```
# Y1 — keep `>>` (today)
main =
  print "steeping the sencha"
  print "warming the cups"
  >> print "serving"                  # fused stage

main = reserve_tatami >> order_wagashi >> confirm   # inline

main = apply (s -> print "s = {s}") 7
  >> apply (_ -> print "ignored, ran anyway") 9     # wrap leader

# Y2 — the word `then`
main =
  print "steeping the sencha"
  print "warming the cups"
  then print "serving"

main = reserve_tatami then order_wagashi then confirm

# Y3 — another symbol (|>, ~>, =>) — same shapes as Y1
```

Committee findings (Metz / Hickey / Pike-school pragmatist, unanimous for
Y1 but for three different reasons):

- **Metz**: a construct must hold up in its WORST position, and `then`'s
  worst is the lone wall line — a bare `then` on its own line reads like a
  conditional with its `if` clipped off. `>>` is the only surface that
  stays legible standing completely alone. `|>` is worse: it borrows the
  pipe shape, which reads as "the value flows through" — precisely false
  here.
- **Hickey**: `>>` is the true name of the operation — bind minus the
  value. `.` and `>>` are visible siblings: narrow mark while the value
  still matters, wide mark the moment it stops. A word severs that kinship
  AND braids in baggage (`then` already names branching). `~>`/`=>` would
  be arbitrary glyphs by comparison.
- **Pragmatist**: the one honest risk nobody else named — `>>` is bitwise
  right-shift in every C-family language the target audience knows. But a
  bare-word infix operator (`a then b`) is a bigger grammar surprise than
  a reused punctuation shape, and position disambiguates in practice (no
  numeric operands in sight). Flag the collision; revisit only if kanso
  ever grows bitwise operators.

Strongest argument against keeping it, stated honestly: kanso spends a
glyph every Go/C/JS/Rust dev already knows on an unrelated meaning — a
"wait, what does this mean here" moment in a newcomer's first five
minutes, and internal elegance with `.` doesn't fix that.

**My rec: Y1, keep `>>` in all four positions.** The committee is
advisory-not-oracle, but three independent lenses landing on the same
answer for non-overlapping reasons is the strong version of the signal.

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
