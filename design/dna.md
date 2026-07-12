# dna: the corpus behind the language

Status: distillation of Clay's published writing (17 Medium posts, fetched
2026-07-12), read against the current language (docs/about.html,
docs/compiler.html, kanso-spec.md). Nothing here is a ruling; section 4 is a
list of candidate gavel items and section 5 is the honest tensions.

## 1. the origin arc — "oop vs. fp" (oop-vs-fp-182475457a01)

The post that started the line of thought. Its argument, faithfully:

The OOP-vs-FP war is mostly a fight about notation. `animal.move(12, 15)` and
`move(animal, 12, 15)` are the same call wearing different clothes — Go
accepts both spellings of the same function. What OOP's dot syntax actually
buys is one substantive thing: dispatch. "OOP syntax is just a limited form of
function overloading" (Clay), where only the receiver position selects the
implementation. Everything else usually credited to the paradigms is
orthogonal: inheritance ("I quite strongly prefer composition over
inheritance"), mutability ("this is orthogonal to OOP vs. FP"), state
management. He closes by sketching a reconciled language — he calls it a FOOP
language — and concludes "the only concrete difference between OOP and FP is a
trivial semantic issue."

The arc from there, as the project tells it: the post's conclusion pointed at
functional languages, so he went to Haskell — and found that its answers to
the dispatch question were the wrong shape. Typeclasses reintroduce a parallel
declaration apparatus (the class, the instance, the pragma zoo) to express
what the post had identified as *one mechanism*: which body runs is a function
of what the arguments are. And monads wrap ordinary application in a second
calling convention (`>>=`, `do`) to get failure propagation and effect
sequencing. Insufficient, in his judgment, not because they are weak but
because they are *extra* — machinery standing where a decision should be. So
he built the language the post implied.

What kanso became is the post taken to its limit:

- **Dispatch is the only eliminator** (spec §"values may inhabit unions").
  The post's "limited form of function overloading where only the first
  argument can be overloaded" is un-limited: kanso overloads dispatch on any
  argument, by literal, concrete type, or generically. No `match`, no tag
  tests, no `instanceof` — the one mechanism the post identified as OOP's
  real content is the language's entire branching story.
- **No typeclasses.** Where Haskell writes an instance, kanso writes another
  arm of the overload group. The open set of behaviors attached to a type is
  just the set of functions that dispatch on it — the post's "FOOP"
  unification with the class apparatus deleted.
- **The monad laws moved into application.** Ordinary function application
  *is* bind: a failure (`err`, `none`) arriving at any argument position
  flows through the auto-generated pass-through arm untouched — the
  `Nothing >>= f = Nothing` equation, compiled in rather than spelled at
  every call. The happy path is the only path anyone writes (the kanso-json
  result: zero lines of failure plumbing in a full parser). `>>` sequences
  effect descriptions; `do`-notation has nothing left to abbreviate.
- **The orthogonal issues got separate rulings**, exactly as the post framed
  them: composition (no inheritance anywhere in the language), and — the one
  place kanso overrules its own origin post — mutability (see section 5).

## 2. the corpus, post by post

### notes-from-99-bottles-of-oop (Metz & Owen)

Core: duplication is cheaper than the wrong abstraction; refactor in tiny
always-green steps; "conditionals are the bane of OO" — confine the
conditional that *selects* an object to a factory and let polymorphism carry
the rest; "make the change easy (warning: this may be hard), then make the
easy change."

In kanso: the mushroom test (compiler.html coda) is the anti-wrong-abstraction
filter applied to language design itself — proposals that add a concept wait,
proposals that reveal one land. Polymorphism-over-conditionals is the house
rule every proposal passes. Always-green micro-steps became CI machinery: the
differential oracle holds three engines byte-identical through every change.

Not yet: nothing structural missing; the book's discipline is the project's
working method.

### dont-use-version-constraints-in-gemfile

Core: the lock file owns versions; constraints in the manifest are "redundant
noise" that discourage the continuous small upgrades which amortize risk.
Constrain only to dodge a known-bad version, with a comment.

In kanso: the planned package manager (gomod-minimal manifest, tool-owned
lockfile, spec-mandated semver-bump enforcement) is this post as
infrastructure.

Not yet: the package manager doesn't exist. Candidate gavel — when it lands,
version constraints in the manifest should be a formatting error (the
lock file is the only place a version may appear), with a single sanctioned
exclusion form for known-bad versions.

### avoid-each-with-object-generally

Core: "the more flexible a function is, the more one has to read the
specifics of what it's doing." Prefer the least-flexible, most semantic
collection method — `index_by` over hand-rolled accumulation, `sum` over
`inject(0)`. Flexibility is a cost the reader pays.

In kanso: the least-powerful-construct doctrine is the whole language (dispatch
over `if`; `at` vs `xs[i]` as two narrow tools instead of one flexible one).
The stdlib's map/filter/sum vocabulary starts on the right side of this.

Not yet: the prelude has no `group_by`/`index_by`/`tally` tier, so a kanso
programmer today would hand-roll exactly the accumulation loops the post
condemns — with recursion, the *most* flexible construct. Candidate gavel:
the semantic-reducer shelf for the prelude/import boundary decision.

### avoid (the `[]` post)

Core: `hash[key]` returns nil silently and detonates far from the cause;
`fetch` fails at the point of reference and *encodes intent* — `fetch(k)` says
must-exist, `fetch(k, default)` says absence is fine. The reader of `[]` can't
tell which was meant.

In kanso: gaveled almost verbatim as strict indexing. `xs[i]` / `m[k]` is
fetch — a miss is `err "missing index"` riding to the endpoint. `at xs i` is
the deliberate expected-absence opt-in (`value | none`). Two ops, no overlap;
the reader always knows which absence-policy the writer chose.

Not yet: nothing — this one is fully landed.

### details-matter

Core: dynamic/relative time in tests causes flaky failures that block deploys;
fix a single hardcoded `now` and derive everything from it; parameterize DB
time rather than calling the database's `now()`. The war story: the exact
failure he'd warned about held up a deploy.

In kanso: the executor architecture is the general answer — `now` is planned
as a *description*, so a scripted executor supplies the clock and
deterministic time falls out of the same mechanism as deterministic IO. No
`travel_to`, no stubbing: time is an effect, and effects are data.

Not yet: `std/time` isn't built. Candidate gavel — when `now` lands, `kanso
test` should provide a scripted clock *by default*, making a test that reads
real wall-clock time unwritable rather than merely inadvisable. The post's
entire failure class becomes unrepresentable.

### avoiding-type-assertions-in-go

Core: type-switches scatter the behavior of a family of types across the
filesystem — shotgun surgery on every new type. "If you find yourself typing
t.(type), there's probably a better way": put the method on the types and let
dispatch do the work.

In kanso: absorbed to the point of syntax. Runtime discriminants are
compiler-owned; *no user syntax reads them*. The type-switch is not
discouraged — it is unwritable. And the overload group answers the scattering
complaint from the other side: all arms of a group live in one file (module
ruling), so the behavior of a family reads as a table.

Not yet: nothing; this post is load-bearing doctrine.

### oauth-and-tdd-in-go

Core: inject the boundary (`http.RoundTripper`), then record real HTTP
interactions once and replay them as fixtures (VCR), with filters redacting
credentials before the fixture is written. Tests exercise real request/
response shapes without live calls.

In kanso: the Executor trait is the RoundTripper generalized to *all* effects,
and the ScriptedExecutor is the replay half — scripted IO in, transcript out,
asserted by `==` on data.

Not yet: the *record* half. Candidate gavel — a record mode where the real
executor captures each effect and its yield into a fixture the scripted
executor replays. That closes the loop for network-shaped effects (the future
`http` description): first run against the real service, every run after
against the recording, redaction filters included. The no-mocks story would
then cover the one case mocking libraries still claim.

### exceptional-go

Core: Go culture over-rotates against `panic`. The real line is between
errors a caller "may reasonably recover from" and situations where
continuation is impossible; "if you're always going to panic or exit...
it's perfectly reasonable to just do it immediately." Same rule as Ruby's
`save`/`save!` pair.

In kanso: the err/defect split is this post as semantics. `err` is the
recoverable return value; `must` is the one-word `!` — it converts an allowed
failure into a `defect` that rides to the root reporter. No doubled API
surface (Ruby needs two methods; kanso composes one word with everything).

Not yet: the endpoint rule treating `defect` as auto-reported rather than
must-be-handled is still owed (already queued on the compiler page).

### tests-dependencies

Core: four goals in tension — fast, clean, confidence, freedom — and freedom
(tests must make refactoring easier, not harder) is the one most often
sacrificed. Isolation is a spectrum, not a binary. Mock commands (side
effects), call through queries; "isolated specs come with the major liability
that they don't ensure correct wiring." And the punchline kanso was built on:
"functional code is inherently isolated, without mocks."

In kanso: that punchline is the testing story. Every function is pure, so
every function is a query and *calling through is the only option* — there is
nothing to intercept. Commands don't exist at test time because effects are
descriptions; the ScriptedExecutor turns would-be side effects into transcript
data. The wiring liability is answered at the toolchain level by the
differential oracle: three engines, byte-identical, so the suite that
matters most is the one the language runs on itself.

Not yet: `kanso test --native` (suites still interpret — already queued), and
the deferred BDD framework. The four-goals framing suggests the test runner
should some day report *freedom* regressions — tests that broke under a
behavior-preserving change — but that is a research note, not a queue item.

### what-goes-in-active-records

Core: persistence classes should wrap the database thinly; "anything that
could be considered application logic... is not going to belong in this
class." Callbacks braid business logic into the persistence lifecycle —
hidden ordering, guard clauses, untestable side effects. Extract collaborators
and call them from the shell. A simple prohibition beats a nuanced policy
because everyone can remember it.

In kanso: functional core / imperative shell is not a discipline here, it is
the grammar. Business logic is pure by construction; the executor is the only
shell; a lifecycle callback — code that fires implicitly on a state
transition — has no representable form, because there is no mutable state to
transition. The post's "simple prohibition beats nuance" is also kanso's
enforcement philosophy: the prohibition is a compile error, so nobody even
remembers it.

Not yet: nothing until kanso grows a persistence story; this post is the
design brief for that day.

### unnecessary-conditionals

Core: a boolean parameter that every caller passes as a constant is a
conditional wearing a disguise — the callee re-decides what the caller
already knew. Split the method; let callers call the branch they meant.
Obfuscation (extraction plus nil checks) hides how unnecessary the
conditional is.

In kanso: two mechanisms delete the pattern. Literal dispatch on `true`/
`false` *is* the post's refactor performed by the compiler — the "boolean
parameter" becomes two arms selected statically at every constant call site,
so the conditional costs zero instructions and the reader sees two bodies.
And auto-propagation is the same principle applied to failure: no callee ever
re-checks what an earlier step already established, because the pass-through
arms carry the known-failed case past every body that doesn't care.

Not yet: nothing structural; the annotation-redundancy checker (gaveled,
unbuilt) extends the spirit — an annotation restating what inference already
knows is clutter, exactly like a conditional restating what the caller knew.

### notes-from-exceptional-ruby (Avdi Grimm)

Core: "use exceptions only for exceptional situations" — the test is "will
this code still run if I remove all exception handlers?" The caller, not the
callee, decides whether a state is exceptional (HTTP 4xx is data to one
caller, failure to another). Rescue specific classes. Structure exception
types by *how they're handled*: user error (fix input, retry), logic error
(our bug — record it), transient error (nobody's wrong — wait and retry).

In kanso: the handler-removal test passes by construction — failures are
values, so a program with no failure arms still runs; it just propagates.
Caller-decides is literal: `err` arrives as data at whatever frame chooses to
write an arm for it, and the endpoint rule guarantees *someone* decided (an
unhandled constructor at a chain endpoint is a compile error). User error vs
logic error is `err` vs `defect`, with `must` as the conversion.

Not yet: the taxonomy's third leg. **Transient** failures — nobody's wrong,
wait and retry — have no home: they are neither `err` (the caller can't fix
a network blip by handling it) nor `defect` (retrying a bug is the
mis-filing the post warns about). Candidate gavel: retry policy as an
executor concern — a description can carry a transience marker, and the
executor (which owns time and IO anyway) owns backoff, the way ActiveJob's
`retry_on` keeps retry out of the business logic. Fine-grained failure types
(typeset-based propagation beyond err/none, already noted as kanso-json
friction) is the same gavel from the other end.

### the-twelve-factor-app

Core: an abridgment, endorsed: config in the environment ("env vars are
granular controls, each fully orthogonal"), backing services as attached
resources, stateless share-nothing processes, logs as event streams to
stdout, dev/prod parity, fast startup and graceful shutdown.

In kanso: the process tier of the parallelism model (processes as the only
opt-in concurrency structure, supervisor messaging for unhandled err) is
twelve-factor's process-first worldview as language semantics. Dev/prod
parity has a compiler-shaped echo: the engines are held byte-identical, so
"works interpreted, breaks compiled" — the toolchain's version of works-on-
my-machine — is a CI failure, not a production surprise.

Not yet: the config story. There is no `env` effect, no capability manifest
entry for configuration, no logs-as-stream doctrine for long-running
processes. Candidate gavel: config-as-effect (an `env` description the
scripted executor can script, making twelve-factor's testability argument
structural), filed with the effect-manifest work.

### the-hierarchy-of-complexity-for-conditionals

Core: a ranked ladder, simplest first — (1) a convention (one rule, zero
code), (2) a map/lookup table, (3) a case statement, (4) arbitrary boolean
expressions, "the last resort." Each rung down buys flexibility the reader
pays for. "Your goal is to minimize costs, and costs are determined by the
situation."

In kanso: the language is the ladder with the bottom rungs sawn off.
Convention is rung one and kanso's founding move — canonical form, canonical
ordering, one rendering per program: the maximal "single overarching rule
that requires minimal code." Dispatch is rung two: an overload group is a
map from argument shapes to bodies, closed and enumerable, which is exactly
why the backend compiles it to a literal jump table. Rungs three and four —
the open-ended case statement and the arbitrary boolean branch — are not
discouraged; they are ungrammatical.

Not yet: see section 5 — the post keeps the bottom rung as a legal last
resort; kanso doesn't, and the lazy `and`/`or` friction from kanso-json is
where that difference bites.

### on-comments

Core: DRY applies to knowledge, not just code — a comment restating the code
is a second copy of one fact, and "a comment is a lie waiting to happen"
(Susser). "A comment is the code's way of asking to be more clear" (Beck).
The legitimate residue is high-level *why*: rationale, external constraints,
the 2am-incident context (Shay's class-level case).

In kanso: nothing-wasted is this post with enforcement. The restating comment
is usually scaffolding around unclear code, and kanso attacks the unclarity
side: unused bindings, dead expressions, and non-canonical renderings —
the noise a rotting comment typically annotates — are compile errors.
Comments themselves are `#` (gaveled), deliberately minimal.

Not yet: kanso has no comment *doctrine* — nothing distinguishes the why-
comment the corpus permits from the what-comment it condemns, and no ruling
on where `#` may appear (inline vs own-line) or what a doc-comment for a
`pub` surface looks like once visibility lands. Candidate gavel, low
urgency: comment placement as part of canonical form, and the documentation
story for public APIs.

### interesting-excerpts-from-extreme-programming-explained (Beck)

Core: the excerpts Clay chose to keep — "you can't get software out the door
faster by lowering quality"; deploy the invisible work continuously and add
the keystone last; "planning in XP is an activity, not a phase"; defects cost
more than prevention; simple design is appropriate, communicative, factored,
minimal.

In kanso: the four criteria of simple design are the compiler's checklist —
factored (nothing-wasted, no duplication of renderings) and minimal (fewest
elements: the mushroom test) are enforced, communicative is the alphabetical-
order bet (see section 5). Quality-as-speed is the two-engines economics:
the project never trades correctness for iteration speed because the oracle
makes correctness cheap. Keystone-last is, verbatim, the launch strategy —
weeks of shipped invisible work, then the Show HN.

Not yet: nothing; this is the project's operating system.

## 3. what the corpus adds up to

One sentence per layer:

- **Mechanism:** dispatch is the only decision-maker (oop-vs-fp,
  type-assertions, hierarchy, unnecessary-conditionals).
- **Absence:** failure is a value that fails fast at the point of reference,
  with intent encoded in which tool you reached for (avoid-[],
  exceptional-ruby, exceptional-go).
- **Effects:** decisions are pure, IO is data, and the boundary object is
  swappable — which makes testing assertion-on-data instead of interception
  (tests-dependencies, oauth-and-tdd, details-matter, active-records).
- **Enforcement:** a rule everyone must remember is worse than a rule nobody
  can break (active-records' "simple prohibition," on-comments, gemfile,
  99-bottles) — kanso's move is promoting the whole corpus from convention
  to grammar.

## 4. candidate gavel-queue items surfaced by this reading

Ranked by how much language they'd exercise:

1. **VCR-mode executor** (oauth-and-tdd-in-go) — a record mode capturing each
   real effect and its yield into a fixture; ScriptedExecutor replays it.
   Redaction filters for credentials at record time. Completes the no-mocks
   story for network-shaped effects before `http` descriptions land.
2. **Transient-failure tier / retry as executor policy**
   (notes-from-exceptional-ruby) — the user/logic/transient taxonomy has two
   legs in the language (`err`, `defect`) and no third; retry/backoff belongs
   to the executor, marked on the description, never written in business
   logic. Merges with the fine-grained-failure gavel already queued.
3. **Deterministic time by construction** (details-matter) — `now` as a
   description, and `kanso test` supplying a scripted clock by default so
   reading real time in a test is unwritable. The flaky-time failure class
   becomes unrepresentable.
4. **Config-as-effect** (twelve-factor) — an `env` description under the
   effect manifest; scripted in tests like everything else.
5. **Manifest bans version constraints** (gemfile post) — when the package
   manager lands, versions live only in the tool-owned lockfile; a constraint
   in the manifest is a formatting error, with one sanctioned known-bad
   exclusion form.
6. **Semantic-reducer shelf** (each_with_object) — `group_by`/`index_by`/
   `tally`-class functions in the stdlib so accumulation-by-recursion (the
   most flexible construct) isn't the default answer; part of the prelude/
   import boundary gavel.
7. **Comment doctrine** (on-comments) — placement rules for `#` under
   canonical form; the doc story for `pub` surfaces when visibility lands.

## 5. tensions — stated for the gavel

Honest disagreements between the corpus and the current language, none of
them secret, all of them rulable:

1. **The origin post defends mutability; kanso banned it.** oop-vs-fp calls
   mutability "orthogonal to OOP vs. FP" and treats encapsulated mutable
   state (Ruby-style private state) as unproblematic. kanso rules purity
   absolute — no mutation, anywhere, ever. The resolution is presumably that
   the post was diagnosing (mutability isn't what makes OOP bad), and the
   language is prescribing (purity is what makes the compiler's license
   possible — restarts, fusion, parallelism-by-default all depend on it).
   But the corpus never actually argues *for* total purity; that step is
   kanso's own. Worth one paragraph of doctrine somewhere, so the origin
   post and the language don't appear to disagree.
2. **The hierarchy post keeps a last resort; kanso deleted it.** "Arbitrary
   boolean expressions" are ranked worst but legal — "there's no hard and
   fast rule about what's best. It just depends." kanso makes the bottom
   rung unwritable, and the kanso-json gauntlet felt exactly that edge: no
   short-circuit `and`/`or`, so a guard costs an eager evaluation (`both`
   can't protect `expensive p`). The lazy-and/or gavel item is really a
   ruling on whether the hierarchy's bottom rung gets a fire escape.
3. **Communicative vs alphabetical.** Beck's simple design and 99 Bottles
   prize code organized to reveal intent; kanso's alphabetical-order rule
   optimizes diff stability and canonical form, and the gauntlet showed
   developers will name-game helpers into adjacency (`str_char`,
   `str_chars`, ...) — the ordering rule pushing back into naming. Modules
   absorb some of it (tests as sibling files), but the corpus would call
   name-gaming a smell, and it's kanso's rule producing it. Already flagged
   as friction; filed here because it is a genuine corpus-vs-kanso
   disagreement, not just a papercut.
4. **Caller-decides vs safe-by-default indexing.** exceptional-ruby says the
   caller decides what's exceptional; kanso's `xs[i]` decides for the caller
   (miss = err toward the endpoint) and offers `at` as the opt-out. The
   fetch-vs-[] post argues this is *encoding intent*, not overriding the
   caller — but the two posts pull in slightly different directions, and
   Clay has already said he wants to litigate indexing ergonomics after
   using it. The corpus material belongs in that litigation.
5. **YAGNI extremism vs designing a language.** The corpus is YAGNI-extremist
   for application code; a language faces the irreversibility asymmetry
   (Clay's own resolution, from the placeholder-partial-application
   discussion: "we design for third parties, not ourselves"). Not really a
   live tension — but it means corpus arguments of the form "wait for the
   second use" don't transfer to grammar decisions unchanged, and reviewers
   quoting the corpus at language proposals should know which regime
   applies.
6. **VCR contradicts nothing but tests the doctrine.** The no-mocks story is
   currently airtight because all scripted IO is hand-written data. A
   record mode (item 1 above) imports the corpus's own tool but introduces
   fixtures that can rot — the exact liability tests-dependencies warns
   about in deep stubs. If it lands, it needs the post's own answer:
   re-record cheaply, redact at the boundary, and never assert on the
   fixture's internals.

## sources

All 17 posts fetched and readable on 2026-07-12 (none paywalled):
oop-vs-fp-182475457a01, notes-from-99-bottles-of-oop-5c902afd3948,
dont-use-version-constraints-in-gemfile-bb594003354a,
avoid-each-with-object-generally-89e3b2800b38, avoid-1dfab4e46790,
details-matter-766dff001d7c, avoiding-type-assertions-in-go-6feaa8762c27,
oauth-and-tdd-in-go-89662448864e, exceptional-go-3943c2230cf8,
tests-dependencies-65f592a46529, what-goes-in-active-records-d6974a3dc0ce,
unnecessary-conditionals-400eb269df31,
notes-from-exceptional-ruby-6ddc2a09ba87, the-twelve-factor-app-7e4ee1a2ad1b,
the-hierarchy-of-complexity-for-conditionals-f6763b23f76e,
on-comments-e2b2e725cc67,
interesting-excerpts-from-extreme-programming-explained-7e07ce67ab77
(all at https://clayshentrup.medium.com/).
