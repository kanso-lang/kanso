# the gavel stack

Ephemeral decision doc — each ruling deletes its section here and lands in the
spec / compiler page / corpus. Every "proposed" example desugars to machinery
that is already merged and byte-identical (the Join/Seq nodes, gather excepted).

Rule by letter: "A yes, C yield, D done, …"

---

## A. the concurrency surface (rule as one unit)

### A1 — bare effect lines become legal, and they're an unordered set ✅ GAVELED (2026-07-15: "of course — parallelism should be the default; you go out of your way to NOT have it")

**Today** (compile error):

```
main =
  print "steeping the sencha"    # error[unused]: expression result unused
  print "warming the cups"
```

**Proposed** — legal; the lines are a group with *no order between them*
(parallel license), and their failures accumulate:

```
main =
  steep_the_sencha
  warm_the_cups
```

If both fail, you get both reasons — `["kettle is cold" "no cups left"]` —
not whichever lost a race. (Accumulation is forced by determinism: "first
error wins" is nondeterministic under real parallelism.)

### A2 — effects on the same resource keep program order ❌ REJECTED (2026-07-15: "no — parallel prints have no order guarantee; if you care about order, indicate sequential, duh")

The rule that makes A1 sane. Two prints share stdout, so they stay in
program order automatically — beginners write top-to-bottom programs and get
top-to-bottom output, **never seeing a mark**, and every existing golden
survives:

```
main =
  print "one"      # same resource (stdout):
  print "two"      # program order preserved, deterministic
  print "three"
```

Different resources genuinely float:

```
main =
  write_file "menu.txt" menu       # different files:
  write_file "prices.txt" prices   # free to run concurrently
```

### A3 — a lone `>>` line is a wall ✅ GAVELED (with Clay's fused-prefix reduction: `>> expr` ≡ wall + first member of the next group, so `>> a / b` ≡ `>>`/`a`/`b`; canonical-rendering rules under committee review with an example battery)

Sequences the group above and the group below. Staircases need nothing else:

```
main =
  steep_the_sencha     # ┐ group 1: unordered
  warm_the_cups        # ┘
  >>
  unlock_the_door      # ┐ group 2: unordered, after group 1 settles
  set_the_table        # ┘
  >>
  serve_tea            # after group 2
```

A failing group **gates** the wall: the next group never runs; the failure
(accumulated) rides the railway out.

Canonical-form rider: a wall between *single* statements renders inline —
`a >> b` — so the block form only ever appears for real groups.

### A4 — `<< labels` for true DAG edges ⏸ PARKED (2026-07-15: "totally unneeded now that we have this new >> syntax" — walls suffice; the DAG boundary stays on record: a true DAG pays a false edge or decomposes into named functions; revive on real demand, per Beck's razor)

Walls express staircases only. When the graph has cross-references, label the
lines (ordinary bindings) and put the edge **on the dependent line** — your
founding requirement, "only the thing that needs the wait changes":

```
main =
  a = brew_pot
  b = rinse_cups
  c = arrange_tray << a b     # needs exactly a and b
  d = unlock_door
  e = light_lanterns
  f = greet_guests << d e     # needs exactly d and e — NOT c
  serve << c f                # needs exactly c and f
```

After `<<` only names are legal (no calls — a restricted position, like
patterns), so `<< a b` can never parse as "apply a to b". A `<<` edge moves
**time only, no values**; needing a value is data flow, which orders itself.

*Consequence:* this settles **binding-as-bind against Metz's proposal** — a
binding on an effect line is a *label*, not run-and-bind-the-yield. The
value-crossing bind stays visible as `.`.

### A5 — `&` retires from the surface ✅ GAVELED (2026-07-15: "& retires from service; maybe ends up doing bitwise stuff but that's orthogonal") — IMPLEMENTED: user-facing & is a teaching error; Join node stays internal

Shipped this week, superseded by A1–A3 (adjacency already says "and").
The Join node stays as what groups *compile to*; the operator leaves the
spec and docs. Inline grouping, if ever demanded, would be `;`
(Haskell's own same-line answer) — parked, YAGNI.

```
steep & warm >> print "serving"    # dies (was legal for ~3 hours)

steep                              # the one rendering
warm
>>
print "serving"
```

**My rec: adopt A whole.**

---

## B. gather at consumption

The third failure posture (besides wall-gating and letting it rise):
*respond to the outcomes*. Question: does consuming description **labels as
value arguments** mean "execute, gather, reify, dispatch"?

```
main =
  r = reserve_tatami
  w = order_wagashi
  confirmation r w        # <- the gavel: this means "run both, hand me
                          #    the outcomes as plain data, dispatch"

fn confirmation booking:tatami_success order:wagashi_success
  print "confirmed: see you at {time booking}"

fn confirmation booking order
  parts = [(failure_message booking) (failure_message order)]
  print (parts . filter (s -> 0 < length s) . join ", ")
```

Key mechanics: outcomes arrive **reified** — the success value, or an inert
`failure` record (`reason f` etc.) — never a live err, so the railway can't
skip your arms and the catch-all can actually say "gomen." A gather is a
mini-boundary.

**Alternative surface** — explicit word, no implicit execution-at-argument:

```
gather [reserve_tatami order_wagashi] . confirmation
```

More visible, one more name; the implicit form reads better and the label
consumption is already unambiguous (a desc-label used as a value has no other
possible meaning).

Same primitive powers value-collecting fan-out (your three-random-numbers
example).

**My rec: adopt, implicit form.**

---

## C. `pure` / `yield` — the unit primitive

The missing piece the fold-yield idiom needs. Today the only spelling dies:

```
fn step _ _ _ _ out none
  out . (_ -> store)      # BROKEN: out ends in a print, print yields none,
                          # the bind railway-skips the lambda -> yields none
```

**Option C1 — a named primitive** (`yield x` = a description that just
yields x):

```
fn step _ _ _ _ out none
  out >> yield store      # run the effects, then the description's value
                          # is the closing balance
```

**Option C2 — Metz's generalization**: a plain value on `>>`'s right is
auto-lifted:

```
fn step _ _ _ _ out none
  out >> store
```

C2 is one token lighter; C1 keeps "everything right of `>>` is a
description" simple and greppable. Either unlocks the one-fold redux
(no more computing the fold twice):

```
main = play 0 moves 1 logger (print "opens at 0") . resume

fn resume monday
  play monday tuesday_moves 1 logger (print "tuesday opens at {monday}")
```

**My rec: C1, named `yield`.**

---

## D. what does `print` yield?

Today: `none` — which is why bind-after-print skips silently:

```
main = print "a" . (x -> print "got {x}")
# today:      prints "a", then NOTHING (none skips the continuation)
# with done:  prints "a", then "got done"
```

**Option D1 — keep `none`**: consistent with "no meaningful value," but a
*succeeded* effect yielding a failure-class value is a footgun (the skip
above surprises everyone once).

**Option D2 — a `done` marker** (zero-field type, ordinary data): effects
that succeed yield `done`; binds after them run; `none` stays purely a
failure/absence.

**My rec: D2 (`done`).** Interacts with C: `out >> yield store` works under
either, but D2 removes the whole class of silent skips.

---

## E. the err-arm restriction (Avdi)

*A live-err arm may only return a description or another failure — never an
ordinary value.* Makes resurrect-the-happy-track unwritable:

```
fn describe (err reason)
  "gomen: {reason}"            # ILLEGAL under E: err in, plain string out —
                               # the failure vanishes into the happy track

fn close day (err reason)
  print "{day} failed"          # legal: handling = describing what happens
                                # next, not undoing the failure

fn retag (err reason)
  err (wrapped reason)          # legal: failure to failure (context added)
```

Reified failure records (from B, or a boundary) are exempt — they're inert
data, formatting them is fine:

```
fn failure_message (failure f)
  "problem: {reason f}"         # legal: f is data, not a live err
```

**My rec: adopt.** (The old redux example violated this; it's already gone.)

---

## F. labeled nameless patterns (5a) + the type-constrained wildcard

*A pattern that binds no name must wear one.* Fixes the wart you found:

```
fn step _ _ _ _ out none          # today: what is none HERE? unreadable
fn step _ _ _ _ out action:none   # with F: the action slot, matching none

fn _value_for 34 cs p             # today (json byte table)
fn _value_for byte:34 cs p        # with F

fn menu_price item:"dango"        # string literals too
```

Patterns that already bind names stay untouched: `n`, `(err reason)`,
`(deposit n)` are self-documenting.

Plus the **type-constrained wildcard** your failure_message sketch needed
(match the type, bind nothing, no nothing-wasted violation):

```
fn failure_message _:tatami_success
  ""
```

Group headers (5b) stay parked behind this — your naming-freedom argument
("a string vs an int in that slot is a different *thing*") killed the
shared-header version.

**My rec: adopt F; keep 5b parked.**

---

## G. eta-reduction as canon

A lambda that only forwards to a named function is derivable clutter —
formatting error, like every other redundancy:

```
slots . filter (s -> failed s) . map (f -> message f)   # ILLEGAL under G
slots . filter failed . map message                      # the one rendering
```

(Bare names as function values already compile — shipped this week.)
Adopting G sweeps the corpus: `fanout.kso`'s `map cities (c -> fetch_quote c)`
becomes `map cities fetch_quote`.

**My rec: adopt.**

---

## H. the `random` effect

Needed for the IO fan-out example ("pass 3 random numbers then be done").
`random n` = a description yielding 1..n, executor-owned. The gavel is the
**determinism policy**:

- **H1**: real entropy by default; `KANSO_SEED=42` pins it (goldens, the
  differential lattice, reproducible bug reports).
- **H2**: fixed seed by default; entropy is the opt-in.

```
main =
  rolls = gather [(random 6) (random 6) (random 6)]   # needs B
  print (rolls . map render_die . join " ")
```

**My rec: H1** — "random is random" is least surprising; tests set the seed.

---

## parked (on the record, no action)

- **dot-absorbs-`>>`** — argued no: erases the visible pure→then→bind
  stratification; `>>` and `.` have different failure semantics.
- **postfix index on `)`** — `(sort xs)[1]` stays illegal; bind-then-index.
- **group headers (5b)** — behind F.
- **`;` inline separator** — the honest borrow if inline groups are ever
  demanded; YAGNI today.
- **`serve` / processes** — the executor-loop primitive; next design
  campaign (three investigations already terminate there).
