# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err rule — GAVELED 2026-08-15: the three-combinator model

The enforcement question dissolved into structure (Clay, 2026-08-15
dialog). Failure handling is three combinators over the effect's two
channels — one callback, one concern each:

- **`bind effect ok`** — maps the success channel; a failure passes
  through untouched, Haskell's `>>=`. Infectiousness is bind with the
  function as its callback. Nothing to check.
- **`annotate effect fail`** (name open) — maps failure to failure: the
  callback reads the reason and returns context; the result re-wraps as
  err with the original as cause (`wrap_err` in flow position,
  `withExceptT` / Rust's `map_err`). Universal — annotating your own
  failure never changes channel. Nothing to check.
- **`rescue effect fail`** — maps failure to success (`catchError`). The
  one licensed door: legal only where the failure is foreign, checked at
  rescue call sites — every reason type the callback's arms name must
  originate in a hako foreign to the caller's; the default arm re-raises
  for free, since an err returned into the success wrap is still an err.
  `src/provenance.rs` already computes the reachable-raiser set; the
  check relocates from every err arm to rescue sites alone.

The July settlements survive unchanged: trapping is naming (pub reason
types are the catchable surface), subtype matching with the
specificity rung, unstoppable = no pub ancestor, `wrap_err` carries the
original, re-export follows the door while trapping follows the leaf,
workspace siblings foreign-with-advisory, vendored code owned. Sharpened
2026-08-15: **the universe boundary is the hako** — local subdirectory
modules are one universe however the author lays them out; splitting
your code buys nothing. What dissolves: the one-bind clause "an err arm
must answer an err" stops being a rule anyone obeys — under
bind/annotate the machinery re-wraps, so it holds by construction.

Precedent (recorded so the novelty stays visible): the trio is the
bifunctor algebra of Either — Haskell `>>=`/`withExceptT`/`catchError`,
Rust `map`/`map_err`/`or_else` with anyhow's `.context` as the annotate
idiom, ZIO `flatMap`/`mapError`/`catchSome`, Wlaschin's railway-oriented
programming. The foreign-only license on rescue has no precedent; it is
kanso's thesis, and everything else is the field's consensus arranged
around it.

### Riders still open under this gavel

- **Spelling**: names and syntax for annotate and rescue — combinator
  call vs marked arm on a chain — and whether the existing chain err-arm
  syntax is annotate's surface (the chain's value arm and err arm are
  bind's and annotate's callbacks already, spelled as dispatch arms).
- **Construction enforcement** (reason building module-private): no
  longer needed for soundness since provenance is computed, still stated
  by the doctrine, still unenforced.
- **The test surface**: a package cannot produce a value about its own
  failure, and an assertion is a value. Either the `*_test.kso`
  file-scope exemption (shipped, crude) or a toolchain assertion surface
  (cleaner, wants shaping — design/testing.md).
- **ch08 pedagogy**: positions.kso dispatches on its own parse_failure —
  unfixable within one program since local modules share a universe. The
  chapter restructures around std/json as the foreign library, or the
  book narrows its failure story to the package boundary.
- **Migration**: the arm-based advisory and the fleet's two violations
  (std/json's failure_position/failure_reason, retired by 1b, plus kq's
  vendored copies) move onto the combinator surface;
  design/err-migration.md updates to this shape.
- **Smaller spellings** carried from the July entry: the dot-prefix
  canon for local imports, the subtype declaration spelling, the
  into-subtype spelling.

## 3. Dependency to_string arms

Should a dependency module's `to_string` arms join the importing
program's render group? Today only the root module's arms merge; a
dep's stay qualified and never join (recorded in
`design/render-plan.md`). A library exporting a money type today cannot
also ship its rendering. Surface question: whether rendering is part of
a module's exportable surface or a root privilege.

## 5. Open dispatch groups

The coherence/orphan rule for user-extensible groups — who may add arms
to a group they did not define, and what keeps two libraries' arms from
colliding. Unlocks user-defined sequences joining std's enumerable
machinery. The annotated-arm split was judged the five-minute half; the
orphan rule is the real question.

## 6. Tail-call promise wording

How the language documents its tail-call guarantee (what is promised
semantics vs what is an optimization), now sharper-edged because TRMC
makes some non-tail shapes loop as well, and the interpreter refuses
unlicensed recursion at 10,000 frames while native's ceiling is the OS
stack. The book currently describes behavior without promising it.

## 8. Exhaustive ratification — RULED 2026-08-15: per-call coverage, no annotation

