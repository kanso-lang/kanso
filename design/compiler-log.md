# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

## 2026-08-29 — gavel: --explain-copies declined

Clay: "yagni violation." The flag — a diagnostic naming the source
site of each evacuation copy — is declined until a copy surprises
somebody. The counters already pin how much the beat machinery copies
and CI catches the number moving; the *where* is two weeks of span
plumbing through the carry machinery for a question nobody has asked.
Its only realistic user today is the compiler's own memory work.
Re-file when someone actually reaches for it.

## 2026-08-29 — supersession: the three words replace the no-bind surface

Clay's rule, stated when the fork surfaced: "there cannot be
incompatible rulings. a later ruling REPLACES an older one." The
timeline, by commit timestamp:

- 2026-08-26 13:58 PDT — #1054 lands compiler.html entry 23, "why
  there is no bind": combinator words off the surface, handling as
  ordinary arms, signature-directed elaboration.
- 2026-08-27 — fa834a71, "three explicit forms": Clay rules `bind`,
  `annotate`, `rescue` as explicit chain steps, chain err-arms retire.
- 2026-08-27 and after — 61774d65 makes bind a word parallel with the
  others (`.` retires from chain position), and the effect-first rider
  fixes the signature: `bind effect callback`.

The later ruling wins. The three-explicit-words design is the
language; entry 23's no-surface-combinators argument is superseded and
the page owes a rewrite or retirement. Consequences:

- The dispatch-vs-elaborator machinery question dissolves in the
  words' favor: the foreign-only license is checked at `rescue`, the
  re-wrap at `annotate`, so the per-arm provenance machinery (the
  fixpoint, `k_not_own_err` at every match) has a smaller home —
  which is what #1054 called World B, now with the words on the
  surface rather than hidden.
- The queued book story ("the boundary language") re-premises on the
  three words; nothing is gated on an elaborator, because there is no
  elaborator to wait for.
- "Nothing is asked of the signature" survives unchanged: the words
  are call-site spelling, not type-level tracking; no signature
  carries an effect row.

## 2026-08-29 — the one-keyword world, explored and declined

After the supersession, Clay probed whether the combinators could go:
"I'd love to not need combinators but i don't see any clean way to
prevent the developer from bypassing effects." The exploration, so it
stays explored:

The requirements split three ways. Can't-drop needs no words — the
railway already makes discard unspellable. Can't-handle-your-own needs
no words — provenance already skips own-origin errs at match time. The
irreducible part is replace-vs-recover: one bit of intent with no
footprint in code shape, since a buggy annotation and a legal rescue
are the same program.

The candidate for carrying that bit without chain words: key on the
arm's declaration — any arm with an err-typed parameter is a handling
site; unmarked, its result is auto-rewrapped (annotation by
construction); a `rescue` keyword in the declaration is the sole door.
Enforcement is sound: bare parameters already refuse errs, so the
err-pattern arms are the exhaustive entry points.

DECLINED, by Clay: "that feels awkward and imprecise. to just say it
any argument matches err." The imprecision is real — with several
parameters nothing says which err is the cause; an arm wanting an err
as plain data (a log formatter, a matcher) is neither annotating nor
rescuing and the rule has no place for it; and a parameter's type
reaching over to rewrite what the return value means is action at a
distance inside one signature. The explicit words are the repair:
`annotate effect callback` and `rescue effect callback` attach the
intent to exactly one effect and one callback.

The three-explicit-forms gavel stands, having survived its strongest
challenger. Koka's row-tracking was examined in the same sitting and
declined on the standing doctrine: the row is signature infection made
ergonomic, and nothing is asked of the signature here.

## 2026-08-29 — gavel: effects are types, and the words are the only doors

The sitting that began at Koka ends with a ruling that supersedes both
the three-chain-words form (fa834a71 and its amendments) and the
procedural bind-sugar proposal (offered this sitting, never gaveled).
Clay's derivation, in his own words: with the combinator form "we'd
have Effect types that are basically indistinguishable. to get
anything you can match/branch on, you'd need to call
bind/annotate/rescue to get a _type_... passing the actual value is a
syntactic sugar for doing: bind my_effect (x -> do_something_with_x x).
but one might argue you _should_ be able to match on something like
<int>effect." And the gavel: "it might be 'convenient' to have bind be
automatic and just not allow passing effects, but it's inconsistent
and threatens to make the language confusing, when our overarching
goal is simplicity. so i think that's a gavel."

The ruling:

- **The effect is a first-class parameterized type.** `<int>effect` is
  the unresolved outcome of an operation: will be an int, or a
  failure. It can be bound, passed, stored, and received — a parameter
  declared `e:<int>effect` takes the box as data. Holding is not
  opening.
- **The three words are the sole eliminators, and they are explicit.**
  `bind effect callback`, `annotate effect callback`,
  `rescue effect callback` — ordinary functions, effect first
  (per the effect-first rider), the only ways from box to branchable
  value. There is NO automatic bind: passing a `<text>effect` where a
  `text` is expected is refused, not silently skipped. The old
  err-in-err-out railway at ordinary calls retires with the sugar
  that would have implied it; propagation is bind's contract — a
  failed effect handed to bind skips the callback and stays a failed
  effect.
- **The box is outcome, not deferral.** `<t>effect` answers "did it
  work"; the `>>` wall's deferred description answers "has it run".
  They stay distinct so holding a result can never delay work.
- **The box is opaque to dispatch.** No arm may match
  "failed `<int>effect`" against "succeeded `<int>effect`" — that
  would be a third, unmarked eliminator. One shape; the words open it.
- Everything the words already carry stands unchanged: annotate always
  re-wraps with the original as cause (resurrection unspellable, the
  callback's return becomes the new reason — string, record, or a
  domain type for downstream dispatch); rescue is the sole exit with
  the foreign-only license checked at the word against the callback
  arms' provenance; callbacks receive the err itself, so dispatch
  groups route by reason type; simultaneous failures merge per the
  2026-08-05 ruling.
- **Why passing must be allowed** (the coherence Clay named): a helper
  can now take `e:<config>effect` and annotate it — the abstraction
  Haskell gets from Either-as-value — without reopening any unmarked
  door, because the parameter type is an explicit spelling of "I hold
  the channel," and the words remain the only openers.

Consequences owed by implementation and record, filed rather than
decided here: ch04's "nothing is asked of the signature" and the
passed-through-label story describe the retired railway and need
re-premising on explicit bind; the provenance hop-per-function now
accrues at binds rather than at skipped calls, which moves the
eta-reduction argument's ground; and what becomes of a box that is
never eliminated (the drop question) needs an answer — the existing
refusal of unused bindings covers the bound case, and the unbound
tail-position case goes to the ledger.

Rider, same sitting, Clay: "effect arguments should work just like any
other type. no superfluous type annotations on functions. if you pass
an effect to foo, and it does rescue or whatever, and it passes the
value to a matching arm, then it Just Works." So `<t>effect` earns no
special ceremony: a parameter that receives an effect needs no
annotation — inference types it from use, the same as every other
type — and dispatch matches effect-typed values like any values. The
`e:<config>effect` spelling exists for when an author wants to state
the shape, never as a requirement. The intent bits stay at the words;
nothing about this ruling ever lands in a signature.

## 2026-08-29 — the digest question bounces: performance is never a gavel

Clay, on being handed the sha256 arena question as "blocking": "you
just have to figure it out, 'algorithmically'. this isn't a call that
involves me. it doesn't affect the language surface area, it's just
about performance and welfare metrics and you have the entirety of the
field of computer science to draw from... if you iterate on this for
an hour or two and cannot devise a solution from your vastly search
ability across the totality of the internet, then that might call for
my intervening. but i highly doubt that's going to happen."

The entry violated the ledger's own charter — implementation details
do not come to Clay — and the "blocking" label does not change whose
question it is. It leaves the ledger and returns to the implementer
with a mandate: survey the literature before touching the collector.
Starting points the field already offers for exactly this shape (a
loop-carried accumulator pinning a region for the length of a call):

- MLKit's region inference with STORAGE MODE ANALYSIS — the classic
  answer to "a region grows under iteration": infer where a store can
  be `attop` (reset the region) rather than `atbot` (extend it). The
  sha256 state accumulator is the textbook case.
- Tofte–Talpin region resetting generally, and Aiken–Fähndrich–Levien
  early deallocation, which decouples region death from lexical scope.
- Koka's answer is Perceus: precise refcounting with drop-reuse, which
  turns a functional update into an in-place one when the count is 1
  — the same "this block is dead the moment the next one exists" fact
  the eight killed hypotheses were circling. kanso declined refcounts,
  but FBIP ("functional but in place") describes the target shape the
  beat machinery could reach by its own means.
- The project's own beat rewind is already a region reset at loop
  granularity; the open engineering question is only why the digest's
  per-block garbage is not being caught by it, and the eight measured
  non-causes have narrowed where to look.

Option 3 (builtin) stays available as the fallback the entry named,
but only after the survey and an honest attempt at 1 fail. The
tests/sha256_peak.rs pins stand so the next move is visible.

## 2026-08-29 — gavel: read_file is text, read_bytes is bytes, per precedent

Clay, handed a three-option menu: "seriously? can't you just look at
what go or rust do?" They agree: Rust ships fs::read (bytes) beside
fs::read_to_string (text, refuses invalid utf-8); Go ships os.ReadFile
(bytes) with text as explicit validated conversion. Two readers, each
naming what it reads. So: `read_file` reads text and refuses non-utf-8
on every engine identically — the interpreter's current refusal
becomes the law rather than a divergence — and a byte-transparent
`read_bytes` lands in the same change, since native's callers
(scripts/fingerprint among them) read binary today and keep working by
renaming their call. The bytes value is the one the archive already
ruled real.

The meta-ruling, second of the sitting: a library-surface question
that mainstream precedent settles unanimously is presented as "X and Y
both do Z; copying it" — one line — or handled by the implementer
citing the precedent in the log, not brought as a menu.

## 2026-08-29 — the inline-name entry bounces the same way the digest did

Main's ledger gained "Whether an identifier's name lives inline" — a
22-byte small-string type for AST name fields, 21.3% of front-end
allocations, ninety mechanical edits, recommendation already "build
it". Zero surface area: no program can tell how the compiler stores a
name. Per the same-day ruling that performance questions with no
surface are the implementer's, it leaves the ledger unruled — build
it, and answer for it in the log beside the numbers. The one
Clay-shaped fragment inside it, whether to take a dependency for the
small-string type, is answered by the file's own precedent: Cargo.toml
carries two crates by policy, so hand-write it. The entry's removal
from the ledger rides the next reconciliation with main.

## 2026-08-29 — gavel: the chain line keeps its dot

On "Whether a chain line keeps its leading dot", Clay: "the
combinators look and act like regular functions." So a chain
continuation spells them the way it spells any function — `. bind (f)`
— and the leading dot stays the one continuation marker the grammar
has. No parser knows the names bind, annotate or rescue; an indented
line without the dot remains one more argument, per the existing rule.
The 346-step migration takes the keep-the-dot shape. The ledger
entry's removal rides the next reconciliation with main.

## 2026-08-29 — gavel: an err has readers

On "Whether an err gains readers a callback can use", Clay took the
recommendation. An err gains `.reason`, `.cause` and `.origin`, and
reading one is the second deliberate hole in infectiousness — the
same carve-out `wrap_err`'s second argument already has. Every other
operation on an err still propagates it. This makes the gavels' own
lambda samples (`annotate e (err -> "config: {err.reason}")`) compile;
dispatch-group callbacks keep destructuring as before. The ledger
entry's removal rides the next reconciliation with main.

## 2026-08-29 — gavel: the drop question closes — explicitness IS the guarantee

On "What happens to an effect nobody eliminates", Clay ruled the
premise backwards: "it's the exact opposite of that. the fact that
you have to explicitly call e.g. bind makes it all the more obvious
that effects can't be dropped." An effect is a value in hand, and the
language already refuses unused values — an unused binding is a
compile error, and a value nothing consumes has nowhere to go. So a
dropped effect is already unspellable under existing discipline,
and visibly so, where the railway's guarantee was ambient machinery
a reader could not see. No new checker rule and no io-edge rule is
minted; the recommendation is declined as solving a non-problem. The
entry leaves the ledger with this commit.

## 2026-08-29 — gavel: a qualified name is its module's declaration

On "Which claim owns `dep/join`", Clay: "yeah of course take the go
option." A qualified name means the named module's own declaration and
nothing else — `dep/join` is dep's `pub fn join`, never a clone of an
import's arm, and an author's right to declare a name an import
happens to export is unconditional. The enrolled clones leave the
qualified namespace; the bare overload space gets an internal
namespace a consumer cannot write; module_differential's known-defect
`w1` leaves the defect ledger with the fix. The measured hazard — the
one-line version silently re-exporting std's arm under dep's name —
is the case the implementation must hold a red spec against. The
ledger entry's removal rides the next reconciliation with main.

## 2026-08-29 — gavel: records print qualified, everywhere

On "What a record prints as, when its module is imported", Clay:
"sure, qualified everywhere is fine." A record prints its qualified
name regardless of how the program was entered — Go's `main.T`
precedent, already this project's package rule. `slow_lane 7` becomes
`lane/slow_lane 7` in both entry paths; the root module needs a name
for the prefix to exist; cross_module_fields' pins were right all
along and stay; tests/entry_file.rs flips its expectation to the
qualified form and stops being ignored. The entry-path divergence —
the actual defect — dies with the ruling. The ledger entry's removal
rides the next reconciliation with main.

## 2026-08-29 — gavel: an instruction is a cost, whoever put it there

On "Whether a compile_instructions move that cannot be work needs an
attribution", Clay declined the waiver: "if it's a different
instruction stream then that's a real cost so what's your question?"
Option 1 stands — every rise in the vein pays the full ritual: golden,
page figure, log sentence, and a welfare_floor attribution. The
correction is to the attributions, not the rule: an entry never again
says "nothing was spent"; it names the cause — rustc codegen movement
from an unrelated edit, when that is what the evidence shows. A ledger
accumulating that attribution repeatedly is the case for attacking the
cause (pinning inlining, splitting the crate), which a waiver would
have hidden. The ledger entry's removal rides the next reconciliation
with main.

## 2026-08-29 — gavel: the backends build the partial over a value

On "Whether the backends should build a partial over a value", Clay
rejected leave-it in the strongest terms: "the whole point is when you
write a library including a language you're writing it for an unknown
future use case... you are deciding what features they are allowed to
use up front." So: BUILD IT. Native and wasm gain the runtime shape
the interpreter already speaks — a partial whose arity resolves when
the arguments arrive — and `&f` over a value callee works on all three
engines, pinned by the differential goldens like everything else.

The meta-line this draws, so the next entry applies the right
precedent: yagni governs additive tooling (a diagnostic flag can land
the day somebody wants it — the --explain-copies ruling), and never
governs language surface. A feature one engine speaks and two refuse
is a transition state under the differential law, not a resting place;
a user meeting the refusal cannot file the use case, they conclude the
language is broken. The ledger entry's removal rides the next
reconciliation with main.

## 2026-08-29 — correction: the yagni axis, in Clay's words

The previous entry's meta-line (yagni governs tooling, never surface)
drew the wrong axis. Clay: "there really is no such thing as yagni in
language design. there may be something you don't have any evidence
you need yet and if the user base or you yourself end up wanting it
then fine you add it at that time. but if there's a feature you say is
important enough to add then you are deciding it's important enough to
add and make usable right now."

So the axis is undecided versus half-shipped. A thing with no
evidence of need waits for the evidence — that is the --explain-copies
ruling, and it needs no other justification. A feature the language
has admitted is a decision already made, and it is finished now:
every engine speaks it, or the decision was not made. "Refuse
honestly" is the differential law's transition state, never a place a
decided feature rests.

## 2026-08-29 — gavel: block-born is the whole cohort

Clay: "okay whole cohort it is." Block-born becomes a dataflow
property: anything the checker can prove was born in the block can be
knotted — through aliases, conditionals, indexes of block-born
collections, fields of block-born nodes — so cyclic structures sized
by data (a graph parsed from input, N linked nodes from a map) gain a
spelling. The theorem's obligation is unchanged: prove the birthday,
or be refused; the analysis widens how the proof is found, never what
must be proven. The book's sentence — everything born in the block,
nothing escaping — was the decision; the syntactic fence was the
checker falling short of it. Ships with red-first fixtures for the
newly admitted shapes and the escape cases that must stay refused.

## 2026-08-29 — gavel: the ambiguous-bare-call refusal is final

Clay, confirming: "I thought we decided this in a pretty reasonable
way and I don't know why you're considering it not final." The
INTERIM stamp from the 2026-07-27 committee ruling retires: a bare
call that two imports answer alike is refused, permanently, and the
qualified name is the fix — the Go-inspired shape, with kanso's
namespacing doing what Go's aliases do. Import order never decides
semantics. check_bare_ambiguity stands as built.

## 2026-08-29 — gavel: arms travel with the type, under the ownership rule

The "dependency render arms stay out" recommendation died in dialog on
the pass-down case: a value handed to a library that never imported
its type would render as a bare record there, so one value would print
two ways in one program depending on whose code called print — and no
import anyone can write fixes it, since a library cannot import every
type its callers might pass.

The ruling, in the general form Clay recalled agreeing to: **an arm is
legal when at least one of its parameter types is owned by the
declaring hako, and legal arms travel with the type — globally active
wherever the value goes.** Render is the special case where the group
is render: money's author writes `render m:money` and every module,
importer or not, prints money the owner's way; `render s:string` from
money is an orphan and is refused, so no third party ever restyles a
type it does not own. The same ownership principle as this morning's
dep/join gavel: a name is its module's declaration, a behavior for a
type is its owner's. If the archive holds the original agreement this
restates, the citation joins this entry at reconciliation; either way
this entry is now the ruling. The ledger entry leaves with it.

## 2026-08-29 — gavel: no `first coll n`; `take` is the answer

Clay confirmed the recommendation. `first` keeps one arity and answers
one question — the element, or none. A count belongs to `take`, which
answers with a list. A second arity on `first` would make the return
shape depend on the argument count: the same name answering two
questions, the disease the `done` gavel cured in the effect world.
The enumerable spec's §9.3 question closes.

## 2026-08-29 — gavel: std ships inside the binary

Clay took the recommendation. `std/` is welded into the toolchain: the
compiler version is the std version, one number in every bug report,
one version axis under every differential golden. Go's shape, for
Go's reason. A pinnable std waits for a demonstrated need, which
would be a new entry, not a reopening. design/hako.md's observation-
clause question closes.

## 2026-08-29 — gavel: the frame guard's standing offer closes

Clay, after the Rust/Haskell/Python comparison: "you've convinced me."
The interpreter keeps its 10,000-frame guard — a deterministic frame
count, Python's mechanism at 10x the number, refusing the same
program at the same depth on every build where a byte limit would
drift with frame size and optimization level, which matters here
because the differential law cannot tolerate engines disagreeing
about how deep a program may recurse. Native stays under the OS byte
ceiling; both are documented. The offer to revisit the constant
closes; a program that needs more depth reopens it with evidence.

## 2026-08-29 — gavel: saturate each counter, then average (task #184)

Clay, on the welfare aggregation question: "you do the log or whatever
function it is on each term before averaging." The saturation curve
applies to each counter's ratio individually, and the term is the
equal-weighted average of the saturated values — never the saturation
of an averaged ratio. So the curve reaches every counter, successive
doublings of any single benchmark pay less as the philosophy says, no
counter dominates a term by raw magnitude (widebench's 68% share of
run speed ends), and the two held improvements are scored under the
new definition. The switch is a change to the objective, so it lands
the way a weights change lands: recorded, and the floor re-set in the
same commit.

The compile_instructions noise observation folds in here rather than
becoming its own gavel, as the session suspected: the attribution
ritual was ruled earlier this sitting (an instruction is a real cost,
full ritual, causes named honestly), and per-counter saturation stops
layout jitter from leveraging a whole term. Both rulings reach the
session in this branch's sweep.

## 2026-08-29 — auto-delete of merged branches is on

