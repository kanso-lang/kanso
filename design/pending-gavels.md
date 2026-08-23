# Pending gavels

Every decision waiting on Clay, in one place. Each entry says what the
question is, where it came from, what the interim state is, and what it
unblocks. Nothing here is urgent in the sense of broken; everything here
is a fork the project has deliberately not taken without a ruling.

## 1. The err rule — GAVELED 2026-08-15: the three-combinator model
(SURFACE SUPERSEDED by 24, 2026-08-17: the triad sinks to elaboration
internals; the license moves to dispatch arms; the semantics survives
as the elaborator's spec.)

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
- **The test surface — RESOLVED 2026-08-17 (Clay's foreign-assert
  insight).** Assertions are ordinary foreign rescue: the assert hako's
  arms receive the test file's err legitimately (the raiser is foreign
  to them), convert it to a report, done — no exemption, no toolchain
  surface, no new mechanism. The `*_test.kso` file-scope exemption
  RETIRES. And no advisory on the general round-trip (a hako passing
  its own err to a foreign converter and receiving a value back):
  nothing was rescued in the direct sense — "you ensured it would
  bubble up to the caller, and it did. if the caller wants to pass it
  back to you, so be it." The rule constrains your arms, not
  downstream dataflow.
- **ch08 pedagogy — RESOLVED 2026-08-17**: the chapter restructures
  around std/json as the foreign library, with "your own failures only
  bubble" as its thesis — the licensed case taught through a concrete
  dependency, which is the same lesson the narrowing option would have
  stated abstractly. The ch08 suite moves with the projection
  migration.
- **Migration** (unblocked 2026-08-17, mechanical): the arm-based
  advisory restructures to gavel 24's arm-site check, and the fleet's
  violations retire — std/json's failure_position/failure_reason plus
  kq's vendored copies, deletable because **1b is GAVELED (Clay,
  2026-08-14**, per-field pub; named reads and keyed patterns cross
  hakos — see the compiler log's "a sitting of gavels" entry): foreign
  clients destructure `(err (parse_failure p _))` themselves. Scope:
  the projections, examples/json_failure_door.kso, and the ch08 suite
  in the restructured-around-std/json shape.
- **Smaller spellings — GAVELED 2026-08-19** (Clay, following the
  recommendations): the dot-prefix canon is adopted — a local import
  starts with `./` or `../`, and any bare path is a hako name, so an
  import's universe is readable from its spelling (migration: bare
  local imports gain the prefix). A subtype declares as
  `type post_body:string`, the ascription shape the language already
  uses. Into-subtype is the ctor form, `post_body ""`; the postfix
  sketch dies.

## 3 + 5. Dispatch architecture — GAVELED 2026-08-15: groups are global objects, extension is licensed by type ownership

One ruling closes both entries (Clay, 2026-08-15 dialog; converged
through the Haskell/Julia comparison — Julia's module-owned generic
functions with the type-piracy taboo made structural):

1. **Arms define or extend groups; the defining file's imports decide
   which.** A bare-imported name means the arm extends that group;
   otherwise it mints the hako's own new group — built-in argument
   types unrestricted, since the group is yours.
2. **Extending a foreign group requires owning a dispatching argument
   type.** The arm then attaches to the group-object and is ambient
   program-wide — every call site, err reasons, the wire — with no
   import needed by anyone. Coherence is by construction (a type has
   one owner) and the reader has one place to look: the type's hako.
   Owning a subtype counts (type slug:string, arm on slug) — that is
   clause 2 working, not an escape hatch.
3. **Independent same-named groups merge per-file in the consumer's
   view** via bare imports; first-place ties refuse (gavel 7);
   qualified names always bypass. Local restyling of rendering falls
   out of ordinary imports — interpolation is a call site in the file
   that contains the string.
4. **Interfaces without a shared group cross boundaries as closures
   carrying their scope** — a closure's body dispatches against its
   home file's view forever, so a hako ships a piece of its scope
   inside the value (dictionary passing; a lazy sequence is its next
   function plus state; the stdlib's pred/key style already works
   this way).