Exhaustiveness is not a group property anyone declares; it is a
question the closed-world compiler answers at each call site: does
every value in the call's inferred value set have an unambiguously
matching arm? Unambiguous is the ratified tie rule (first-place ties
refuse). A provable gap is a compile diagnostic; a call inference
cannot prove keeps the runtime no-match err, honestly, since a decoded
value cannot be covered at compile time without the annotations the
no-needless-annotations gavel declines. The entanglement with the
orphan rule (5) evaporates: closedness is not declared either.

## 15. Should `>>` defer its right side

`a . f` hands the continuation over as a closure, so nothing past the
current link exists until the link runs. `a >> b` takes `b` as an
already evaluated description, so building the first link requires
evaluating the second, which requires the third: the whole chain is
constructed before any of it runs, and the construction is what
exhausts the stack. A loop written with `>>` dies where the same loop
written with `.` does not.

INTERIM SHIPPED: all three engines now name the operator in the
diagnostic rather than blaming the recursion, since a function calling
itself in the right operand of `>>` is visible in the source and the
runtime cannot work the cause out at the moment it reports. That makes
the failure legible; it does not make the loop run.

## 16. Should block-born widen to a dataflow property

Today `block-born` is a syntactic property, which is why in-place graph
algorithms inside build blocks stay out of reach (ledger 4.4). Widening
it to a dataflow property is the unblock. It is a language-surface
question rather than an allocator one, which is why it sits here rather
than being settled by measurement.

## 17. Printing a lazy sequence — GAVELED 2026-08-15: the wire is the demand

Print forces, fully. In a fully lazy language demand originates at the
boundary, and the wire is the final boundary: a value reaching output
IS its demand, so showing adapter machinery (or `<thunk>`) is the
demand chain stopping one step short. An infinite sequence therefore
never finishes printing — honest, and already the spec's answer for
`last` on an infinite source. Bounded viewing is spelled `take n`.
Build note: prefer streaming emission (elements print as they force,
Haskell-style), so an accidental `print naturals` is visibly growing
and interruptible rather than a silent hang.

## 18. `pure` as a record type (Clay's proposal)

Clay proposed `type answer / value` plus one executor arm per engine.
Recorded when it was raised and never taken further. It touches how a
pure value and a description relate, which is the same seam as 19
below, so the two probably want deciding together.

## 19. Should io infect — auto-lifting operators over descriptions

Whether an operator applied to a description should lift automatically,
so a description behaves like the value it will produce. The pull is
ergonomic; the cost is that the type of an expression stops being
readable from the expression. Raised, not argued.

## Also open, not blocking any current work

- **TRMC v2**: license operands by inferred set (any provably-int
  expression, not just literals — covers `n * fact (n - 1)`). Needs
  `Inference` threaded out of `check::check` rather than recomputed, or
  the compile-cost golden pays a phantom infer run.
- **`--explain-copies`**: the *where* half of the observability item —
  a diagnostic naming the source site of each evacuation copy. Needs
  span plumbing through the carry machinery; the CLI surface deserves a
  shape ruling before building.
- **The interpreter's 10,000-frame guard**: constant chosen to hold
  under debug builds on the 1 GB thread. If you want it higher, or an
  env override, say the word.
- **Survivor cap 4× block threshold (2 MB)** in the cohort's
  survivor-ratio guard: the multiplier is a judgment call; the principle
  (the dance's transient must stay at threshold scale) is recorded in
  the log.
- **An `os` package** (Clay): what moves out of `io` — `MkdirAll` was
  the example. A surface question, cheap once the shape is agreed.
- **Sequencing more than two binds prettily** (Clay's
  `multiplyTwoRandoms`): today the third bind nests. Wants a form, not
  a mechanism.
- **Dot chains route around accessor privacy** (Demeter): a chain can
  reach a field the owning module would not expose directly. Low
  priority, and a real hole in the privacy story.

## 20b. Pending cells at output — GAVELED 2026-08-15: render forces

Same principle as 17: output demands everything it emits. Render (the
value-to-text machinery: interpolation, print, err reasons, harness
output) forces any thunk it meets, with the cycle guard extended from
records to thunk cells so a self-naming knot — legal under gavel 20 —
terminates as `<cycle>`. `a_constant_that_holds_itself` re-pins from
`[<thunk>]` to `[<cycle>]`; the per-site alternative (teaching
demanding sites one at a time: #889, #890, #892) is retired as having
no end condition. Build note: the oracle's `render_seen` is a free
function with no interpreter handle and needs restructuring to force —
whoever builds it starts there (eval.rs:3491, runtime.c:3185, the
runtime.c:3363 comment's every-cycle-passes-through-a-record claim is
the thing being corrected).