Clay flipped "Automatically delete head branches" in the repo
settings, which no session could do — the GitHub proxy refuses
repository-settings writes (the 403 task #109 kept hitting). Every
branch merged from now on deletes itself. Task #109 closes on this;
what remains is the one-time purge of branches that merged before the
setting existed, which is ordinary API work any session can do.

## 2026-08-29 — directive: bring binary size back down, keeping the wins

Clay, reading the trend chart: "the recent numbers are mind-blowing.
if we can just get that binary size back down to where it was, the
recent wins on compile memory and run instructions are just
astonishing." So binary size is the next priority target: return it
to its earlier level WITHOUT giving back the run-instructions and
compile-memory gains. The chart's shape is a lead — the size spike
lands beside the big run-instructions drop, which is what a speed win
bought with inlining or specialization looks like — so the first move
is attribution (which change grew the emission, and whether the
growth is the win's cost or a separable side effect), not a revert.
If the two turn out inseparable, that is a welfare trade to present
with numbers, and the weights arbitrate as always.

## 2026-08-29 — the binary-size directive, softened to its real shape

Clay, immediately after: "a slightly larger binary size is a pretty
small price to pay. i just want to make sure it's not a fixable
issue." So the directive is attribution, not a target: find which
change grew the emission and answer one question — incidental or the
price? If the growth is separable from the wins (a duplicated
specialization, a dropped dedup, dead emission), fix it. If it is the
genuine cost of the run-instructions and compile-memory gains, keep
it and record the trade; no campaign to force the number back down.

## 2026-08-29 — the memory repo never existed; the phantom is cut out

Task #146's mystery closed with a screenshot: the kanso-lang org holds
kanso, kq, kanso-json, vse and homebrew-tap — no `memory`. The
"Clay's memory, in a container" section of CLAUDE.md, and
.claude/sync-memory.sh beside it, described machinery whose one
manual step (creating the private repo) was never run. Every session
that "was refused the attach" was asking for a repository that does
not exist, and then reporting itself as running without instructions
that also do not exist. CLAUDE.md now states the truth — this file is
the complete instruction set a container gets — and the sync script
is deleted. #146 closes: there was never anything to attach. If a
cross-project memory is ever really wanted, it starts by actually
creating the repo, and the section comes back with it.
## 2026-08-30 — the last per-line scratch, and a decision filed

```
compile_allocs   35,639 -> 34,945   -694   -1.9%
compile_peak_bytes         730,120  unmoved
```

```
compile_instructions  48,743,776 -> 48,582,098   -161,678   -0.33%
```

The allocator rows are the whole of it — `_int_free` 2,284,655 -> 2,210,015,
`malloc` 1,588,013 -> 1,529,690, `_int_malloc` 2,808,434 -> 2,788,486, `free`
998,200 -> 978,768, `__rust_alloc` 755,343 -> 739,381 — and `lex_line` is
byte-identical at 777,023, which is right: the vector this removes was built in
the loop AFTER lexing rather than inside it. Welfare 86.66 -> 86.73, ratcheted
here.

`check_needless_continuation` groups a line's tokens by the source line each one
came from, to ask what the statement would measure written one line wide. It
built a `Vec<(usize, usize, Span)>` to do the grouping — per line of every file
compiled, for a scratch that dies at the end of the call, and for most lines
after one push and the early return two statements later.

One buffer for the file, cleared per line, the same treatment as #1149's gather
vector in `callee_first`. Its two neighbours in the same loop were read and left
alone: `check_partial_chain` and `validate_spacing` allocate nothing.

That is the end of the per-line and per-declaration scratch family. Over five
changes today the front end went from 46,998 allocation blocks to 34,945, a
quarter of them gone, and what is left at the top of the map is one thing.

### The decision that is left

design/pending-gavels.md gains **"Whether an identifier's name lives inline"**,
under Open, not blocking. The short version: `Expr::Ident`'s `String` and the
`Tok::Ident` it is cloned from are 6,983 blocks, **19.6% of every allocation the
front end makes**, and the two treatments measure very differently on the
library's own sources — 11,870 identifier occurrences, 1,399 distinct, and
**11,869 of the 11,870 are 22 bytes or shorter**. Interning removes the 88% that
are repeats and was declined in #1033 at 365 conversion sites; an inline name
removes 99.99% and touches only the ninety construction sites, because reads go
through `Deref`.

It is filed rather than built because it changes the type of a core AST field
across the compiler and means hand-writing the small-string type, and because
the other treatment of the same row was refused once already. The entry carries
the measurement, the site counts and a recommendation.

## 2026-08-30 — the last slash, found by hand

```
compile_instructions  48,582,098 -> 47,302,573   -1,279,525   -2.63%   (runner)
                      48,942,128 -> 47,651,194   -1,290,934   -2.64%   (local)
compile_allocs                       34,945   unmoved
compile_peak_bytes                  730,120   unmoved
front_end_rounds 40, front_end_visits 16,806   unmoved
welfare                       86.73 -> 86.79
```

The two hosts agree to within one per cent of the fall, which is the closest
they have come on this vein.

`memrchr` and `__memcmp_avx2_movbe` both leave the profile's top fifteen, and
`parser::parse` enters it at 593,309 having not moved at all. `lex_line`,
`eval_expr` and every allocator row are byte-identical.

A kanso name is qualified with a slash — `json/decode`, `std/io/read_file` — and
eighteen places in the compiler split one on the last slash to get at its owner
or its bare half. Every one of them wrote `name.rsplit_once('/')` or
`path.rfind('/')`, and those go through `str`'s `CharSearcher`, which lands in
`memrchr`'s vector path.

That path is built to scan kilobytes. The average identifier in `lib/json` and
the standard library it imports is **five bytes** — the length histogram over
11,870 occurrences puts 7,589 of them at four bytes or fewer — so the setup is
most of what a call costs, and there is nothing for the vector loop to do.

`ast::last_slash` is a backward byte loop, and `ast::split_qual` is the pair it
returns. A `/` is one byte and cannot appear inside a multi-byte character, so
the scan is over bytes and the index it hands back is a character boundary.

### What the profile said, and what it missed

`callgrind --separate-callers=2` put 794,000 instructions on three callers:
`mentions_in_expr` at 377,352, `provenance::package_of` at 209,540 and
`provenance::Walk::expr` at 206,628. Converting exactly those three measured
**1,095,492** — three hundred thousand more than the rows named, because the
searcher's own construction is charged to the caller rather than to `memrchr`.
Converting the other fifteen took it to 1,290,934.

That is the third time in two days the map has been read one way and the fix
measured another. Twice the rows read HIGH — `Tok::clone`'s 3,788 returned 196,
and `infer.rs:590` turned out to be a hash set rather than the call it was
attributed to. This one read LOW, and for the same underlying reason: a
profile's rows are where the cost was charged, and the fix moves whatever the
charge was standing in for.

### What did not move

Allocations, peak, rounds and visits are all identical, and so is the emitted
golden. Nothing here changes what the compiler decides or writes — only how it
finds a byte it was already looking for.

## 2026-08-30 — an answer the fixpoint recomputed forty times

The provenance pass runs a fixpoint over the call graph, and on `lib/json` it
takes forty rounds. Every round walked every declaration, and the first thing
it did for each was ask which package the declaration belongs to:

```rust
let pkg = bit(&table, package_of(&decl.file));
```

`package_of` splits a path on `.hako/` and then on its last slash; `bit` walks
the interned package table comparing strings until one matches. A declaration's
file does not change between rounds and neither does the table, so the answer
is fixed from the moment `intern` finishes. It was computed forty times anyway,
once per declaration per round.

It is a `Vec<Pkgs>` built before the loop now, indexed by zipping the
declarations against it. The `binds` map each declaration fills with its
err-carrying locals moved out too — it was a fresh `HashMap` per declaration
per round, and clearing one costs nothing where allocating a table costs a
block.

### The searcher under `package_of`

With the recomputation gone, `package_of` still ran twice per declaration —
once in `intern`, once building the table above — and callgrind still charged
108,446 instructions to its own row. `StrSearcher::new`, the setup for
`split_once(".hako/")`, had a row of its own at 230,143 with
`core::slice::memchr::memchr_aligned` at 171,128 under it: the searcher cost
more than twice the function it served, beside it rather than inside it.
Two-way substring search is built for haystacks
that justify its preprocessing; a source path is forty bytes and the needle is
absent from every one of them in a program that fetched nothing, so the search
never gets past its own setup. `after_hako` is a byte scan that tests the first
character before comparing the rest, and `.hako/` is ascii, so the index it
returns is a character boundary.

Reading the row afterwards is instructive in the same way #1154 was:
`package_of` went UP, 108,446 to 130,437, because the searcher's cost moved
from its own row into the caller that now does the work inline. The program
went down 315,908.

### The numbers

Measured in the box, container host, same binary either side:

| | instructions |
|---|---|
| main | 47,651,194 |
| the package table hoisted | 47,194,917 |
| the byte scan as well | 46,878,322 |

−772,872 in total, −1.62%. Allocations 34,945 → 34,812, the `binds` tables.
Peak, rounds and visits are unchanged: the pass decides exactly what it decided
before, and the emitted golden is byte-identical.

### The runner's number

    compile_instructions  47,302,573 -> 46,616,238   -686,335   -1.45%  (runner)
                          47,651,194 -> 46,878,322   -772,872   -1.62%  (local)
    compile_allocs            34,945 ->     34,812       -133   -0.38%  (both)
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806     unmoved
    welfare                            86.79 -> 86.83

Copied out of job 99257297594, which also confirmed the allocation and peak
rows to the digit. The two hosts disagree by 0.17 points of the fall — the
container's glibc is the same version but the CPU is not, and the searcher's
setup is where a host divergence would land.

## 2026-08-30 — a hash map for four names

Inference carries the locals a declaration has bound in a
`HashMap<&str, Set>`, and asks it a question for every identifier in every
body it walks. `callgrind --separate-callers=2` put three rows on it:

    668,032  HashMap::get'infer::eval_expr'infer::infer
    479,552  HashMap::insert'infer::bind_pattern'infer::infer
    359,917  HashMap::contains_key'infer::eval_expr'infer::infer

1.51 million instructions, 3.2% of the compile, for a map that is almost
always tiny. Instrumented, `lib/json`'s deepest scope holds **seven** entries
and the average lookup would walk 2.39 of them. The repo's own kanso programs
agree: `scripts/welfare` peaks at 30 and averages 2.89, `prose_check` at 11 and
2.77, `fingerprint` at 11 and 2.70, `trend_gate` at 16 and 2.41. The mean stays
under three even where the maximum is thirty, because the name a body asks
about is usually the one it just bound.

So `Env` is a `Vec<(&str, Set)>` read back to front. A bind pushes rather than
replaces, which gives shadowing for free and cannot grow past the bind sites
written in the declaration. Lookup answers exactly what the map answered.

### The clone was the point

Every child scope — a block, a lambda's body, a guard's untaken side, an
applied lambda's beta step — took `env.clone()`, and a `HashMap`'s clone
allocates a table and rehashes every entry into it. A `Vec`'s clone is a
memcpy.

The first cut of this regressed allocations by 216, which is the sort of thing
the counter is for. `Vec::clone` sizes to what it copied, so the child's first
bind reallocated; the map's clone had carried its source's load-factor headroom
and usually did not. `Env::child(extra)` takes the count the caller already
knows — a block's statement count, a lambda's parameter count — and allocates
once. Allocations come back to identical.

### The numbers

    compile_instructions  46,878,322 -> 45,764,825  -1,113,497  -2.38%  (local)
    compile_allocs                        34,812   unmoved
    compile_peak_bytes                   730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

The three rows above held 1.51 million and the change returned 1.11 million, so
this one read HIGH — the opposite of #1154, and for the same reason in reverse:
a `HashMap::get` row is the whole lookup, where the searcher rows were only
part of one. The vector's own scan is what is left.

### The runner's number

    compile_instructions  46,616,238 -> 45,468,261  -1,147,977  -2.46%  (runner)
                          46,878,322 -> 45,764,825  -1,113,497  -2.38%  (local)
    compile_allocs                        34,812   unmoved, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts
    front_end_rounds 40, front_end_visits 16,806   unmoved

Copied out of job 99264116335. The runner reads the fall three per cent larger
than the container does, the two hosts' closest agreement on this vein after
#1154's one per cent.

## 2026-08-30 — a hash per identifier, to ask about thirty-one names

`prune_unused_getters` drops the field getters nothing reads. It builds a set
of every name any non-getter body mentions, then asks that set, once per
getter, whether the getter's name is in it. `callgrind --separate-callers=2`
charged the building 540,464 instructions plus 84,064 in `memcmp`, the largest
single hashing caller in the compile.

Instrumented, the numbers are lopsided. `lib/json`'s largest module walks
about twelve thousand identifier occurrences into a set that ends up holding
**325 distinct names**, and asks it about **31 getters**. Almost every insert
is a duplicate of one already there, and almost every entry is a name no
getter could have.

A getter is synthesised in exactly one place, as `Get_{field}`, and the binder
that makes `is_getter` true — `Read` — is one no source can spell. So a
declaration is a getter only if it came from there, and a name that is not
`Get_`-prefixed can never be one. The walk tests four bytes and skips the
insert.

A qualified mention reaches the same getter under its bare half, so the test
is on the bare half and both halves go in together when it passes. The set the
question is asked of is unchanged; what changed is everything that was never
going to be asked about.

The reservation went with it. It was `program.fns.len() * 2` — 864 entries for
the largest module — sized for a set that held one name per mention. It now
holds at most two per getter.

### The numbers

    compile_instructions  45,764,825 -> 45,155,678  -609,147  -1.33%  (local)
    compile_allocs             34,812 ->     34,804        -8
    compile_peak_bytes                     730,120   unmoved

The two halves were measured separately against #1155's head, where the pair
came to 603,447: the prefix test was 601,800 of it, and dropping the
reservation was the remaining 1,647 and all eight allocations.

### The runner's number

    compile_instructions  45,468,261 -> 44,862,145  -606,116  -1.33%  (runner)
                          45,764,825 -> 45,155,678  -609,147  -1.33%  (local)
    compile_allocs                        34,804   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99268499869. The two hosts read the same percentage.

## 2026-08-30 — thirteen tables that start at nothing

`check_merged` runs twenty-two whole-program checks, and a dozen of them open
by grouping declarations: arities by name, return sets by group, discarded
positions, handled positions, torn arms, the reference graph. Each builds its
own map with one entry per declaration, and each starts that map empty.

A `hashbrown` table doubles, and doubling means allocating the new table and
rehashing every key already in the old one. Filling a table to twelve hundred
entries from zero pays that seven times. `callgrind` charged
`reserve_rehash` 654,170 instructions, and `fallible_with_capacity` under it
another 190,355.

Thirteen of these now open at `program.fns.len()`. Two are keyed by
`(name, arity, position)` and can hold more than that, so they still grow
once; the rest never grow at all.

### Not the same verdict as the last time

Pre-sizing was measured and DECLINED on 2026-08-30 for the six
`filter().map().collect()` sets in `check.rs`, at 4,514 instructions and one
allocation. That decline stands and this is not in tension with it: those
collects are FILTERED, so their results are a small fraction of the
declarations and the table they build never doubles more than once. These
thirteen hold one entry per declaration.

### The numbers

    compile_instructions  45,155,678 -> 44,778,375  -377,303  -0.84%  (local)
    compile_allocs             34,804 ->     34,682      -122
    compile_peak_bytes                     730,120   unmoved

Peak does not move, which is the answer to the obvious worry: a table sized to
what it will hold occupies no more than one that grew into the same size, and
the intermediate tables it does not build are the allocations that went away.

### The runner's number

    compile_instructions  44,862,145 -> 44,483,421  -378,724  -0.84%  (runner)
                          45,155,678 -> 44,778,375  -377,303  -0.84%  (local)
    compile_allocs                        34,682   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99272811720.

## 2026-08-30 — the other direction

#1154 replaced eighteen backward slash searches with `ast::last_slash`, a
plain byte scan, and took 1,279,525 instructions off the compile. It left the
forward family alone. There are twenty-four of those: `name.contains('/')`
across `advisory.rs`, `check.rs`, `lib.rs` and `trmc.rs`, `split_once('/')`
in `used_quals`, and four `rsplit('/').next()` chains that want the last
segment and pay a full pattern search to get it.

`contains` and `split_once` on a `char` go through `memchr` the way the
backward pair went through `memrchr`, and the argument is the same: the
average identifier in the shipped library is five bytes, and a routine built
to scan kilobytes spends most of that setting itself up.

`ast` gains `first_slash`, `has_slash`, `split_module` and `bare_name`, and
every one of the twenty-four sites reads one of them.

### Why `split_module` is not `split_qual`

They cut in different places and mean different things. An owner is everything
before the LAST slash — `std/net/http/get` belongs to `std/net/http` — which
is what `split_qual` gives. A qualifier is the FIRST segment, the module name
an import wrote, so `used_quals` credits `std`. Folding the two together would
have been the kind of change that passes every test and quietly credits the
wrong import.

### The numbers

    compile_instructions  44,778,375 -> 44,491,145  -287,230  -0.64%  (local)
    compile_allocs                        34,682   unmoved
    compile_peak_bytes                   730,120   unmoved

Smaller than #1154's, and it should be: two of the twenty-four run per
identifier occurrence and the rest run once per declaration.

### The runner's number

    compile_instructions  44,483,421 -> 44,130,291  -353,130  -0.79%  (runner)
                          44,778,375 -> 44,491,145  -287,230  -0.64%  (local)
    compile_allocs                        34,682   unmoved, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99276951052. The runner reads the fall a fifth larger than
the container does, the widest the two have parted on this vein today; both
directions of the slash family are pattern-search setup, and setup is where a
host divergence lands.

## 2026-08-30 — DECLINED: the ascii fast path in the scanner

`Scanner::new` turns a line of source into the `Vec<char>` the lexer reads with
`chars.extend(content.chars())`, and `Chars` decodes every character whether or
not there is anything to decode. `str::is_ascii` answers in one pass over eight
bytes at a time, and when it says yes the widening loop is straight-line. The
row it was aimed at: 298,362 instructions on the fold under `lex_line` and
242,520 on the extend beneath it.

Measured against #1156's head it took 150,938 instructions. Measured against
this branch's head, four merges later, the same patch takes **53,017**:

    compile_instructions  44,491,145 -> 44,438,128   -53,017  -0.12%
    compile_allocs             34,682 ->     34,680        -2
    compile_peak_bytes        730,120 ->    730,332      +212

Nothing about the patch changed. What changed is everything around it — the
getter prefix test, thirteen sized tables, twenty-four byte scans — and with
them the inlining and layout the lexer's own code gets. **A micro-change
measured against a moving baseline is measured once, and the number is only
about the tree it was taken in.** This one lost two thirds of itself while
sitting in a queue.

Peak goes UP by 212 bytes, and not because of the fast path itself: a byte
slice's iterator reports an exact size where `Chars` reports a lower bound of a
quarter of the length, so `extend` reserves exactly what it needs and the
pooled buffer settles at a capacity the doubling schedule would not have
reached. Replacing `extend` with a plain `push` loop to dodge the size hint
made it worse on both counts — 44,557,543 instructions, one more allocation,
and the peak still up at 730,312.

Welfare reads 86.94 either way. The sum does not move, the memory term pays,
and the rule is that a term getting worse is only defensible when the sum goes
up. So this is declined, and it stays declined unless the lexer is reworked in
a way that makes the decode itself the question.

## 2026-08-30 — DECLINED: the operator table's two-character key

`lex_line` searches `OPS` by text, and built the text to search for out of the
current character and its successor:

```rust
let two = [c, s.peek(1).unwrap_or(' ')].iter().collect::<String>();
```

A two-character `String` is a heap allocation, made and dropped once per
character that reaches this line. Removing it removes **163** allocations from
`lib/json`'s compile — and costs instructions:

    the String, as it stands                44,491,145   34,682 allocs
    a stack buffer and `str::from_utf8`     44,552,688   34,519
    a byte compare against `op.as_bytes()`  44,521,443   34,519
    the table read as a `match (c, next)`   44,521,899   34,519

Three shapes, one answer: whatever replaces the `String`, the compile does
about thirty thousand instructions MORE. The search is not the cause — the byte
compare and the match agree to within five hundred, and they search in quite
different ways. Something about the surrounding code compiles differently once
the allocation leaves, and 163 allocations at roughly two hundred instructions
apiece do not pay for it.

Welfare prices the trade at +0.01, because allocations and instructions share a
dimension and the allocation fall is proportionally the larger. That is a real
gain by the objective and it is also inside the noise of what a day's merges do
to this file's inlining — the ascii scanner above lost two thirds of its value
to exactly that. Declined on those grounds: a change that makes the compiler do
more work, cannot say where the work went, and buys a hundredth of a point is
not worth the line it changes.

The row it was aimed at is still there and still worth a better idea:
`String as FromIterator<&char>` under `lex_line` is 361,176 instructions with
another 156,728 in `finish_grow` beneath it, from the four sites that build a
token's text out of the scanner's `Vec<char>` one character at a time. The fix
for those is not a faster copy; it is a scanner that keeps the source `&str`
and hands out slices of it, which is the shape gavel "whether an identifier's
name lives inline" is about.

## 2026-08-30 — DECLINED: the precedence ladder, and two fixtures that say why

Expression parsing descends ten levels — pipe, join, or, and, not, cmp, bits,
add, mul, app, atom — and on `lib/json` the descent itself costs **681,679**
instructions, 1.53% of a compile:

    parse_pipe   2,963,379  inclusive
    parse_atom   2,281,700  inclusive

Everything between is a frame entered to find no operator of its precedence.
Precedence climbing collapses the middle into one frame with a binding-power
table, and that is the standard answer.

It is the wrong answer here, and two programs say so.

    print "{6 < 3 & 1}"     error[syntax]: unexpected trailing tokens
    print "{1 + not true}"  error[syntax]: expected an expression

`parse_cmp` takes its left side from `parse_bits` and its right side from
`parse_add`, one rung TIGHTER, so `&` may stand to the left of a comparison
and not to the right. `6 & 3 < 1` is `(6 & 3) < 1` and answers false; `6 < 3 &
1` has nothing to attach the `& 1` to. And `parse_not` is reachable only where
an `and` operand is expected, six rungs looser than an arithmetic operand, so
`1 + not true` has no expression after the `+`.

A single binding-power table makes both of those symmetric. `6 < 3 & 1` would
become `6 < (3 & 1)` and `1 + not true` would parse, and neither change would
show up anywhere: no fixture covered either, so the whole differential corpus,
the error corpus and 77 suites would have stayed green while the language
quietly grew two readings it does not have.

So the rewrite is declined, and the gap it exposed is closed instead:

- `tests/golden/errors/a_comparison_refuses_a_bitwise_tail`
- `tests/golden/errors/an_arithmetic_operand_refuses_not`
- `tests/golden/micro/bitwise_binds_tighter_than_a_comparison`

Two refusals and the reading that survives. If the ladder is ever regularised
these three go red, which is the point: whether the grammar SHOULD be uniform
here is a question for the gavel, and a performance change is not the place to
answer it.

### Watched fail, each for its own reason

A fixture written after the fact is a guess about what the code does, so each
of the three was put in front of the mutation it exists to catch:

| mutation | fixture | what it did |
|---|---|---|
| `parse_cmp`'s right side from `parse_bits`, symmetric | a_comparison_refuses_a_bitwise_tail | printed `false`, exit 0, where the golden is a syntax error and exit 2 |
| `not` admitted as a prefix in `parse_app` | an_arithmetic_operand_refuses_not | reached the runtime and failed on `+`, exit 1, where the golden refuses at parse time |
| `parse_cmp`'s left side from `parse_add` | bitwise_binds_tighter_than_a_comparison | `6 & 3 < 1` became a syntax error where the golden is `false` |

The first mutation is exactly what a binding-power table would do. It costs
nothing to write and the corpus said nothing about it until now.

## 2026-08-30 — a vector per name, for a table that is only read

dhat's map of the front end's remaining 34,682 allocation blocks put 937 of
them on one line: `infer.rs:287`, the call to `callee_first`. That function
builds the callee-first visit order the fixpoint sweeps in, and it opens with

```rust
let mut by_name: HashMap<&str, Vec<usize>> = ...;
for (i, decl) in program.fns.iter().enumerate() {
    by_name.entry(decl.name.as_str()).or_default().push(i);
}
```

which is one `Vec` per DISTINCT declaration name. `lib/json` has about nine
hundred of them, the table is built once and afterwards only read, and every
one of those vectors holds a handful of `usize`.

It is a flat `Vec<usize>` and a range per name now, the shape #1140 gave the
dispatch table and #1150 gave the call table. The arms of a name land in it by
a counting pass: count per name, turn the counts into starts, then walk the
declarations once placing each index at its name's cursor. Within a name the
indices come out ascending, which is the order `push` gave them, so the flat
callee list the fixpoint reads is byte-identical.

### The sort that did not work

The obvious way to group is to sort `(name, index)` pairs, and that was
measured first: allocations fell by 519 and instructions rose by **117,356**.
Twelve hundred string comparisons through a comparison sort cost more than the
nine hundred allocations they removed. The counting pass does the same
grouping with two hash lookups per declaration and no comparisons.

    the sort               44,608,501   34,163 allocs
    the counting pass      44,427,648   34,158 allocs
    as it stood            44,491,145   34,682 allocs

### The numbers

    compile_instructions  44,491,145 -> 44,427,648   -63,497  -0.14%  (local)
    compile_allocs             34,682 ->     34,158      -524
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

Both terms move the right way, which is what makes this one uncomplicated:
welfare 86.94 -> 86.99.

### Where the rest of the blocks are

The same map, for the record, since two of the top four are gavel territory:

    3,197  parser.rs:2127   the `String` in `Expr::Ident`, charged at parse_atom_base
    3,157  lexer.rs:631     the `String` `lex_word` returns
    1,100  parser.rs:2114   an application's argument vector, reserved and DECLINED
    1,067  parser.rs:2121   the `Box` on an application's head
      959  lib.rs:607       the prelude and the synthesised getters
      937  infer.rs:287     this entry
      644  trmc.rs:180      a `Vec` per group, the same shape as this one

`trmc.rs:180` is left alone deliberately: its map is ITERATED to build the
rewritten arms, so the order the table hands its keys back is part of what the
compiler emits, and changing the value type changes that order. The emitted
golden would catch it, but a reordering is not what 644 allocations are worth.

### The runner's number

    compile_instructions  44,130,291 -> 44,100,285   -30,006  -0.07%  (runner)
                          44,491,145 -> 44,427,648   -63,497  -0.14%  (local)
    compile_allocs                        34,158   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99289426525. The runner reads less than half the fall the
container does, the widest the two have parted all day — every earlier change
today agreed within a fifth. What differs between the hosts here is the cost
of an allocation, and this change is almost entirely allocations: the
counting pass and the `or_default().push` do nearly the same number of hash
operations, so what is left is nine hundred mallocs, and a malloc is exactly
the thing whose price is a property of the allocator and the machine rather
than of the compiler. The allocation row, which is not host-dependent, agrees
to the digit.

## 2026-08-30 — two more tables that allocate a vector per group

#1161 did this to `callee_first`. The same shape was in two more places, and
between them they were worth four times as much.

**`infer.rs:204`** builds the dispatch groups the fixpoint reads:

```rust
let mut by_group: HashMap<(&str, usize), Vec<usize>> = ...;
for (i, decl) in program.fns.iter().enumerate() {
    by_group.entry((decl.name.as_str(), decl.params.len())).or_default().push(i);
}
let mut group_members: Vec<usize> = Vec::with_capacity(program.fns.len());
let groups: HashMap<(&str, usize), (u32, u32)> = by_group.into_iter().map(...).collect();
```

The flat vector and the ranges were already the destination. The per-group
`Vec` existed for one line, to be poured into `group_members` and dropped —
529 blocks whose only job was to be flattened. The counting pass writes
straight into the flat vector.

**`check.rs:2757`**, `check_constants`, collects each run of consecutive
same-named declarations into a `Vec<&FnDecl>` to ask three questions of it:
whether any arm takes no parameters, how many arms there are, and where the
second one is. All three read off a slice of `program.fns`, so the run is
walked in place.

### The numbers

    compile_instructions  44,427,648 -> 43,955,167  -472,481  -1.06%  (local)
    compile_allocs             34,158 ->     32,856    -1,302   -3.8%
    compile_peak_bytes                     730,120   unmoved
    front_end_rounds 40, front_end_visits 16,806   unmoved

dhat's map said 529 and 579, and 1,302 went away. The gap is the runs: dhat
charges an allocation to the line that made it, and `vec![decl]` at
`check.rs:2762` was reached once per RUN while the map's line was reached once
per group — the map undercounts a site whose work is spread over a loop it
does not name. The direction was right and the size was not, which is the
third time this week the map has been read one way and the change measured
another.

Welfare 86.99 -> 87.13, the largest single step the compile vein has taken
since the counters went in.

### The runner's number

    compile_instructions  44,100,285 -> 43,618,236  -482,049  -1.09%  (runner)
                          44,427,648 -> 43,955,167  -472,481  -1.06%  (local)
    compile_allocs                        32,856   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99293723322. The two hosts agree within two per cent of the
fall — where #1161, which removed only mallocs, had them a factor of two
apart. This change removes hash operations and vector copies as well as
mallocs, and those cost the same on both.

## 2026-08-30 — eight changes, and what they did to gavel #159

#1155 through #1162 all went at the front end, each small enough to state in a
sentence. Together:

    compile_instructions  47,302,573 -> 43,618,236  -3,684,337  -7.79%  (runner)
    compile_allocs             34,945 ->     32,856     -2,089   -5.98%
    compile_peak_bytes                     730,120   unmoved across all eight
    welfare                     86.79 ->      87.15

The falls came from four shapes, and the queue is now out of instances of all
four.

**A decision hoisted out of a loop.** `package_of` ran once per declaration per
fixpoint round; it runs once per declaration now, and the `.hako/` search that
fed it is a byte scan rather than a substring searcher (#1155, -686,335).

**A map replaced by a vector when the map was small.** Infer's scope holds a
handful of names and was a hash map; it is a flat vector with a reverse linear
scan (#1156, -1,147,977).

**A necessary condition tested before the expensive one.** Only `Get_`-prefixed
names can name a getter, so `prune_unused_getters` no longer builds a set of
every identifier in the module to ask about thirty-one of them (#1157,
-606,116).

**A table of per-group vectors flattened by a counting pass.** Count per key,
turn the counts into starts, then place each item at its key's cursor: one
`Vec` and a range per key instead of a `Vec` per key. Three tables took it —
`callee_first` in #1161, `by_group` and `check_constants` in #1162 — for
512,055 instructions and 1,826 blocks between them.

Two changes were the same idea applied twice with no cleverness: thirteen maps
in `check.rs` sized at construction (#1158, -378,724) and twenty-four
`find('/')` calls replaced with a forward byte scan (#1159, -353,130).

Three things were built, measured and declined, each with its number: the ascii
fast path in `Scanner::new`, the operator table's two-character `String`, and
the precedence-climbing rewrite of the expression parser. The last was worth
681,679 instructions and was declined because two of the grammar's precedence
asymmetries were observable and unpinned; #1160 pinned them with three
fixtures, so the decline is a decision somebody can revisit rather than a
hazard.

### What is left

dhat on the new head. The three rows gavel #159 names have not moved a byte
across any of the eight:

```
3,197  parser.rs:2127  Expr::Ident's String, cloned out of the token
3,157  lexer.rs:631    the same String, built by lex_word
  629  parser.rs:1793  Pattern::Var's String, same source
------
6,983
```

The share moved because the denominator did. These were 19.6% of 35,643 blocks
when the gavel was filed and they are 21.3% of 32,860 now. Below them the map
is a long tail: 1,100 at parser.rs:2114 (the App args vector, measured and declined
earlier today), 1,067 at parser.rs:2121, 959 at lib.rs:607, then nothing over
750. `trmc.rs:180` (644)
is deliberately left alone — its map is iterated to build the rewritten arms,
so key order is part of what the compiler emits.

The gavel entry has been refreshed with the current share. Nothing else in it
changed: the two treatments measure the way they did, and the ruling is still
Clay's.

## 2026-08-30 — three sets opened once per declaration

`check.rs` walks `program.fns` in three separate passes, and each pass opened a
fresh set of the names the declaration binds before walking its body:

```rust
for decl in &program.fns {
    if decl.synthetic { continue; }
    let mut bound: HashSet<&str> = HashSet::default();
    ...
}
```

hashbrown allocates nothing for an empty set, so the cost lands on the first
insert — which every declaration that binds anything reaches. The capacity the
set had just grown to is dropped with it, so the next declaration starts from
nothing and grows again.

One set per pass, cleared at the top of each iteration, keeps that capacity and
allocates once. The names borrow from `program`, which outlives the loop, so
the lifetime the hoist needs is the one the set already had. The three passes
are `check_call_shaped_list`, `check_call_arities` and
`check_literal_arguments`.

### The numbers

    compile_instructions  43,955,167 -> 43,448,641  -506,526  -1.15%  (local)
    compile_allocs             32,856 ->     31,316    -1,540   -4.7%
    compile_peak_bytes                     730,120   unmoved

The instruction fall is larger than 1,540 mallocs would explain on its own: a
set that starts empty rehashes as it fills, and one that starts at the capacity
the previous declaration needed usually does not.

dhat put 726 and 732 on two of the three and the change removed 1,540, so this
time the map's arithmetic held. It has been read one way and the change
measured another three times this week; it is worth writing down when it
agrees.

### And a guard asked one line too late

`synthesize_getters` built each candidate arm's pattern vector — one `Vec` and
a `String` for the binder — and then asked whether that (type, field) already
had a getter and threw the arm away. The question moved above the construction.
It is worth 62 allocation blocks on `lib/json`, where most fields do not
already have one; on a program built mostly of imported types it is worth more.
Recorded here at its measured size rather than its imagined one.

### The runner's number

    compile_instructions  43,618,236 -> 43,096,058  -522,178  -1.20%  (runner)
                          43,955,167 -> 43,448,641  -506,526  -1.15%  (local)
    compile_allocs                        31,254   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99301368868. Three per cent apart on the fall — the saving is
a malloc, a free and the rehashes a set that starts empty pays on the way up,
and all three cost about the same on both hosts.

`HashMap<&str, ()>::insert` is now the largest kanso-owned row after inference
at 1,517,265 (3.52%), where the sets that just went away were feeding it.

Welfare 87.15 -> 87.32, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — three more tables that allocate a vector per key

The counting pass of #1161 has now taken six tables. Three were left in
`check.rs`, and they are the last of the shape in the front end.

**Two arity tables.** `check_call_arities` and the shadow checker's `Declared`
each built a `HashMap<&str, Vec<usize>>` of the distinct arities a name is
declared at — a heap allocation per name, for a list that holds one number in
almost every case. They share an `Arities` now: count the arms per name, turn
the counts into starts, then place each declaration's arity at its name's
cursor, skipping one already there. The counts bound the ranges from above, so
a name declared three times at one arity leaves two cells unread between it and
the next name; nothing iterates the flat vector whole, so the gaps cost
nothing to carry.

**The overlap checker's groups.** `check_overlapping_arms` collected each
dispatch group into a `Vec<&FnDecl>` to compare its arms pairwise. The pairwise
loops index a range of the flat vector instead.

### The numbers

    compile_instructions  43,448,641 -> 43,353,124   -95,517  -0.22%  (local)
    compile_allocs             31,254 ->     30,406      -848   -2.7%
    compile_peak_bytes                     730,120   unmoved

Split: the two arity tables were 499 blocks and 42,087 instructions, the
overlap groups 349 and 53,430. The group table costs more per block because its
`Vec` was reallocated as arms were pushed, where an arity list took one
allocation and stopped.

dhat said 353 for the group table and 366 + 270 for the two arity tables. The
group row was right to the block; the arity pair came in at 499 against 636,
because part of what those rows held was the `fields` and `type_arity` maps
beside them, which have not moved.

Two of the shape remain: `check_literal_arguments`' `Vec<&FnDecl>` per group,
and `advisory.rs`'s `Vec<usize>` per name. Both are read through walks that
take the table as a parameter — five signatures in advisory's case — so
flattening either means threading the flat vector beside the ranges and giving
the lookup key a lifetime the walk can name. Left for now, with the reason.

### The runner's number

    compile_instructions  43,096,058 -> 43,001,117   -94,941  -0.22%  (runner)
                          43,448,641 -> 43,353,124   -95,517  -0.22%  (local)
    compile_allocs                        30,406   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99303996583. Six tenths of a per cent apart on the fall,
the closest two hosts have come on this vein.

Welfare 87.32 -> 87.41, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — the log is forty entries again

`design/compiler-log.md` had reached 111 entries and 6,352 lines against a
stated cap of forty. Seventy-one entries move to
`design/log/compiler-log-archive.md`, unedited and in the order they were
filed; the live file is 2,732 lines and forty entries counting this one.

The archive gains a note at the seam. The eight entries dated 2026-08-26 that
open the moved run were written on a branch that did not merge until
2026-08-28, so they sit after the 2026-08-27 entries already in the archive.
The live log's header carried that explanation while those entries were in it;
it goes with them.

Nothing was edited on the way. The move was verified by reconstruction: the
archive's old content is still a prefix of the new file, and the moved run
concatenated with what stayed is byte-identical to the body that was there
before, whitespace normalised.

`scripts/page_drift` counts `+## ` lines in the log's diff since the page's
last commit, so a trim — which only deletes from that file — cannot move its
number. It reads 0/3 here because the page moved in #1165.

## 2026-08-30 — the last two tables of the shape

#1165's entry named two that were left, and said what stood in the way:
`check_literal_arguments`' `Vec<&FnDecl>` per group and `advisory.rs`'s
`Vec<usize>` per name, both read through walks that take the table as a
parameter. Both are done, and the obstacle was smaller than it read.

**A tuple key cannot be probed with a borrowed name; a `&str` key can.**
`HashMap<&'a str, V>::get` accepts any `&str` because `&'a str: Borrow<str>`,
so a struct with a named lifetime still answers a lookup from a shorter one.
`HashMap<(&'a str, usize), V>` gets no such implementation, which is why the
literal-argument groups had to be rekeyed rather than merely flattened: the
ranges are keyed by NAME, and each arm's arity travels beside its index in the
flat vector, so `arms(name, arity)` slices the name's range and filters. Group
sizes are two or three, so the filter is cheaper than the second hash the tuple
key would have cost.

**advisory's table needed only the flattening.** It was already keyed by name,
and it skips synthetic declarations — so the counts are an upper bound and a
group whose synthetic arms were dropped leaves cells nothing reads, the same
slack the arity tables carry.

### The numbers

    compile_instructions  43,353,124 -> 43,302,778   -50,346  -0.12%  (local)
    compile_allocs             30,406 ->     29,876      -530   -1.7%
    compile_peak_bytes                     730,120   unmoved

Split: the literal-argument groups 349 blocks and 10,222 instructions, the
advisory groups 181 and 40,124. The instruction split runs opposite to the
block split, and the reason is where each table is read. advisory's is asked
once per identifier in every expression the door analysis types; the
literal-argument one is asked only at applications whose callee is a bare
name. A table's cost is the reads, and the allocations are what building it
happened to take.

No table in the front end now holds a `Vec` per key. Eight of them went over
four changes — #1161, #1162, #1165 and this one — for 3,204 allocation blocks
between them: 524, 1,302, 848 and 530.

### The runner's number

    compile_instructions  43,001,117 -> 42,914,281   -86,836  -0.20%  (runner)
                          43,353,124 -> 43,302,778   -50,346  -0.12%  (local)
    compile_allocs                        29,876   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99309718552. The widest the two hosts have been on this vein
since #1161, and in the direction where the runner gains more. Both changes
trade an allocation and a pointer chase for a short linear scan of a flat
vector, and the two hosts price that trade differently — a scan that stays in
cache costs what the machine's cache costs. The allocation row agrees to the
digit either way, which is the point of having both.

Welfare 87.41 -> 87.47, banked with `--set`. The page's two `data-golden`
compile figures move with it.

## 2026-08-30 — the bare-name walk asks a question it already knows the answer to

`mark_bare_quals` decides which import qualifiers a file actually uses. It
walks every expression of every declaration, collects every bare identifier
occurrence into a set, and then asks that set two questions: is each surfaced
name in it, and is any import rename's target in it.

The set therefore holds every distinct bare name in the module, and is asked
about the surfaced names and the rename targets — a much smaller list. A name
outside that list can never answer either question, so the walk tests
membership in the asked-about set before inserting.

This is #1157's move in a different pass. There the necessary condition was a
`Get_` prefix, cheap enough to test on four bytes; here it is membership in a
set built once from the two readers' own inputs. What makes both exact is the
same thing: the condition is on the MENTION, and the readers are known.

### The numbers

    compile_instructions  43,302,778 -> 43,138,593  -164,185  -0.38%  (local)
    compile_allocs             29,876 ->     29,864       -12
    compile_peak_bytes                     730,120   unmoved

The allocation move is twelve blocks, and the instruction move is a hundred and
sixty times that. The set was not costing much to hold; it was costing to fill.
`HashMap<&str, ()>::insert` under `walk_children` under `mark_bare_quals` was
267,972 instructions on the previous head — the largest single insert caller in
the compile — and it is a lookup in a small table now.

### The runner's number

    compile_instructions  42,914,281 -> 42,748,207  -166,074  -0.39%  (runner)
                          43,302,778 -> 43,138,593  -164,185  -0.38%  (local)
    compile_allocs                        29,864   confirmed, both hosts
    compile_peak_bytes                   730,120   unmoved, both hosts

Copied out of job 99313216447. One per cent apart on the fall. A hash lookup
traded for a hash insert is the same trade on both hosts, where #1167's scan
for a pointer chase was not.

Welfare 87.4654 -> 87.4740, banked with `--set` — three hundredths of a point
on the compile-speed term, which is what a 0.39% fall is worth at this end of
the satiation curve. The page's two `data-golden` compile figures move with it.

## 2026-08-30 — two playground tests, one file

`tests/playground.rs` failed once during a full run with
`the interpreter failed on the hello example: a play file needs at least one
statement to run`. The example is one line and runs fine; the file the
subprocess read was empty.

`written` staged every example into one fixed directory,
`kanso-playground-test/<name>.kso`. Two of the three tests in that binary write
files, they run on separate threads of one process, and both write the same
fourteen names. `std::fs::write` truncates and then fills, so a subprocess
spawned by one test can read the file the other is part-way through rewriting.

### The chain, each link measured

- an empty `.kso` produces exactly that diagnostic, byte for byte;
- the truncate window is wide — a reader racing a rewriter on this filesystem
  saw an empty file on 28,055 of 35,205 reads, 79.7%;
- the two tests write the same paths, which is plain from the code.

What the window does not explain on its own is the rate: running the playground
suite alone, twelve times, fails zero times either way. The tests spend almost
all of their time in subprocesses rather than inside `written`, so the two
threads are rarely in that window together — the observed failure came during a
full `cargo test` with a valgrind run and two compiles alongside it. A rare
race is still a race, and it fails on whichever test loses.

One directory per test. The fix is four lines and the argument for it is the
mechanism rather than a failure count, which is the honest way round for a race
this thin.

## 2026-08-30 — a word is a copy, not a re-encode

`lex_word` built its `String` by collecting `&char` out of the scanner's
`Vec<char>`, which encodes each character back into UTF-8 one at a time.
`String::from_iter<&char>` under `lex_line` was 361,176 instructions, 0.84% of
the front end.

The scanner already receives the line as a `&str`; it kept only the `Vec<char>`
because `pos` is the column a caret goes under. It keeps both now, with one bit
saying whether they agree: on a line that is ascii throughout, a character index
is a byte index, so `start` and `pos` slice `src` directly and the word is one
copy. A line with anything wider falls back to the collect, and the check is
`str::is_ascii` once per line — a vectorised byte scan.

### The numbers

    compile_instructions  43,138,593 -> 42,731,199  -407,394  -0.94%  (local)
    compile_allocs                        29,864   unmoved
    compile_peak_bytes       730,120 ->    728,030    -2,090   -0.29%

Peak moves because a `String` built by `to_string` on a slice is allocated at
the length it needs, where one grown a character at a time overshoots. That is
the first fall in `compile_peak_bytes` since #1152, and it was not the point of
the change.

**This is not #1159's declined ascii scanner.** That one made the whole scan
byte-oriented, measured 53,017 instructions by the time it was queued, and cost
212 peak bytes. This changes one line of `lex_word` and leaves the scanner
indexing characters everywhere else.

**Gavel #159 would delete this.** The `String` this builds more cheaply is the
one an inline name removes; if that ruling goes the inline way, `lex_word`
stops building a `String` at all and 407,394 instructions become moot along
with the 3,157 allocations beneath them. Recorded so the next reader knows the
two are the same code and not a contradiction: how a `String` is filled is a
question the compiler can answer today, and whether it exists is Clay's.

### The runner's number

    compile_instructions  42,748,207 -> 42,297,878  -450,329  -1.05%  (runner)
                          43,138,593 -> 42,731,199  -407,394  -0.94%  (local)
    compile_peak_bytes       730,120 ->    728,030    -2,090   both hosts
    compile_allocs                        29,864   unmoved, both hosts

Copied out of job 99320841238. Ten per cent apart on the instruction fall, the
widest the two hosts have been on this vein — a memcpy against a per-character
encode loop is the kind of trade two machines price differently, where a malloc
removed is the kind they agree on. The peak row agrees to the byte, as it always
has.

Welfare 87.4740 -> 87.4996, banked with `--set`. The page's `data-golden`
instruction figure moves with it; the allocation figure does not, because
allocations did not.

## 2026-08-31 — a character count is a population count

`length` on a string counts characters, and `k_str_chars` counted them by
walking: read a byte, decide from it how wide the character is, step that far,
add one. A branch per byte, about 8.9 instructions each on this box.

The shape that pays for it is asking a document its length. encodebench's
harness does exactly that — `rounds v (n - 1) (acc + length (encode v))` —
four hundred times over 188,698 bytes, and the cost golden had the numbers all
along: `str_scans=400`, `str_scan_bytes=75479200`. That one call was
671,356,000 instructions, 6.8% of the benchmark, and it read as
`k_b_length` in the profile rather than as anything the encoder does.

A character's first byte is any byte whose top two bits are not `10`, so the
count is the byte length minus the number of continuation bytes and nothing
has to be decoded. Eight bytes at a time: load a word, mark the bytes with bit
7 set and bit 6 clear, population count. About one instruction per byte.

```
    encodebench  9,866,614,705 -> 9,254,332,066  -612,282,639  -6.206%   (local)
    pendbench      988,282,947 ->   957,236,125   -31,046,822  -3.141%
    oneshot         47,277,156 ->    45,769,889    -1,507,267  -3.188%
    basket          57,392,199 ->    57,117,622      -274,577  -0.478%
    jsonbench    2,910,241,528 -> 2,913,835,115    +3,593,587  +0.123%
    widebench       83,967,604 ->    84,031,191       +63,587  +0.076%
```

**The two rises are one number.** jsonbench makes 898,500 `k_str_chars` calls,
all from `k_b_slice`'s test for whether a string is one byte per character,
and each costs +4.000 instructions — 898,500 × 4 is the whole 3,593,587 to the
unit. The strings are shorter than eight bytes, so the word loop never runs
and they pay only for its guard and the closing subtraction. Traded knowingly:
the four benchmarks that improved are the ones that ask a document its length,
and the decoder's four instructions buy the encoder six per cent.

Three shapes were measured, not two. A plain branch-free byte loop —
`conts += (p[i] & 0xc0) == 0x80`, no word at all — leaves jsonbench exactly
where it was and gives encodebench only 3.55%; clang does not vectorise it,
so it lands at 4.3 instructions per byte instead of 8.9. Disabling
vectorisation on the *word* loop costs 186,305,200 instructions on
encodebench, which is what the SIMD popcount is worth, so it stays. The tail
loop is told not to vectorise: it trips at most seven times, and clang's
widening of it was 368 bytes of prologue per copy against a loop the processor
barely enters.

That last one is the text vein's whole movement: +752 bytes wherever the
function is inlined twice, +368 where once, +1,520 on basket, which holds four
copies. Regenerated with the reason in the file.

**The harness came second, and says so.** `scripts/utf8_differential` already
extracts the validator's text from `src/runtime.c` at run time rather than
copying it; it extracts `k_utf8_chars` the same way now, under a name of its
own, and checks it against a reference in `scripts/utf8/harness.c` that walks
the text a character at a time from the rfc 3629 widths. Two routes to the
same answer on valid utf-8. The sweep is every arrangement of character widths
that fits in twenty-four bytes — three whole words plus the tails either side
of each — and then two hundred thousand valid strings over code points the
four representatives do not reach: 8,346,016 counts, 0 mismatches.

It was watched red before it was believed. Writing the mask as
`w & ~(w >> 1)` gives 7,036,227 mismatches; stopping the tail one byte short
gives 2,079,600. Both are the mistakes actually available in this function.

The counter this kernel is present by is `str_scans` / `str_scan_bytes`, which
already existed and did not move — they count scans and bytes, never words or
lanes, so they say the same thing on a host with no popcount instruction.

`k_chars_list` twenty lines above does the same scan to size the list it is
about to build. Left alone deliberately: it allocates a string per character
straight afterwards, so the count is not what that path pays for.

Not measured, and worth saying: the interpreter counts characters in Rust and
is untouched, so this moves native only. Whether the same shape helps the
front end is a separate question — the compiler asks for character counts too,
and `compile_instructions` will say.

DONE.

## 2026-08-31 — a one-byte append writes the byte

Same PR, and the second time in an hour that a byte-level operation in the
runtime turned out to be paying a call to move less than a register's worth.

`k_b_append_mut` is 23.24% of encodebench on its own: 42,318,000 calls at 54.2
instructions each, and 48,945,694 calls into `__memcpy_avx_unaligned_erms`
beneath it at 17.6 apiece. The encoder's commonest append is one byte — a
comma between elements, a colon between key and value, the brace that opens a
map — and `elem_onto` writes it as `text/append acc 44`. Moving one byte
through libc's dispatcher costs more than the move.

```c
if (n == 1) ((unsigned char*)a->data)[a->len] = *src;
else memcpy((unsigned char*)a->data + a->len, src, (size_t)n);
```

```
    jsonbench    2,913,835,115 -> 2,862,072,365   -51,762,750  -1.777%
    encodebench  9,254,332,066 -> 8,715,312,466  -539,019,600  -5.824%
    oneshot         45,769,889 ->    44,077,255    -1,692,634  -3.698%
    widebench       84,031,191 ->    84,047,191       +16,000  +0.019%
```

The other four rows do not move. **This pays back the character count's
decoder cost and then some**: jsonbench ends the pair at −48,169,163 against
main, −1.655%, where the counter alone had left it +0.123%. The two changes
land together for that reason.

The obvious generalisation is worse. A byte loop for every `n <= 8` — which
would also catch `"true"`, `"null"` and `"[]"` — reads encodebench at
8,878,376,066, jsonbench at 2,874,946,565 and oneshot at 44,570,742: worse
than the single byte on all three, because clang's memcpy for a known-small
dynamic length beats an explicit loop everywhere except at one. Measured, not
assumed.

+32 bytes of text wherever it lands, which is one compare and one store
against the call it replaces.

**And a refuted one, worth writing down because it looks free.** Two counters
on this fast path — `k_stat_append_fast++` and `k_stat_append_grow++` — are
incremented unconditionally, where most of the runtime's statistics sit behind
`__builtin_expect(k_stats_on > 0, 0)`. Putting them behind the same guard
costs +42,323,600 on encodebench and +4,016,400 on jsonbench: exactly +1.0001
instructions per fast append. Loading the flag and branching on it is one
instruction more than incrementing a counter that is already in cache, so the
guard buys nothing and is not a saving waiting to be taken.

The thread the entry above left open is closed too. The front end asks for
character counts as well — `trimmed.chars().count()` runs on every line of
every source file for the eighty-column cap — but rust's std already counts
this way: `core::str::count::do_count_chars` and `char_count_general_case`
together are 99,709 instructions of the front end, 0.23%. Nothing to take.

DONE.

## 2026-08-31 — the runner's numbers for the pair, and what welfare made of them

```
    work_jsonbench    2,910,241,528 -> 2,862,072,778   -48,168,750   -1.655%
    work_encodebench  9,866,614,705 -> 8,715,312,865 -1,151,301,840  -11.669%
    work_oneshot         47,277,156 ->    44,077,654    -3,199,502   -6.767%
    work_pendbench      988,282,947 ->   957,236,511   -31,046,436   -3.141%
    work_basket          57,392,199 ->    57,118,035      -274,164   -0.478%
    work_widebench       83,967,604 ->    84,047,604       +80,000   +0.095%
    work_deepbench, work_escapebench                            unmoved
    compile_instructions 42,297,878 ->    42,297,900          +22
```

**work_widebench is the one rise, and it is the character count's four
instructions.** widebench asks for a string's length on strings shorter than
eight bytes, so every call pays for the word loop's guard and its closing
subtraction and enters neither; nothing in that benchmark appends a single
byte, so the second change has nothing to give it back. jsonbench pays the
same +4 on 898,500 calls and takes 51.8M back from the append, which is why
the two shipped together and widebench is left where it is.

The local box reads deepbench −3,680 and escapebench −399 where the runner
reads neither, and its glibc is 2.39-0ubuntu8.7 against the golden's 8.8. The
runner's rows are the ones in the file.

`compile_instructions` moves 22 and none of it is the front end. `k_utf8_chars`
is twenty lines added to `src/runtime.c`; `main.rs` holds that file as an
`include_str!` and hashes it for the build cache key, and a longer string takes
twenty-two more instructions to hash. Allocations, peak, rounds and visits are
byte-identical.

**Welfare moved 87.50170 to 87.51104 — nine thousandths.** Worth writing down
because the size is surprising: #1170 bought 0.0277 for a 1.05% fall in
compile instructions, and this bought a third of that for an 11.7% fall in
encode. Two things make it so. Runtime satiates at 2.0, so a term already
better than baseline gains little from getting better still — encode crosses
from 5.9% below to 6.5% above and the crossing itself is most of the value.
And the five runtime speed counters share a weight with wide, deep and pending,
whose ratios are 138, 29 and 28 times baseline and therefore contribute almost
their whole allowance already. The model is doing what it was written to do.
Nothing is being argued here; the observation is that the encoder had a lot of
room and the index says the project had already been paid for most of it.

DONE.

## 2026-08-31 — the string index gets the cursor the slice already had

`s[i]` on a string counts characters, so finding the ith one means walking the
text until you have passed i characters. `k_b_at` started that walk at byte
zero every time, which makes reading a string one character at a time
quadratic in its length:

```
    2,000 characters    27,591,142 instructions
    4,000 characters   106,971,148            a doubling for 3.88x the work
```

`k_b_slice` solved this and left the comment saying so — "starting the walk at
zero every time is what made reading a page one character at a time quadratic"
— and it has two shortcuts for it. A string whose characters are all one byte
skips the walk entirely, because then a position is an offset. Anything wider
uses a remembered cursor: one string, one character index, one byte offset, so
the next question resumes where the last one stopped rather than from the
front, walking backward to it when the question moves back.

`k_b_at` had neither. The two are now `k_str_seek`, which the index calls and
the slice does not: the slice needs both ends of a run out of one pass and
would cross the text twice if it went through here. They share the cursor,
which is the part that matters — a sweep alternating `s[i]` and `text/slice`
over one subject stays linear because both hands move it.

```
    2,000 characters     1,690,119   -93.9%
    4,000 characters     3,169,125   -97.0%    a doubling for 1.875x the work
```

**The corpus had no home for this, which is why it survived.** Two were added.

`bench/indexbench` reads a string of twenty thousand characters forward, one
index at a time, over an alphabet mixing one-, two-, three- and four-byte
characters. It reads 5,266,521 instructions here and 2,570,995,073 on the
unfixed runtime — 488 times. No other benchmark asks `s[i]` of multibyte text;
they all read through slices or byte scans, which is precisely why nothing
went red for a year.

`tests/golden/micro/a_string_index_counts_characters` is the behaviour, on
both engines: forward through the string, backward through it, both ends, past
both ends, and an ascii subject for the direct path. It was watched red twice.
Resuming at `k_seek_char - 1` instead of `k_seek_char` — the off-by-one this
kind of cursor invites — turns `aé😀b€z` into `a 😀 € <none>` and the reversed
walk into `<none><none><none>zzz`. Writing the ascii bound as `from >= s->len`
loses the last character of an ascii string. Both are mistakes available in
these fifteen lines.

**What it costs, by the name each vein watches it under.** `text` rises 704
bytes in seven binaries and 688 in deepbench, which is `k_str_seek`;
escapebench indexes nothing, so the linker drops the function and its row does
not move. `emitted_other_defines`, `emitted_other_calls`,
`emitted_other_branches` and `emitted_other_lines` all rise because
indexbench's own 35, 159, 76 and 1,104 join a file that is a sum over
programs — a new benchmark is arithmetic there, not a regression, and the four
totals will rise again the next time one is added.

The instruction rows FALL slightly across the board — encodebench 7,237,599
(0.083%), which is 1.0001 instructions for each of its 7,237,200 `k_b_at`
calls: the list and bytes branches got a smaller function to sit in when the
string walk moved out of line.

**Not fixed, and worth writing down.** The cursor is compared by pointer, and a
`KStr` freed and reallocated at the same address with `cap != 0` would match
it. The guard added here is `k_seek_byte < s->len`, which turns the dangerous
case into a miss rather than a wrong character, but it is not a proof. The
same exposure has been in `k_b_slice` since the cursor landed. Closing it
properly means invalidating the cursor when a string dies, and that is a
separate change with its own reasoning.

DONE.

## 2026-08-31 — the append's grow path, built and HELD by the welfare number

Built, measured, and taken back out of the branch it was written on, because
the project's own scalar prices it negative. Recorded here so it is not
rediscovered.

`k_b_append_mut` is 26.5% of encodebench, and the first eight instructions of
every call were six callee-saved pushes and a frame:

```
    push %rbp / %r15 / %r14 / %r13 / %r12 / %rbx
    sub  $0x28,%rsp
```

One function holds both the frontier check and the growth. The fast path — two
failure tests, a tag dispatch, a capacity comparison, a store — needs two
registers; the growth needs eight, because it decides which allocator the new
buffer comes from, copies twice and frees the old header. Every append pays
for both. A `noinline` `k_b_append_grow` taking the six things it needs
reduces the prologue to three pushes.

```
    work_encodebench  8,708,075,266 -> 8,400,334,065  -314,978,800  -3.614%   (runner)
    work_jsonbench    2,862,072,778 -> 2,841,643,378   -20,429,400  -0.714%
    work_oneshot         44,077,654 ->    43,154,011      -923,643  -2.096%
    work_widebench       84,047,604 ->    84,271,604      +224,000  +0.267%
```

**widebench is why it is held, and the number is exact.** Its counters read
`append_fast=16000` and `append_grow=16000` — the only benchmark in the tree
where half the appends grow — and 224,000 over 16,000 grows is 14.0
instructions each, which is the call frame the grow path now enters. Every
other benchmark grows a handful of times against millions of fast appends, so
they take the three fewer pushes and pay nothing for the call.

The index disagrees with the arithmetic that looks obvious:

```
    floor after #1171                              87.511035
    the string index alone                         87.511654    +0.000619
    the string index and the outlining together    87.508343    -0.002692
```

Bisected by holding one row at a time. **widebench's 0.267% rise costs 0.003311
points on its own, where encodebench's 3.6% fall, jsonbench's 0.7%, oneshot's
2.1% and basket's 0.06% together earn 0.000619.** So the package falls below
the floor and the rule is unambiguous: the sum is the objective, a fall means
the change is worse by the weights as written, and the floor does not move to
accommodate it.

**What is worth asking, and is Clay's to answer, is whether the weights are
right about this.** wide_instructions sits at 138 times its baseline. A term
that far out contributes nearly its whole allowance already, so on the shape
of the curve a small move there should be worth almost nothing — and it is
worth five times what an eleven per cent improvement in encode was worth an
hour earlier. Either the model means that and the reason is worth writing down
beside the weights, or one benchmark 138x better than its baseline has more
leverage over the score than the two rows the front page makes claims about.
Filed in design/pending-gavels.md.

The change itself is small and its measurements are above; whoever picks it up
does not have to find it again.

HELD.

## 2026-08-31 — the runner's numbers for the string index

```
    work_encodebench  8,715,312,865 -> 8,708,075,665    -7,237,200   -0.083%
    work_basket          57,118,035 ->    57,083,993       -34,042   -0.060%
    work_oneshot         44,077,654 ->    44,059,561       -18,093   -0.041%
    work_widebench       84,047,604 ->    84,031,604       -16,000   -0.019%
    work_jsonbench, deepbench, escapebench, pendbench           unmoved
    work_indexbench                        5,266,934   new
    compile_instructions 42,297,900 ->    42,299,530        +1,630   +0.004%
```

Nothing in the work vein rises. The four falls are one number counted four
ways: `k_b_at`'s list and bytes branches got a smaller function to sit in once
the string walk moved out to `k_str_seek`, and encodebench's 7,237,200 over
its 7,237,200 `k_b_at` calls is 1.0001 instructions each.

**work_indexbench is new**, so it reads as a rise against a file that did not
have the row. It is the benchmark this change adds, at 5,266,934 — against
2,570,995,073 on the unfixed runtime, which is the 488 times the fix is worth.
A new row in a summed vein is arithmetic rather than a regression, and the
same will be true of the next benchmark somebody adds.

**compile_instructions rises 1,630 and none of it is the front end.**
`k_str_seek` adds lines to `src/runtime.c`, which `main.rs` holds as an
`include_str!` and hashes for the build cache key; a longer string takes
longer to hash. `compile_allocs`, `compile_peak_bytes`, rounds and visits are
byte-identical.

Welfare 87.511035 -> 87.511217, recorded with `--set`, and the page's tagged
compile-instructions figure follows the golden.

DONE.

## 2026-08-31 — the seek cursor outlived its string, and the engines disagreed

A differential-law violation, in shipped code, found while looking for one.

```
    fn shaped n 0 / "éé{n}ééééé"
    fn shaped n _ / "abcdefg{n}é"
    fn read s     / "{s[5]}{s[3]}/{text/slice s 5 5}{text/slice s 3 3}"

    interpreter   [abcdefg3é=>ec/ec]
    native        [abcdefg3é=>ge/ge]
```

Character 5 of `abcdefg3é` is `e`. Native answered `g`, which is character 7,
through both readers.

`k_seek_str` / `k_seek_char` / `k_seek_byte` are one remembered position, so a
forward sweep over a string resumes rather than restarting at the front. The
cursor names its string by address. `k_alloc` is a bump arena and
`k_beat_rewind` moves the frontier back, so the next string built in a loop
sits exactly where the last one sat — and inherits a byte offset that was true
of a string that no longer exists.

The comment above the cursor said a reset was unnecessary, on the grounds that
"a string whose bytes changed is a builder, which never qualifies". The string
whose bytes change across a rewind is not a builder. It is somebody else,
wearing the address. That sentence has been there since the cursor landed and
was the whole defence.

**Pre-existing, and not what #1172 introduced.** `text/slice` reproduces it
identically and has had the cursor since it landed; #1172 gave `s[i]` the same
cursor, which widened the exposure to the commonest way to read a character
but did not create it. Both readers are in the fixture for that reason.

The fix is one store in `k_beat_rewind`, which is the only place the arena
frontier moves backward — `k_beat_iter`, `k_beat_pop` and the carry paths all
reach it. Every address above the mark is free to be handed out again, so the
cursor is forgotten.

**What correctness cost**, and the rule says to pay it: welfare does not weigh
a change that makes the engines agree, so this ships and the floor moves to
whatever it costs.

```
    work_basket          57,083,993 ->    57,197,600   +113,607   +0.199%   (local)
    work_encodebench  8,708,075,665 -> 8,713,107,668  +5,032,003   +0.058%
    work_oneshot         44,059,561 ->    44,071,744    +12,183   +0.028%
    the other six                                    under a ten-thousandth
    text                                     +16 bytes in every binary
```

The rise is not the store. It is the cursor hits that stop happening: a sweep
that crossed a beat boundary used to resume and now restarts at the front.
That is the cost of the answer being right.

**A narrower reset was considered and not taken.** Keeping the cursor when the
remembered string lies below the mark and inside the surviving block would
recover some of it, but it needs the string's address checked against a block
whose own bounds are moving, and getting that wrong reintroduces exactly this
bug. The blunt version is provably correct and the measurement above is what
it costs. If that ever matters, the narrower test is the thing to build, with
this fixture already in place to catch it.

`tests/golden/micro/a_seek_cursor_does_not_outlive_its_string` is the pin, and
the harness runs it on both engines. Removing the store turns `ec/ec` into
`ge/ge` on native while the interpreter holds, which is the failure that
started this.

DONE.

## 2026-08-31 — the runner's numbers for the cursor fix, and a wrong note corrected

```
    work_basket          57,083,993 ->    57,198,013   +114,020   +0.1997%
    work_encodebench  8,708,075,665 -> 8,713,108,067  +5,032,402   +0.0578%
    work_oneshot         44,059,561 ->    44,072,143    +12,582   +0.0286%
    work_deepbench      760,475,193 ->   760,475,924       +731   +0.0001%
    work_jsonbench    2,862,072,778 -> 2,862,072,931       +153   +0.0000%
    work_pendbench      957,236,511 ->   957,236,647       +136   +0.0000%
    work_widebench       84,031,604 ->    84,031,648        +44   +0.0001%
    work_indexbench       5,266,934 ->     5,266,960        +26   +0.0005%
    work_escapebench                                    unmoved
    text                    716,626 ->       716,754      +128
    compile_instructions 42,299,530 ->    42,297,011     -2,519
```

Every work row rises except escapebench, and the rise is not the store. It is
the cursor hits that stop happening: a sweep crossing a beat boundary used to
resume where it left off and now restarts at the front. basket carries most of
it because it sweeps text across beats; the five rows under a thousandth are
the store itself, and `text` is its sixteen bytes in each of eight binaries.

**Welfare rose, and the reason is not the fix.** 87.511217 -> 87.511303, all of
it from `compile_instructions` falling 2,519, which is layout. Recorded with
`--set` because the rule says a rise is held rather than banked, and recorded
here as a ratchet step to spend back when `src/runtime.c`'s length next moves.
The fix itself costs runtime instructions and would have been right to ship at
a fall: welfare does not weigh a change that makes the engines agree.

**A note from #1172 was wrong and is corrected in the golden.** It said
`compile_instructions` moves when `src/runtime.c` changes length because
`main.rs` holds the file as an `include_str!` and hashes it for the build cache
key. That hash is in `cached_program_binary`, which is on the run and build
paths; `kanso check lib/json` never reaches it. What moves is layout — the
static's length shifts what follows it — and the direction does not follow the
length: twenty added lines read +1,630 and 713 added bytes read −2,519.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are unmoved across
all three, which is what says the front end's work did not change.

DONE.

## 2026-08-31 — the welfare number's leverage sits in one benchmark, and the reason is where satiation is applied

The gavel filed an hour ago named the right question and got the mechanism
wrong. Its "why it happens" paragraph described a weighted ratio of costs, and
the script does not compute one. Corrected in design/pending-gavels.md, with
the arithmetic, and repeated here because the log is where the number's
history is read.

`dimension_score` averages the counters' ratios and satiates the average:

```
  mean = list/sum ratios / (1.0 * length ratios)
  satisfaction mean t.satiation * t.weight
```

Satiation therefore applies to no counter. A counter's raw ratio enters the
average linearly and without bound, so the counter furthest past its baseline
carries the term. Run speed at today's floor:

```
    wide_instructions        ratio  138.37     68.45% of the mean
    deep_instructions        ratio   29.88     14.78%
    pending_instructions     ratio   29.22     14.45%
    oneshot_instructions     ratio    1.46      0.72%
    decode_instructions      ratio    1.14      0.56%
    encode_instructions      ratio    1.07      0.53%
    basket_instructions      ratio    0.99      0.49%
```

The model that produced those shares reads the live goldens and returns
87.511303209 against a recorded floor of 87.511303209, so it is the script's
arithmetic rather than a description of it.

**A second change is held behind the gavel.** `k_b_append_into` wrote its grow
path twice — once taking arena storage when the header dies at the innermost
rewind, once taking malloc — and the two differed in the allocator and in the
sign of the cap and in nothing else. One tail serves both. Measured on this
container, same sitting, callgrind, nine benchmarks:

```
    encodebench  8,713,107,668 -> 8,670,774,068   -42,333,600   -0.4859%
    jsonbench    2,862,072,518 -> 2,860,449,668    -1,622,850   -0.0567%
    oneshot         44,071,744 ->    43,955,091      -116,653   -0.2647%
    widebench       84,031,235 ->    84,111,235       +80,000   +0.0952%
    basket, deepbench, escapebench, pendbench, indexbench: identical
```

widebench's 80,000 is 5 instructions across its 16,000 grows. It is the only
benchmark in the tree where half the appends grow, because
`text/append (text/bytes lead) "  "` builds a fresh accumulator per element
and the first append onto one has no spare capacity to write into.

Priced two ways:

```
    as written    (average the ratios, then satiate)   -0.001096   held
    per counter   (satiate each ratio, then average)   +0.008015   ships
```

The sign turns on the order of those two operations. Under the index as
written the rule is unambiguous and the change is held: the sum is the
objective and it falls. What it does is written above in enough detail to
redo — hoist the allocator choice above the copy, keep one tail, carry the
regime in the sign of a local — and it is thirty fewer lines than what it
replaces.

HELD.

## 2026-08-31 — a deliberate exit's two tests staged one file

Found while running the suite for something else: `cargo test --release` went
red on `the_interpreter_agrees`, which asserted the shell saw 3 and got 2.

Two is what the compiler exits with. Both tests in
tests/a_deliberate_exit_carries_its_code.rs stage their fixture at
`/tmp/kanso-deliberate-exit/main.kso`, cargo runs them on separate threads, and
`fs::write` truncates before it writes — so one test could read the file while
the other was inside that call and compile an empty program.

Measured back to back, twenty runs each: two failures before, none after. That
is often enough to redden an unrelated pull request and rare enough to be read
as something else, which is how it survived. kanso#1169 fixed the same shape in
the playground tests and this pair was missed.

One directory per test, named by the caller.

DONE.

## 2026-08-31 — the beat sweep: every layout pair, every hand that moves the cursor

kanso#1173 fixed a cursor that outlived its string across a beat's rewind and
shipped one golden pinning one shape of it. One shape is what the bug happened
to take, so the sweep here asks the whole family instead.

`scripts/beat_differential` builds a program per case: a beat that rebuilds a
string every iteration, alternating between two byte layouts, and reads it with
one hand. Four layouts — all ascii, all two-byte, all four-byte, and one that
changes width inside a single string — crossed with themselves and with six
hands: forward indexing, backward indexing, forward slicing, backward slicing,
a hand that mixes index and slice, and one that asks `length` between reads.
Ninety-six cases, both engines, ten seconds.

Alternation is what makes a case sharp. Two iterations of the same layout put
the next string's characters at the byte offsets the last one used, so a stale
cursor is accidentally right; two layouts that disagree about where character
five begins make it wrong.

**Watched fail, for the right reason.** With `k_seek_str = NULL` deleted from
`k_beat_rewind` — the whole of #1173's fix, and now
`scripts/ratchet/mutations/a_cursor_that_outlives_its_string.sh` — sixteen of
the ninety-six disagree. Some of them answer a character from the wrong
position; some answer a byte from the middle of a codepoint, which the terminal
renders as U+FFFD. With the store restored, all ninety-six agree.

Two sweeps ran on the way and are recorded because they say what the fix
covers: thirty-six single-layout programs and ninety alternating-layout ones,
both zero disagreements against the fixed runtime.

The row is `beat_cursor` in scripts/ratchet/ratchet.kso and the step runs in
the diagnostics-differential job beside the eight sweeps already there.

DONE.

## 2026-08-31 — two things the wide benchmark paid for and did not need

widebench had never been profiled. It carries 68.45% of welfare's run-speed
mean, so where its time goes decides most of what the index can be moved by.
Two answers came out of one profile.

**A whole number rendered as a float went through glibc's multiprecision
formatter.** 11.0% of widebench was inside three libc frames:

```
     5,661,266  6.74%  __printf_fp_buffer_1
     2,192,140  2.61%  hack_digit
     1,424,000  1.69%  __printf_buffer
```

The caller is `k_render_at`. A double that is a whole number takes a branch
guarded by `d == floor(d) && fabs(d) < 1e15 && isfinite(d)`, and that branch
called `snprintf("%.1f")`. `%f` reaches `__printf_fp`, which is multiprecision
— it also pulls `__mpn_divrem` and `__mpn_mul_1`, which `k_b_to_float`'s strtod
shares. `render_ryu`'s `d == 0.0` branch made the same call.

The guard has already proved the number is an integer under 1e15, so the cast
to `long long` is exact, the digits are an integer's digits, and the fraction is
exactly `.0`. `k_itoa` writes it. Negative zero is the one value the cast loses,
and `signbit` keeps it.

**The utf-8 validator paid full vector setup for a forty-one byte token.**
`k_utf8_bad` was 4.15% of jsonbench. Counting the calls in the callgrind file
rather than guessing: 265,950 calls carrying 10,975,500 bytes, so 41.3 bytes a
call and 446 instructions a call — about eleven instructions a byte, where a
vectorized validator on a long input runs at a fraction of one. The setup is
why: seven constant loads and two zeroed accumulators before the first block.
The old short-circuit only fired below sixteen bytes.

Eight bytes at a time answers the whole question for an ascii run of any
length and never reaches the vector pass. A run that is not ascii falls
through and the wide pass reads it from the start, so the scan is the only
waste.

The runner's rows, which are the ones the golden carries. Every delta matches
what this container measured to the instruction; only the absolute values
differ, because the two hosts run different glibc:

```
    widebench       84,031,648 ->    67,553,998  -16,477,650   -19.6090%
    basket          57,198,013 ->    56,781,353     -416,660    -0.7285%
    jsonbench    2,862,072,931 -> 2,860,868,131   -1,204,800    -0.0421%
    oneshot         44,072,143 ->    44,066,375       -5,768    -0.0131%
    encodebench  8,713,108,067 -> 8,714,005,635     +897,568    +0.0103%
    deepbench, escapebench, pendbench, indexbench: identical
```

`compile_instructions` rises 678, and it is layout for the third time this
week: `include_str!("runtime.c")` changed length, so what follows it in the
binary moved. compile_allocs, compile_peak_bytes, rounds and visits are
byte-identical.

The floor is 87.773793, up 0.262490 — the largest single rise the index has
taken.

Held apart, so the two are separable: the float render alone is widebench
-19.019% and welfare +0.253991; the ascii pre-scan alone is basket -0.7241%,
widebench -0.5900%, jsonbench -0.0421% and welfare +0.007426.

`work_encodebench` rises 897,568, which is 1.003 instructions per ryu_render
and no counter encodebench owns moves; it is layout, and it is under a
hundredth of a per cent.

`text` rises, and it is bought rather than explained away: 128 bytes of .text
on the five benchmarks that link both changed functions, 80 to 96 on the four
that link a subset because the linker drops what they never call. That is the
integer path and the word-wise scan against two `snprintf` call sites removed,
and it buys widebench 19.6%. Every allocation counter is byte-identical — all
eight counter gates and the emitted-line gate pass untouched — which is what
says the machine code is the only dimension that moved.

Removing `"%.1f"` made a ratchet mutation stale, which is the check doing its
job: `native_renders_a_float_wider` widened that format to `"%.2f"` so native
printed `1.00` where the oracle printed `1.0`. There is no format to widen now,
so it breaks the cast instead — `(long long)d` becomes `(long long)d * 10` and
native prints `10.0`. Watched red before it was believed: the render sweep
reports `native: 10.0 / interp: 1.0` under the mutation and 86 values, 0
disagree, without it.

Byte-identical on both engines across the float edges — 0.0, -0.0, ±1.0,
±42.0, ±1e14, 999999999999999.0, the 1e15 boundary where the exponent form
takes over, 1e-07, and `0.0 * -1.0`. Pinned in
tests/golden/micro/a_whole_float_keeps_its_point.kso, which goes red on the
mutation that drops the `signbit` test: three of its `-0.0`s render as `0.0`.
All eight differential sweeps agree, the utf-8 one included.

**What is left in widebench, for whoever picks it up.** The carry evacuation:
`k_survives_x` at 7,522,206, `k_copy_size` at 8,098,822 across both frames,
`k_ptrmap_at` at 2,724,570 and `k_deep_copy` at 1,695,365 — about 29% of what
this leaves. `k_survives_x` is not slow per call and the chain is short
(widebench holds two arena blocks); it is called roughly 375,000 times, because
the evacuation walks the carried document every iteration. That is a design
question about the carry rather than a peephole.

DONE.

## 2026-08-31 — a third staging collision, and this one hid behind a name that looked unique

kanso#1175's flake was found by a suite run going red for reasons that had
nothing to do with what was being measured. Two more runs of the same suite
named the rest of it: `a_list_that_was_never_bytes_reads_the_same_on_both_engines`
and `genuine_bytes_already_read_the_same_on_both_engines`, together, in one run
of three.

tests/a_bare_list_is_or_is_not_bytes.rs stages its program in a directory named
by the expression's LENGTH. Two pairs collide across the file's two tests, which
cargo runs on separate threads:

```
    text/to_float [1]                and  text/utf8 [97 98]                  17
    text/find2_below [97] 0 97 98 1  and  text/to_float (text/bytes "ab")    31
```

Each call writes `run.kso` into that directory and calls `remove_dir_all` when
it is done, so one test deletes the other's program between the write and the
run. Twenty runs each on a settled tree: five failures before, none after.

A hash of the expression names the directory now.

**The rule this is the third instance of.** kanso#1169 fixed it in the
playground tests and kanso#1175 in the deliberate-exit pair, both where the
name was a constant. Here the name was derived and looked unique, which is why
neither of those fixes covered it: a staging path has to be keyed by something
injective over everything that can reach it, and `len` is not. Every test
binary in the tree was then run repeatedly to find the rest by failure rather
than by reading, because reading is what missed this one twice.

DONE.

## 2026-08-31 — the tenure test answered for objects, and the question was about regions

kanso#1177 left a note naming the carry evacuation as what remained in
widebench: `k_survives_x` at 7,522,206 instructions, about 29% of the
benchmark. Instrumenting it said the shape was nothing like the guess in that
note. The arena walk is one step every time — widebench holds a single block —
so `k_survives` is already O(1) and costs nothing. Of 120,418 asks, 16,079 are
answered by the arena and 104,339 fall through to the tenure test, and **80,415
of those, 83%, are the tenure test proving a pointer is NOT tenured.** The cost
was the hash probe on the miss, at roughly 40 instructions a time.

An address-span filter — reject anything outside [lowest tenured byte, highest]
before probing — rejected 1 ask of 96,496. The two tenured blocks sit 45 TB
apart, because glibc mmaps one and takes the other off the heap, and the arena
lands between them.

**Two blocks.** That is the whole finding. The block list is short, so the
membership question can be a walk, and a walk answers on ranges where a hash
answers on allocation bases. `k_ten_alloc` now doubles each block instead of
taking a fixed 64 KiB, which bounds the list: K_TEN_CAP is 64 MiB, so doubling
from 64 KiB runs out of licence after eleven blocks and there is never a
twelfth. That bound is what pays for deleting the hash set, which cost a probe
on every ask AND an insert on every single tenured allocation.

A one-entry cache of the block that answered last was built and DROPPED. The
instrumentation said it would take 16,080 of widebench's 96,496 asks on its
own, and it does; it also costs the other 80,416 a load and two compares before
they walk anyway, and the walk is two blocks. Measured: 65,170,283 with the
cache against 64,535,358 without, so it cost widebench 634,925 instructions to
save 16,080 walks of length two. Dropping it also retires a second question —
the cache had to carry the beat depth it was filled at, because `k_beat_pop`
releases tenured storage on its way out but three other sites lower
`k_beat_depth` without releasing, and a cache naming a block the walk would no
longer reach would answer where the walk says no. That check was another
257,197 instructions on widebench. Neither is in the code.

Answering on ranges is where the behaviour changes. A list's
elements sit in a buffer the header points into, so the copy that tenured the
list put the header at an allocation start and the elements at an offset; the
hash answered no for the buffer and the next rewind evacuated all of it again.
`a_loop_invariant_capture_is_copied_every_rewind` goes 32,672 evacuated bytes
to 16,448 — the list stops travelling forward a second time. widebench's
`evac_bytes` goes 1,032,336 to 519,728 and its `beat_iters` 43 to 40, because
the size walk that decides whether a chain step stages or leaves its value now
sees the tenured storage as surviving and three steps come out under the
threshold.

    jsonbench     2,860,867,718 -> 2,858,844,840     -0.0707%
    encodebench   8,714,005,236 -> 8,704,347,511     -0.1108%
    oneshot          44,065,976 ->    44,000,222     -0.1492%
    basket           56,780,940 ->    56,457,649     -0.5694%
    widebench        67,553,585 ->    64,535,358     -4.4679%
    deepbench       760,472,244 ->   726,483,254     -4.4695%
    escapebench     258,582,701 ->   253,818,697     -1.8424%
    pendbench       957,236,261 ->   946,377,688     -1.1344%
    indexbench        5,266,547 ->     5,244,328     -0.4219%

Nine benchmarks, nine falls. Container numbers on both sides, measured the same
sitting; the runner's rows are in bench/instructions_golden.txt.

**Half of that is one attribute.** `k_ten_holds` is `noinline` now. Inlined, it
was pulled into `k_survives_x`, which is inlined into `k_born_this_beat`, which
is inlined into `k_b_push_mut` — so a program that never tenures still carried
the whole tenure test inside its hottest fast path. escapebench, deepbench and
jsonbench never call `k_ten_holds` at all: the symbol is absent from their
profiles, as it is from four of the other six — only widebench and indexbench
reach it. Their falls are entirely the fast path being compiled better once
the cold code is out of it. Measured separately: `noinline` alone on the
unchanged hash gives deepbench -4.44% and escapebench -1.84%, and the walk adds
the rest on the benchmarks that tenure. Without it the walk COST escapebench
1.38%, at exactly +3 instructions per push with every call count identical.

`text` rises 22,656 bytes over the nine binaries, between 2,432 and 2,624
each. The optimiser did that: `k_survives_x` shrinks 346 bytes to 118 once the
tenure test is out of line, and then inlines into more of its callers —
`k_copy_size` +1,911, `k_repair_interior` +800, `k_slots_survive` +267,
`k_interior_survives` +263, `k_deep_copy` +252, against `k_repair_size` -940
inlined away entirely. Bigger
code, fewer instructions executed, on every benchmark. Two of the nine reach
`k_ten_holds` at all — widebench and indexbench — so seven of these rows,
`basket`'s -0.57% among them, are the fast path alone.

**The archive declines positional membership by name, and this is not it.**
design/log/compiler-log-archive.md, under "Declined by measurement: the same
idea as an arena block": appending a KBlock at the tail of the arena chain was
built three times and reached 91,241,398,272 bytes of resident set. The reason
given is that positional membership recognises every pointer inside a block
where a hash recognises only bases, and a text accumulator handed through an
intermediate function is the shape that turns that into unbounded growth.

That variant put the answer in `k_survives` — the narrow walk the mutation fast
paths ask per append. This one is only in `k_survives_x`, which the copy
machinery and `k_born_this_beat` ask and no mutation path does. The split
between the two was made in that same archived entry, for this reason. Checked
rather than argued:

- `book_panels --write` runs clean under `ulimit -v 8000000`, rewriting 0
  panels. The declined variant killed it.
- `effect_push_shape.mem`, the fixture the declined variant broke, does not
  move. One file in the 51-fixture mem corpus moves, and it is the carry one.
- Peak RSS across the nine benchmarks: -100 KB to +16 KB, widebench and
  deepbench byte-identical.
- The accumulator shape itself, built and run at n = 1,000 / 2,000 / 4,000 /
  8,000: 20,864 / 71,900 / 267,912 / 659,492 KB on main against 20,844 /
  71,916 / 267,876 / 659,560 with the change. The curve is superlinear and it
  is superlinear on both sides, unchanged to a tenth of a per cent.

Running the declined variant's own shape — the tenure answer moved back inside
`k_survives` — moves exactly one fixture in the mem corpus, the same carry one.
So the corpus can no longer see the thing that cost 91 GB. That is a gap, and
it is recorded rather than fixed here: an attempt to fill it with a text
accumulator crossing a beat was written, generated its goldens, and was DELETED
because it read identically on main, on this change, and under the declined
variant. A golden that cannot go red is worse than no golden.

**The runner's rows, and what the compile veins did.** Every delta below was
measured on the container first and reproduced on the runner to the
instruction; the absolute numbers differ by the usual few hundred, and the
subtractions do not.

    jsonbench     2,860,868,131 -> 2,858,845,253     -0.0707%
    encodebench   8,714,005,635 -> 8,704,347,910     -0.1108%
    oneshot          44,066,375 ->    44,000,621     -0.1492%
    basket           56,781,353 ->    56,458,062     -0.5693%
    widebench        67,553,998 ->    64,535,771     -4.4679%
    deepbench       760,475,924 ->   726,486,934     -4.4694%
    escapebench     258,583,100 ->   253,819,096     -1.8423%
    pendbench       957,236,647 ->   946,378,074     -1.1344%
    indexbench        5,266,960 ->     5,244,741     -0.4218%

`compile_instructions` rises 42,297,689 to 42,300,376, +2,687 and +0.0064%.
Nothing about compiling got harder: `compile_allocs` and `compile_peak_bytes`
are byte-identical at 29,864 and 728,030, the front end's rounds and visits do
not move, and the diff touches src/runtime.c and nothing the measured path
runs. `kanso check lib/json` compiles a library and executes no program, so
the runtime is not reached at all — but runtime.c arrives in the compiler
through `include_str!`, so changing its LENGTH rearranges the compiler's own
binary. This is the fourth instance this week and the sign does not track the
direction of the edit. It is the case design/pending-gavels.md asks about
under "Whether a compile_instructions move that cannot be work needs an
attribution", and the entry is unchanged by this: the vein earns its place,
and the question is only what a rise with every sibling counter identical
should have to buy.

Welfare 87.7738 to 87.8371, banked with `--set` in this PR.

All nine differential sweeps agree, the beat and utf-8 ones included. The whole
Rust suite passes. `evac_bytes` in two veins is what pins the walk: revert it
and both go red, which is how this was watched before it was believed.

**A fourth staging collision, folded in.** Running the suite to validate the
above turned it up, which is the point of running it repeatedly rather than
reading it. `tests/a_toolchain_the_path_cannot_reach.rs` staged both its tests
into `/tmp/kanso-no-clang`, and `fs::write` truncates before it writes: one
test emptied `main.kso` while the other test's `kanso` was reading it, so the
build reported `an entry file needs at least one statement` instead of the
message under test. One failure in twenty; none in forty with a directory per
test. kanso#1169, kanso#1175 and kanso#1177 are the same defect at three other
sites, and this is the second of those four found by a suite going red rather
than by anybody looking.

DONE.

## 2026-08-31 — the depth loop was already one iteration, and the block walk was two

kanso#1178 left `k_ten_holds` as its own symbol in widebench at 37.7
instructions an ask, and a task naming the outer loop over beat depths as where
that went. The estimate came from arithmetic on 44 instructions per call rather
than from a counter, and it was wrong.

Instrumented, widebench reads `calls=112,529 depth_iters=112,529
block_iters=208,982 max_beat_depth=1`. **One depth iteration per ask**, because
the beat depth never exceeds one on this program: there is no depth loop to
bound. indexbench, the only other benchmark that reaches the function, reads
`calls=89 depth_iters=178 max_beat_depth=2` — 89 asks is not a cost.

The 1.86 block iterations per ask are the cost. widebench tenures about 128 KiB,
which needed a second 64 KiB block, so 71% of asks — the misses — walked both.
Starting the doubling at 256 KiB holds it in one. 256 is the smallest power of
two that does, and nothing larger can help: the walk cannot go below the one
block it now takes, so a bigger base would buy nothing and cost peak to every
program that tenures at all.

    widebench     64,535,358 -> 63,756,786    -778,572   -1.2064%
    indexbench     5,244,328 ->  5,242,681      -1,647   -0.0314%

Every other benchmark moves by 0 or 14 instructions, which is nothing. All nine
counter gates are unmoved — no evacuation counter, no arena peak — and the
machine code is byte-identical on all nine binaries, because a constant is all
that changed. `k_ten_holds` costs 30.4 instructions an ask against 37.7, at the
same 112,529 calls, which is one block test removed and matches the total to
five per cent.

Peak resident set: widebench 3,624 KiB to 3,704, indexbench 2,416 to 2,452. The
block is mmap'd and its untouched tail never becomes resident, so what a
program that tenures nothing pays is zero — `k_ten_alloc` runs only when
something is promoted.

**The runner's rows.** widebench 64,535,771 to 63,757,213 and indexbench
5,244,741 to 5,243,094; every other row byte-identical. The container measured
-778,572 and -1,647 against the runner's -778,558 and -1,647, so the two agree
to fourteen instructions on one row and exactly on the other.

`compile_instructions` FALLS 628, 42,300,376 to 42,299,748, with
`compile_allocs` and `compile_peak_bytes` byte-identical at 29,864 and 728,030.
The diff is a constant and nine lines of comment in runtime.c, which
`kanso check lib/json` never runs and which reaches the compiler only through
`include_str!`. kanso#1178's entry recorded the same mechanism moving this
vein UP by 2,687 the day before, which is the point the ledger entry
"Whether a compile_instructions move that cannot be work needs an attribution"
makes: the sign does not track the direction of the edit, because neither is
about the front end's work.

Welfare 87.8371 to 87.8507, banked with `--set` in this PR.

The bound the doubling exists for still holds. K_TEN_CAP is 64 MiB, so doubling
from 256 KiB runs out of licence after nine blocks where it took eleven from
64 KiB, and k_ten_holds' walk is shorter at every size.

**The idea that started this is declined.** Bounding the depth loop buys
nothing, because the loop is already one iteration on every program that
reaches it. Recorded so nobody prices it again from the same arithmetic: 37.7
instructions an ask is a call, a prologue, one depth iteration and two block
tests, and only the last of those was ever worth attacking.

DONE.

## 2026-08-31 — a refusal that blamed an edit for what a checkout's write order did

Validating kanso#1179 in a git worktree, two wasm tests refused to run:
`docs/kanso.wasm predates ast.rs, lexer.rs, diag.rs, ...` and twenty-three more.
The diff under test touched src/runtime.c and a markdown file, neither of them
in that list, so the sentence sent me looking for an edit that had not
happened.

The timestamps say what did:

    1788165438.949546191  docs/kanso.wasm
    1788165438.977546193  src/eval.rs
    1788165438.977546193  src/lexer.rs

Twenty-eight milliseconds, inside one second. git checks a tree out in path
order and `docs/` sorts before `src/`, so the blob is written first every time.
This is not a coincidence that a second checkout would shake off: **every fresh
clone and every new worktree lands in exactly this state**, and both tests are
unrunnable there until `scripts/build_wasm.sh` has run. CI never sees it
because CI builds the blob.

Refusing is right — the blob is genuinely not known to match those sources. The
sentence was the defect, and it is the family kanso#1084 and kanso#1086 already
took out of two other refusals: a check whose stated reason is not its reason.

When every source is newer and the widest gap is under a second, that is a
checkout rather than an edit, and the guard now says so. Watched on all three
branches before it was believed:

    fresh worktree            "36ms older than all 26 sources, which is what a
                               checkout looks like rather than an edit"
    one source touched        the original sentence, all 26 named — the gap is
                               real now, so the checkout reading no longer
                               applies
    blob rebuilt              12 passed, 0 failed

The mtime comparison stays a mtime comparison, so the ruling in design/ that
declined replacing it with a digest is untouched. Only what it says when it
refuses has changed.

DONE.

## 2026-08-31 — the reference said kq keeps three veins and it keeps five

Task #190 said kq's pin needed bumping past today's runtime work, and predicted
the bump would be a one-line change: kq pins allocation counters, and neither
kanso#1178 nor #1179 moved an allocation counter — all nine counter gates
passed unmoved in both.

The prediction came from CLAUDE.md's list of kq's veins, which names
`bench/cost_golden.txt`, `bench/cost_golden_decode.txt` and
`bench/numbers_stamp.txt`. Reading the repo instead, kq keeps five:

    bench/cost_golden.txt              allocations
    bench/cost_golden_decode.txt       allocations
    bench/cost_golden_escapes.txt      allocations
    bench/instructions_golden.txt      RETIRED INSTRUCTIONS
    bench/numbers_stamp.txt            keyed to the first

The fourth is the one the prediction turned on, and it is the one most changes
reach. A change that moves no allocation counter still moves it — which is the
whole reason kanso added its own instructions vein, recorded in that file's
header: eight and a half per cent of decode went missing with every allocation
counter byte-identical.

So the pin bump is a re-stamp of five veins rather than one line. And the pin
is not two commits behind, it is **fifty-nine**, sitting at kanso#1120 with
#1171 (encode -11.67%), #1172 (indexbench 488x), #1177, #1178 and #1179 all in
the gap. Why it drifted is not recorded anywhere and this entry does not guess;
what is checkable is that the short list would licence exactly this drift, by
telling a session to ask a question that returns the wrong answer.

**This is the second time today a stale description beat a measurement.** The
other was `k_ten_holds`: a task priced its depth loop at four iterations from
arithmetic on a profile total, and the counter said one. Both were reasoning
from a written summary rather than from the thing. The rule the log already has
for specs — enter where a user enters, assert what the program does — applies
to reference documents read as evidence.

The list is corrected. The pin bump is kq's own PR and needs a branch there.

DONE.

## 2026-08-31 — kq's pin absorbed, and a fifth staging collision found by scan

Two things landed off the same afternoon.

**kq#82, the pin bump.** `.kanso-version` walks from #1120 to #1181, sixty-one
merges. Every allocation counter and the published-numbers stamp are
byte-identical across the whole window, which is why nothing here noticed the
drift. The instructions vein moves and every row falls: print_small -1.46%,
path_small -1.91%, print_big -1.37%, path_big -1.68%.

Print and path do not collect the same number, so the saving is not one thing
in the decode they share. Bisected at seven points, small rows only, and both
columns sum to the end-to-end delta exactly:

    #1143  a subtype of a primitive is a heap value   print +128,464  path   +186
    #1171  two byte-level costs in the runtime        print -955,739  path -320,184
    #1172-#1173  the string index gets the cursor     print +113,192  path   +470
    #1174-#1177  the wide benchmark's two costs       print  -40,141  path  -59,157
    #1178-#1180  the tenure walk answers on ranges    print -402,592  path  -13,561

Fifty of the sixty-one commits move neither row by one instruction: two runs of
them, 22 commits and 26, plus two that edit only markdown. They are front-end
work, and kq's vein counts what the compiled program executes.

The container measuring this is one glibc revision behind the runner and read
print_small 409 instructions below the committed row under the pinned
compiler — the same offset that file recorded on 2026-08-29. The rows were
committed as the runner's values moved by the container's deltas, and the
runner then said "instructions: every row is where it was". All four
predictions correct to the instruction, which is the second time this host pair
has been shown to differ by an offset and not a slope.

**The fifth staging collision.** Task #191 asked whether the family that cost
#1169, #1175, #1177 and #1178 had more members. Six full-suite passes had found
nothing, so the question was put to a scan instead of to luck.

The scan looks for a function that writes under `temp_dir()` and is reached by
two or more `#[test]`s in its file, then asks what distinguishes the paths those
tests stage. Run against the four pre-fix trees it goes red on three of the four
at exactly the commit that fixed them; #1177's is a length key, invisible to the
first shape it checks. Two false positives taught it two legitimate
distinguishers — a `process::id()` in the path, and an `AtomicUsize` counter
making it unique per call. `view_cache_is_returned.rs` carries the second, with
a comment saying two tests there ask for 20,000 maps: a fifth instance of the
family, already found and fixed by somebody, and never written down here.

On today's tree the scan found `goldens_move_expires.rs`, whose staging
directory is `kanso-gm-{body.len()}` while the file inside it is the constant
`sibling-goldens-move`. The four fixtures are 78, 82, 38 and 59 bytes, so it
does not fire. Four bytes of prose added to the first makes two of them 82, and
then it fires hard: **27 failures in 40 runs**, reporting exit 2 where the test
asserts 0, because `verdict` ends in `remove_dir_all` and the first test to
finish deletes the other's program mid-run.

The key is a hash of the body now, as in `a_bare_list_is_or_is_not_bytes.rs`
after #1177 and `numeric_parity.rs`. The same forty runs go 0 for 40. The two
82-byte bodies are left at 82 on purpose: that is the condition a length key
needs to fail, so the corpus carries it and a regression goes red on the next
run.

**The scan itself is not shipped.** It is regex over Rust with five special
cases for what counts as a distinguisher, and the log's own rule against specs
written against an internal decomposition applies to a gate written against a
parse. What would be sound is structural rather than analytic — every staging
site reaching one helper, so the family is unrepresentable — and that is the
83-site change a blanket regex already botched once this session. Recorded as
the shape, not attempted here.

Six full-suite passes could not find this. A scan found it in one run, because
the condition was latent rather than live: the corpus was four bytes away from a
27-in-40 flake and no amount of running it would have said so.

DONE.

## 2026-08-31 — the log is forty entries again, and the branch purge is not blocked on access

Sixty-two entries and 3,995 lines, against the rule's forty. Twenty-two move to
the archive unedited, in two runs, and the file is 40 entries again. Nothing is
rewritten; the archive is where a thread this file no longer mentions is found.

Checked rather than assumed, because a trim moves text that gates read:
page_drift reports 2/3 entries since the page moved, and welfare reads 87.85
against a floor of 87.85. Both are green on the trimmed tree.

**And a correction to task #109, which has said BLOCKED ON ACCESS for weeks.**
Access is not the blocker. The count is 389 branches, not 324, and the reason
nobody can purge them is that git cannot say which are dead:

    git branch -r --merged origin/main       1 branch
    git branch -r --no-merged origin/main  387 branches

That first number is not a measurement of anything. This repository
squash-merges, so a landed branch's commits never become ancestors of main and
`--merged` can only ever report the one branch that is main itself.
`git merge-tree --write-tree` was tried as the sounder test and fails the same
way for a different reason: merging an old landed branch into today's main
produces a tree that differs from main's, because main has moved on in the
files that branch touched. Checked against
`a-backend-that-indexes-an-argument-it-never-counted`, which is kanso#1121 and
certainly landed: both predicates call it live.

So the only authority on whether a branch is dead is the pull request record,
and a purge is a GitHub query rather than a git one. `delete_branch_on_merge`
IS on — a branch deleted under this session on merge had to be recreated by
the next push, which is how that was established — so the 389 are branches
whose pull requests predate the setting, plus any whose PR never merged. The
count only falls by a deliberate sweep.

Deleting 388 remote branches is not a thing to do on inference from a
predicate that has already been wrong twice on this question. What is recorded
here is the measurement and the reason; the sweep is Clay's.

DONE.

## 2026-08-31 — the runtime is a minority of what a decode costs now

Profiled `./jsonbench` under callgrind in the container and attributed every
function to the file it came from. 2.81 billion instructions, split:

    emitted kanso   1,728,709,950   61.6%
    runtime.c       1,078,786,257   38.4%

The largest single symbol is `d_jsonbench/value_for_3` at 22.67%, and reading
it as one fat function would be wrong: `parse_string`, `parse_array`,
`parse_object`, `parse_number` and `skip_ws` are all absent from the profile
under their own names, because clang inlined them into it. The figure is the
whole value-parsing path collected under one symbol. The dispatch that names it
is a real `switch i64` over all six literal arms with a default, checked in the
emitted ir, so §05's switchboard claim holds where a reader would test it.

The largest runtime entries are `k_b_append_mut` at 7.05%, `k_b_put_mut` at
4.82% and `k_utf8_bad` at 4.11%.

That ratio is the finding. Roughly twenty merged changes have taken cost out of
`runtime.c` — the byte-level pair in #1171, the string cursor in #1172, the
tenure membership walk in #1177 through #1179 — and the arithmetic result is
that the runtime is now the smaller half of a decode. Further decode work of
the shape the last month took is buying from a 38% pool.

Two seams were checked and neither is open. `k_utf8_bad` already carries an
eight-byte ascii skim with its measurement written beside it, and the
Keiser-Lemire wide pass it falls through to is `#if defined(__aarch64__)`, so
on the linux runner the wide pass is scalar and never reached by an ascii
document anyway. The front end is in the same state: profiled at 42.7 million
instructions with nothing above 5.53%, `reserve_rehash` diffuse at 0.10% for
its largest named caller, so the pre-sizing seam #1158 opened is exhausted.

What both point at is the same place. The front end's remaining 13.2% is malloc
and free, which is the inline-name gavel; the decoder's remaining 61.6% is what
codegen emits. Neither is a kernel to tighten.

DONE.

## 2026-08-31 — the 22,656 bytes beside the tenure win, attributed

Clay asked whether the binary-size rise that landed with #1178 is a
separable oversight or the price of the win, and said he would accept a
real cost given proof it is not fixable. It is the price. Two levers
were built and measured; one is worse and the other costs more than the
bytes are worth.

First the thing that should not have happened. #1178 moved every row of
`bench/text_golden.txt` — +22,656 bytes over the nine binaries, between
2,432 and 2,624 each — and wrote no sentence in the file. The corrected
figure went in the PR body instead. That file's own header says a number
that changes without a sentence is the thing to catch, and the sentence
belongs in the file, because the PR body is not what the next reader
opens. The entry is written now, late, and says so.

The attribution, measured on the container at clang 18.1.3, the same
release the golden is stamped with. It reproduces the runner's rows to
the byte, before and after, so these are the published numbers rather
than a proxy for them.

Twelve symbols move on jsonbench and every one is in `runtime.c`:

    k_copy_size          +2,031      1,659 -> 3,690
    k_repair_size          -940        940 -> 0 (inlined away)
    k_repair_interior      +800        984 -> 1,784
    k_slots_survive        +270        142 -> 412
    k_interior_survives    +255      1,122 -> 1,377
    k_deep_copy            +240      1,924 -> 2,164
    k_survives_x           -228        346 -> 118
    k_copy_alloc           -171        607 -> 436
    k_ten_holds            +138          0 -> 138 (new, out of line)
    k_b_push_mut            +96      1,189 -> 1,285
    k_map_sorted            +78      1,202 -> 1,280
    k_beat_pop              -38      1,253 -> 1,215

They sum to +2,531 against a .text row of +2,528; the three bytes are
alignment. Nothing in the emitted kanso moves at all, which is what the
golden's header already predicts — `runtime.c` is embedded whole, so
every program carries the same added bytes whether its own walk reaches
them or not.

So the growth is the evacuation size walk being re-inlined, and the
question is which half of #1178 caused it. The 2x2, on jsonbench:

    hash set,   k_ten_holds inlinable    82,722   (#1177)
    hash set,   k_ten_holds noinline     85,538
    block walk, k_ten_holds inlinable    88,034
    block walk, k_ten_holds noinline     85,250   (shipped)

The walk on its own is +5,312, and the `noinline` — added for a run-time
reason, to keep the tenure test out of `k_b_push_mut`'s fast path —
takes 2,784 of that back. So the shipped configuration is the cheapest
of the three that contain the win, and the only cell below it is the one
that gives the win up.

A block walk is smaller code than a hash probe, so the inliner is what
grew the binary. `k_survives_x` drops from 346 bytes to 118, which puts
it under clang's threshold at all eight of its call sites in the size
walk, and `k_repair_size` follows it in.

Two levers, built and measured:

- `noinline` on `k_repair_size`: +352 bytes on every binary. It does not
  stop the cascade, it only stops one participant from being folded into
  a caller that then keeps its own copy of everything else.
- `noinline` on `k_survives_x`: stops the cascade at its source and
  returns every byte. jsonbench 82,434, which is 288 BELOW where #1177
  left it, and -2,816 on each of the nine. It costs 33,136,876
  instructions across the nine, +0.2426%, and the cost is concentrated
  where the win was: deepbench +2.39%, escapebench +1.87%, pendbench
  +0.86%, widebench +0.71%, basket +0.40%, jsonbench +0.07%,
  encodebench +0.002%, oneshot +0.08%, indexbench -0.005%.

#1178 bought deepbench -4.47%. Handing back +2.39% of it to recover
22 KB of code that costs no instructions to carry is a trade in the
wrong direction, so the lever is declined and written down here rather
than left as a thing somebody re-derives.

One thing this does not settle. The golden's header says padding an old
build up with functions it never calls cost 1.6% of decode time on its
own, which is a wall-clock claim, and instructions are not wall clock.
jsonbench's instruction count FELL across #1178 while its .text rose, so
whatever the code growth costs is not work. Whether it costs cache is a
question for an idle box at a release sitting, not for this container.

## 2026-08-31 — the one-time branch purge, and the test that made it possible

Clay turned on auto-delete-on-merge, which covers every merge from here
and none of the 388 branches that had already piled up. This is the
sweep of those.

The obstacle was named in an earlier entry and it is real: the
repository squash-merges, so a landed branch's commits never become
ancestors of main. `git branch -r --merged origin/main` answers 1 out of
389, `--no-merged` answers 387, and `git merge-tree --write-tree` calls a
branch that certainly landed live. Every git-native test agrees the
branches are unmerged and every one of them is wrong.

What does work is the squash commit's own subject. A squash merge writes
the branch's subject with `(#N)` appended, so a branch carrying a commit
whose subject appears on main that way has landed. Two shapes were tried.
Matching the branch TIP's subject flags 176 of 388 and misses 75 of the
95 heads the pull-request record confirms merged — branch tips carry
fixup subjects that never became a PR title. Matching ANY commit on the
branch, computed as `origin/main..origin/NAME` so main's own history is
excluded, flags 325 and misses 11.

Precision on the ground-truth set is 84 of 84: every branch the test
flags among the 95 recent merges was in fact merged. The 11 it fails to
flag stay on the remote, which is the direction to fail in. Six more were
held back by hand because their matching subject was short enough for the
match to be luck — `a-record-field-waits-too`, `curly-maps`,
`forward-slash-scan`, `the-chart-gains-the-work`, `the-chart-has-to-draw`,
`welfare-reads-the-work`. That leaves 319 deleted and 69 standing.

The deletion did not run from here, and that is the honest state of it.
A ref-deletion push from the container is refused by the agent proxy with
HTTP 403 — `git push origin :refs/heads/beat-differential` gets "RPC
failed; HTTP 403" and the branch is still there — and the GitHub tools
this session has expose create_branch and no delete. The proxy's own
README says to report a 403 rather than route around it, so that is what
this is.

What ships instead is the sweep itself, ready to run:
`design/log/branch-purge-2026-08-31.txt` names all 319 beside the commit
each points at, and `scripts/purge_merged_branches.sh` reads that file
and does the deletion in batches. It re-reads the remote first and keeps
any branch whose tip has moved since the list was written, because a
branch somebody has pushed to is a branch with work on it; that guard was
watched refusing a row before it was trusted. Every row carries its
commit, so a branch deleted by it goes back with a single push. A sweep
that cannot be undone is a sweep nobody should run, and the record costs
21 KB.

## 2026-08-31 — the curve reaches every counter now, and the floor moves with it

Built to the 2026-08-29 gavel: the saturation curve applies to each
counter's ratio, and a term is the equal-weighted average of the
saturated values. Four lines of arithmetic, and the objective is a
different function.

```
    fn share now base t
      sat = t.satiation
      each = list/map t.counters (c -> satisfaction (ratio_of now base c) sat)
      sats = list/to_list each
      list/sum sats / (1.0 * length sats) * t.weight
```

`standing` moves with it. A counter new to the model enters where its
dimension already stands so that its arrival is not a score move, and
under the old rule that meant the mean ratio. The term is a mean of
saturated values now, and the curve is not linear, so a newcomer at the
mean ratio would shift the term. What leaves it alone is a newcomer
whose satisfaction equals the mean satisfaction, so that is computed and
read back through the curve: `r / (r + s) = m` inverts to
`r = m * s / (1 - m)`.

The floor falls 87.85 to 73.8273894965, and nothing about the compiler
moved — every counter reads today what it read yesterday. It is set by
hand in `bench/welfare_floor.json`, because the tool refuses a fall and
should: an objective change is exactly the case that belongs in a diff a
reviewer reads rather than behind a flag.

Two specs, in `tests/welfare_saturates_each_counter.rs`, and both pin
numbers that are properties of the weights rather than of the compiler.
The fixtures set every counter's baseline to its own current value, so
every ratio is exactly one whatever the goldens say today, and the
scores do not move when a benchmark gets faster.

- Every ratio at one scores **46.67**: run speed and run memory saturate
  at 1/(1+2.0) and carry 0.30 each, compile speed and compile memory at
  1/(1+0.5) carrying 0.28 and 0.12.
- One of the seven run-speed counters a thousand and twenty-four times
  better than its baseline, the other six at parity, scores **49.52**.
  Saturating the mean instead answers **66.26** on that same fixture,
  measured by putting the old `share` back and running it — one
  benchmark takes the term almost to its ceiling while six sit at
  parity, which is the shape the ruling closed.

The now-values the fixtures need come from welfare's own report rather
than from a second reader of the goldens. A spec that re-parses what the
tool parses is asserting its own copy of the tool, and the copy is what
goes stale.

Both were watched failing. With the old `share` restored, the runaway
spec reads 66.26 against 49.52; the parity spec passes under both
definitions, which is correct — at a ratio of one the two orders agree,
and that spec is pinning the weights rather than the ruling.

The ratchet's welfare mutation still bites, harder than before. Claiming
jsonbench did 9,999,999,999 instructions now costs 0.96 points where the
old rule diluted one counter's collapse across an unbounded mean.

What this does not do yet: the two improvements held behind the gavel —
`k_b_append_grow` outlined, and `k_b_append_into`'s duplicated grow tail
unified — are still out of the tree. The second was already priced under
both orders when it was held (-0.001096 as written, +0.008015 per
counter), and re-scoring them against the definition that now ships is
the next thing this owes.

## 2026-08-31 — the first held improvement ships, because the objective changed under it

`k_b_append_into` wrote its grow path twice — once taking arena storage
when the header dies at the innermost rewind, once taking malloc — and
the two differed in the allocator and in the sign of the cap and in
nothing else. Both then copied twice, freed the old header on the same
condition, and returned the same two ways. One tail serves both now, with
where the buffer comes from and the sign the cap carries decided in a
local above the copy. Twenty-seven lines in, thirty-nine out.

Measured on this container, callgrind, against a baseline built from the
same tree in a worktree so the two differ only in this patch:

```
    encodebench  8,704,347,511 -> 8,661,983,911  -42,363,600  -0.4867%
    jsonbench    2,858,844,826 -> 2,856,424,140   -2,420,686  -0.0847%
    oneshot         44,000,222 ->    43,878,175     -122,047  -0.2774%
    widebench       63,756,786 ->    63,788,800      +32,014  +0.0502%
    deepbench, pendbench: +14 each. basket, escapebench, indexbench: 0.
```

The container sits a constant below the runner — 399 to 427 instructions
on eight of the nine, 3,694 on deepbench — so the golden rows are the
runner's plus these deltas, and CI checks that arithmetic rather than
being asked to trust it.

widebench is the only benchmark in the tree where half the appends grow,
because `text/append (text/bytes lead) "  "` builds a fresh accumulator
per element and the first append onto one has no spare capacity. Its
32,014 over 16,000 grows is two instructions each.

Every allocation counter is byte-identical, on all eight veins. `.text`
falls 176 bytes on the four binaries that link the grow path and does not
move on the five that do not.

This is the change held on 2026-08-31 with the arithmetic already
written down: -0.001096 under the aggregation as it was, +0.008015 under
per-counter saturation. The second number is now the live one, and
welfare reads 73.8273894965 to 73.8358594052, a rise of 0.0084699 against
the 0.008015 that entry predicted, banked. Nothing about the change
moved; the objective did, and the entry that held it said in advance
which way it would go.

The second held improvement — `k_b_append_grow` outlined so the fast
path stops carrying six callee-saved pushes — is still out of the tree
and owes the same treatment.

## 2026-08-31 — the second held improvement ships too, and both are on the page

`k_b_append_grow` is out of line. One function held the fast path and the
growth: the fast path — two failure tests, a tag dispatch, a capacity
comparison and a store — needs two registers, and the growth needs eight,
because it decides which allocator the new buffer comes from, copies
twice and frees the old header. So every append pushed six callee-saved
registers and built a frame before it could test anything. Split, the
common path pushes three.

```
    k_b_append_mut:
      push %r15 / %r14 / %rbx
```

Measured on the container against the one-tail commit that precedes it,
so the two builds differ only in this patch:

```
    encodebench  8,661,983,911 -> 8,396,568,711  -265,415,200  -3.0641%
    jsonbench    2,856,424,140 -> 2,838,415,440   -18,008,700  -0.6305%
    oneshot         43,878,175 ->    43,094,579      -783,596  -1.7858%
    widebench       63,788,800 ->    63,996,800      +208,000  +0.3261%
    basket, deepbench, escapebench, pendbench, indexbench: 0, to the
    instruction.
```

widebench's 208,000 over its 16,000 grows is thirteen instructions each,
which is the call frame the grow path now enters. It is the only
benchmark where half the appends grow. Every other benchmark grows a
handful of times against millions of fast appends, so they take the three
fewer pushes and pay nothing for the call — which is why five of the nine
do not move at all.

Every allocation counter is byte-identical on all eight veins. `.text`
rises 128 bytes on the four binaries that link the grow path, which is
the outlined function's own prologue and epilogue where before it shared
the caller's, and does not move on the five that do not link it.

Welfare 73.8358594052 to 73.8913121796. Under the old aggregation this
change was -0.002692 and held; the entry that held it bisected the number
by holding one row at a time and found widebench's 0.267% rise costing
0.003311 points on its own, outweighing encodebench's 3.6% fall. Under
per-counter saturation encodebench's fall is worth what it is worth and
widebench cannot spend a whole term.

Both held improvements are now in, and §31 of the compiler page says so
with what each bought. What the two of them together demonstrate is what
the ruling was for: neither change moved, the objective did, and both
went from negative to positive. The per-counter goldens could not have
told the difference — every allocation counter is byte-identical across
both.

## 2026-08-31 — the runner disagreed with the container by fourteen instructions, four times

CI priced the outlined grow path and four of the nine rows came back
fourteen instructions off what the container's delta implied.

```
                    implied        runner
    jsonbench    2,838,415,867  2,838,415,853
    widebench       63,997,227     63,997,213
    deepbench      726,486,948    726,486,934
    pendbench      946,378,088    946,378,074
```

Every row is exactly fourteen lower, and deepbench and pendbench do not
move on the runner at all — the container's +14 on each was the whole of
their reported change. So the container adds fourteen instructions
somewhere this patch happens to touch, and the golden takes the runner's
rows, which is what it is stamped for.

Worth writing down because the practice this session has been using —
measure the delta on the container, add it to the golden — held to the
instruction across the tenure attribution and the one-tail change and
then did not here. The deltas are still the right way to reason; the
golden is still the runner's; CI is what tells the two apart, and it did.

`compile_instructions` falls 42,299,748 to 42,298,874. Nothing about
compiling got cheaper: `src/runtime.c` reaches the compiler through
`include_str!`, this change shortens it by twelve lines, and the
compiler's own binary is laid out differently as a result. This is
codegen movement and it is named as such — `compile_allocs` and
`compile_peak_bytes` are byte-identical at 29,864 and 728,030, which is
what a change that did no compiler work looks like. Fifth instance this
week of the same cause.

Welfare 73.8913121796 to 73.8913693718 on the corrected rows.

## 2026-08-31 — the inline name exists, and closing its variants was the finding

First half of the 2026-08-29 inline-name ruling: the type, with its spec.
The twenty-nine AST fields that will adopt it are a separate change, and
this one is worth landing on its own because it is the piece the rest
rests on.

Twenty-two bytes of inline capacity, and it is a measurement. Across
`lib/`, 89.8% of identifier occurrences are seven bytes or fewer and
99.77% are twenty-two or fewer; thirty distinct names run longer and each
appears exactly once, nearly all of them test function names. So the heap
path is real and exercised and is not what anything hot takes. Twenty-two
also lands the whole type on twenty-four bytes, which is what a `String`
already costs, so no AST node grows by adopting it — pinned, not assumed.

The finding came out of watching a mutation fail to fail. A `PartialEq`
comparing representations instead of text passed the entire spec. The
reason is that `Name::new` decides by length, so through the constructor a
name's representation follows from its text and the two can never
disagree — but the variants were public, so a caller could box a short
name by hand and produce a name that read the same as another and
compared unequal to it.

The answer was to close the variants rather than to write a bigger
assertion. `Name` is a struct around a private `Repr` now, `new` is the
only way in, and the invariant is enforced instead of tested for. That is
the difference between a spec that catches a mistake and a type that
cannot have it made.

Four mutations, each watched going red for its own reason:

- never take the inline path: the allocation counter reads 1 where 0 is
  required, and three specs go red including the multibyte one.
- compare representations in `PartialEq`: passed everything, which is what
  closed the variants.
- hash the representation, as a derive would: the map lookup by `&str`
  answers `None` where it should answer `Some(1)`. `Borrow<str>` requires
  the borrowed form to hash identically and this is what enforces it.
- order by representation: the sorted list stops matching the sorted text.

The allocation property is asserted through a counting global allocator
around `Name::new`, comparing a reading before against one after, rather
than by asking the type which variant it is. `is_inline` exists for the
spec to cross-check with and nothing in the compiler should need it.

A seventh instance of the shared-fixture family, found by the spec flaking
before it was committed. The allocation counter was a `static`, and cargo
runs these tests on parallel threads, so another test's allocation landed
between the two readings and the delta read 2 or 3 where the spec demands
1. Measured both ways: **10 failures in 40 runs** with the shared counter,
**0 in 40** with a thread-local one. A counter shared by threads measures
the process; what is under test is one call on one thread.

The six before it were staged files and directories keyed by something two
tests could collide on. This one is a counter, which is why the sweep that
found those could not have found it — the shape is "one mutable thing two
tests reach", and the file path was only ever the commonest spelling of it.

## 2026-08-31 — DONE: the AST's identifiers are inline names (#1188)

The second half of the inline-name ruling, and the half that pays. The type
landed on its own first and welfare refused it: 96,963 compile instructions
for a module nothing called. That was the objective working. A type with no
user is a cost with no benefit, and the unit of change is the type together
with its first user.

Six per-occurrence positions changed from `String` to `Name`: `Tok::Ident`,
`Expr::Ident`, `Expr::Partial`, `Pattern::Var`, `Pattern::Nullary`, and the
type name on `Pattern::Ctor` and `Pattern::Annotated`. The dhat map from
2026-08-28 put 6,983 of the front end's blocks on three of those sites —
`parser.rs:2127` at 3,197, `lexer.rs:631` at 3,157, `parser.rs:1793` at 629.

Downstream tables keyed by declaration stay `String` and convert at the
boundary. They are filled once per declaration rather than once per mention,
so a `to_string()` there is O(declarations) and leaves the traffic where the
win is. Measured rather than assumed: the whole conversion is worth 4,470
blocks, which is 64% of the three sites' 6,983, and the remainder is names
over twenty-two bytes plus the boundary conversions.

Measured in the box on `lib/json`, both sides on the same host (glibc
2.39-0ubuntu8.7, rustc 1.94.1, so these are deltas and the goldens' rows
come off the runner):

    compile_allocs        29,864 -> 25,394     -14.97%
    compile_peak_bytes   728,030 -> 713,606     -1.98%
    compile_instructions 42,734,177 -> 41,923,829  -1.90%
    compile_rounds        40 -> 40
    compile_visits        16,806 -> 16,806

The runner's rows, which are what the goldens carry:

    compile_allocs        29,864 -> 25,394        -14.97%
    compile_peak_bytes   728,030 -> 713,606        -1.98%
    compile_instructions 42,298,874 -> 41,496,870  -1.90%

Allocations and peak read the same number on both hosts, as they have every
time; the instruction rows differ by host and agree to within four hundredths
of a percent on the direction and size of the move, which is what the
delta-plus-golden practice is for.

Rounds and visits are byte-identical, which is the check that this changed
how the front end holds names and not what it decided. The emitted code is
identical for the same reason: the cost-goldens job reported every runtime
vein green — emitted, machine code, work, and the six benchmark counter sets
— and only the three compile veins moved. kq's five stay where kq#83 left
them.

welfare 73.8913 -> 74.3319, ratcheted in the same commit.

#1033 declined the interned symbol at "365 conversion sites", and the
propagation this hit is the same phenomenon at a tenth the size: 126
compiler errors, all mechanical, because `Expr::Partial` or-patterns with
`Expr::Ident` in the walks and had to travel with it. The difference from
#1033 is that an inline name needs no interner, no arena and no lifetime —
it is the same twenty-four bytes a `String` occupied, so the conversion is
the whole cost and there is no ongoing one.

### the 23.5% that was hiding in `as_str`

The first build read **+32% instructions** — 56,538,789 against a baseline
of 42,734,177 — and `core::str::converts::from_utf8` was 13,295,370 of them,
23.52% of the whole front end. `as_str` was validating utf-8 on every read
of bytes that had come out of a `&str` to begin with. The comment above it
already argued the invariant and the code checked it anyway, which is the
shape to watch for: a proof written down beside a runtime test of the same
claim means one of the two is redundant, and here the expensive one was.

It reads unchecked now, and what makes that sound is structural. `Repr` is
private to the module, `Repr::Inline` is built in exactly one place, and
that place copies `s.as_bytes()` whole and records the slice's length. The
same closing of the variants that #1188's first commit made for `PartialEq`
is what licenses this: a caller cannot hand-build an inline name, so the
range is always a complete `&str`.

`a_name_holds_every_encoding` reads the range back through the public door
for one-, two-, three- and four-byte characters, at the longest length that
still fits inline and again one character over. Watched red two ways:

- length recorded in characters rather than bytes: the invalid `&str`
  reaches the formatter and the process aborts. Under the checked read this
  was a panic with a message; under the unchecked one it is undefined
  behaviour that happens to be loud, which is the reason the spec has to
  read the bytes back rather than trust the type.
- threshold strict, `< INLINE` rather than `<=`: `é` eleven times is
  twenty-two bytes and spills, and two specs go red.

The existing multibyte spec covers three-byte characters only, so it catches
the first mutation and not the second. Four widths at the boundary is what
the unchecked read is worth.

### a smaller thing, recorded because it will look like noise later

Removing nine redundant `Name::new(&x.to_string())` calls that clippy
flagged moved compile_instructions **up** 80,439, from 41,843,390 to
41,923,829. Nine fewer allocations should not cost 0.19%, and the reason is
the same one #1186 attributed: the compiler's own binary rearranges when its
source does, and `Name::new` is small enough for the inliner to change its
mind about. The shorter source is kept because it is the right code; the
number is recorded so a later reader does not go looking for a regression.

## 2026-08-31 — MEASURED, OPEN: a digest holds every block it walked

Task #82 asked whether sha256 wants an arena. It has one; the arena never
rewinds inside the walk, and that is the whole answer.

Measured in the container against files, so the input is read rather than
built — an earlier attempt grew the message by repeated interpolation and
half the peak was the benchmark's own quadratic copying:

    input      allocs    arena_peak_bytes   bytes of arena per input byte
     8,800    713,523        90,177,536          10,247
    88,000  7,118,218       889,192,464          10,104

Ten times the input for ten times everything, and 849 MB of resident set to
digest 88 KB. `alloc_bytes` and `arena_peak_bytes` agree to within one per
cent at both sizes, which is the finding stated exactly: nothing is reclaimed
between blocks, so the peak IS the total. A megabyte would want about ten
gigabytes.

sha256 is streaming. Its working set is sixty-four schedule words and eight
state words, and everything a block builds is dead the moment the next block
starts. The peak should be flat.

massif attributes it to two owners, both inside the walk:

    66.21%  59,769,744  k_b_push_into_proven
    32.52%  29,360,576  k_b_concat, 31.36% of it from sha256/compress

`compress` builds an eight-element list literal on each of its sixty-four
rounds; `schedule`, `first_sixteen` and `padded_bytes` do the pushing.

KANSO_BEAT_REPORT says why nothing sweeps. `sha256/digested`, `sha256/blocked`,
`sha256/compress` and `sha256/turned` all read "bracketed with its cluster",
where `sha256/filled` — the one self-recursive loop in the file — reads
"rewinds every iteration". The outer block walk and the inner sixty-four-round
compression are in ONE cluster, so the only place a mark can go is outside
both.

Five hypotheses were built and killed, and every one is worth recording
because each is an obvious guess that a reader would otherwise make again:

- "A trampoline defeats the rewind." A loop written self-recursively and the
  same loop written as the two-function trampoline the library uses everywhere
  both hold one arena block over 500,000 iterations and 1,000,002 allocations.
  Bracketing on its own reclaims fine.
- "Carrying a list rather than a scalar defeats it." A loop carrying an
  eight-element list rebuilt every round reads "carry beat: rewinds every
  iteration, evacuating argument 1" and also holds one block, with 1,000,000
  evacuations. The machinery handles that case.

- "A cluster spanning two NESTED loops defeats it." An outer walk with an
  inner sixty-four-round loop, all four functions reading "bracketed with its
  cluster", holds one block over 1,024,002 allocations.
- "The nested shape with lists on both sides defeats it." The same, with the
  inner returning a fresh eight-element list and the outer folding it into its
  own, reads 6,864 allocations for 512,000 iterations — the uniqueness
  analysis turns the literals into in-place writes and there is nothing left
  to reclaim.
- "Building one long list by repeated `push` is quadratic, as `padded_bytes`
  does it." Ten allocations for eight thousand pushes; the growth doubles and
  the arena holds one block at 2,000, 4,000 and 8,000.

So it is not the spelling, not the carried type, not the nesting, and not the
growth path. Four reproductions that ought to show the habit do not, which
means the cause is narrower than any of them and the next session should start
by bisecting sha256 itself rather than by building a fifth model of it. The
attribution above is the place to start: `k_b_push_into_proven` at 66% and
`sha256/compress`'s list literal at 31%, with the whole thing in one cluster.

`tests/golden/mem/a_digest_holds_every_block_it_walked` pins the constant at
one block — 1,980 allocations and 424,369 bytes for forty-four bytes of
message, which is forty-five allocations per input byte before any
accumulation. A fixture that ran at the sizes above would cost the golden
suite minutes, so the slope lives in this entry and the constant lives in the
vein. Watched, not frozen.

The arena question in #82 is answered and closed: sha256 has an arena and the
arena does not rewind inside the walk, at 10,000 bytes of it per input byte.
What replaces it is narrower and still open — WHICH of sha256's sites holds,
given that four models of the shape all reclaim. The way in is to bisect the
file: run the walk with `compress` stubbed to return its state unchanged, then
with `schedule` stubbed, and see which stub flattens the peak.

## 2026-08-31 — IDENTIFIED: the digest's 86x is a source-path prefix

The entry above said the cause was not identified and that the way in was to
bisect sha256 rather than model it. Bisecting found it in one step, and the
answer is not in sha256 at all.

Copy `lib/sha256/sha256.kso` into a program's own directory and import it as
`./sha256` instead of `std/sha256`. Same bytes, same digest, same allocation
count to within 0.04%:

    import "./sha256"     allocs 713,807   arena_peak_bytes  1,048,576
    import "std/sha256"   allocs 713,523   arena_peak_bytes 90,177,536

Eighty-six times the peak from the import line. The emitted IR says what
changed: the local build emits four `k_beat_iter_carry` rewinds, at
`sha256/compress`, `sha256/turned`, `sha256/digested` and `sha256/blocked`.
The std build emits none. Every one of the nineteen sha256 functions has the
same emitted name in both, so it is not a lookup miss on a spelling.

`src/beat.rs`, the filter this file's 2026-07-27 entry describes narrowing
from "every group whose name contains a slash" to "imported groups stay out
of the carry tier only":

    let imported = program.fns.iter()
        .filter(|d| d.file.starts_with("std/") || d.file.starts_with("lib/"))

`d.file` is the source path, and `ast::FnDecl` documents it as the thing err
origins are built from — "{name} at {file}:{line}". It is a diagnostic field
being read as a semantic marker, by string prefix. Two consequences, and they
are different in kind:

**One, deliberate.** A genuine `std/` import loses its carry beat, and the
`ids.retain` two lines below then drops the id as well, so the group gets
neither the carry rewind nor the plain bracket. sha256's block walk is
collateral: it carries an eight-word state list, not the unbounded
caller-invariant the rule was written against. That is the 90 MB, and it is
the implementer's to settle — the ledger bounced this class on 2026-08-29.

**Two, a defect.** The `lib/` arm is a RELATIVE PATH PREFIX on the user's own
source, so a program's memory behaviour depends on the directory it was built
from. Proven with identical bytes:

    built from an arbitrary directory   arena_peak_bytes  1,048,576
    built from lib/app                  arena_peak_bytes 90,177,536

The arm is not dead weight — `kanso check lib/json` is how the compile gate
measures the library, so removing it moves the compile veins. But a user who
keeps their code in a directory called `lib` gets a different program, and
nothing in the tree says so.

Both are one change away from each other and neither is started. What the
fix needs is a real marker for "this declaration came from an imported
module", set where imports are resolved rather than inferred from a path,
and then a decision about whether the carry exclusion should survive on that
marker at all. The measurement to make first is the cheap one: strip the
exclusion, and read the compile veins and the sha256 peak together.

The four A/B models in the entry above are not wasted. They are what proves
the exclusion is the only thing holding the digest, because every one of them
is the same shape without the `std/` import and every one reclaims.

## 2026-08-31 — the measurement the entry above asked for: the exclusion costs eleven lines

Stripping the imported-group exclusion entirely — `.filter(|_d| false)` in
place of the path-prefix test — and reading every vein the tree has, on this
container:

    sha256 via std/, arena_peak_bytes   90,177,536 -> 1,048,576   (86x)
    sha256 via std/, allocs                713,523 ->   713,807   (+284)

    compile_allocs        25,394   unchanged
    compile_peak_bytes   713,606   unchanged
    compile_rounds            40   unchanged
    compile_visits        16,806   unchanged

    decode, encode, one-shot, basket, escape, wide and pending-cell
    counters: all seven byte-identical
    machine code: byte-identical
    kq's three allocation cost goldens: byte-identical
    welfare: 74.33, unchanged

    emitted code: scanbench calls 3,743 -> 3,753, lines 20,019 -> 20,030.
    Every other benchmark byte-identical.

Eleven emitted lines in one benchmark is the whole measurable price, and the
+284 allocations are the evacuation copies the carry rewind pays for. The
compile veins do not move at all, which answers the worry the entry above
raised about `kanso check lib/json`: the library's own groups either never
wanted a carry or were already getting one.

The rule's stated hazard — a shared library driver threading its caller's
invariant source through the loop, so carrying copies an unbounded value per
iteration — does not appear anywhere in the corpus. That is a finding in its
own right and cuts both ways: it is evidence the exclusion is over-broad, and
it is evidence that nothing in the tree would catch the hazard if it were
real. A fixture that exercises it is owed either way, and it does not exist.

So the naive strip is measured and cheap, and it is deliberately NOT what
ships next. What ships next is the marker: `d.file` is a diagnostic field and
reading it as a semantic one is what made a directory name change a program.
Whether the exclusion then survives on a real marker is the question the
fixture above has to be written before answering, because a rule kept for a
hazard nothing demonstrates is a rule kept on faith.

## 2026-08-31 — CORRECTION: the exclusion has a pin, and it is about correctness

The entry above says the rule's stated hazard "does not appear anywhere in the
corpus" and reasons from there that the exclusion is over-broad. That is wrong,
and the way it was wrong is worth more than the conclusion was.

`beat::tests::json_decode_loops_stay_conservative` in src/beat.rs is the
corpus statement of it:

    assert_eq!(licensed, vec![("encode_items", 3), ("encode_pairs", 3)],
        "only the byte-builder encoders may rewind; scanners threading
         records or lists stay on the grow-only arena");

with a comment that argues safety rather than cost: the two encoders thread a
byte builder by pointer identity, "raw bytes hold no pointers, so nothing in
the accumulator can dangle across a rewind." Removing the exclusion admits
`list/bisect`, `list/found_in`, `list/holds_all?` and `list/holds_any?` to the
carry tier and turns that assertion red.

I missed it by searching design/compiler-log.md and the archive and not the
test suite. The filing gate in design/pending-gavels.md names three places to
search before calling a question unanswered, and all three are prose. A pin
lives in code, and this one carries an argument no log entry repeats.

So the naive strip is refused, and it should be. What looked like a
performance heuristic with no evidence behind it is a boundary on what may
rewind at all, and a carried list of records is on the far side of it. Whether
the carry machinery's evacuation already covers the case the comment worries
about — `k_carry_stage` and `k_carry_take` exist precisely so a carried value
survives a rewind — is a real question, and it is a question about pointer
lifetime that wants a differential fixture rather than my reading of a
comment.

What the measurements above still say, unaffected:

- Peak goes from linear in the message to flat when a digest's walk may
  rewind: 108,003,328 bytes to 1,048,576 at ten kilobytes, 426,770,448 to
  4,194,320 at forty-three, for 0.04% more allocations.
- At forty-four bytes — one padded block, nothing to reclaim — the same change
  costs 1,980 allocations to 5,259 on the interpreter and four natively. The
  rewinds are pure overhead when there is only one block.
- Only the digest fixture moves in the whole mem corpus, and only scanbench in
  the emitted vein (+10 calls) and the scan counters (beat_iters 15 -> 16).
- The compile veins, the seven runtime counter veins, machine code, kq's three
  allocation goldens and welfare are all unmoved.

And the defect is untouched and still real: `d.file.starts_with("lib/")` is a
relative path prefix, so a program built from a directory called `lib` gets a
different program. Fixing THAT without touching the boundary needs the marker,
because the `lib/` arm is what makes the repo's own `lib/json` behave like an
installed module — this very test compiles `lib/json` directly and depends on
it. A marker set where imports are resolved would have to answer for a module
compiled as a root as well as one reached through an import, and that is the
design question, stated properly at last.

Nothing shipped from this. The branch is reverted to main and the fixture, the
bisection and these measurements are the whole product.

## 2026-08-31 — the fixture the carry boundary was missing

The correction above says the carry exclusion is a boundary on what may rewind
and that the pin's argument — raw bytes hold no pointers, a list of records
might — wants a fixture rather than a reading of the comment. This is that
fixture, and it lands on its own because it is worth having whatever the
boundary turns out to be.

`tests/golden/micro/a_library_scanner_threads_records_across_a_rewind` carries
a window of eight records through four hundred rounds, shifting it each time,
and prints a field out of the element that has passed through the carry eight
times since the round that built it. `m/shifted/2` reads "beat: rewinds every
iteration", so the records really are live across a rewind, and both engines
answer `8 18974 19170`.

It is asked at the observable end. If a rewind ever left those elements
pointing into arena that had been given back, the printed field is where it
shows, and the micro corpus compares the interpreter against native and
against a release build — so a divergence appears as a diff rather than as a
plausible-looking number one engine invented.

Watched red before it was kept: shifting the window wrong by one slot
(`s[6]!` twice) turns both the library arm and the release-built arm red on
the right line. It passes today because these groups are not carried today;
it exists so the change that admits them has something to be wrong against.

The corpus had nothing in this shape. Every mem fixture pins allocation counts
and every micro fixture that touches records reads them without a rewind in
between.

## 2026-08-31 — the carry exclusion removed, measured properly, and REVERTED

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, and — after the correction two entries above — the test suite,
`beat.rs` itself, and the runtime functions the exclusion's comment makes
claims about.

The exclusion is `d.file.starts_with("std/") || d.file.starts_with("lib/")` in
`beat_loops`, and three things are wrong with it. Two of them survive this
entry; the third is the reason nothing shipped.

**The field is a diagnostic.** `file` is what error origins are built from, so
`std/sha256` and a user directory called `lib` read alike, and the same package
compiled from `lib/app` holds 27,262,976 arena bytes where the same sources
under `elsewhere/app` hold 1,048,576. `tests/a_program_is_not_its_directory.rs`
pins BOTH numbers now, in the shape `tests/sha256_peak.rs` already uses: the
defect asserted as a fact, so it cannot be lost while the fix waits.

**The premise the comment gives is false.** It says a shared library driver
threads its caller's invariant source through the loop, so carrying it copies
an unbounded value per iteration. `k_beat_iter_carry` copies only what lies
above the mark, and a loop's mark is pushed at entry, so the caller's source is
below it and shared. A fold over a list built through `std/list` allocates
556,768 bytes at two thousand elements and 2,223,856 at eight thousand.

**And it is not enforcing the pin that refused its removal.** With the test
gone, `lib/json`'s licensed set gains `list/bisect`, `list/found_in`,
`list/holds_all?` and `list/holds_any?` — three boolean predicates and a binary
search — and every group json itself declares reads the same on both sides.
json's scanners are refused by the classifier. The exclusion's whole live
effect is those four predicates and sha256's cluster.

WHY IT IS REVERTED. The trade is memory for time and the time is quadratic.
Native, ASCII message read from a file, each arm built in its own directory:

| message | before, time | before, peak | after, time | after, peak |
|--------:|-------------:|-------------:|------------:|------------:|
|   8,000 |      0.084 s |   79,691,776 |     0.309 s |   1,048,576 |
|  16,000 |      0.170 s |  159,383,552 |     1.152 s |   1,048,576 |
|  32,000 |      0.331 s |  316,669,952 |     4.367 s |   1,048,576 |
|  64,000 |      0.675 s |  633,339,920 |    16.535 s |   4,194,320 |
| 128,000 |      1.304 s |1,262,485,520 |    68.168 s |   4,194,320 |

The digest is linear in time today. With the carry it is quadratic — 52x at
128 KB, and CI said so before this table existed: the `asset digests` job
digests a 1,604,098-byte wasm and took 49 seconds on main, then sat past
twenty-five minutes on the branch. Trading 1.2 GB for 52x is not a trade this
project's weights would take, and welfare cannot arbitrate it because no
benchmark in the suite streams (**OPEN**, and the reason a digest benchmark is
worth building).

**HOW THE FIRST REPORT GOT IT WRONG, because the mistake is easy to repeat.**
Three separate A/B runs said the change was wall-clock neutral. All three were
invalid. `kanso build` caches the native binary, and rebuilding the RUST
compiler does not invalidate that cache — the key is over the sources and
`runtime.c`, not the compiler. So rebuilding `src/beat.rs` and re-running
`kanso build` in the same directory re-ran the SAME binary in both arms, and
the two columns agreed because they were one column. The numbers above come
from a fresh directory per arm. Any A/B on emitted or runtime behaviour has to
build into a directory the other arm never touched.

WHAT IS LEFT OF THE PEAK, run down while the branch was still alive and true
whichever way this goes: `sha256/padded_bytes` opens with `list/to_list b`, so
the message is held as a list of integers at sixteen arena bytes an input byte
for as long as `digested` indexes it. massif at 64 KB put 46.17% of the peak —
2,097,184 bytes — on `k_b_push_into_proven` under `d_list/fold_3`, the buffer
that list is grown into. A program that reads the same file and prints its
length holds one block at 400 KB with eight allocations, because a bytes value
never becomes a list. That is lib/sha256's to fix and nothing here is in the
way of it.

TWO THINGS FOUND ON THE WAY, both surviving the revert:

- **`thunk_evals` is not engine-shared and never was.** Measured on main: a
  1,000-byte sha256 reads `thunk_forces=1024 thunk_evals=1024` native and
  `1088`/`17` interpreted. `k_memo_outlives` declines the memo when the answer
  was built inside the innermost beat; the interpreter has no arena to rewind.
  `mem_corpus_interp_matches_the_semantic_counters` classes evals with allocs
  and forces as engine-shared semantics, and no fixture has ever asked. The
  classification is left alone here — a gate that is not failing is not
  weakened on the way past — and the measurement is recorded so the next
  fixture in that shape is not a surprise. **OPEN.**
- **Two tests in `a_file_that_is_not_text.rs` shared one temp directory and one
  `run.kso`, and raced.** Caught once as `an entry file needs at least one
  statement` about a file that has one. One directory per test now, the same
  fix kanso#1169 made for the playground pair. That is the only compiler-facing
  change that ships from this branch.

WHAT SHIPS: the race fix, the directory defect pinned with both its numbers,
and this entry. The removal is built, measured and declined — recorded so the
next attempt starts from the curve rather than from the idea.

**OPEN, and it is the whole question now:** where the quadratic is. Every
deterministic counter is linear across the same range — allocs, alloc_bytes,
beat_iters, evac_allocs, evac_bytes all double when the message doubles — so
the cost is in work no counter watches. `k_beat_iter_carry` sizes and copies
the carried slots per iteration and `k_copy_size` walks rather than counts;
that walk is where to look first.

## 2026-08-31 — the quadratic has a name: k_slots_survive, at 80.66%

The entry above left it open. callgrind, on the digest built WITHOUT the
exclusion, 8,000-byte ASCII message from a file, 3,871,213,343 instructions
total:

```
3,122,380,806 (80.66%)  k_slots_survive
  106,378,584 ( 2.75%)  k_index
   73,497,528 ( 1.90%)  k_b_bit_shr
   71,987,328 ( 1.86%)  d_thunk_eval
   67,448,520 ( 1.74%)  k_shift_of
```

Four fifths of the run, in one function, on a program whose every allocation
counter is linear.

`k_slots_survive(slots, n, m)` loops over a node's immediate interior asking
`k_survives_x` of each heap slot, and its answer decides whether `k_copy_size`
and `k_deep_copy` may SHARE the node instead of copying it. For a list that
loop is `l->len` long, and the list the digest's evacuation reaches is
`padded` — the whole message as a list of integers, from `list/to_list b` in
`sha256/padded_bytes`. So one ask is O(message), the evacuation asks per
iteration, and the iterations are O(message).

THERE IS ALREADY A MEMO ABOVE IT and it only helps in one direction. The
`K_LIST` arm of `k_interior_survives` caches, for lists of `K_ISV_MIN` (64) or
more, whether the list survives the OUTERMOST mark — keyed on the list pointer,
invalidated when `k_beat_stack[0].ptr` moves. When that cached answer is yes it
returns 1 and the scan never runs. When it is no it falls through to the full
`k_slots_survive` anyway, and stores the no. `padded` is built long after mark
zero, so it is never an outer survivor, so the memo answers no forever and
every ask pays the whole list.

So the shape of a fix is narrow rather than architectural: the negative answer
has to be worth something too. Whether a node's interior survives depends on
the mark it is asked about, and within one evacuation that mark is fixed —
which is the seam. Nothing is built here; this entry is the profile and where
it points.

WHAT ELSE IT SAYS. A counter for this would have caught the regression in the
cost goldens instead of in a CI timeout: `evac_bytes` counts what an evacuation
COPIES, and the whole cost here is in deciding not to copy. Slots examined per
evacuation is platform-invariant, algorithm-level, and exactly the missing
dimension. That is a presence counter this project's own rule already asks for
and it does not exist.

The 2026-08-30 note beside `k_is_heap` says `k_copy_size` is 36% of deepbench
and that a tenth `case` in the tag switch cost that benchmark 6.14%. The same
walk is 80.66% here. deepbench is the closest thing the suite has to this
shape and it is nowhere near it.

## 2026-08-31 — digestbench: the one benchmark whose peak is the point

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, and the benchmark corpus itself — `bench/` and
`scripts/gates/build_benchmarks.sh`, which is the list this joins.

The entries above this one describe a change that took an 8 KB sha256 digest
from 79,691,776 arena bytes to 1,048,576 and scored **exactly zero** against
welfare, and whose 52x wall-clock regression nothing in the corpus was shaped
to notice. Both blindnesses are the same gap: every program the suite weighs
either holds its whole subject on purpose — the decoder and the encoder do —
or is small enough to sit in the arena's first block. So "does the peak grow
with the input" is weighted at zero by omission.

`bench/digestbench` is that shape. It reads `bench/digest_input.txt` — 8,192
deterministic printable bytes — at runtime and digests it once. Runtime read
for the reason bench/make_jsonbench already gives: a compile-time fixture
would let the optimizer fold the digest and flatter every row. Once rather
than in a loop, because the peak of one run is the number this exists for and
a loop over a walk that never reclaims would measure the loop.

What it reads today:

```
allocs=652817  alloc_bytes=81846129
arena_blocks=79  arena_peak_bytes=82837504
```

82,837,504 arena bytes for 8,192 bytes of message — 10,112 bytes an input
byte. sha256 walks its message sixty-four bytes at a time carrying eight state
words, and everything a block builds is dead when the next one starts, so that
number is a property of the algorithm in a compiler that reclaims between
blocks and a property of the MESSAGE in one that does not. It is the second
case, and `scripts/gates/digest_counters.sh` says so where a reader will meet
it.

It costs 0.333 s, which sits between jsonbench at 0.283 and encodebench at
0.819 — the middle of what the suite already pays.

WHY THE EMITTED VEIN WORSENS, and the gate defect it exposed.
`emitted_other_defines` 1,163 -> 1,323, `emitted_other_calls` 12,739 ->
14,310, `emitted_other_branches` 7,484 -> 8,341 and `emitted_other_lines`
73,669 -> 82,751. Those four rows are SUMS over the benchmarks in
`bench/emitted_golden_others.txt`, and a tenth benchmark adds a tenth
program's code to each. No existing program emitted a line more than it did
before; digestbench's own row is 160 defines, 1,571 calls, 857 branches and
9,082 lines, and the four deltas are exactly it. The trend gate called that a
pure regression and refused the change.

The gate was right to sum and wrong to compare. `on_line` drops a line's
leading name so the fields add across a golden's samples, which is the
treatment compile_golden's five samples get and is correct for a fixed set —
and meaningless the moment the set changes, which is the error library_box.sh
names in its own domain. `against _ ""` already said a golden this branch
CREATED has nothing to be compared against; the gate now says the same one
level down, for a row added inside a golden that existed.

It drops the joining sample from the current side and sums both over the
samples they share. THE FIRST DRAFT SKIPPED THE WHOLE GOLDEN instead, and the
mutation caught it: with `scanbench calls` moved 3,743 -> 9,999 in the same
commit that adds a benchmark, the skip passed silently and the intersection
still fails. Three mutations were run and all three answer right — a rise with
no sample joining, a rise with one joining, and the real case where only a
sample joins.

STILL OPEN, and deliberately not done here: the welfare wiring. Adding
`digest_peak_bytes` to `run_memory_counters` and `digest_instructions` to
`run_speed_counters` re-baselines the index — every term is a ratio against
`bench/welfare_floor.json`'s baseline block, so a new counter needs a baseline
entry and the floor re-set with its reason. That is the change that makes the
objective able to arbitrate the trade it could not arbitrate today, and it
wants doing on its own with the ratchet read carefully rather than bolted to
the benchmark's arrival.

---

## 2026-09-01 — an unclassified counter could pay for a worsening

**DONE.** The trend gate refuses "a pure regression: something got worse and
nothing got better". `worsened?` reads a counter's direction out of two tables,
`lower` and `higher`, and answers **false** for a name in neither. The listing
then printed every move that was not worsened as `improved`, and `better` was
built by rejecting the worsened ones. So a counter no table classifies landed
in `better` whichever way it moved, and no worsening was ever alone.

Found while lifting the carry-tier prefix (see the entry above): that change
read `evac_allocs 27 -> 33,827`, `evac_bytes 1,520 -> 2,705,520` and
`thunk_evals 1 -> 64` as three improvements. Each of those is work done, and
each rose.

A move whose counter no table names is now a third state. It prints as `moved`
under an UNCLASSIFIED heading and counts toward neither side, so the
pure-regression rule is decided only by counters with a direction. The heading
also names them, which is the actionable half: a counter nothing classifies
wants classifying, and until it is, the gate says so out loud rather than
quietly crediting it.

The direction tables are deliberately NOT extended here. Adding
`evac_allocs`/`evac_bytes` to `lower` and calling it done would leave the same
hole for the next unnamed counter, and the tables are a judgement about what a
counter means that wants making one at a time rather than in a batch beside a
gate fix. **OPEN:** the mem corpus alone carries `cohort_frees`,
`cohort_kept`, `evac_allocs`, `evac_bytes`, `put_mut_fast`, `put_mut_grow`,
`push_mut_fast`, `push_mut_slow`, `thunk_evals`, `str_scans`,
`str_scan_bytes`, `view_allocs`, `view_frees`, `perm_live_bytes` and
`perm_peak_bytes` with no direction. The gate now lists them on any run that
moves one.

Ratchet row `trend_adrift`, mutation
`a_worsening_paid_for_by_an_unclassified_counter`: it raises `scanbench calls`
(in `lower_d`, a real worsening) and `evac_allocs` in the digest golden (in
neither table) in the same patch. A gate that reads the second as an
improvement goes green; this one stays red.

---

## 2026-09-01 — a memo declined inside a beat, and the third state that keeps it

**DONE.** Closes the 64x thread opened on 2026-08-31.

`k_force` memoized a thunk's answer only when `k_memo_outlives` said the
storage sat below the innermost beat mark. Inside a beat it usually does not,
and the memo was declined outright: the cell stayed unforced and the whole
computation ran again on the next force. In the streaming shape that is once a
block. On the 8,192-byte digest, `thunk_forces` and `thunk_evals` were both
8,256 — every single force re-evaluated.

**The third state.** The memo is kept and marked AT RISK instead of declined.
A rewind may take the answer away; a carry evacuation may move it somewhere a
rewind cannot reach, which `k_deep_copy`'s thunk arm already does on purpose
(`if (t->forced) t->result = k_deep_copy(t->result, cp)`). Which of the two
happened has a cheap answer — is the storage still in the live chain — and it
is asked at the next force, where it costs a walk of a block list instead of a
re-evaluation. `forced` carries three values now rather than two, so the flag
rides on the cell and there is no second structure to keep in step.

**Why not a register of at-risk cells checked at each rewind.** That was
written first and thrown away. A thunk cell is refcounted and can be freed and
handed out again while a list still names it, so the list would have to be
kept in step with the free list, and touching a freed cell to un-memo it is the
bug the register exists to prevent. The flag cannot go stale because it dies
with the cell.

The captures are NOT dropped for an at-risk memo: a cell that may be asked to
run again has to still have them. A kept memo drops them as before.

**Measured on the digest benchmark, exclusion untouched, main as the base.**

| counter | before | after |
|---|---:|---:|
| thunk_evals | 8,256 | **129** |
| allocs | 652,817 | **230,214** (-64.7%) |
| alloc_bytes | 81,846,129 | **54,149,841** (-33.8%) |
| arena_peak_bytes | 82,837,504 | **54,525,952** (-34.2%) |
| arena_blocks | 79 | 52 |
| push_mut_fast | 628,289 | 514,511 |
| push_mut_slow | 41,479 | 25,225 |
| sh_buf | 73,376,000 | 52,051,280 |

Wall clock, interleaved against a worktree of origin/main built in its own
directory, best of five, this container: **0.093 s -> 0.039 s**, 2.4x.

**`digest_buf_reuse` 24,768 -> 16,640 is the counter that fell the wrong way,
and it is the same fact.** A buffer is reused when a builder finds one to
reuse; 8,127 fewer evaluations build 8,128 fewer buffers, so there are fewer
reuses to count. Nothing lost a reuse it used to get.

**Every other vein is byte-identical**: decode, escape, pend, oneshot, basket,
encode, wide, scan counters, and the emitted-code golden. The machine-code
golden falls about a hundred bytes a binary — `k_force` gained a state and lost
the branch that declined a memo.

**What it cost, and why the first measurement of it was misleading.** CI on
the pinned host reported four benchmarks worse: deepbench +0.59%, oneshot
+0.46%, basket +0.44%, pendbench +0.34%, with every allocation counter on
those programs byte-identical. Nothing was doing more work, which is the shape
that says look at the code rather than the algorithm.

callgrind named it: `k_force` at 4,752,000 instructions on deepbench, a
function that does not appear in main's profile at all. It had outgrown what
the inliner would take, so every force paid a call and a prologue it used to
get for free. The delta was 4,375,985 and `k_force`'s own cost was 4,752,000 —
the same number twice.

So the cold half is out of line now (`k_force_slow`, `noinline`) and `k_force`
is the memo hit and nothing else, which is the shape it had before. deepbench
reads **726,483,240** against main's 726,483,254 on this container: fourteen
instructions BELOW main, measured on the same host with both binaries built in
their own directories.

`text` rises 740,226 -> 741,202 across the nine binaries, about 108 bytes
each.
`k_force_slow` is a real function now where the whole of `k_force` used to be
one, so there is a second prologue and a call site to pay for. That is the
price of the fourteen instructions above, and it is the trade the outlining
makes on purpose.

The lesson is not about thunks. A runtime function that grows past the
inliner's budget charges every caller, and no counter in the tree can see it —
allocations, arena blocks and evacuations were all identical while 0.6% of
deepbench went missing. kanso#1186 outlined `k_b_append_grow` for the same
reason; this is the second instance, and the first where the growth was
incidental rather than intended.

**The work vein after the split, measured by CI.** Nothing regressed and one
row fell properly:

| counter | before | after | |
|---|---:|---:|---:|
| `work_pendbench` | 946,378,074 | 937,566,473 | **-0.93%** |
| `work_encodebench` | 8,396,569,110 | 8,396,587,878 | +18,768 |
| `work_indexbench` | 5,243,094 | 5,243,104 | +10 |
| `work_widebench` | 63,997,213 | 63,997,231 | +18 |
| `work_jsonbench` | 2,838,415,853 | 2,838,415,815 | -38 |
| `work_basket` | 56,458,062 | 56,458,024 | -38 |
| `work_deepbench` | 726,486,934 | 726,486,920 | -14 |
| `work_escapebench` | 253,819,096 | 253,819,082 | -14 |
| `work_oneshot` | 43,094,978 | 43,094,978 | 0 |
| `work_digestbench` | — | 152,573,619 | joins |

`compile_instructions` 41,496,870 -> 41,496,028, a fall of 842 and free.

The deepbench row is fourteen below main, which is exactly what this container
measured before the push — the local method and CI agree to the instruction on
the one row both could see. pendbench's 0.93% is the memo doing its job on a
program that forces cells inside a beat and used to re-evaluate every one.

Welfare rises and is banked in the same change, per the rule that a gain
nobody ratchets is a gain the next change is free to spend.

**A fourth memo state, built on the misattribution and removed.****A fourth memo state, built on the misattribution and removed.** Before
callgrind was asked, the four rises were blamed on the at-risk check running
again and again on cells whose answer a rewind keeps taking, and a K_MEMO_SPENT
state was added so such a cell stops asking. It was measured after the split
and buys nothing: deepbench reads 726,483,240 with it and 726,483,240 without,
and every counter in every vein is byte-identical either way. No program in the
corpus exercises it. So it is gone, and the three states stand.

The comment it carried asserted the 4,375,985 as ITS motivation, which was the
misattribution written down as fact — the same number was `k_force`'s own
inlining cost, and a plausible story reached for it first. A state that costs
nothing to keep is still a state somebody has to read, and this one had a wrong
measurement attached.

**An intermediate state worth recording, because it nearly changed the
objective.** Before the split was found, the four rises were taken at face
value: welfare read 74.32 against a floor of 74.33, a fall with nothing on the
other side, and the argument being drafted was that the index is blind to a
streaming program's peak and should gain a digest term. That argument is still
true and still worth making one day — welfare cannot see this change's 64.7%
and 34.2% — but it was about to be made in service of a cost that did not
exist. A model is easiest to argue with when a measurement is embarrassing,
which is exactly when the measurement deserves another look first.

Also learned on the way: welfare already has a rule for a counter joining the
model — `baseline_of`/`entering` gives it the standing its dimension already
has, so a new term is score-neutral on the day it lands and only improvement
after that pays. Hand-writing a baseline into `bench/welfare_floor.json` was
the wrong instinct and the machinery was already right.

**Welfare is 74.3323 before and after**, and now for the right reason: nothing
regressed. No digest counter feeds the index, so a 64.7% fall in allocations
and a 34.2% fall in peak on a real program still scores zero — the omission
digestbench (kanso#1195) exists to make visible. Deliberately not wired here:
adding a term to the objective in the same change the term would reward is the
wrong order, and it is a weaker argument now that nothing needs it. **OPEN.**

`digestbench` joins `bench/instructions_golden.txt` in this change even so. A
benchmark counted on one axis and not the other is a trade the index cannot
see, and every change that buys the digest's memory spends something in work.
The vein is a tripwire; whether welfare gets a term from it is a separate
question and not this change's to answer.

**The reduced fixture that would not reduce.** Four attempts at a
postcard-sized program: a body binding used once, one used behind a two-arm
dispatch, one built outside the loop and passed in, and one built inside the
loop's own cluster and handed to a thirty-round inner loop — the shape sha256
has, where `w = schedule ...` in `blocked` is read by `compress` on each of
sixty-four rounds. Every one of them compiled strict: the demand analysis
proved the binding needed and no thunk was allocated at all. So what pins this
is bench/cost_golden_digest.txt, whose `thunk_evals` row is exact and which
went 8,256 -> 129 under this change and back again when it was reverted. That
is the observation the rule asks for, on a fixture larger than the rule wants.
**OPEN:** what keeps sha256's schedule lazy where four hand-written copies of
its shape are strict. Whoever answers that gets the postcard.

**What this does NOT close.** The carry-tier prefix removal, built and measured
on 2026-08-31 and on the entry above, is still not landed. This change removes
the reason it could not be: with the memo kept, lifting the prefix no longer
takes native's `thunk_evals` to 64 where the interpreter reads 1, which is the
differential the oracle refuses. Re-measure and land it next.

## 2026-09-01 — the digest joins the objective, on both sides

SEARCHED FIRST: design/compiler-log.md, design/log/compiler-log-archive.md,
design/*.md, design/pending-gavels.md, and `scripts/welfare/welfare.kso`'s own
counter lists, which are the record of what the model has ever weighed.

The digest benchmark landed on 2026-08-31 with a CI gate, a ratchet row and an
emitted-code row, and welfare could not see a single one of its counters. So
the gap the benchmark was built to close was still open: a change that takes an
8 KB digest from 79,691,776 arena bytes to 1,048,576 still scored zero, and so
did the 52x that bought it.

**What the model reads now.** `bench/cost_golden_digest.txt` joins the eight
goldens welfare already chains through, and three counters come off it:

| counter | dimension | today |
|---|---|---:|
| `digest_instructions` | run speed (satiation 2.0, weight 0.30) | 152,573,619 |
| `digest_peak_bytes` | run memory (2.0, 0.30) | 54,525,952 |
| `digest_arena_blocks` | run memory | 52 |

The instructions row comes from the work vein rather than from the digest
golden, the same place every other speed counter comes from.

**Both sides, and this is the whole point of the entry.** The peak is the row
digestbench exists for. A term that priced only the peak would rank any change
that reclaims per block above one that does not, however long it takes — which
is precisely the trade the 52x slowdown was, scored as free in the other
direction. Pricing the work beside the retention turns that into a number the
model settles instead of a preference somebody argues. Adding only the memory
half would have been rigging the answer to a question still open.

**The score did not move, and that is the entering rule working.** A counter
new to the model enters at its dimension's standing, so run speed's eight
counters and run memory's eight leave both terms where they were. The floor
went from 74.33468320932070 to 74.33468268729726 — a fall of 5.2e-7, which is
the rounding in `math/round` on the three entering baselines and nothing else.
Ratchet 132 records why.

**One pinned number moved.** `one_counter_running_away_cannot_carry_its_term`
answered 49.52 and answers 49.16: the fixture puts one run-speed counter a
thousand times better than its baseline and the rest at parity, so the term is
`(7/3 + 1024/1026) / 8 * 0.30` where it used to be over seven. The number is a
property of the weights and the counter count, not of the compiler.

**The spec, watched red.** `tests/the_digest_is_priced_on_both_sides.rs` stages
`bench/`, doctors one row of one golden tenfold, and requires welfare to go red
naming the digest row that moved — once for the peak, once for the work. A
third fixture runs the undoctored goldens and requires green, so neither of the
first two can pass on a welfare that fails on everything. With `digest_work`
and `digest_memory` taken back out of the counter lists both doctored fixtures
go red and the control stays green; that is the failure that was watched, and
the message on each is the assert's own sentence rather than a parse error.

**What this unblocks.** The carry tier measured on 2026-08-31 trades 2.24x on
the clock for 68x less peak at 131,072 bytes, and 1.6x for 52x at 8,192. That
is a trade across two dimensions, which is the one thing the per-counter
goldens cannot arbitrate and the one thing the index is for. It is now a
question with an answer rather than a judgement call.

## 2026-09-01 — the carry tier, arbitrated: DECLINED at −0.56, and the reference that decides it

SEARCHED FIRST: design/compiler-log.md (the 2026-08-31 entries "the carry
exclusion removed, measured properly, and REVERTED" and "the quadratic has a
name"), design/log/compiler-log-archive.md, design/pending-gavels.md — whose
charter bounced the sha256 question on 2026-08-29 with "performance questions
with no surface area are the implementer's", so this decision is recorded here
rather than sent anywhere.

The entry above put digestbench's peak and work into the objective so this
trade could be settled by the model. It has been.

**The arms.** `origin/main` against `wip/carry3`, which removes beat.rs's
`std/`/`lib/` prefix exclusion so a library loop carries like any other, and
adds `k_isv_flat` — a mark-free memo on the interior-survives walk, keyed on
the items pointer, that answers "this list holds no heap slot and no thunk"
without rescanning. Each arm built in its own directory, every benchmark binary
deleted before rebuilding, both swept on this container. The main arm
reproduced all six allocation goldens byte-identically, which is what says the
sweep measured the compiler rather than a stale binary.

**What moved.**

| counter | main | carry |
|---|---:|---:|
| digest arena_peak_bytes | 54,525,952 | **1,048,576** |
| digest arena_blocks | 52 | **1** |
| digest allocs | 230,214 | 653,077 |
| digest thunk_evals | 129 | **8,256** |
| digest beat_iters | 56 | 16,826 |
| digest evac_allocs | 27 | 66,851 |
| digestbench instructions | 152,573,220 | **820,087,049** (+437.5%) |
| compile_instructions | 41,921,600 | 41,800,396 (−0.29%) |

Every other benchmark is within a tenth of a per cent, and `scan`'s
`beat_iters` moves 15 to 16. Nothing else in any vein.

**The at-risk memo and the carry tier are in direct conflict, and this is the
mechanism.** `thunk_evals` goes back to 8,256 — equal to `thunk_forces`, so
every force re-evaluates, which is the state before yesterday's memo. The memo
is correct and it is being correctly invalidated: the carry evacuation copies
what is reachable from the staged value and rewinds the rest, and a memoized
result the loop does not carry is exactly what the rewind takes. So the 52x
memory win is bought by discarding the memos, and the recomputation is where
the 5.4x work goes. Both changes are right on their own and each one's win is
the other's cost.

**The verdict: 74.31 → 73.75, a fall of 0.56.** Same host, same sweep, the
repo's own baselines. Welfare names `digest_instructions` as the term that
paid, at 0.373 points. **The carry tier is DECLINED.** wip/carry3 is not
merged and the prefix exclusion stays.

**A second measurement, which is the part worth arguing about.** Rerun both
arms with the three digest baselines set to the main arm's own values — every
digest ratio exactly one — and the answer reverses: main 70.14, carry
**72.99**, a rise of 2.85. The same change is worse by 0.56 or better by 2.85
depending on nothing but where the digest counters' reference sits.

The reference is not a measurement. `entering` chose it this morning so a new
counter would not move its dimension on landing day, which put
`digest_peak_bytes` at a ratio of 11.2 — the model now asserts that the
digest's memory has already improved elevenfold, and it has no history at all.
On a satiating curve that assertion is not free: a counter placed at 11.2 is
84.8% satisfied and a further 52x buys 0.15 of satisfaction, where the same 52x
from parity buys 0.63. The rule spends most of a new counter's headroom on the
day it enters, and the same placement makes the work counter's regression
proportionally dearer. Both effects push the verdict the same way.

**This is not grounds to move the reference now.** The rule was written for a
real problem — entering at parity costs welfare on landing day, and here it
costs 4.17 points — and a model is easiest to argue with when its answer is
inconvenient, which is when the measurement deserves another look first. That
was written in this log four days ago about this same benchmark and it applies
to me again. So the decline stands on the model as it is.

**The question the two numbers leave open, stated once and left open.** A
newcomer needs two things that `entering` supplies with one number: a reference
that reflects where the counter actually stands, and no score move on the day
it lands. They can be separated — enter at parity and absorb the landing-day
step in the floor, in the same PR, with the reason recorded. That is what
`--set` exists for, except that `--set` refuses a fall by ruling, so the step
would have to be a hand edit of bench/welfare_floor.json where a reviewer sees
it, which is the designed override and the right shape for "the model gained a
term". The cost is a visible four-point drop in the published number and an
honest one in place of an invented history. Nobody should settle this while a
particular change's verdict hangs on it; digestbench is the only counter that
has ever entered by this rule, so there is one instance and it is this one.

## 2026-09-01 — why sha256's schedule is lazy: an arm that ignores a parameter

SEARCHED FIRST: design/compiler-log.md — this closes the question the
2026-08-31 memo entry left open ("what keeps sha256's schedule lazy where four
hand-written copies of its shape are strict") —
design/log/compiler-log-archive.md, design/*.md, and
tests/golden/mem/a_digest_holds_every_block_it_walked.kso, whose comment named
the four copies.

`digested` binds `w = schedule ...` and hands it to `compress s w 1 false`,
which reads `w[at]!` sixty-four times. The binding is a thunk, and the reason
is one arm:

```
fn compress s _ _ true
  s

fn compress s w at false
  ...
```

The exhausted arm ignores `w`, so `w` is not demanded on every path into
`compress`, so the argument is thunked. Demand is decided per parameter across
all arms rather than per call site, and the call site's guard is the literal
`false`.

**A controlled pair, built to check it rather than to argue it.** Two programs
identical but for the exit arm: one written `fn walk s _ _ true` reads
`thunk_allocs=1`, `thunk_forces=8`, `thunk_evals=1`; the same program with the
exit arm reading `w` reads `thunk_allocs=0`. `allocs=5` in both. The four
hand-written copies were strict because their consumers read the parameter in
every arm.

**Costed, and no change is warranted.** callgrind on digestbench puts
`d_thunk_eval` at 1,151,583 of 152,573,220 — 0.75%, and that is the 129 real
evaluations of the schedule, which a strict compile would do too. `k_force`
does not appear in the profile at all: #1197 outlined its cold half and the
hot half is inlined, so 8,256 forces cost less than the annotator's threshold.
The obvious refinement — decide demand per call site when the guard argument is
a literal, which would make `w` strict here — buys a fraction of a per cent of
one benchmark and adds a case to a pass that already runs on every declaration.

The behavior is pinned where it already was:
`tests/golden/mem/a_digest_holds_every_block_it_walked.mem` reads
`thunk_allocs=1` and `thunk_forces=64` for a one-block message, so a demand
analysis that started answering differently moves that golden.

## 2026-09-01 — sixteen counters gain a direction, five keep none on purpose

SEARCHED FIRST: design/compiler-log.md (the 2026-08-26 entry adding the two
measured compile veins to these tables, and the 2026-09-01 entry adding
digestbench), design/log/compiler-log-archive.md, design/*.md, and the tables
themselves in scripts/trend_gate/trend_gate.kso, which are the record of what
has ever been classified.

Twenty-two counters across the goldens had no direction. The gate's third state
prints them under UNCLASSIFIED and counts them toward neither side of the
pure-regression rule, which is right for a counter whose direction is genuinely
unknown and wrong for one nobody had got round to. Sixteen were the second
kind.

**To `lower`, twelve.** `evac_allocs` and `evac_bytes`, what a carry evacuation
spends. `str_scans`, `str_scan_bytes`, `find2_calls`, `utf8_bytes`, bytes
walked. `perm_live_bytes` and `perm_peak_bytes`, malloc-backed storage the
process still holds — it only leaves through `free()`, so a peak that scales
with iteration count is a leak by definition, which is what the counter's own
comment in runtime.c says. `append_grow`, `push_mut_slow` and `put_mut_grow`,
the slow halves of three pairs whose fast halves were already classified.
`thunk_evals`, a lazy binding actually running.

**To `higher`, four.** `push_mut_fast` and `put_mut_fast`, the fast halves.
`bytes_freed`, what `bytes_malloc` is measured against. `carry_dedup`, a node
the carry found already copied and reused rather than copying twice.

**Five stay out, and the reason is written beside them.** `cohort_frees` and
`cohort_kept` are the two outcomes of the cohort dance and both scale with how
many cohorts ran, so what means anything is the ratio and this gate cannot say
ratios. `view_allocs` and `view_frees` are the same shape, and runtime.c says
so at the counter: the difference is memory the process is still holding, and
either side alone is not. `thunk_forces` counts asking a thunk for its value
rather than computing one — #1197 took `thunk_evals` from 8,256 to 129 on the
digest with `thunk_forces` byte-identical, which is what a working memo looks
like, and a change that made the same program strict would lower it while doing
identical work.

**The risk the `higher` table carries, stated rather than left to be found.** A
change that does strictly LESS of the work lowers a presence counter, and the
table reads that as a worsening. `append_fast` and `utf8_zerocopy` have sat
there since the table was written without it biting, because a change that
removes work almost always moves its slow twin the same way and the verdict
comes out mixed. A pure regression that is really a pure simplification is the
shape to watch for.

**A ratchet row was about to start passing for the wrong reason, and this is
the part worth remembering.** `a_worsening_paid_for_by_an_unclassified_counter`
raised `scanbench calls` and `evac_allocs` together: one classified worsening
beside one unclassified move, so a gate that intersects correctly refuses the
first and lets the second count toward neither side. Classifying `evac_allocs`
makes BOTH halves classified worsenings — which any pure-regression rule
refuses, so the row goes on turning the gate red while testing nothing it was
written to test. Verified by running it: before the repoint the listing had no
UNCLASSIFIED section at all.

The mutation now raises `thunk_forces`, which stays unclassified on purpose,
and `tests/a_mutation_keeps_its_unclassified_counter.rs` reads the counter out
of the mutation's own `sed` line and requires the tables not to name it. It was
watched red by putting `thunk_forces` into `lower_k`. That is the general
shape: a classification sweep can silently disarm a mutation that depends on
something being unclassified, and nothing else in the tree was watching that
edge.

## 2026-09-01 — two ratchet rows went blind on the same day, and nobody read the nightly

SEARCHED FIRST: design/compiler-log.md (the 2026-08-24 entry that fixed the
same `compile_allocs_unwatched` row for the same class of reason, and the
2026-08-31 entries), design/log/compiler-log-archive.md, design/*.md, and the
mutation corpus itself — 59 scripts in scripts/ratchet/mutations.

`scripts/ratchet -- prove` applies each gate's claimed defect and refuses a
gate that stays green. It runs nightly. It has been RED since 2026-08-30, with
two rows reported BLIND, and the two nights of failure were not read. What
follows is what they were.

**Row one: a mutation had been inserting itself into a comment.**
`a_string_the_builder_corrupted_in_place` matched `k_stat_append_fast++;`,
skipped one line and appended after it. kanso#1171 (2026-08-30) wrote a
three-line comment above the copy, so the skipped line became the comment's
opening and the injected statement landed inside `/* ... */`. It was not code.
The compiler built clean, kq's suite ran green, and the script's own
`grep -qF` found its text in the file and reported success.

Reproduced end to end: apply the mutation to a worktree at origin/main, build,
clone kq to /tmp/kq, run `KANSO=$K KQ_STORED=report sh spec.sh` — exit 0, "kq
specs: all green", and kq's own counters read `append_fast=242226`, so the
mutated path ran a quarter of a million times and corrupted nothing.

It substitutes the copy now:
`else memcpy(...)` becomes `else { memcpy(...); ...[a->len] = 0; }`. A
substitution cannot land in a comment, because the anchor is the code being
replaced. Watched red: the kq gate dies with `invalid utf-8, born in
text/utf8`, which is the sentence the mutation's own comment had been
promising and not delivering.

**Row two took two merges to kill and a rewrite to fix.**
`compile_allocs_unwatched` makes a front-end pass own the program's names
instead of borrowing them. kanso#1188 (2026-08-30) made an identifier a `Name`
rather than a `String`, so `out.insert(name.clone())` stopped type-checking and
the mutation became a compile error — the exact failure its own comment had
warned about once already, in different words.

**Repairing the type is not enough, and this is the part worth keeping.** With
`name.as_str().to_string()` the mutation builds and `compile_allocs` reads
25,394 either way, on binaries that differ. kanso#1157 (2026-08-30) gave the
walk an early return for any name that cannot be a getter's, and by its own
comment eleven thousand of lib/json's twelve thousand occurrences take it. An
owned insert behind that guard is reached almost never. A mutation can be
applied, compile, and still be inert.

So the row restores the shape the vein was built to catch rather than one line
of it: the guard goes and the names are owned. Measured on this container,
`kanso check lib/json` in a fixed box: **compile_allocs 25,394 -> 31,138**,
compile_alloc_bytes 3,950,766 -> 4,062,065.

**Both scripts now fail loudly rather than silently.** Each greps its anchor
before substituting and exits 1 with a sentence if it is gone; the second also
greps that the guard is GONE afterwards, so a mutation that applied and left
the code inert is an error rather than a green row. That is the durable half:
the two rots were both silent, and silence is what let a nightly failure sit
unread for two nights.

**Why the per-PR half did not catch either, which is sharper than it looks.**
That half does more than count rows: since the #1015 incident it also applies
every mutation to a worktree of HEAD and fails if one no longer matches the
source it patches — added, in its own comment's words, because the nightly
"said so the next morning — correctly, precisely, and to nobody". Both rots
slipped past it because both mutations still APPLIED. #1171's sed matched, and
put its statement in a comment. #1188's sed matched, and the build then failed.
Applying is not proving, and only proving costs a build.

So the gap is narrower and more specific than "the cheap half is cheap": what
is missing is a middle. A mutation names the source files its seds touch; a
pull request names the files it changes; the intersection is usually empty and
occasionally one to three rows. #1171 touched src/runtime.c and #1157 and #1188
touched src/lib.rs, so both would have been caught at merge time for one to
three extra builds on the pull requests that could break them, and none on the
rest. Both rows here were killed by merges dated 2026-08-30, which is to say a
single busy day put two of the repo's own gates to sleep.

## 2026-09-01 — the survivorship walk gained a counter, and minting it found a hole in the trend gate

**DONE.** `k_slots_survive` reads a node's whole immediate interior on every
ask, and nothing in the tree bounded how often it is asked. `evac_allocs` and
`evac_bytes` count what a carry evacuation COPIES; nothing counted what
deciding costs. On the branch that lifted the carry-tier prefix and was
declined for other reasons (kanso#1198), the 8,192-byte digest asked 33,024
times over 8,256 slots — **272,646,144 examinations finding zero heap slots**,
against 33,827 allocations that actually evacuated. Every counter beside that
one read the workload as nearly free.

`survive_slots` counts slots examined rather than calls made, because the calls
were linear and the slots were not; and slots rather than bytes, because it is
an algorithm-level step no platform can widen. Readings on main, where the
carry tier is not admitted: encodebench 129,873, pendbench 118,087, basket
16,002, widebench 16,000, and zero on the other five cost goldens — including
digestbench, which is the point: the pathology lives behind a prefix decision,
and the counter is what will show it the day that decision moves.

**What it costs, and a correction to what I first wrote.** The guard is
`k_stats_on > 0`, read inside the loop, and I measured it on basket and
pendbench, found two retired instructions per slot on both, and wrote that
down as the rule. CI measured all ten and two of them do not follow it:

| benchmark | slots | `work_*` delta | per slot |
|---|---:|---:|---:|
| basket | 16,002 | +32,004 | 2.00 |
| pendbench | 118,087 | +236,180 | 2.00 |
| encodebench | 129,873 | +5,104 | 0.04 |
| widebench | 16,000 | +3 | 0.0002 |
| deepbench | — | +1,456 | — |

The guard is loop-invariant, so a compiler may hoist it out; these are ten
separately compiled programs and it evidently did in two of them. The
supporting evidence is that `k_slots_survive` costs 31 instructions a slot in
basket and 12 in widebench with counters off — different code for the same
source. Reading the assembly to confirm the hoist is not done, so the
mechanism is an inference and the deltas are the measurement.

Accumulating once at the exit instead was written and measured on the two
benchmarks that pay: it is **not** cheaper — the same two per slot on both —
so the count stays where it is read.

As a fraction: `work_basket` +0.057%, `work_pendbench` +0.025%,
`work_deepbench` +0.0002%, `work_encodebench` +0.00006%, `work_widebench`
+0.000005%. `text` rises 741,202 -> 744,562, a flat +368 bytes of machine code
in every benchmark, which is the guard plus the extra argument the dump
marshals. `compile_instructions` FELL 41,496,028 -> 41,494,642, unattributed:
the front end does not run this code, and the only reachable connection is the
runtime.c text the compiler carries. Welfare 74.33 against a floor of 74.33.

**The hole it found.** The trend gate read the first reading as eight
worsenings and refused the branch as a pure regression. `or_zero` cannot tell a
counter that read nought from a counter the baseline does not carry at all, and
every golden in a vein gains the new row on the same commit — so a change whose
whole content is that the runtime now measures one more thing printed as a
tree-wide regression with nothing on the other side.

The gate already says this sentence one level up, for a benchmark that JOINS a
golden (`new_samples`, added when a tenth benchmark read as four simultaneous
worsenings). This is the transposed case: a row joining every sample rather
than a sample joining every row. `missing?` distinguishes absence from nought,
a minted counter is reported under MINTED and counts toward neither side of the
pure-regression rule, and the exemption lasts exactly one commit — the dumps
carry every counter on every run, zeros included, so a name absent from the
baseline was absent from the runtime. The reverse, present then gone, is a
deleted kernel and the hard golden diff already refuses that byte for byte.

`tests/a_minted_counter_is_not_a_regression.rs` stands up a scratch repository
whose committed goldens lack the row and whose working tree has it, runs the
gate against that commit, and requires a pass. Its second fixture raises a
counter the baseline DOES carry and requires the refusal, which is what stops
the rule widening into an escape hatch. Watched red both ways: with the mint
rule removed the first prints the eight worsenings; with the exemption widened
to every counter, both go red.

`survive_slots` is classified `lower` in the direction tables, so the next
change to move it has a direction to be judged against.

**OPEN.** The digest reads zero here because the carry tier is declined. Nobody
should read that as the walk being cheap — it is the counter being pointed at a
workload that currently does not enter it.

## 2026-09-01 — a branch proves the ratchet rows its own diff could have made blind

**DONE.** kanso#1199 repaired two rows that went blind on 2026-08-30 and asked
why the per-PR half had not caught either. The answer is sharper than "the
cheap half is cheap": that half applies every mutation to a worktree of HEAD
and fails if one no longer matches the source it patches, and **both mutations
still applied**. #1171's sed matched and put its statement in a comment;
#1188's sed matched and the build then failed. Applying is not proving, and
only proving costs a build.

So there is a middle, and it is cheap because the intersection is usually
empty. A mutation names the source files its seds patch; a branch names the
files it changes:

    kanso run scripts/ratchet -- touched origin/main

keeps the rows whose mutation script names a file the branch changed and proves
that handful. Naming is the test because a mutation patches what it writes
down. `touched origin/main list` names them and stops, for somebody deciding
whether to spend the runner.

**What it costs.** Measured on this container against the survivorship-counter
branch, which touches src/runtime.c — the worst realistic case, because that
file is patched by eight mutations: **thirteen rows, six minutes eight
seconds, every one red.** One of the thirteen is the kq row #1199 had just
repaired, so the mechanism selects exactly the row the incident was about. A
branch touching src/lib.rs selects the compile-allocs row that #1157 and #1188
made blind between them. A prose-only branch selects nothing and the step is
a second.

**What it does not cover, stated rather than left to be found.** A row can go
blind from a distance. The guard #1157 added is in the file its mutation
patches, which is why this catches it, but a change to what a GATE reads could
hollow a row while touching no line any sed matches. `cover` still runs on
every change and the nightly still proves the whole table; this is a third
thing between them, not a replacement for either.

`tests/a_branch_proves_the_rows_it_could_have_broken.rs` asserts the selection
rather than the proving, because the proving is a build per row and the
selection is the half that can go quietly wrong — a mutation whose paths stop
matching selects nothing, exits zero, and reads as a branch that broke no rows.
Three fixtures: a src/runtime.c branch selects the row #1171 killed and not the
python-free row; a src/lib.rs branch selects the row #1157 and #1188 killed; a
README-only branch selects nothing. Watched red both ways — with the naming
test never matching, the two positive fixtures fail; with it always matching,
the negative ones do.

**The first CI run of the step was red, for a reason the spec could not see.**
`git diff origin/main...HEAD` needs a merge base and `actions/checkout@v4`
takes a shallow clone, so the diff failed outright and the ratchet job went red
with nothing to do with a mutation. Fixed with `fetch-depth: 0` on that job,
and the refusal now carries git's own stderr — "would not run" sent a reader to
the ratchet where the answer was in the workflow. The lesson generalises past
this row: a spec that enters where a user enters still cannot see the shape of
the box CI enters from.

## 2026-09-01 — nine of the objective's counters stand on a rule, and nothing said which

**DONE.** `entering` gives a counter new to the model a baseline of
`now * standing`, where standing is the ratio whose satisfaction equals its
dimension's current mean. Landing day is therefore neutral, which is the whole
point: entering at parity instead — the rule before kanso#910 — makes a
measurement-only change spend the floor, and an objective that charges for
measuring is paying people not to measure.

**The rule is not neutral about anything after landing day, and that was not
written down.** Saturation is concave, so a counter granted a high standing has
little headroom left and one entering at parity has a great deal; how much a
later change to that counter is worth follows from where it entered. Measured
on the carry-tier arms of 2026-09-01: with the digest baselines at their
dimension's standing the trade scored **74.31 -> 73.75 and was declined**; the
same two arms with those baselines at parity score **70.14 -> 72.99, an
acceptance**. The entering rule decided that verdict.

**Nine of twenty-one.** The floor file's own history says which counters were
granted, because the commit that first wrote each baseline key is either an
ancestor of kanso#910 — which added `entering` on 2026-08-14 — or a descendant
of it:

| counter | first written | granted? |
|---|---|---|
| `wide_instructions` | 2026-08-14, kanso#887 | no, predates the rule |
| `deep_instructions` | 2026-08-15, kanso#912 | yes |
| `scan_arena_blocks`, `scan_peak_bytes` | 2026-08-17, kanso#945 | yes |
| `pending_instructions` | 2026-08-21, kanso#981 | yes |
| `compile_allocs`, `compile_instructions` | 2026-08-25, kanso#1041 | yes |
| `digest_arena_blocks`, `digest_instructions`, `digest_peak_bytes` | 2026-08-31, kanso#1198 | yes |

Everything from 2026-07-26 to 2026-08-14 was measured or hand-seeded. So nine
of the model's twenty-one counters have a reference no measurement produced,
and until this the floor file recorded them exactly like the twelve that do.

**The rule stays; the arbitrariness stops being invisible.** `granted` names
them in `bench/welfare_floor.json`, `--set` carries the list forward and adds
whatever this run had to grant, and the report prints a line naming them. The
score is unchanged — 74.33 against a floor of 74.33 — because nothing about the
computation moved. A reader comparing two counters' ratios is comparing unlike
things unless they know which, and until now nobody could.

`tests/a_granted_baseline_says_it_is_one.rs` pins both halves: the report names
all nine and does NOT name `decode_instructions`, which predates the rule by a
fortnight; and a run that has to grant a counter writes it into the floor
without losing the ones an earlier run granted. Watched red both ways — remove
the report line and the first fixture fails, drop the persistence and the
second does.

**OPEN, stated rather than buried.** Whether a granted baseline should be
replaced by real history once the counter has some. The argument for is that a
granted reference is a guess and a measured one is not; the argument against is
that re-basing a counter mid-life moves the objective without saying so, which
is the thing the ratchet exists to stop. Nothing here does it.

## 2026-09-01 — the runner pool is four CPUs, and the first fix for that was wrong

kq#85 established what moved four kq instruction rows between two runs: not a
toolchain. Both job logs printed rustc 1.98.0 (88d9e12ae), LLVM 22.1.8, image
ubuntu-24.04 20260823.283.1, glibc 2.39-0ubuntu8.8, valgrind 3.22.0-0ubuntu3,
gdb 15.1 — every version identical to the commit hash. The one field that
differed was `Azure Region`. Different silicon under the same image.

**The mechanism, measured twice.** glibc resolves memcpy, memcmp, strlen and
their neighbours by ifunc at load time, reading CPU features, so one libc runs
different code on different CPUs. The first measurement swapped the resolver's
choice on one host by tunable, on kq's `print_small` row:

| GLIBC_TUNABLES | Ir | memcpy chosen |
|---|---:|---|
| default | 76,742,430 | `__memcpy_avx_unaligned_erms` |
| `-AVX2_Usable` | 76,746,433 | `__memcpy_avx_unaligned_erms` |
| `-AVX_Fast_Unaligned_Load` | 76,488,416 | `__memcpy_sse2_unaligned_erms` |
| `-ERMS` | 76,262,756 | `__memcpy_avx_unaligned` |

0.63% from the dispatch alone, against a runner shift of 0.06% to 0.10%.

The second used a switch that actually differs between this container and a
runner, after CI printed the runner's block:

| GLIBC_TUNABLES | Ir | vs default |
|---|---:|---:|
| `rep_movsb_threshold=0x2000` (Intel, default) | 76,742,736 | — |
| `rep_movsb_threshold=0x840` (a runner's) | 77,523,061 | **+1.02%** |
| `non_temporal_threshold=0x1800000` (a runner's) | 76,744,279 | +0.00% |
| both together | 76,744,207 | +0.00% |

Byte-identical over two sittings each. One switch is worth ten times what the
vein saw. The pair nearly cancels because glibc derives
`rep_movsb_stop_threshold` from `non_temporal_threshold`, so no single line
predicts a row.

**The first fix was a pin, and CI killed it in two runs.** Record one host's
feature block; refuse anywhere it does not match, the way `measured_on.sh`
refuses a moved glibc. The first run refused and printed an AMD EPYC Zen 3
Milan (family 0x19, model 0x1). The second refused and printed an Intel Ice
Lake-SP (0x6/0x6a). The third, after the restructure below, named an AMD Genoa
(0x19/0x11). This container is a Cascade Lake (0x6/0x55). **Four CPUs in four
runs.** A check that refuses every run but one is red for a reason no pull
request causes, which is a gate nobody can act on, and it would have been
merged on the strength of a local verification that could not see the pool.

**And on that fourth CPU every kq row matched exactly.** That qualifies the
story rather than undoing it: kq#85's two runs really did differ by 0.06% to
0.10% with every version identical, and a Genoa really does count kq's four
rows byte for byte the same as whatever counted the golden. Both are what the
ifunc account predicts — most CPUs land on the same memcpy and the counts are
identical, and now and then one lands elsewhere and they are not. It also
means the pool's heterogeneity is survivable rather than fatal, which is the
difference between this vein reporting sometimes and being unusable.

**What the fix is instead.** `scripts/gates/dispatch.sh` never refuses. It
answers, and the two instruction gates ask only about a row that already moved:

- `name` prints this host's CPU family and model on every run, so the next
  divergence is one line of a job log rather than an afternoon of version
  archaeology — which is what kq#85 cost.
- `differs` answers 0 for the recorded silicon, 1 for other silicon with the
  differing lines named, and 2 when there is nothing to compare against.

A row landing on its recorded value is right whatever counted it, so the
question is worth asking only about a row that moved. On answer 1 the gate says
the run does not gate this vein and exits green — neither a pass nor a
regression, because the run cannot establish either. Calling that a regression
is exactly the mistake kq#85 spent a pull request undoing; this makes the
correction structural.

**`bench/dispatch.txt` is deliberately absent, in both repos.** It has to hold
a CPU on which the rows are known to verify, and no run has both named its
silicon and matched a golden — the naming only starts here. A guessed block
would be worse than none, because it would let a real regression on the true
recorded CPU read as other silicon. Answer 2 gates exactly as these veins
always have, so the absence costs nothing and the block goes in from the first
run that names its CPU and matches every row.

One more thing had to be built before any of it could work: a block may be
taken only from a run that BOTH names its CPU and matches every row, and a run
that matches never reaches `differs`, which was the only place a block
printed. A bootstrap with no first step. So while no block is recorded, CI
prints the whole thing beside the rows, and stops the moment the file exists.

`tests/the_silicon_a_row_was_counted_on.rs` pins all three answers plus three
properties: the pasteable block prints under `GITHUB_ACTIONS` and nowhere else
— `measured_on.sh`'s own header records a container printing a diff, somebody
pasting, and the container's numbers landing in a golden over the runner's —
an unknown verb answers 2 rather than a yes or a no nobody gave; and the
bootstrap print happens in CI while nothing is recorded and stops once there
is. Watched red five ways: a `differs` that always matches fails two fixtures,
one that treats an absent block as a match fails the third, printing the block
everywhere fails the fourth, and the bootstrap fails in both directions —
never printing, and never stopping. The spec also caught a live bug on its first run:
`grep -v '^#'` answers 1 on an all-comments file and `set -e` took the script
out before it could say which answer it meant.

**A second host was hiding in the spec.** The macos/arm job went red on the
first run that reached it: five of the seven fixtures called the x86 loader by
path and expected it to be there. Skipping them on aarch64 would have left that
host uncovered by the very check that says which host a number belongs to, so
each fixture states both arms instead. Where there is no loader the gate must
say the cpu is unnamed and answer 2 — never 0, which would let a moved row pass
as verified on silicon nobody read, and never 1, which would blame silicon
nobody read either. `dispatch.sh` takes its loader path from an environment
variable defaulting to the real one, which is what makes that arm reachable on
x86: a fixture that can only run on aarch64 is a fixture nobody watches fail.

**OPEN, and it is the real one.** These rows claim to be exact and the pool is
not. Three CPUs seen in one day means an instruction golden gates properly only
on the fraction of runs that land on its recorded silicon, and nothing here
measures that fraction. The honest options are a golden per CPU, or accepting
that the vein reports more often than it gates. Nothing is decided; what is
built refuses to lie about which case a given run is in.

## 2026-09-01 (later) — the silicon note was an excuse, and would have blinded the ratchet

The entry above shipped `scripts/gates/dispatch.sh` and wired it into both
instruction gates. On answer 1 — a row moved, and this is not the silicon the
rows were counted on — the gate printed a warning and **exited green**. That is
wrong, and the argument is arithmetic rather than taste.

**Most real regressions would have been waved through.** Four CPUs were seen in
four runs that day. A block records one of them, so roughly three runs in four
land somewhere else. On those runs any moved row — a genuine regression
included — got the warning and a green tick.

**And the ratchet's rows would have gone blind on the same runs.** Two of its
mutations exist to redden exactly these gates: `a_counter_worsens_for_nothing`
and the decoder's instruction row. Applied on a run that landed off the
recorded cpu, the gate would have exited 0 and the mutation would have proved
nothing. A row that proves nothing is a BLIND row, which is the single failure
the ratchet was built to catch — kanso#1199 repaired two of them a few hours
earlier. Shipping a mechanism that manufactures them is worse than shipping no
mechanism.

**What the answer is for.** The dispatch diff is a named cause printed beside a
failure, never instead of one. Both gates now fail on a moved row whatever
counted it, and when the silicon differs they say so and name the lines, so a
reader knows in one screen whether to re-run for the recorded cpu or start
reading the diff. Deciding that silicon accounts for a move is a person's job
in a pull request, with a re-run on the recorded cpu as the evidence — it was
never a thing a shell script should conclude on its own.

This also settles, in the safe direction, the OPEN the entry above recorded.
The vein does not report-instead-of-gate on a fraction of runs; it gates on all
of them and explains itself on the fraction where the explanation is available.
What is still unmeasured is how often the pool's CPUs actually move these rows
— kq#85 saw 0.06% to 0.10% on one pair, and an AMD Genoa matched kq's golden
exactly on another. If that turns out to be frequent, the answer is a golden
per cpu, not a gate that shrugs.

kq carries the same wiring and owes the same correction.

## 2026-09-01 (later still) — forty-seven instructions to store one byte

`k_b_append_mut` is 2,000,259,200 instructions of encodebench, 23.82% of the
run and the largest single symbol anywhere in the suite. It is called
42,318,000 times, which is 47.3 instructions a call. The callgrind file has
said so for weeks; reading it needed an id-to-name map, because callgrind names
a function only on its first `cfn=` line and a grep for the name undercounts
by two orders of magnitude — 461,200 against the true 42,318,000.

Disassembled, the forty-seven are honest: the fast path really does execute
that many instructions to put a comma in a buffer that already has room. Two
causes, and both are the common path paying for a rare one.

**The byte went to memory and came back.** The general path reaches its store
through `src`, a `const unsigned char*` that is a phi of three predecessors —
a string's data, a byte string's data, or the address of a one-byte local. So
a byte a caller passed in a register was stored to a stack slot and reloaded
four instructions later, and the frame that slot needs was built by every
append of every shape. Straight-lining the byte case ahead of the other two
keeps it in the register. In the same block, `a->len` was read three times
where one would do: a store through `unsigned char*` aliases every field of
the header it is stored into, so the compiler had to reload the length after
writing the byte. Read once into a local, it does not.

**And the frame served an error path.** `k_die` calls `exit`, and nothing said
so. Unmarked, clang inlined its fprintf and its exit into every runtime entry
that validates a tag — so the entry pushed the callee-saved registers that
error path needs, and built a frame, before it could test anything. Marked
`noreturn` and `noinline`, along with its eight siblings, it costs a call at
the point of death and nothing anywhere else.

| row | before | after | |
|---|---:|---:|---:|
| jsonbench | 2,838,415,815 | 2,781,834,881 | -1.99% |
| encodebench | 8,396,592,982 | 7,870,153,008 | **-6.27%** |
| oneshot | 43,094,978 | 41,401,995 | -3.93% |
| basket | 56,490,028 | 56,459,146 | -0.05% |
| widebench | 63,997,234 | 64,077,244 | **+0.13%** |
| deepbench | 726,488,376 | 717,299,279 | -1.27% |
| escapebench | 253,819,082 | 249,019,060 | -1.89% |
| pendbench | 937,802,653 | 930,587,850 | -0.77% |
| indexbench | 5,243,104 | 5,243,096 | -0.0002% |
| digestbench | 152,573,619 | 143,472,199 | **-5.96%** |

(CI's rows. The container this was developed in runs glibc 2.39-0ubuntu8.7
against the runner's 2.39-0ubuntu8.8, so `measured_on` refuses a local
regeneration and the numbers above come out of the instructions job.)

The two changes were measured apart. `noreturn` alone is digestbench -5.97%,
escapebench -1.89%, deepbench -1.27%, encodebench -1.27%, pendbench -0.77%,
widebench -0.48%, jsonbench -0.40% — it reaches everything, because every
benchmark runs runtime entries that validate a tag. The append split is the
rest of encodebench's fall and most of jsonbench's.

**work_widebench rises, and the objective was asked rather than told.** The
split
puts the string and byte-string cases behind a call into `k_b_append_wide`, and
widebench appends strings, so every one of its appends pays that call: 384,000
instructions. The alternative was built and measured — leave the byte case
inline and outline nothing — and it costs widebench nothing, but gives back
142M instructions on encodebench and 20M on jsonbench. It reads 74.47 where the
split reads 74.50. The model prefers the split by 0.03, so the split shipped
and the rise stands with its cause written down. This is the trade the weights
exist to license, and the counterfactual is here so that a later reader can
argue with the weights rather than with the measurement.

Every allocation counter is byte-identical: all nine counter gates pass
unchanged. That is the point of a separate instruction vein — a decode that
allocates identically and executes six per cent less work moves nothing else in
the tree. Every binary also falls about 2,000 bytes, 2.4%, which is the
inlined error text and call sequence leaving dozens of sites.

**compile_instructions rises 1,630 to 41,496,272, and it is layout for the
third time.** `kanso check lib/json` runs none of the runtime, so nothing the
front end does changed. What changed is `include_str!("runtime.c")`, a static in the binary
that grew 2,962 bytes and shifted what follows it. Measured rather than
assumed, because the same claim was made twice before on the strength of
elimination: build this branch's front end against main's runtime.c and
against this branch's, on one host, and read 41,922,834 and 41,925,168 — the
same rise with no Rust changed at all. compile_allocs, compile_peak_bytes,
rounds and visits hold.

Welfare 74.33 to 74.50, floor set. kq links this runtime and owes a pin bump;
its instructions vein will move and none of its allocation counters will.

## 2026-09-01 (later still, second) — naming a counter licensed it, and the listing only printed

The ratchet went red on the branch above, with two rows BLIND: `a counter
worsens for nothing` and `a runtime counter worsens for nothing`. Both are
trend-gate mutations, both had been proving something, and both stopped
because of the shape of that branch rather than anything wrong with them.

**A counter's name was a blanket permit.** The gate priced a worsening when
the branch's compiler-log delta mentioned the counter anywhere. The branch
above raised `compile_instructions` by 1,630 for a layout reason and wrote a
paragraph explaining it — and that paragraph then licensed the mutation to set
the same counter to `compile_instructions=999999999`. The gate printed `every
changed counter is priced` and exited 0. (Written the way the mutation writes
it, ungrouped: the rule below reads comma-grouped figures, and an entry that
quotes a sentinel in the gate's own spelling prices it.) Any branch that legitimately moves a counter and says so
disarms the gate for that counter, which is every branch that touches a
golden.

**And the listing was advisory.** With the counter unnamed the gate printed
UNPRICED, listed the row, and exited 0 anyway. So the runtime mutation — set
`jsonbench 9999999999` — was listed and still green. The pure-regression
rule beside it could not catch that either: it refuses a branch where
something worsens and nothing improves, and the branch above improves nine
rows and ratchets the floor, so the licence was already bought.

**What it takes now.** A worsening is priced when the log delta names the
counter AND quotes the value it landed on. `compile_instructions` names
41,496,272 above; the mutation's figure appears nowhere in the grouped form
the gate reads, so it is unpriced. And unpriced exits 1 rather than printing. No band, no tolerance:
the log already states the figure a move landed on, and this is the gate
reading what the log is for.

Both rows are red again, and the four other trend-gate mutations still are:
`a worsening hidden behind a joining sample` and `a worsening paid for by an
unclassified counter` were re-run against the new rule and both exit 1.

The cost is that a branch worsening a counter must now write the number, not
just the name. That is what the log's own rule already asks for — pin the
number, never a band — so the gate is asking for the record it was always
supposed to be reading.

## 2026-09-01 (later still, third) — sixty-four depths walked on every beat pop

`k_beat_pop` is 2,205,202 calls in encodebench at 282 instructions each, and
216 of the 282 are one loop. `k_ten_release` frees the tenure blocks at a
depth, then recomputes `k_ten_any` — a summary over all sixty-four depths of
whether ANY holds a block — by walking all sixty-four. It did that on every
pop, including the pops where this depth held nothing and the summary
therefore could not have changed.

An early return when the depth's list is already empty takes `k_beat_pop` to
66 instructions a call and removes 476,323,632 from encodebench, which is that
row's entire fall to the instruction. The argument is arithmetic: `k_ten_any`
is a disjunction over all depths, nothing at any depth changes on the early
path, so the summary that was correct on entry is correct on exit. And
`k_ten_bytes[d]` cannot be non-zero with `k_ten_blocks[d]` NULL, because the
only `+=` follows a push at that depth.

encodebench falls to 7,393,829,376 (-6.05%), oneshot to 40,210,971 (-2.88%),
deepbench to 705,258,631 (-1.68%), escapebench to 248,370,844 (-0.26%);
jsonbench, basket, pendbench and digestbench each fall by less than a
thousandth. Encodebench's fall is 476,323,632 on the runner and 476,323,632 in
this container, on two different cpus — the saving is the walk, so it is
host-invariant even where the totals are not.

**work_widebench reads 64,077,249 and work_indexbench 5,243,101, each five
instructions ABOVE the row it replaces, and the cause is silicon rather than
this change.** The rows on main were counted on the AMD Genoa that ran the
previous pull request; these were counted on an Intel the pool had not shown
before. Measured in this container, where both arms sit on one cpu, the change
takes 408 instructions off each of them. Five instructions on sixty-four
million is what a cpu change looks like at this scale, and the gate is right to
make that sentence exist rather than let two rises pass as noise — which is the
discipline the entry above shipped, applied to its own author.

**The same early-out in `k_chunkreg_migrate` was measured and dropped.** It is
a wash — deepbench -225,082, widebench +40, basket +21 — and the change is
smaller without it.

**And nothing in the tree could see the inverse.** Break `k_ten_release` on
purpose so it never frees and never recomputes, and: all nine allocation
counter gates stay green, because `k_ten_alloc` mallocs without touching
`k_stat_allocs`, `k_stat_bytes_malloc` or `k_stat_held_live`; the whole test
suite passes but for the two `docs/kanso.wasm` staleness guards, which fail on
the correct build too; and the instruction vein reads the leak as a further
WIN, 7,378,392,563 against the correct fix's 7,393,828,977, because skipping
the frees is cheaper than doing them. A change that leaked every tenure block
would have landed green and looked like an improvement.

So the fix ships with the counter that catches it. `ten_blocks` counts storage
claimed and `ten_frees` counts it given back, in all nine cost goldens and all
fifty-two mem fixtures, every one pinned exactly. The coverage is thin —
widebench and one mem fixture tenure a single block each and everything else
tenures none — and sufficient, because the broken version reads `ten_frees=0`
against a pinned 1. Both veins move purely additively: every line that was
there is byte-identical, which is the same statement the nine counter gates
make about this change. `ten_blocks` joins the lower table beside
the other allocation counters and `ten_frees` the higher table beside
`bytes_freed`, because with blocks held constant a fall in frees is a leak.

`text` rises 1,072 to 727,778 — the two counters and the early return, about
110 bytes a binary. `compile_instructions` rises 981 to 41,497,253, which is
`include_str!("runtime.c")` growing again and shifting what follows it in the
front end's binary, the same layout effect the entry above measured directly.
Those two are the whole cost of the change on any vein.

The run that produced these rows named a cpu nobody had seen: family 0x6 model
0xcf, an Intel that is neither the Cascade Lake this container is nor the four
the entry above counted. The pool holds at least five.

Welfare 74.50 to 74.59, floor set.

## 2026-09-01 (last) — the decode costs 98.3 instructions an input byte

jsonbench decodes a 188,698-byte document 150 times for 2,781,834,449
instructions, which is 18,545,563 a decode and 98.3 an input byte. Nothing in
the tree stated that figure; the page states it now. It is the number to quote
when somebody asks what a kanso decode costs, because it is independent of how
big the document is and of how many times the benchmark runs it.

**Where it sits, re-measured after the two runtime changes above.** Every
function attributed to where it came from: 1,729,183,050 instructions in
emitted kanso, 1,001,841,947 in `runtime.c`, 50,803,205 in libc — 62.2%, 36.0%
and 1.8%. The same measurement on 2026-08-31 read 1,728,709,950 emitted against
1,078,786,257 in the runtime. So the emitted half moved by 473,100 and the
runtime half fell by 76,944,310, which is what two days of runtime work looks
like from outside: `k_b_append_mut` went from 7.05% of the decode to 3.75%, and
the largest runtime entries are now `k_b_put_mut` at 5.00%, `k_utf8_bad` at
4.22% and `k_b_append_mut` at 3.75%.

`value_for` is still the largest single symbol at 23.30%, and still a merged
one — clang inlined `parse_string`, `parse_array`, `parse_object`,
`parse_number` and `skip_ws` into it and none of the five appears under its own
name. The 2026-08-31 entry said the runtime had become the smaller half of a
decode; it is smaller again, and what is left of the larger half is the
backend's output rather than anything the runtime can be asked to do better.

**Two things measured in that profile, one worth taking and one not.**
`k_utf8_bad` costs 117,522,450 instructions over 1,571,250 calls, and
10,975,500 bytes pass through it, so the mean run is SEVEN bytes — the comment
in the function claiming forty-one is wrong, and it is wrong in the direction
that matters. At seven bytes the eight-byte loop usually never runs: its head
executes 159,450 times against 1,571,250 calls, and the byte-at-a-time walk
underneath answers for nearly every token. That walk is 30,347,250
instructions, 25.8% of the function and 1.09% of the whole decode.

The second is declined by the same profile. `k_stat_utf8_bytes += len` is
ungated where every other counter tests `k_stats_on` first, and gating it
would be a regression: it compiles to one `add` to memory, executed 1,571,250
times, where the gate is a compare and a branch. Gating costs 1,571,250
instructions and saves none. The rule that every counter is gated is a rule
about counters expensive enough to gate.

## 2026-09-01 (last) — seven bytes, and a walk that answered for all of them

The comment over `k_utf8_bad`'s ascii prologue said the average token was
forty-one bytes. The counters say seven: jsonbench calls it 1,571,250 times
for 10,975,500 bytes. That number decides the shape of the function. At
forty-one the eight-byte loop does the work and the byte-at-a-time walk below
it is a remainder; at seven the loop's head executes 159,450 times against
1,571,250 calls and the walk answers for nearly every token in the document.
Callgrind at instruction granularity puts the walk at 30,347,250 instructions
— 25.8% of the function, 1.09% of the whole decode.

`k_all_ascii` answers with loads that overlap instead. Eight bytes or more:
read whole words while eight remain, then read the LAST eight, repeating bytes
the loop already saw rather than walking whatever is left and without any
arithmetic to work out how many that is. Under eight there is no word to read,
so four and two do the same a step down, and one byte is one compare. Nothing
reads outside `data[0..len)`, which is what makes the overlap free.

Measured on this box, before and after, same binary set:

```
jsonbench    2,781,834,036 -> 2,747,404,386   -34,429,650  -1.238%
oneshot         40,210,572 ->     39,981,042      -229,530  -0.571%
widebench       64,076,836 ->     63,873,599      -203,237  -0.317%
basket          56,457,221 ->     56,448,794        -8,427  -0.015%
encodebench  7,393,828,977 ->  7,393,599,846      -229,131  -0.003%
deepbench, escapebench, pendbench, indexbench, digestbench   identical
```

Five fall, five do not move, none rises. The whole of jsonbench's fall is
`k_utf8_bad` itself: 117,522,450 down to 83,092,800. Re-splitting the decode
by origin, the emitted half is byte-identical at 1,729,183,050 and the runtime
half goes 1,001,841,947 to 967,380,120, which is what a change confined to
`runtime.c` should look like from outside.

**A mutation the harness could not see.** The differential extracts the
validator's text from `src/runtime.c` rather than copying it, so it now
extracts `k_all_ascii` too — the piece where a width bug would show. Breaking
the four-byte overlap turns it red at once. Breaking the EIGHT-byte overlap
did not: 45,189,025 cases, zero mismatches, with a validator that never looked
past byte eight of a run. The sampled band ran from four to eight bytes, which
is exactly the region where the word loop does everything and the tail has
nothing to answer for, and `unsigned char buf[8]` was what held it there. The
band now runs to twenty-four, which also reaches past the sixteen-byte vector
boundary, and the same mutation fails on 1,097,135 cases. It is a ratchet row
of its own now, so the band cannot quietly shrink back.

**CI's rows, and a delta that reads across silicon.** The runner counts
jsonbench 2,781,834,449 -> 2,747,404,799 and the four other movers by
-229,530, -203,237, -8,427 and -229,131. Every one of those five deltas is
IDENTICAL to the container's, on a Genoa against a Cascade Lake. The absolute
rows differ by a few hundred as they always do; the differences do not differ
at all. A change with no dispatch-sensitive path in it can have its delta read
across silicon even where its rows cannot, and this is the cleanest instance
the vein has produced. Welfare 74.59 to 74.61, floor set.

**Two veins that could have moved and did not.** `.text` is byte-identical for
all nine benchmarks — a walk removed and a load ladder added come to the same
size, so the machine-code golden, which carried 17% of the last regression,
says nothing here and is right to. `compile_instructions` fell 161 (0.0004%),
banked as layout: `kanso check lib/json` never emits, never links and never
reads runtime.c's contents, but the compiler carries it as an `include_str!`,
so a longer one moves the binary underneath the measured path. Front-end
allocations and peak are identical.

**The other residual is declined, by the same profile.**
`k_stat_utf8_bytes += len` is ungated where every other counter tests
`k_stats_on` first. Gating it would be a regression: it compiles to one `add`
to memory, and the gate is a compare and a branch. The rule that every counter
is gated is a rule about counters expensive enough to gate.

## 2026-09-01 (last) — the growth path ran more often than the fast one

`k_b_put_mut` was the largest runtime entry left in the decode: 139,045,650
instructions over 1,254,150 calls, 110.9 each, 5.06% of jsonbench. The counters
say what the profile only implies. put_mut_grow read 669,750 against
put_mut_fast's 584,400 — more than half of every map insert reallocated the
pairs buffer, memcpy'd it and donated the old one. Encode was the same shape at
4,465 against 3,896.

The arithmetic was in the growth arm. It started at `cap = 4` KValues, which is
two pairs, and doubled from there, so a map of k keys grew at k = 1, 3, 5, 9,
17. A JSON object with two keys therefore paid a growth for its first insert
and a three-key object paid two, and the objects in this corpus are small
enough that the doubling never got going. Four is room for two pairs, and it is
the wrong four: the LIST path has never used it.

`k_b_push_into_proven` sizes a fresh list's buffer with `cap = 4`, doubles
while the length needs it, and then doubles once more unconditionally, so a
list holding its first element gets eight KValues. `k_b_put_mut` did the first
two steps and not the third, so a map holding its first pair got four. The two
containers grow the same way and started a factor of two apart, and this change
is the map taking the list's second doubling. Eight is not a constant tuned to
this corpus; it is the number the sibling path already used, and the ladder
below is the check rather than the choice.

**The ladder, measured rather than reasoned about.**

```
cap   jsonbench       vs 4       put_mut_grow  arena_blocks  peak_bytes
 4    2,747,404,386      —          669,750         2          2,097,152
 8    2,718,705,486   -1.044%       419,850         2          2,097,152
16    2,708,688,349   -1.409%       334,350         3          3,145,728
32    2,710,734,799   -1.335%       334,350         3          3,145,728
```

Sixteen is the instruction minimum and the objective refuses it. Welfare reads
74.14 there against a floor of 74.6054: the third arena block and the extra
megabyte of peak cost 0.47 points, where the 0.37% of instructions sixteen buys
over eight are worth a fraction of one. Eight reads 74.6146, a rise of 0.0092,
and thirty-two is slower than sixteen for the same growth count because the
memcpys it does are bigger.

This is the trade the index was built to settle, and it settled it against the
number a per-counter reading would have picked. The instruction vein alone says
sixteen; the sum says eight.

**What eight costs.** jsonbench's alloc_bytes goes 262,667,408 to 268,048,208,
two per cent more bytes requested, and its allocs FALL 5,334,608 to 5,334,308.
The extra bytes are transient — peak does not move on any benchmark, and
neither does any arena block count. On the small fixtures the bytes fall too:
`map_put.mem` reads 9,136 where it read 9,216, because one fewer allocation
outweighs one bigger buffer.

Four rows fall and six do not move: jsonbench -1.044%, oneshot -0.481%, basket
-0.040%, encodebench -0.003%. `.text` is byte-identical for all nine binaries
again, which is what a constant change should look like. (Three more rows move
once the tenure fix below joins the branch; the combined figures are at the end
of that entry.)

**The counters the shelf keeps.** A growth donates the outgrown buffer to the
shelf and a later allocation takes it back, so halving the growths halves both
halves of that trade. `buf_reuse` reads 85,350 on jsonbench where it read
334,950, `encode_buf_reuse` 1,780, `oneshot_buf_reuse` 569 and
`basket_buf_reuse` 98 — every one of them a buffer nobody had to hand back
because nobody outgrew one. `sh_buf` is the bytes the shelf saw pass through
and it rises with the buffers being bigger: 143,306,400 on jsonbench,
`encode_sh_buf` 73,270,544, `oneshot_sh_buf` 1,133,328. The same two bytes per
pair show up as `encode_alloc_bytes` 853,137,424 and `oneshot_alloc_bytes`
4,490,268. Basket goes the other way on both, because its maps are large enough
that eight is one doubling it no longer needs.

## 2026-09-01 (last, later) — the tenure block a survivor still pointed at

kanso#1209 could not go green. The cost-goldens job died with `the program ran
out of stack: recursion went deeper than the stack holds`, and the program that
died was the trend gate. The message was wrong twice over: the stack was eight
frames deep, and the fall was a SIGSEGV the parent translates to that sentence
because native cannot see its own recursion.

Reproduction, on origin/main at 21d5c933 with nothing else changed: move one
digit in `bench/cost_golden.txt` and run `scripts/trend_gate`. Native
segfaults; `--interp` prints the listing and exits 0. #1209 was the first
change in a while to move a counter in that file, which is why it surfaced
there and not earlier.

**Where it died.** gdb puts the fault in `k_copy_size` reading `s->data` for a
KStr at 0x7ffff7e45b10, an address in the hole between two mappings — a block
malloc had served with mmap and freed back to the kernel. A `free`-recording
wrapper named the site: `k_ten_release`, the tenure allocator's block release.
The path from the beat's result to the dead byte was a list in the arena, a
record in the arena, and a string in the freed block.

**Why the walk could not see it.** `k_survives_x` answers yes for a pointer
into a tenure block, and that answer is what lets the copy prune: a survivor
whose immediate interior survives is shared rather than copied, and the walk
stops there. For the arena the prune is sound, because arena allocation is
monotonic — a survivor can only point at storage older than itself, which is
therefore also a survivor. Tenure storage is younger than the survivors that
come to hold it, so an arena record can carry a tenured string with no arena
pointer anywhere on the path to say so. `k_beat_pop`'s copy-out walks the
result with a null mark, which does turn the tenure answer off, and prunes at
the arena list one level above the record. Then it freed the block.

**The fix.** A heap result keeps the region alive — `k_beat_pop` does not
rewind for one — so the blocks are handed to the depth outside instead of
freed, and released on the branch that does rewind, which is where everything
the beat allocated goes back. `k_ten_bytes` travels with the blocks, so
`K_TEN_CAP` still bounds what one depth may hold.

One case is narrower rather than closed, and the comment in runtime.c says so:
a node below that mark, repaired during the beat to hold a tenured pointer,
outlives the rewind that frees the block. `k_repaired_settle` is where it would
close — it exists to move repaired slots into the arena and leaves the tenured
ones where they are, because `k_survives_x` answers yes for those too. Making
that pass tenure-blind was built and did NOT change the gate's crash, so the
route this bug took is the one above; the residual is recorded rather than
patched on a guess.

**What it costs.** Across all nine benchmarks exactly one counter moves:
`scan_ten_frees` 1 -> 0. One 256 KiB block on scanbench is now handed up rather
than freed at the pop. No allocation counter, no arena block, no peak, and
widebench's `ten_frees` still reads 1 because its beat pops with a non-heap
result. Every binary's `.text` rises 144 bytes, deepbench 160, and the machine
code vein `text` lands on 729,106 across the nine.

**The rows, measured by CI on the recorded Genoa.** Seven fall and three hold:

```
jsonbench     2,747,404,799 -> 2,718,705,899   -1.045%
encodebench   7,393,600,245 -> 7,393,366,858   -0.003%
oneshot          39,981,441 ->    39,789,223   -0.481%
basket           56,449,207 ->    56,426,594   -0.040%
deepbench       705,258,631 ->   705,257,898   -733
pendbench       930,587,202 ->   930,587,200   -2
indexbench        5,243,101 ->     5,242,731   -370
```

widebench, escapebench and digestbench are byte-identical. The first four are
mostly the capacity change; the last three are this fix alone — a branch added
to the beat pop and a free taken off it, which is what a change that removes
work from a hot exit looks like when the exit runs a few thousand times.
`compile_instructions` falls 1,320 for the reason the vein has recorded four
times now: main.rs holds runtime.c as an `include_str!` and hashes it for the
build cache key, so the C source's length moves the front end's arithmetic
without moving its work. `compile_allocs` and `compile_peak_bytes` are
identical.

Welfare 74.6146 to 74.6196, and the floor is set.

**The fixture.** `tests/a_tenure_block_a_survivor_points_at.rs`. It took three
things at once and no fewer: an inner beat that builds a batch, an outer beat
that accumulates the batches so the batch nodes live a lap and are promoted,
and a SECOND pass over the accumulated list so a later evacuation walks the
promoted nodes after their block has gone. Every earlier attempt had two of the
three and read green — the dangling pointer was there, a detector saw it, and
nothing dereferenced it. Watched red on origin/main's runtime.c rebuilt in
place: the assertion fails in 0.69 seconds. Green here in 21, with the oracle
agreeing to the byte.

## 2026-09-02 — the dispatcher moves to the call site

**DONE** (kanso#PR). A call whose head is a value — a lambda, a parameter, a
bound function — compiles to `call @k_call{n}`, and the runtime dispatcher it
lands in is 26 instructions for arity two. Ten of them ask about the callable:
is it a failure, is it a closure, does its arity match. A fold passes the same
callable through its self-call unchanged, so once TailCallElim turns the
`musttail` recursion into a loop those ten are loop-invariant and LICM would
hoist them out. LICM cannot hoist across a call, and the dispatcher is a call.

Deleting every one of those ten checks from `k_call2` bounded the prize at
437,205 instructions on oneshot, 1.096%. That number corrected an earlier
estimate of 0.51%, which had counted only the fold's 29,147 applications;
oneshot makes about 43,720 `k_call2` calls in all.

**Two levers were tried before the one that shipped.** LTO is real here —
`cached_runtime_object` compiles runtime.c to genuine LLVM bitcode under
`-O3 -flto`, and `k_call2` is internalized in the linked module — but the
inliner declines it on cost. `__attribute__((always_inline))` on the C
definition is ignored without a warning, and `.text` came back byte-identical
at 100,530: the repo's working pattern is `static inline
__attribute__((always_inline))`, and `k_call2` cannot be static because the
emitted `.ll` calls the symbol by name.

**What shipped.** `call_twin` in codegen.rs writes an `internal alwaysinline`
twin per arity the program uses, in the module the optimizer is already in —
the same shape as `k_force_fast` and `k_b_append_byte`, which exist in
DECLARES for the same reason. The twin covers a closure of the arity written
with no failure in an argument, and everything else calls `k_call{n}`, which
re-asks the lot and answers as before. That is why the twin may test in a
different order from the runtime: the arm only fires where all the orders
agree.

The twins are generated against the body rather than carried in DECLARES,
because an unused `internal` definition is free after optimization and not
free in `bench/emitted_golden*.txt`.

**The rows, measured by CI.** Six fall and four hold.

```
digestbench     143,471,767 ->   137,057,629   -4.471%
pendbench       930,587,200 ->   912,184,212   -1.978%
encodebench   7,393,366,858 -> 7,257,716,458   -1.835%
oneshot          39,789,223 ->    39,450,097   -0.852%
deepbench       705,257,898 ->   701,525,898   -0.529%
basket           56,426,594 ->    56,139,380   -0.509%
```

This sitting is on family 0x6 model 0xad, not the recorded Genoa. The rows
were also measured locally, on a third chip and a different glibc, and every
delta agreed to within a per cent of itself — a per-row constant offset of
about 400 separates the two hosts, which is the process startup the empty
environment does not remove. The moves are two to four orders of magnitude
larger than that, so the silicon is not what is being read here.

**Welfare 74.6196 to 74.6928, and the floor is set.**

jsonbench, widebench, escapebench and indexbench are byte-identical.
jsonbench is the interesting one: it writes fifteen sites for the twins and
reaches none of them from its entry, so the linker drops both twins and its
`.text` does not move either. The decoder does not dispatch on a value.

**What it costs.** Six binaries gain 192 to 672 bytes of `.text`, 0.28% to
0.60%; the three that never link a live site gain nothing. Every emitted
program gains two defines, four calls, six branches and fifty-three lines.
No allocation counter moves, on any of the nine — all nine counter gates are
byte-identical. `compile_golden.txt` does not move; only the module sample in
`compile_golden_modules.txt` does, by the same two defines.

**The fixture.** `tests/golden/micro/a_callable_that_is_a_value.kso`, on all
three engines and again under a release build. Nineteen lines of output over
arity one and two: a lambda, a fnref, a capturing closure, a lambda that
ignores its argument, a failing argument in each position, both failing, and
a failing callable.

The failing arguments are computed inside a body, not passed by a caller. The
first version passed them in, and a declared group refuses a failing argument
before its body runs — so the dispatch was never reached and the whole family
read green under every mutation.

Four mutations, four verdicts. Dropping the argument failure test: red,
because a lambda that ignores its argument answers 42 where the dispatcher
answered the failure. Reading the env pointer or the fn pointer from the wrong
slot: red, and loudly — the capturing closures jump into nothing and the
program dies on stack exhaustion. Reading arity from the capture count: output
identical, caught instead by `bench/instructions_golden.txt`, where oneshot
rises to 40,090,932, above the baseline it started from.

**The disassembly says it worked the way the argument said it would.**
`d_list/fold_go_3` in oneshot now tests the callable's tag ONCE, at `3cc4`,
before the loop header at `3cd0` — and LLVM went further than hoisting: it
unswitched the loop on that test and emitted two copies of the body, one for
the closure case and one for everything else. The hot copy has no dispatch in
it at all. That is where the few hundred bytes of `.text` went, and it is why
`k_call2` has left oneshot's profile entirely; the top twenty is now
`d_json/value_for_3` at 10.95%, `k_b_append_mut` at 9.40% and the fold itself
at 3.31%.

`w_klam17` sits at 3.10% and was checked as the next candidate: the emitter
writes a plain-C wrapper beside every lifted lambda so `k_call{n}` has
something to call at the C convention, and the wrapper looked like a pure
`ccc`-to-`tailcc` hop worth deleting. It is not one. LLVM has inlined the
lambda's body into the wrapper, so the symbol IS the body — a JSON string
escape switch — and there is no forwarding frame to remove.

**A divergence the fixture found on its way in.** The wasm backend disagreed
with the other two engines about a value-headed call, in two ways, and neither
had anything to do with this change — nothing had ever asked.
`call_closure` in wasm_rt.rs read its arguments before its callable and handed
back the first failing argument. `k_call2` and the interpreter both name a
failing CALLABLE first, and both MERGE two failing arguments into one err
carrying both reasons. So `bad (boom "a") (boom "b")` answered `a` on the page
and `["a" "b"]` on the two engines that agree, and a call with a failure in
both the head and an argument named the argument on the page and the head
everywhere else.

Both are the same reading `rt_mkrec` already applies to a record's fields —
its comment says returning the first failure there "was a divergence from the
oracle that no fixture built" — and the fix is the same two lines: test the
callable first, then reduce the failing arguments with
`eval::accumulate_failures`. A single failure keeps its own handle rather than
a copy of its value. Watched red before it was watched green: the corpus test
prints both output strings side by side and names the sample.

**Pricing the thirteen counters that worsened**, so the trend gate has its
sentence and its number for each. Twelve of them count the same twenty-six
lines of IR the generator writes per arity, times the arities a program uses.
`emitted_defines` lands on 156 and `emitted_lines` on 11,580 for the decoder;
`emitted_branches` on 1,174 and `emitted_calls` on 1,789 beside them.
Across the other ten programs `emitted_other_defines` lands on 1,339,
`emitted_other_calls` on 14,342, `emitted_other_branches` on 8,389 and
`emitted_other_lines` on 83,175 — sixteen defines, thirty-two calls, forty-
eight branches and 424 lines for ten programs at two arities each. The module
sample in the compile golden lands on `module_defines` 78, `module_calls` 753,
`module_branches` 375 and `module_lines` 4,480, the same two twins once.

Those twelve count DEFINITIONS rather than code. The bodies are `internal` and
`alwaysinline`, so after optimization they exist only at their call sites; the
vein that says what actually got built is `text`, which lands on 730,978
across the nine — 1,872 bytes for six binaries and nothing for the three whose
sites the linker drops. Bought with 24.9 million instructions off the work
vein.

`compile_instructions` lands on 41,500,519, a rise of 4,747 on CI and a fall
of 20,230 on this container for the same diff. `kanso check lib/json` emits
nothing, so the code this change writes never runs during the measurement:
both numbers are the front end's own layout moving under a larger codegen.rs,
and the two hosts disagreeing on the sign is the clearest statement of that.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are byte-identical.

**Not attempted.** Arity zero, three and four have twins and no call sites in
any benchmark. The generator writes them if a program asks; nothing measured
does.

**OPEN, small, unpinned.** `k_call2` tests its arguments for failure before it
tests arity; the interpreter tests arity first. A value-headed call with the
wrong arity AND a failing argument would therefore return the failure on
native and die with an arity message on the oracle. No program in the corpus
reaches it and I did not find a way to write one — a literal lambda's arity is
checked at compile time, and reaching the runtime check means passing a
callable of one arity into a body that applies another. Recorded rather than
guessed at.

## 2026-09-02 (later) — the record read and the record test follow the dispatcher

**DONE** (kanso#PR). #1210 moved the value-headed dispatcher to the call site
and the profile it left named the next two: `k_check_rec` at 2.16% of
encodebench's own instructions and `k_field` at 0.74%, before the call frames
at any of the 134 sites the compiler writes for them across the ten
benchmarks. A fold that matches a record pays both once a lap.

Both are the same shape as the twins DECLARES already carries for
`k_check_tag`, `k_check_int` and `k_check_bool`, so they go in beside them
rather than being generated against the body.

**The first version tested the wrong thing and three rows rose.** It asked
`tag == K_REC` and sent everything else to the runtime, which put every
"this value is not a record at all" answer through a call it did not need:
pendbench fell 18.9% and encodebench only 0.18%, while widebench rose 0.100%
and digestbench 0.482%. The runtime's own first line is `if (v.tag == K_SUB)`,
and asking that instead lets the twin answer every shape the runtime answers —
a record by reading it, anything else with a flat zero — leaving it only a
wrapper to walk. Nothing rises after that.

Reordering to test `K_REC` first and decide between the wrapper and the flat
no in the else arm was built and measured: byte-identical output, because LLVM
canonicalises the two orderings to the same code. The shorter form is the one
that shipped, because it is also the one the C is written in.

**The rows, measured locally against the same tree built both ways.** Seven
fall, three hold, none rises.

```
pendbench       912,183,826 ->   749,657,820   -17.817%
encodebench   7,257,716,059 -> 6,947,803,659    -4.270%
oneshot          39,449,698 ->    38,669,395    -1.978%
digestbench     137,057,230 ->   134,729,014    -1.699%
deepbench       701,522,218 ->   692,270,218    -1.319%
basket           56,138,967 ->    55,762,087    -0.671%
widebench        63,873,599 ->    63,601,599    -0.426%
```

pendbench carries it because its pending cells are records read once a lap in
a loop that does almost nothing else. jsonbench, escapebench and indexbench are
byte-identical: the decoder BUILDS records rather than matching them, and its
`k_field` sites sit in library code its entry never reaches.

`k_field` is gone from every profile — fully inlined. `k_check_rec` is not:
encodebench still spends 116,146,800 in it against 156,995,200 before, because
a good share of what it matches is a subtype and a subtype still walks its
chain in the runtime.

**What it costs, and this one is not small.** Six binaries gain 2,992 to 4,752
bytes of `.text`, 3.0% to 6.1% — the largest single move this vein has
recorded, and about six times what #1210 paid. A record read plus a record
pattern test is more code than a call to one, at 134 sites. No allocation
counter moves on any of the nine, and every emitted program gains two defines,
three calls, three branches and forty-nine lines.

**Welfare cannot see the bytes.** Its twenty-one terms are allocations, arena
blocks, peaks and instruction counts; `.text` is in none of them, so the index
will read this as pure gain. The text golden is the vein that watches the
half welfare is blind to, and this entry is where the trade is stated: 24.9
million instructions of runtime work for 22,704 bytes of machine code across
six binaries.

**Four mutations, four verdicts.** Moving `k_field`'s fields offset from 16 to
24 reddens a single record sample. The other three are invisible to one sample
and caught by the corpus: `K_SUB` 15 to 14 fails 2 of the golden suite's 10
tests, `K_REC` 7 to 6 fails 5, and reading `nfields` from offset 16 instead of
8 fails 5. Each was watched red before it was watched green.

**The rows CI counted**, on family 0x6 model 0xad rather than the recorded
Genoa. Every delta matched the container's to the instruction, the same way
kq#91's four rows did on the same day.

```
pendbench       912,184,212 ->   749,658,206
encodebench   7,257,716,458 -> 6,947,804,058
oneshot          39,450,097 ->    38,669,794
digestbench     137,057,629 ->   134,729,413
deepbench       701,525,898 ->   692,273,898
basket           56,139,380 ->    55,762,500
widebench        63,874,012 ->    63,602,012
```

`compile_instructions` lands on 41,495,470, a FALL of 5,049 — and the change
before this one ROSE 4,747 on the same host for the same reason. `kanso check
lib/json` emits nothing, so neither number is the front end doing more or less
work; both are its own code layout moving under a larger codegen.rs.
`compile_allocs`, `compile_peak_bytes`, rounds and visits are byte-identical.

**Welfare 74.6928 to 74.8069, and the floor is set.**

**Pricing the seventeen counters that worsened.** Sixteen count the same two
definitions the compiler now writes into every program, times the programs.
The single-file samples in the compile golden land on `defines` 104, `calls`
112, `branches` 87 and `lines` 1,787; the module sample on `module_defines`
80, `module_calls` 756, `module_branches` 378 and `module_lines` 4,529. The
decoder lands on `emitted_defines` 158, `emitted_calls` 1,792,
`emitted_branches` 1,177 and `emitted_lines` 11,629, and across the other ten
programs `emitted_other_defines` 1,359, `emitted_other_calls` 14,372,
`emitted_other_branches` 8,419 and `emitted_other_lines` 83,669.

The seventeenth is the one that counts code rather than definitions: `text`
lands on 752,930 across the nine, up 21,952. That is the number this entry is
really about, and the paragraph above says what bought it.

**OPEN.** The subtype walk is what `k_check_rec` still costs encodebench, and
the twin hands every subtype to it. Whether a one-level unwrap belongs in the
twin is a measurement nobody has taken: the chain is usually one deep, and
the loop is there for the case where it is not.