5. **Module-less surfaces** (an err reason at the top, harness output)
   see ambient arms plus the qualified default.

Consequences: a dependency's to_string arm for its own type renders
that type everywhere (entry 3 answered); the orphan rule IS clause 2
(entry 5 answered); Julia's social taboo becomes a compile refusal;
Haskell's class ceremony is declined knowingly — kanso keeps dispatch
as the foundation and pays with closed-world value-set inference,
already the plan of record. What was weighed and declined: name-global
pooling (common words become permanent ties; a dependency upgrade can
change existing programs — Julia's method-invalidation pathology),
and scoped third-party arms attached to foreign groups (subsumed by
clause 3's per-file merging with less machinery).

## 6. The tail-call promise — GAVELED 2026-08-17

One sentence becomes semantics, promised in the book for all three
engines: **a call in tail position consumes no stack.** A program may
lean on it forever — recursion is kanso's only loop, now as a contract
(Scheme's position). Explicitly NOT promised: TRMC — some non-tail
shapes loop today, gratefully, as an optimization the compiler is free
to change. The interpreter's 10,000-frame guard and native's OS
ceiling are documented as engine limits on the unpromised shapes. The
mechanism was already settled (#373 trampoline, #393 diagnostics);
this ruling is the contract. Queued: the book's promise paragraph.

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

## 15. `>>` defers its right side — GAVELED 2026-08-15

Yes: defer, avoid strictness. Under "the wire is the demand" (17/20b),
`>>` evaluating its right operand at construction was the last strict
position in the language. Deferred, `fn control_loop = step >>
control_loop` is productive — one link per demand, the robotics idiom —
and the interim diagnostic (all three engines naming the operator)
retires because the failure it named stops existing. The wall's laws
(associativity, first-failure-absorbs, not overloadable) are
untouched: laziness changes when links exist, not what they mean.

## 16. Block-born — GAVELED 2026-08-17: a dataflow property

Widened. A value is block-born if the analysis proves every path to it
originates inside the block — not only if its allocation is literally
written there. The syntactic rule was the conservative first cut; the
dataflow property is the semantics the license always meant, and it
unblocks in-place graph algorithms built through helper functions
(ledger 4.4). Enforcement is the escape analysis already running;
correctness gets pinned by goldens and the mem vein when built —
the compiler's lane.

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

## 18 + 19. The effects surface — GAVELED 2026-08-16 (SUPERSEDED by 24)

Closed together, with the spelling questions they dragged in. The
rulings, in dependency order:

- **19: operators do not lift over descriptions.** Running a
  description is observable — unlike forcing a thunk, which is pure and
  silent — so `r + 1` on a description has no legal reading: the
  expression cannot say when or how many times the effect runs. The
  bind lambda is the access path to the produced value, and the purity
  boundary holds because narrowing has no other spelling.
- **18: `return x` (named 2026-08-17; was `pure` in the first record) is the trivial description** — executes as nothing,
  answers x — spelled as an ordinary record with one executor arm per
  engine. The carrier is data, not compiler magic.
- **A combinator owns its output channel and wraps whatever its
  callback answers into it** (Clay's phrasing): bind wraps plain
  answers into the effect, annotate into err, rescue into success. So
  `return` is only ever written in non-callback positions (holding a
  trivial effect, seeding a fold, handing a known answer to an
  effect-shaped interface).
- **Dot is application, always.** `x . f args` = `f x args` for every
  x — a description pipes as a value like anything else (`fetch url .
  retry 3` hands retry the effect). The carrier-dispatched pipe was
  considered and rejected: it makes `x . f`'s meaning depend on x's
  inferred type, the same unreadability that killed lifting.
  Left-identity (`bind (pure v) f = f v`) is a law of the bind
  function, not a behavior of the pipe. This also settles the
  long-pending "binding-as-bind vs dot" question: dot is not bind.
- **The effect-chain operator is `.>`** — sugar routing an arm bundle
  to the triad: a value arm is bind's callback; an err arm naming a
  foreign reason type is rescue (provenance-checked at this site); a
  bare err arm is annotate; an unmatched failure passes through to the
  next link. One operator, three combinators underneath, no new
  semantics. Glyph family: dot means the value flows, `>` means
  effects sequence — `.` (value, pure), `.>` (value + effects), `>>`
  (effects, no value). `?` was rejected as a glyph: the suffix gavel
  gave `?` a checker-enforced meaning (answers bool).
- **`>>` keeps its glyph, now knowingly.** Haskell's sequencing pair
  is `>>=` (value passed) and `>>` (value dropped, `m >> k = m >>= \_
  -> k`); kanso independently chose the same glyph for the same half.
  With `.>` in the language `a >> b` is derivable as `a .> (_ -> b)`;
  retiring the wall under fewest-elements was recorded and declined —
  value-free sequencing is common enough for a dedicated glyph, and
  the wall's laws are gaveled, taught, and fuzzed under this spelling.

## 21. The effect block — GAVELED 2026-08-16 (SUPERSEDED by 24)

The do-notation analog, ruled the same hour the `.>` operator landed.
Inside an explicitly-opened block (keyword: `do`, confirmed), statements sequence via bind and
`=` means one uniform thing: **name the answer**. An effect RHS binds
the produced value; a pure RHS binds the value itself — the same
behavior, by left-identity, so pure lines don't notice the block.
Left-to-right, no `<-`: Haskell's convention was rejected outright
("it should go left to right... `body = fetch url` ... reads
dramatically better"); the `fetch url -> body` variant was declined
for colliding with the lambda arrow.

- The block boundary is load-bearing: at the ambient statement level
  `x = fetch url` stays a pure name for the description (the night-2
  ruling, untouched). The keyword is the explicit signal that `=`
  names answers.
- Naming an unrun description inside a block: `d = pure (fetch url)` —
  the bind unwraps one layer, Haskell-identical, rare and explicit.
- Failure: a failed line skips the rest (infectiousness); per-line
  rescue is the ordinary pipe (`body = fetch url . rescue (e ->
  ...)`), provenance-checked at the site; whole-block rescue wraps the
  block.
- Desugar target: nested `.>` chains. Pure routing sugar, no
  semantics. Closes the also-open "sequencing more than two binds
  prettily" item (multiplyTwoRandoms).

## 22. The lifting fork — RESOLVED 2026-08-16/17, then RE-RESOLVED by 24 (the no-pass rule dissolved the two-regimes argument)

The morning corner is ratified; the Koka corner is declined, and the
deciding argument outranks the trilemma: **the failure channel forces
explicitness anyway.** Handling a failure is a decision — which
failures, whose, converting to what — so rescue and annotate can never
be ambient. The Koka corner therefore ships TWO visibility regimes in
one surface: success invisible (lifted), failure explicit. A user
skates on ambient lifting until the day something fails, then meets
the whole channel machinery at once with no scaffolding. Uniform
explicitness is one mental model. (Clay: monads are confusing as
hell, and the fix is that kanso has no monads-in-general — one effect
type, a handful of ordinary functions, a block — not hidden ones.)

## 23. The effects vocabulary — GAVELED 2026-08-17 (SUPERSEDED by 24: the vocabulary sinks below the surface; `skip` survives)

`bind`, `rescue`, `annotate`, `return`, `do`, `skip`.

- The block keyword is **do**.
- The unit is named **return**, Clay's call with Haskell's regret
  (the 2014 pure migration, "return doesn't return") considered and
  overruled: familiarity and descriptiveness for the Go/Ruby/JS
  audience outweigh the early-exit misread, and kanso has no return
  statement to collide with. `answer` was offered and declined.
- **return is deliberately a rare word.** The combinator-owns-its-
  output-channel rule auto-wraps plain answers in every callback and
  block position, so everyday code never writes it. Its residue:
  mixed conditionals (`if cached (return cached_value) (fetch url)`),
  collections of effects, seeding folds.
- **skip** is the no-op effect constant (`return none` under the
  hood) for the conditional-do-nothing branch, so the commonest
  residue case has a word that says what it means.
- The book teaches bind/rescue/annotate/do/skip early and return
  late; the word "monad" appears nowhere.

## 24. The boundary language — GAVELED 2026-08-17

The effect system stops being a feature and becomes a property. The
user's surface: functions, arms, `>>`, adjacency, and one doctrine
sentence — **your own failures only bubble.** Gaveled as a whole
document after the fourth reversal on this axis in a day; the arc and
its corrections are in the log.

### The five clauses

1. **Own errs are unreceivable.** No arm may match an err whose origin
   hako is the arm's own — a compile refusal via the provenance set
   (same fixpoint, simpler check than the return-channel rule it
   replaces). Own failures bubble to the boundary, always. Trapping is
   naming and unstoppable = no pub ancestor, both unchanged.
2. **Foreign errs are handled by ordinary dispatch arms.** An arm
   naming a foreign reason type receives the failure and answers
   whatever it wants — a value (conversion) or an err (annotation).
   The arm is the explicit decision site; provenance checks the
   license there. A bare `err e` arm can only ever hold a foreign err,
   so it too may answer anything.
3. **The success channel lifts ambiently, signature-directed.** An
   effect in a position whose inferred type demands the product
   elaborates to bind; plan-taking and unconstrained (parametric)
   positions receive the plan — so `retry (fetch url) 3` needs no
   mark, `id`/containers/list literals hold plans as data, and
   `fetch_num url * 3` lifts. Independent effects combine
   applicatively under the adjacency laws; a join with one plan branch
   is a plan, value branches wrapping. A name is a shared node — used
   thrice, runs once; execution count is readable from naming.
4. **No suspension mark exists.** `&` retains only its nullary-call
   gavel meaning. The unit dissolves (joins auto-wrap; `skip` is the
   no-op constant). Order is walls and data: adjacency unordered, `>>`
   sequences, dependence orders, the wire runs what arrives.
5. **What sinks below the surface:** bind, rescue, annotate, return,
   the do block, `.>` — elaboration vocabulary the compiler writes and
   the book never teaches. The combinator laws become the elaborator's
   spec, pinned by the differential machinery.

### Sworn trades (Clay, with the record quoted)

- Interior plan-ness is visible via tracking and the LSP, not
  spelling. Affirmed repeatedly.
- Shape-predictability is redefined as *predictable given signatures*:
  the 2026-08-16 impasse verdict ("i don't like doing it
  conditionally... it really is an impasse") is reversed knowingly —
  the conditional is signature-directed and deterministic, and the
  named hazard is sworn: **refactoring a callee's body can silently
  re-elaborate its callers** (a retry whose parameter drifts to
  product-typed retries nothing, well-typed). Mitigations: the
  unused-value rule, goldens, LSP signature drift; not forced by the
  language.
- Own-annotation dies; std's `number_ok`/`string_ok`/`must` restructure
  to raise the right reason initially, blunted by auto-carried origin
  and hop traces.

### What survives untouched

The description carrier and executors, wire-is-the-demand (17/20b),
provenance and the foreign-only license, the dispatch architecture
(3+5), `>>`/adjacency laws, dot-as-application, the test-surface rider
(the harness is a foreign party), gavel 20's knot semantics.

### Pins ruled with the entry

Lambda capture (effects in a lambda body lift within the lambda); the
join rule as stated; sharing as stated; bare-foreign-err arms as
stated. Build order and elaborator design are the compiler's lane;
welfare and the veins gate the cost.

## Also open, not blocking any current work

- **Is a bare list of small ints bytes?** The engines answer differently,
  which the differential law forbids outright, and nothing on a live
  list said so — it was measured on 2026-08-02, recorded in the archive,
  and re-found by a sweep on 2026-08-23. `text/to_float ["a"]` dies on
  native with `to_float takes a string, bytes, or number, not ["a"]` and
  answers an err on the oracle, `"bytes are not a number"`, because the
  interpreter has no distinct bytes value and runs any list through
  `bytes_to_str` where native has a `K_BYTES` tag. Genuine bytes agree
  on both. Widening native says a list and bytes are interchangeable,
  which is what the native representation exists to deny; giving the
  interpreter a real bytes value removes the ambiguity and touches every
  place it builds or reads them. Either answer settles `append`,
  `find2`, `find2_below` and `utf8` at the same time, since all four
  name bytes in refusals nothing pins. No golden can pin the shape until
  it is decided, and the diagnostic differential cannot see it: its one
  wrong value is a record, and a record is refused identically.

  The archive predicted the other four would settle with it. They are
  measured now, on 2026-08-23, and four of the six are worse than a
  wording difference — the oracle ANSWERS where native refuses, so a
  program written against the oracle compiles and then dies:

      text/append ["a"] "x"           native refuses    oracle: ["a" 120]
      text/append [65 66] "x"         native refuses    oracle: [65 66 120]
      text/find2 [65 66] 1 65 66      native refuses    oracle: 1
      text/find2_below [65 66] …      native refuses    oracle: 1
      text/utf8 ["a"]                 native: an err    oracle: a refusal
      text/to_float ["a"]             native refuses    oracle: an err

  `text/utf8 [65 66]` agrees, answering "AB" on both, because a list of
  small ints is bytes to each of them. The disagreement is everything a
  list can be that bytes cannot.

- ~~**TRMC v2**~~ — SHIPPED 2026-08-23. The operand may be any arithmetic
  over integer literals and the group's own counters, which covers
  `n * fact (n - 1)`. Inference was never threaded out of
  `check::check`: the wrapper already ascribes every counter `int`, so
  requiring each recursive call to hand those positions arithmetic over
  counters carries the integer property down by induction, and the pass
  reads it off the syntax it already has. The compile-cost golden pays
  no phantom infer run because there is no infer run.
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
- **An `os` package — RULED 2026-08-17**: mirror Go's split exactly
  (`os` takes filesystem/env/args/process; `io` keeps the abstract
  read/write surface; MkdirAll → os); any boundary case Go does not
  answer goes to the language committee, never back to Clay.
- **An assert hako** (future design pass, queued 2026-08-17): a real
  assertion library in the rspec direction Clay sketched —
  `(expect 1) . to (equal x)` — as its own small surface design, never
  improvised inside a test fix. Its arms are foreign to every tested
  hako, so the err license needs nothing special.
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

## Fixed-length list types — GAVELED 2026-08-21: declined

Clay supplied the argument himself: a position that means something is
a pet, and pets get names — a record. Cattle's count is a fact about
the herd, not about the type, and anyone insisting on integer
positions has discovered they want a map with integer keys. Composes
with the no-positional-products clause of the type-syntax gavel: a
fixed-length list IS a positional product with a uniform field type.
One counter-case recorded so it is not re-litigated sideways: numeric
vec3/mat4 shapes argue for dedicated types in a future numerics
story, never for general length-in-the-type, which would cost a
type-level number grammar the language deliberately does not have.

## The name of `[]T` — GAVELED 2026-08-21: "list" stays

Clay's words: "list seems like the name. you've convinced me." The
fork that was weighed: "list" carries the Python precedent (their
contiguous growable O(1)-index sequence bears this exact name) and is
the friendliest word in the room; against it, Lisp and Haskell
trained systems readers to hear linked-list-with-O(n)-index. "array"
is mechanism-honest but Ruby/JS-flavored; "slice" is Go's word for
the `[]T` spelling kanso borrowed, but names the mechanism; "vector"
collides with the numerics future and intimidates. The book owes one
sentence — contiguous, constant-time index — and nothing else moves:
no sweep of std, diagnostics, book, or siblings. Never re-ask.
