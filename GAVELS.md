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

## Z. errors without exceptions (open — Z1/Z2; supersedes B and dissolves E if adopted)

Clay, from a run: "do we REALLY need exceptions at all? can't methods just
use whatever kind of error type they like? a default zero-width error
type, users can sub-type it — does it need any special behavior really?"

**The proposal (Z1):** no exception mechanism. Errors are ordinary
zero-width value types declared into a flat family (concrete type →
`error`, one level, no deeper). Exactly ONE kept rule: *an error-family
value passes unchanged through any call that has no arm literally naming
its type* — the compiler writes the pass-through arms (spec §06's
auto-propagation, generalized off the `err` builtin). Corollaries: joins
accumulate, walls gate, top reports with provenance. The guard: error
values NEVER match `_` or unannotated params — handling requires naming
the type, so accidental swallowing is unrepresentable.

```
error cant_halve

fn halve 0
  cant_halve

fn halve n
  n / 2
# inferred: halve returns int | cant_halve

fn describe (cant_halve)
  "can't halve that one"          # deliberate, named — legal

fn log_all x
  print "{x}"                     # x never binds an error value:
                                  # a cant_halve passes THROUGH log_all
```

**Committee findings (Avdi / Hickey / SPJ / Pike, all four: the shape is
right):**

- Unanimous: the pass-through rule + wildcard exclusion IS the
  raise-for-unhandled doctrine as dispatch, Go's errors-are-values with
  the `if err != nil` industrialized, and Either-with-ambient-bind
  without the monad tax. No hidden unwinding returns.
- **Loophole to close:** an arm naming the base `error` type itself is
  `rescue StandardError` reincarnated — forbid arms on the bare family
  type (or confine them to the outermost boundary).
- **Keep the family flat.** Subtype hierarchies + multiple dispatch =
  CLOS/Julia ambiguity hell (SPJ). Avdi's counter-want (a small standard
  taxonomy — user/logic/transient — for coarse boundary handling) is the
  live tension; flat won the room.
- **Cleanup — evaluated against history (Clay: the record tells us
  WHETHER it's useful, not that it is):** the release-on-every-exit
  problem is real and not an exception artifact (Go needed `defer` with
  no exceptions; C needed `goto cleanup`). But every language that
  needed it EXPOSES HANDLES. Kanso can refuse to create the problem:
  locks unrepresentable (no shared mutable state), transactions are
  descriptions, connections are the pending `serve` design, file IO is
  executor-owned. Verdict: `defer` cleans up after a design decision
  kanso hasn't made. Bracketed descriptions (`with_file path (f -> ...)`,
  executor-owned release on every exit incl. pass-through) are the
  CONTINGENCY if a user-visible resource lifetime ever proves
  unavoidable; the bet is none will.
- **Context — evaluated against history:** Go's pre-1.13 pain came from
  poor births (bare strings) plus no provenance; exceptions never had it
  because the stack trace WAS the context; `%w` patches the lack of
  both. Kanso has both: typed fields at birth, provenance trail in
  flight. The remaining legitimate use — semantic context the birth site
  can't know ("while processing user 42's upload") — is ALREADY
  expressible in Z with no new API: an arm NAMING the error type returns
  its own richer error with the inner as a field (Avdi's
  wrap-on-re-raise, spelled as dispatch). Only touching an error you
  didn't name is banned.
- **Exceptions themselves, against the record:** every language of the
  last fifteen years moved away (Go values, Rust Result, Swift typed
  throws); the convergent endpoint is typed error values with ambient
  propagation — Z1 nearly verbatim. Kanso starts at the endpoint.
- **The checked-exceptions disease stays dead only because unions are
  INFERRED** — a leaf adding an error type edits nobody's source. SPJ's
  condition: higher-order fns must stay polymorphic over their argument's
  error family. Pike's condition: when public module boundaries exist,
  allow PINNING a declared union there (semver signal), inferred
  everywhere internal.
- **Dissent worth keeping:** the committee's own recommendation asked for
  three subsystems before shipping; the recorded counter-argument is
  Hickey's — ship the one rule now, design cleanup/context from observed
  pain, not speculation.

**Z2 — status quo:** keep the blessed `err` builtin as-is.

**My rec: Z1, shipped lean per the dissent** — one rule + flat family +
wildcard exclusion + no bare-`error` arms; birth-site fields already
exist (types have fields); provenance already exists on `err`. If
adopted: B's letter is superseded (its spirit survives as the wildcard
guard: no handling *by accident*), and E dissolves (arms naming your own
error types are ordinary dispatch).

---

## AA. newtypes and dispatch across a family (open — AA1/AA2/AA3)

Clay: "go lets a subtype of string be passed to a function that accepts
string [note: Go actually requires the explicit conversion for defined
types]. should we be able to cast a subtype of string to string so
dispatch matches it? should it match automatically to the first ancestor
type with a matching arm?"

```
type user_id string          # a newtype: distinct on purpose

fn greet s:string
  print "hello {s}"
```

**AA1 — ancestor-walking dispatch:** `greet (user_id "mika")` matches the
`string` arm because user_id's ancestor matches. REJECTED by every prior
finding: action at a distance (adding a `user_id` arm anywhere silently
changes which arm fires), the CLOS/Julia multi-param ambiguity explosion
(SPJ, gavel Z), and it destroys exact-type preservation — the typeset
novelty Clay named as a favorite. The value arrives wearing a costume.

**AA2 — explicit cast only:** `greet (string uid)` — Go-strict. A
newtype exists to NOT be its underlying type; crossing back is a visible
decision. Casts get syntax you can see.

**AA3 — callee-declared acceptance via typesets (+ AA2's cast as escape
hatch):** the arm says what it takes; the value never changes type:

```
fn label t:text              # text = string | user_id — the CALLEE
  print "[{t}]"              # opted in; t keeps its EXACT type inside
```

Same construct answers Avdi's coarse error handling (gavel Z):
`transient = timeout | connection_reset` is a typeset, not a supertype —
no hierarchy anywhere, acceptance is visible at the arm.

**My rec: AA3** (typeset acceptance as the idiom, explicit cast as the
deliberate escape hatch, no ancestor-walking ever).

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
