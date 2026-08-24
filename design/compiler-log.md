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

## The test-file exemption retires, and it was hiding one function (#939)

Clay ruled the test surface on 2026-08-17: assertions are ordinary foreign
rescue. The assert hako's arms receive the test file's err legitimately,
because the raiser is foreign to them, so nothing needs excusing and the
`*_test.kso` file-scope exemption in the err-license advisory has no work
left. #938 recorded that in the log and in pending-gavels and changed no
code; this is the code half, one line of `violations()` in provenance.rs.

WHAT IT WAS HIDING, and the number is the whole argument. lib/json reported
two advisories before the change and three after:

    before   failure_position, failure_reason
    after    failure_position, failure_reason, defect?

The first two live in json.kso, were never exempt, and were already firing —
worth stating because the obvious reading of a three-line diagnostic after a
one-line diff is that the diff caused all three. Checking the baseline first
is what separates them. So the exemption covered exactly one function across
twelve `*_test.kso` files: `defect?` in json_test.kso, a test file's own arm
rescuing an err its own program raised.

That is a true positive rather than a gap. Widening the rule to cover "a test
file's own helpers" would rebuild the file-scope exemption in narrower
clothes, which is the thing that just retired. The direction is fix-the-test.

WHAT THE FIX WAITS ON. `defect?` is the third instance of an idiom json ships
publicly — `failure_position` and `failure_reason` make the same move, an arm
destructuring an err its own program raised to answer a plain value. Those two
are retired by gavel 1b, which is ruled, so the projection API is dead weight
by ruling rather than by opinion. What is genuinely open is the ch08 leg: the
chapter either restructures around std/json as the foreign library or narrows
its failure story to the package boundary, and the suite plus
examples/json_failure_door.kso move with whichever shape wins.

A TRAP WORTH THE SENTENCE. Reading pending-gavels alone, 1b looks unruled: the
file says "retired by 1b", `1b` appears nowhere else in it, and no section
carries the marker. The marker had fallen out of the index during a rewrite
while this log kept it, at "### 1b. Foreign structure access, gaveled —
per-field pub". The index is maintained by hand and can lose a fact; the log
is append-only and cannot. They disagree in one direction only, so when they
do, the log wins.

## 2026-08-17 — ch08 restructures, kq goes gate-then-write, os apes Go

Three rulings Clay delegated with "I think you already know the
answer," recorded with their reasoning.

ch08: the chapter restructures around std/json as the foreign library,
thesis "your own failures only bubble" — the licensed case taught
through a concrete dependency. This unblocks the projection migration
(failure_position/failure_reason and kq's vendored copies retire; 1b's
per-field-pub gavel of 2026-08-14 licenses clients to destructure the
reason themselves), whose scope is the projections,
json_failure_door.kso, and the ch08 suite. The other session takes the
mechanical migration; #939's advisory gives it something to verify
against.

kq publish: gate-then-write, Clay's own design. The PR run computes
the race and FAILS on decrease — it never writes, so PR heads always
carry real checks and the zero-checks class (a bot commit as PR head
suppresses the status rollup; measured on kq PR 61) dies at the root.
The post-merge run on main recomputes and commits the ratcheted
artifact via a branch-protection bypass for the Actions bot — safe
because the write sits behind the gate, and it banks improvements
automatically, which improves on hold-not-banked. Standing protocol
rule recorded with it: a PR head with ZERO check runs is not green —
dispatch CI and wait; kq's ci.yml already grew workflow_dispatch for
exactly this.

os: mirror Go's split exactly; committee handles residue; Clay never
sees the boundary cases.

Also restored in the same commit: the visible "1b is GAVELED" marker
in the migration rider — the #921 rewrite had left the ruling only in
the log, and another session nearly blocked the migration on the
pending file's silence. A ruling lives where the next reader greps.

## 2026-08-17 — the description knot: a composition, not a gap

Task #240 found three engines giving three answers to a knotted
constant whose body is a description — `d = print "x" >> use (box d)`:
oracle runs, native refuses, and the browser switched sides as a
side effect of #934/#936. The finder read gavel 20 as ruling records
and asked whether descriptions were a gap in it.

They are not: gavel 15 supplies the missing premise. Gavel 20's
principle was never about constructors — it is guardedness wherever
laziness creates a lazy slot — and 15 ruled that `>>` defers its right
side, which makes a `>>` right operand exactly such a slot. So the
knotted description ties by the same argument as `ring = cell ring`;
native is right under pre-15 semantics and wrong after; the browser's
side-switch was transitional noise on unbuilt gavels. Ruled by
derivation, veto window offered to Clay, verified independently by the
finding session against both gavels' recorded text.

Sequencing, so the corpus does not encode transitional behaviour: the
fixture is pinned WITH the 15+20 builds, oracle-shaped but
20b-rendered — the knot displays `<cycle>`, not today's `<thunk>`.

One display boundary pinned alongside, because the oracle's current
output brushes it: a plan embedded in a rendered value renders as its
inert, cycle-guarded structural form. Render forces thunks (20b) but
NEVER executes effects — execution belongs to the wire alone. Output
demands everything it emits; it runs nothing it merely shows.

The finder's own diagnosis is the method lesson: "I was looking for a
gap where there was a composition." With this many principles ruled,
compositions now outnumber gaps — the first question about an apparent
fork is which OTHER ruling supplies the missing premise.

## 2026-08-17 — gavel 51: one module

Clay ruled the question task #51 had held open since before 2026-08-03
(mis-closed in the task index as "completed"; the log kept it open, and
the log won): a module reached by two import paths is ONE module. Its
identity is its canonical path, never the route an import took;
every route resolves to the single instance; one emission, one set of
type identities, one dispatch group per name.

The double emission is therefore a bug, with the three-file proof in
task #243: main imports shape directly and through mid, the .ll
carries both shape/describe and mid/shape/describe plus two blank
types, and cross-module dispatch dies at runtime on a program check
calls ok. The doubly-qualified diagnostic name stops existing along
with the second instance it honestly named.

Grounds recorded with the ruling: the dispatch architecture anchors on
singular type ownership; purity deletes forking's only benefit (no
mutable module state to isolate); precedent is unanimous (Python, Go,
Haskell instantiate once; npm's duplicated-package instanceof failures
are the cautionary tale); qualified rendering already assumes one
canonical name per type. Blast radius measured before the ruling:
std/list is the only stdlib module both declaring pub types and being
re-imported by lib modules (json, sha256) — narrow, under the
most-used abstraction. Unblocks #241 and is the true cause behind the
json/encode-of-null symptom. The loader/emission fix is build-lane.

## 2026-08-17 — gavel 51 lands, and pays for itself

The ruling above is built. The loader stops composing a prefix onto a
name that already carries one, and a module reached twice contributes
one copy of each declaration. Identity is the canonical path, derived
where it is compared rather than stored on the declaration: a field on
every FnDecl made compile cost depend on how the files were named,
which tests/import_order forbids and which caught it.

Two veins moved and both fell. Machine code: jsonbench -2,320,
encodebench -2,176, oneshot -2,352, basket -2,160, widebench -2,352,
with deepbench and escapebench flat. Work: oneshot -5.54%, widebench
-2.94%, jsonbench -1.82%, basket -0.28%, encodebench -0.0041%.

The cause took three attempts and the first two were wrong. It is not
deleted functions — jsonbench emits the same 151 defines as main and
its IR differs by five lines. It is not the shortened name constants,
which live in .rodata where the machine-code gate does not read, and
whose sizes do not track the falls at all: deepbench sheds 4,508 bytes
of them and does not move. It is the cohort pop. Calls to
k_cohort_pop go 2 to 0 on jsonbench, 3 to 0 on encodebench, 4 to 0 on
oneshot, 3 to 0 on basket, 3 to 0 on widebench, and the two rows whose
count is unchanged are the two rows that hold still. No exceptions
across the corpus.

Those pops were reclaiming nothing, which is what makes this a win and
not a leak: every allocation counter is byte-identical, the lazy tier's
mem goldens are unchanged, and escapebench — the one benchmark whose
storage escapes its beat — keeps its pop. Welfare rose 84.51 to 84.56
and was ratcheted in the same PR. What remains unexplained is why the
count goes to zero rather than halving; the pops were emitted per
duplicated instance and the answer is in the emitter's cohort
condition. Left alone because the behaviour is right either way.

A second crash came out of reviewing the branch's own diff. Two
directories both named `shape` qualify under the same prefix, so both
their `blank` types answer to `shape/blank`. Main gave each its own
type id and wrote both into one switch, and clang refused the module
with `duplicate case value in switch` — on a program with no
overlapping function to blame. One name is one type, so the emitter
writes one case. tests/golden/samename holds it.

One spelling is deliberately gone. `geo/list/order` resolved on main
and is refused now: it named a route through geo's private import of
list, and it only worked because geo's copy of list was a second
instance. The qualified door `geo/order` fails identically on main and
here, so the missing door is not this change's regression — it is #246,
and it is wider than that task said, because no re-export has a
qualified door.

Diagnostics improve as a side effect, and the goldens say so. A stdlib
name stops carrying the entry module: `endpoint_trace/text/to_int` is
`text/to_int`, `length_points_at_to_list/list/mapped` is `list/mapped`.
A user's error message named their own program in the middle of a
stdlib function's origin, which was the doubled instance being honest
about itself.

Siblings verified before merge rather than after. kq gates inside
kanso's CI; kanso-json runs 18 tests green; vse is byte-identical to
main's compiler under KANSO_SEED=12345. Unseeded it is not comparable
at all — two runs on one compiler differ by as much as two compilers
do, which nearly produced a false regression here. Grepping all four
repos for route-shaped names found only path strings and fuzzer seeds.

Found alongside and not fixed: #248, where native refuses a knotted
description that the oracle runs. Gavel 20 swapped that divergence
rather than closing it — before #931 native ran it and the oracle
refused — so both readings are implemented, one per engine, and
choosing is the work.

## 2026-08-18 — the gavel 51 landing had two consumers it did not look for

The entry above is wrong in one place and this corrects it rather than
edits it. Both corrections are the same mistake made twice: the change
stopped producing a name spelling, and nothing went looking for what
READ that spelling.

The first was found by review and is fixed. An import used only through
names it re-exports read as unused, and the program was refused, where
main answered on both engines. `mark_bare_quals` credited an import by
splitting the qualified name and taking the first segment — its own
comment said `geo/list/select` — and gavel 51 leaves a re-exported
`sort` spelled `list/order`, so the segment read `list` and the app's
`geo` was never credited. The qualifier is now recorded in qualify,
where it is known, and a fixture uses a re-exported name and nothing
else, which the old one could not.

The second is not fixed and retracts a published claim. The falls in
both cost veins were recorded as cohort pops that were reclaiming
nothing. They are cohort ENTRY no longer being detected.
`crosses_down` in src/codegen.rs asks whether the callee's module name
extends the caller's by a segment, and that test is the compounded
route spelling and nothing else. Instrumented on jsonbench, main takes
the branch 13 times — twelve of them `jsonbench` into `jsonbench/text`
— and this compiler takes it once.

The evidence that made the first reading look solid holds up and says
something else. Every allocation counter is byte-identical because no
benchmark in the corpus depends on cohort freeing across a module
boundary, which is the gap already recorded as #224. escapebench keeps
its pop because its one hit has an empty caller and the test still
admits that, not because it is the row that collects.

So the predicate needs rebuilding before the veins mean anything.
Module nesting by name is gone deliberately — identity is the canonical
path — and the entry test has to ask the real relation instead: whether
the caller's module imports the callee's. The banked numbers and the
welfare floor that moved with them both describe a compiler with cohort
entry switched off.

## 2026-08-19 — the wrong-verb hint is licensed

Clay ruled yes: "the compiler knows nothing about `play`" constrains
language semantics — no identifier named play ever means anything to
the grammar, checker, or runtime — and never CLI help text. A
diagnostic naming `kanso play` as the verb that would have worked is
the toolchain knowing its own subcommands, the same way cargo build
may suggest cargo run without Rust-the-language knowing either word.
The shipped entry-file hint ("run its definitions beside their
statements with `kanso play`") is ratified as-is; future diagnostics
may hint verbs freely.
## 2026-08-18 — the qualified door, and two more re-export gaps behind it

`geo/order` named nothing where `order` resolved, and the axis was not
the rename: `geo/select` and `geo/to_list` failed the same way, so a
module's own pub had two doors and a re-exported name had one. A
re-export keeps the spelling its owner gave it, and there is no second
declaration to carry the importer's qualifier — nor should there be,
which is what gavel 51 settled.

The door is a second spelling for a declaration that already exists.
`surfaced` already knew which
import surfaces each bare name; it now also records who owns the name
when that is somebody else, and a pass rewrites `qual/bare` to the
owner's spelling in the importing program. A clone would have minted a
second instance. It opens only where the qualified spelling is free and
one declaration answers it, so a module that declares its own `select`
beside an import's keeps what it had.

Two more gaps came out of the fixture, each watched red on its own.

A module whose only reason for an import is re-exporting it read as
unused, and no spelling would have satisfied the check — the per-file
import rule counted expressions and a re-export is not one. The
module-wide pass already had that rule; the per-file pass never got it.

And a type arriving by two routes took the visibility of whichever route
loaded last. Gavel 51 gave functions the rule that an open route is not
vetoed by a sealed one; types were left with an unconditional insert, so
a facade's re-exported type read as private from an entry that reached
it both ways. One line, and it is the same ruling.

Ordering is the thing to keep straight in that function now: credit the
imports, then open the doors, then judge the surface. Rewriting first
credits the owner and the caller's import reads as unused, which is
[[removing-a-spelling-needs-its-consumers]] from the other side.

Every runtime counter is byte-identical and welfare holds at 84.56;
front_end_visits did not move, because the pass walks bodies the front
end already walks and the door map is empty for a program with no
re-exports.

## 2026-08-18 — the door had to answer in pattern position too

Copilot caught it on the PR, and the reproduction confirmed it with a
divergence: `pub fn tell mid/blank` died on native as `unknown type
mid/shape/blank` while the oracle silently picked the wrong arm. The
door rewrote expressions and left patterns alone, so an arm matched a
spelling its caller could not write.

The variant that carries it is `Var`, not `Nullary`: the parser reserves
`Nullary` for the built-in names, so a user's nullary type parses as a
binding and is decided later by whether the name is a declared type.
Isolated one at a time — with only `Nullary` the program still failed,
with only `Var` it answers on both engines. A door key always carries a
qualifier and a bound name never does, so rewriting a `Var` whose name
is a door key cannot touch a binding.

## 2026-08-18 — one Copilot suggestion held and one dissolved under a control

Two more places a type is named, both raised on the PR, and the pair is
worth keeping because they came apart under the same test.

`(v):mid/filled` needed the door and now has it. Watched red — `` `:mid/filled`
widens; this value is not a mid/filled `` — and green with the upcast's
type name rewritten alongside the pattern's.

The other was type declarations: a subtype's parent, a typeset's
members, a field's member list. Extending the rewrite to all three did
not fix either program, and the reason showed up in the control. A
subtype over a DIRECTLY imported parent fails identically, with no
re-export anywhere:

    type narrow shape/filled

    pub fn label (narrow n)
      "narrow {shape/describe n}"

`no overload of sub/label matches these arguments`, on both engines, for
a value that is a shape/filled. The typeset form does the same. So a
type declaration naming an imported type matches nothing today, the
door was never what was wrong, and the speculative rewrite came back out
rather than shipping unproven. Filed on its own, where it belongs.

The failure is silent until run time, which is the sharper half: nothing
refuses the declaration, and the call that dies reads as well-typed.

## 2026-08-18 — a typeset member kept picking up a second prefix

The type-declaration door came back in, and the reason it looked
unnecessary the first time is the interesting part.

The control that dissolved it was malformed. A typeset is matched by an
annotation — `(err e:lane_err)`, as tests/golden/micro/err_trap_order.kso
has spelled it all along — and the fixture used a constructor pattern,
which a typeset has no constructor for. So the refusal was correct and
said nothing about doors. The subtype half was wrong twice over:
`type narrow shape/filled` makes narrow a SUBTYPE, and a plain
shape/filled is not one.

Spelled correctly, the program failed for a different reason and named
it: `native backend: unknown type sub/shape/blank`, against
`no overload of sub/label matches` on the oracle. `own_types` in qualify
was every name in the merged program, including types that arrived
through this module from its own dependencies. `owned` has filtered
qualified names since gavel 51 landed; `own_types` never got the same
filter, so a member already spelled `shape/blank` took a second prefix
and named a type nothing declares.

With that fixed the door gap became visible, and the two isolate cleanly
against one fixture: strip the filter and it says `teller/shape/blank`,
strip the member door and it says `no overload`.

The lesson is about evidence rather than types. A refusal is not evidence
of a defect until the program is known to be well formed, and two of the
three reproductions here were not. What separated them was checking how
the corpus already spells the construct.
## 2026-08-18 — GAVEL: equality refuses a value that names itself

Clay, ruling #235: "to me 'which object is this' makes more sense... how could
you possibly compare two formulas for equality?" and then "yeah i think refuse
is right."

A knotted constant is a definition, and comparing two definitions is not a
question equality answers. `x = [x]` and `y = [y]` are two formulas, and `==`
refuses them the way it already refuses a function or an effect.

WHAT THE RULING REPLACES. Today both engines answer `false`, and not by
decision: `k_eq`'s switch has no arm for a cell, so a thunk drops off the end
and reports unequal — and that same drop is what terminates the walk. A reader
cannot tell it was never decided. The refusal makes the rule sayable.

THE ARGUMENT THAT WAS PUT AND NOT TAKEN, recorded so it is not relitigated as
though it were missed. A knot is finite in memory — two nodes and a back-edge —
and takes no input, so comparing two of them is graph comparison rather than
the function-equality problem, and bisimulation decides it for finitely
generated cycles. Clay's answer is that being decidable is not the same as
being a question worth asking of a formula, and the identity reading is the one
that matches what a knot is. The ruling stands on that, not on tractability.

THE BOUNDARY, which follows from the reasoning rather than from the sentence.
A cycle that closes through a CELL is a definition naming itself, and refuses.
A cycle a build block closes by writing a record in place is one object the
program built, and keeps the structural comparison bisimulation already gives
it (#190-#196). Two cyclic values, two rules, split by whether the cycle is
definitional. Worth stating out loud because it will otherwise read as a bug.

TWO CONSEQUENCES TO CARRY.

Render and equality now answer different questions on purpose: `print "{x}"`
gives `[<cycle>]` for both x and y, which is the SHAPE, where `==` asks which
object and declines. The page owes that sentence, because showing two values
identically while refusing to compare them is otherwise a seam.

`==` is an ambient dispatch group (#98), so the refusal can now surface from a
generic container comparison wherever a knot can reach. That is the cost of the
ruling and it is accepted: a refusal in an unexpected place beats a false
answer, and a user who means something specific writes the arm — which is what
the existing message already tells them to do.

## 2026-08-18 — the knot refusal fires on RE-ENTRY, not on seeing a cell

Clarifying the entry above before it was built wrong. Clay: "you don't refuse
an actual lazy evaluation of a comparable value. the equality itself is lazy."

Equality is a demand site, so it forces a cell the way any demand does. A lazy
binding compared against a value forces and compares; nothing refuses. The
survey had been about to take the simple rule — refuse on encountering any
cell — and that rule would have refused `n == 3` for a merely lazy `n`, which
the gavel does not say.

The discriminator is re-entry. Comparing `x = [x]` against `y = [y]` forces
both to `[cell-x]` and `[cell-y]`, whose elements are those same two cells: the
walk arrives at a pair it is already inside. That is the cycle closing through
a cell, and it is the definitional case the gavel refuses.

So: force cells, carry a seen-set of cell pairs, refuse on re-entry. The
oracle already keeps such a set for records (src/eval.rs:3402), so the shape
exists. And the rule has a property worth naming: `x == [1]` forces, mismatches
at once and answers false, never reaching the refusal — only two knots that
would chase each other forever get it.

## 2026-08-18 — the knot's cells are PENDING, so equality has to be able to force

First attempt at the build, reverted, and the measurement is why.

`values_equal_seen` is a free function. Wiring the refusal into it and running
the fixture printed the cell states directly:

    PROBE pair states: Pending { expr: Ident("knots/x"), env: None, frame: ... }

Both cells of `pub x = [x]` / `pub y = [y]` arrive UNFORCED, holding the name
as an expression. So the walk cannot answer them by reading a memo — evaluating
`Ident("knots/x")` needs the interpreter, which a free function does not have.
The arm that read only `ThunkState::Forced` therefore fell through to false and
nothing changed: the refusal never fired, and the fixture still printed `false`
on both sides.

That fixes the shape of the build. Equality forces, per the ruling, so the
comparison MOVES ONTO THE INTERPRETER — or takes a forcing callback — rather
than staying a free walk over values. The C side has the same requirement and
no seen-set at all yet.

ONE HAZARD TO CHECK BEFORE TRUSTING RE-ENTRY DETECTION: the seen-set keys on
Rc pointer pairs today. If forcing `Ident("knots/x")` hands back a fresh cell
each time rather than the same one, pointer identity will not recognise the
second arrival and the walk will not terminate. Whatever the seen-set keys on
has to be stable across a force — verify with the two-knot fixture before
building the rest.

The Option<bool> plumbing this needed (every arm of the match propagating a
refusal) works and is mechanical; it was reverted with the rest rather than
landed as a behaviourless refactor.

## 2026-08-19 — gavel: `==` refuses a value that names itself, three engines

Built. The hazard the last entry named is answered: forcing a cell rewrites
the SAME cell rather than handing back a fresh one — `force_thunk` assigns
`ThunkState::Forced` into the existing `Rc`, and `k_force` sets `forced`/
`result` on the existing `KThunk*`. Pointer identity therefore survives the
force, which is what makes a second arrival recognisable.

The comparison did not have to move onto the interpreter. It takes a seam
instead:

    pub struct Cells<'a> {
        pub id: &'a dyn Fn(&Value) -> Option<usize>,
        pub force: &'a dyn Fn(Value) -> Result<Value, RuntimeError>,
    }

`id` says which values are cells and gives each a stable identity; `force`
demands one. The oracle passes an `Rc` pointer and `force_thunk`. The browser
passes a slot handle and `forced` — and it needed the widened seam, because a
browser cell is a `TableFn` handle rather than a `Value::Thunk`, so a
thunk-shaped test found nothing there and the corpus reported `false` against
the other two engines. Native carries the same rule in `k_eq_rec` over the
`k_eq_assume` table the record walk already uses.

The rule fires on RE-ENTRY, per Clay: an ordinary lazy operand forces and
compares as its value. Two goldens pin both halves, and the second is the one
that stops the refusal widening unnoticed:

    tests/golden/runtime/equality_refuses_a_value_that_names_itself.kso
    tests/golden/micro/a_knot_compared_against_a_plain_list.kso   # false

The first was watched red on native (`left: ""`, no diagnostic) and on wasm
(`left: "false\n"`) before either arm existed.

## 2026-08-19 — what the knot refusal cost, in the two veins that can see it

CI on #952 moved exactly two rows, and both are legible.

**Machine code: +208 bytes on every one of the six binaries.** The uniformity
is the reading — what grew is `k_eq_rec` in the runtime object, which every
binary links and which none of these benchmarks calls more or less often than
before. Between 0.2% and 0.3% each.

**Work: encodebench alone, 9,724,874,773 to 9,724,924,773.** Fifty thousand
instructions, five ten-thousandths of a per cent, and the other six rows do not
move at all. Equality now asks whether each side is a cell before comparing it;
encodebench is the row with a comparison in its inner loop.

No allocation counter moves, which is right: the refusal allocates nothing and
the forcing it added only runs where a cell is actually met.

## 2026-08-19 — the mixed comparison already worked, and my measurement of it did not

Task #252 filed a bisimulation violation: a knot and the same cycle built in a
block rendering identically, each self-comparing true, and comparing false
against each other. It reproduced on main.

The refusal branch already fixes it. `k == q` and `k.peers == q.peers` both
answer TRUE there, on native and on the oracle, which is the answer structural
equality owes — it is blind to provenance, and the two spellings are one value.
The cell arms do it: the walk meets a cell against a record, forces, arrives at
a pair of records it is already inside, and answers.

I recorded the opposite earlier and it was wrong. The binary I measured had
been built from main; the branch's own build says true. Same class as
`stdlib-is-compiled-into-the-binary` — a stale binary and a real null result
are indistinguishable from the output alone.

Pinned by tests/golden/micro/a_knot_equals_the_same_cycle_built_in_a_block.kso,
which is also why the refusal is scoped to a pair of CELLS: that is the only
place the question is unanswerable.
## 2026-08-19 — gavel 15 built: the wall defers, and the loop runs

A loop written with `>>` now runs to the end on both engines. Four hundred
thousand links, and the fixture in tests/seq_chain_runs.rs (renamed from
seq_chain_names_itself, whose whole subject was the diagnostic for the failure
that no longer happens) prints `done` on native and on the oracle.

It took two changes, and the second is the one a survey would have missed.

**The wall holds its right side.** codegen emits it through `emit_cell` — the
lazy-slot machinery factored out of the lazy-bind path, unchanged — and the
oracle holds a Pending cell inside `Desc::Seq`, which now carries a `Value` and
a span rather than a second `Desc`. `k_seq_right` and `Interp::seq_right` force
it where the wall reaches it, which is also where its failure and its type are
checked.

**The executor walks the right spine instead of recursing into it.** With the
chain no longer built all at once, native still died — at RUN time now, one C
frame per link in `k_exec`. The oracle's `execute_chain` has always been a
loop; `k_exec` case 1 is one now too. Deferral is what made that reachable and
it is not sufficient by itself.

Two consequences, both stated rather than absorbed:

  - A failure in the right side now surfaces AFTER the left has run.
    `print "a" >> print "{[1][5]!}"` printed nothing and errored; it prints `a`
    and then errors. This is the wall's own law finally holding — "the first
    failure is the answer and what follows it never speaks" was contradicted by
    a failure in what follows speaking before the left ran at all.
  - `--plan` builds what the wall holds rather than reading it, through a
    forcing seam `run_plan` supplies. A plan that names itself has no end,
    which is honest.

`tests/golden/mem/build_cycle.imported.mem` gains one cell: thunk_allocs,
forces, evals and live_exit all 0 to 1. That is the one wall in the fixture.

The browser is not done. Its cell is a slot handle rather than a `Value::Thunk`
and its `Seq` has a lazy shape of its own (`Slot::Seq`), so it still evaluates
where it stands and would diverge on failure ordering. Nothing lands until it
agrees.

## 2026-08-19 — the browser's wall, and the two doors it needed

Gavel 15 is built on all three engines. The browser was the last, and it was
not a transcription of the other two.

Its emitter builds the right operand as a capturing cell — the closure
`emit_lambda` already builds, with no parameters and the arity that marks a
cell rather than a function — and `rt_seq` stops asking what the right side is
until the wall reaches it.

Two forcing doors had to widen, for a reason particular to that engine.
`forced` answers a `Value`, but the browser's `>>` and `.` build SLOT shapes
that are not Values, and gavel 15 has just put such a cell to the right of
every wall. So both `forced` and the executor's new `demand` hook materialize
one through `as_desc` rather than reading it as data, and a handle-level
`demanded` exists beside them because `exec_slot` walks handles.

The mem vein caught a real differential on the first program with a wall in
it: native counted the deferred cell as an allocation and the oracle did not.
The oracle records it now.

Full suite green, 60 binaries. wasm_engine 7/7.

## 2026-08-20 — what the deferred wall cost, in the two veins that can see it

**Machine code: +592 bytes on six binaries, +608 on escapebench.** The
uniformity is the reading — `k_seq_right` and the loop `k_exec`'s wall arm
became both live in the runtime object every binary links, and none of these
benchmarks reaches a wall more or less often than before. Between 0.6% and
1.3% each, most of it on the smallest binary.

**Work: deepbench alone, 833,453,153 to 833,609,159** — 156,006 instructions
and 0.019%. It is the row whose loop is written with the wall, so it is the row
that pays for a cell per link. The other six are noise with disagreeing signs:
jsonbench -61, widebench -40, encodebench +4, oneshot +3, basket +1,
escapebench unmoved.

No allocation counter moves and no other vein does either — every counter gate
was run locally and passed, which is how the two linux-only veins were
isolated.

What neither vein can show is the trade: a 400,000-link `>>` loop goes from a
stack exhaustion to a program that prints `done`. There is no benchmark of a
chain that long, because until now there could not be one.

## 2026-08-19 — the July spellings, and the vse tie explained

Clay cleared the last of the small stack ("i'll just follow your
recommendations").

The three spellings: local imports wear the dot prefix (`./util`,
`../geo`; bare multi-segment = hako name) so every import's universe
is readable from its spelling — the migration adds the prefix to bare
local imports fleet-wide. Subtypes declare as `type post_body:string`,
the ascription shape used everywhere else. Into-subtype is the ctor
form `post_body ""`; the postfix sketch dies. Gavel Y is noted closed
in passing — `>>` keeps its glyph, settled when Clay learned it was
Haskell's own value-dropping sequencer ("steal from the best") — and
the July labeled-patterns question (5a) is parked as
needs-a-fresh-look against the post-24 language, not pending.

The vse README call: publish the reproducible 40k numbers with the
honest-model caveat. The score-vs-STAR tie was investigated on Clay's
suspicion of a missing runoff and the suspicion was refuted by
measurement: the runoff is present (methods.kso:80-86) and flips 393
of 4,000 elections (9.8%); the tie is the honest-voter regime doing
what the literature says it does, since STAR's differentiator is
strategic robustness, which the sim does not model. The README gets
the numbers, the caveat, and loses the unsupported Score > STAR
ordering claim; the flip counter stays as a sim sanity gate (flips > 0
pins that STAR is really STAR).

## 2026-08-19 — the testing hako designed, and a seeding rule retires

The queued testing design pass ran as the first refinement-phase
artifact (survey, draft, committee), and design/testing.md is now the
settled design: a test stays a boolean `test_` constant (go test's
shape with less ceremony, per Clay's ruling); one small std hako —
`testing` — owns `failed?` and `when_failed`, ordinary foreign code
licensed by the rules as they stand; the `failed?` BUILTIN and its
_test.kso file gate retire; json_test's error-position and defect
assertions survive the projection migration by reading REASON RECORDS
(the licensed round-trip), with type-dispatch moving from err arms to
reason-type arms; the 2026-07-28 describe/context sketch is recorded
as superseded by the simpler ruling.

The committee pass caught a real collision: July's rule seeding every
pub group's receivable-err set with its own hako would, under gavel
24's clause 1, statically refuse every pub bare-err arm — killing
generic foreign rescue wholesale. Resolution, derived from "your own
failures only bubble": clause 1 is dispatch semantics — an arm cannot
SEE an own-origin err; at match time it skips, infectiousness carries
the err onward. Static refusal keeps what provenance proves without
self-seeding; the pub seed retires with the return-channel rule it
served. Veto window Clay's.

Two refinement stitches logged in the doc: lambdas cannot carry arms
(type-dispatching a reason needs a named local group), and
when_failed's false-on-success conflates two failure modes in a
report. Build hand-off: the hako, the builtin retirement, and the
json_test migration shapes are all in the doc.

## 2026-08-19 — an adversarial sweep of the week's semantics, and the promise paragraph

Under Clay's standing goal ("keep trying to do relevant helpful
things... look for bugs"), a twenty-probe adversarial sweep ran over
the freshly merged surfaces, interp against native, in a worktree at
a283ce15. The engines agreed byte-for-byte on every probe — the
differential law held throughout — and the sweep's findings were
uniformity gaps and confirmations rather than divergences.

Filed with the build lane (its #256/#257, both reproduced there):
the equality refusal is carrier-split (list knots refuse per #953,
record knots answer true through the bisimulation seen-set — the
collision is Clay's to rule, both branches reverse a gavel of his);
and the guardedness check does not treat a bare name as guarded by a
`>>` right side (`loop = print "tick" >> loop` refused while the
constructor-wrapped golden ties), contradicting the description-knot
ruling's own sentence.

Noted for whoever is next in the encoder: json/encode of a cyclic
knot exhausts the stack on both engines, identically — lawful but
unkind; it wants the cycle-aware refusal render just gained.

Confirmed clean under fire: mutual knot fields, knot rendering
(`[1 <cycle> 2]`), the description-knot golden, lazy-wall absorption
with the endpoint trace, the ?-suffix checker teeth, the torn-import
tie diagnostic, cross-hako group merging by specificity, three-deep
diamond dispatch, subtype-plus-as-pattern composition (the as-binding
keeps its subtype identity), float rendering, unicode lengths, and
the int64 divergence (licensed: native refuses with the diagnostic
that names the gap).

One doctrine clarification the sweep forced, recorded here so the
sentence stops being ambiguous: the adjacency law's "all members
always run" is a RUN-PHASE fact. Build-phase failures (an err
arriving while a line's description is being constructed) accumulate
— two failures reach the endpoint as a list — and gate the whole
group's execution; run-phase failures let siblings finish, and
same-resource ordering holds. Measured on p24/p25/p26; a
stderr-vs-stdout buffering difference in the merged stream was a
harness artifact, not a divergence, since the streams are compared
separately everywhere that matters.

And the book paid gavel 6's debt: ch10 now states the contract (a
call in tail position consumes no stack, in every engine, lean on it)
and the appendix draws the line (the accumulator rewrite is an
optimization that may move; the promise is tail position; the
interpreter's ten-thousand-frame guard and the OS stack are the
bounds on what is not promised).

## 2026-08-20 — the accounting audit the welfare sitting was owed

The corpus-blindness campaign flagged a future sitting on what
`peak_of` sums. This is the measurement that arms it, taken on main
with every benchmark rebuilt, real peak RSS beside the pool counters:

    bench          rss          arena        perm_pk    held_pk   bytes_net  unaccounted
    jsonbench      3,735,552    2,097,152    0          0         0          1,638,400
    encodebench    6,094,848    4,194,304    0          719,516   0          1,181,028
    oneshot        4,685,824    2,097,152    0          632,284   0          1,956,388
    basket         5,357,568    2,097,152    2,752,560  71,136    16         436,720
    widebench      3,522,560    2,097,152    0          0         0          1,425,408
    deepbench      2,048,000    1,048,576    0          0         0          999,424
    escapebench    1,687,552    1,048,576    10,272     0         -3,000     628,704
    scanbench      202,260,480  198,180,864  0          0         30         4,079,616

Three conclusions. First, the pools are honest today: the unaccounted
residue is 0.4-2.0 MB per bench, which is the process floor (binary,
runtime, stacks — deepbench's 999,424 IS the floor), two per cent on
the largest row. Second, the thunk-malloc class that hid 92 MB from
the objective last week is structurally EMPTY on this main
(bytes_net ~ 0 everywhere, the per-arm laziness fix's doing) but
UNGUARDED — nothing prevents a future thunk-heavy path from reopening
it, invisibly, at any size. Third, the guard is already lying on the
table: bytes_malloc and bytes_freed are counted on every run, and
welfare reading their difference into peak_of is a one-line model
change that would have priced last week's 92 MB the day it appeared.
That one-liner, plus the growth-class question scanbench raised
(the objective prices a quadratic at its fixture's n), is the whole
agenda for the sitting.

One quirk for whoever is next in the allocator: escapebench frees
3,000 bytes more than it mallocs. Signed, small, and odd.

## 2026-08-20 — the soak, the counter asymmetry, and ch04's absolutism

Three results from the goal loop's afternoon, recorded so none of them
lives only in a conversation.

The utf-8 soak: the validator's differential harness re-run at ten
times CI volume with five fresh xorshift seeds — 384,215,045 cases in
all, zero mismatches. The harness doctrine asks for the count in the
record; this is the count. The other differentials are deliberately
bounded rather than sampled, so the soak lane starts and ends with
utf-8.

The escapebench bytes_net quirk from the accounting audit is
diagnosed to the line and handed to the perf lane: k_buf_perm counts
allocs and alloc_bytes but not bytes_malloc, while the permreg flush
counts bytes_freed when freeing exactly those buffers — one
uncounted-malloc/counted-free pair per registry slot, times
escapebench's 3,000 rounds, equals the −3,000. The fix is one line
and moves five pinned cost goldens, so it rides the counter law in
the lane that owns the veins. It is also a prerequisite for the
peak_of guard the accounting audit proposed, which would otherwise
read a counter with a known skew. Worth knowing while reading:
bytes_malloc and bytes_freed are EVENT counts despite the names.

And ch04 teaches the pre-July absolutism — "no function gets to turn
one back into a value... there is no rescue block to write and none
to import", plus the personified "nobody catches me" — which the
two-universes settlement and gavel 24 superseded: a FOREIGN err is
precisely the handleable case. Filed into the projection migration's
book sweep so chapter 4 and chapter 8 teach one rule. The chapter's
mechanics all survive as probed.

## 2026-08-20 — the fuzz campaign pays: an `if` owns its arity

The mutation campaigns' counts, recorded per the harness doctrine:
800 byte-mutants of 60 fixtures through check — zero panics; 25
survivors executed on both engines — zero panics, signals or
divergences. 8,000 more at higher mutation depth — zero everything,
243 survivors clean. Then the generator changed and the results did:
12,000 CROSSOVER children (fixtures spliced at line granularity, the
recombination class byte-flips cannot reach) found the campaign's one
crash — a compiler panic in eval_expr, infer.rs:533.

Reduced to two lines: `fn judge n` / `if n`. Inference treats `if` as
a call and indexes its branches unconditionally, so one argument
panicked at arg_sets[1], two panicked at arg_sets[2] — and FOUR
arguments passed check entirely and then diverged at runtime, the
interpreter refusing the arity while native evaluated the condition
and failed differently: a differential violation on a program check
called ok, hiding in stderr where the campaign's stdout comparison
could not see it.

One rule closes all three widths: check refuses an `if` whose
argument count is not three, before inference runs — "an `if` takes a
condition and two branches, got N argument(s)" — with an early return,
so inference's indexing becomes safe by contract rather than by
defense. Pinned as tests/golden/errors/an_if_owns_its_arity.kso (all
three widths, direct and imported forms), watched red as a panic and
a silent pass before the rule, green after. Golden suite 10/10;
diagnostic coverage reports the new message pinned, nothing newly
unpinned.

The method note worth keeping: three generators, three outcomes.
Byte mutation proved robustness; the seeded utf-8 sampler proved the
validator; only recombination of VALID programs found the shape no
human had written — a truncated `if` is not a typo anyone makes, it
is two programs' halves meeting. The corpus is the mutation fuel, so
every golden added makes the fuzzer smarter.
## 2026-08-20 — gavel 1b enforced: only a type's owner constructs one

Ruled by Clay ("yes 251 is mine"), so this is 1b implemented rather than a
second decision. The doctrine had been recorded as stated-but-unenforced since
provenance made it unnecessary for SOUNDNESS; what it buys now is honesty — a
value wearing a type's name was built by its owner.

The check sits in check.rs beside `check_call_arities`. The first attempt sat
beside `foreign_destructures` in lib.rs and silently did nothing, which is the
useful part: `program.types` is EMPTY there, because that pass runs per-file
before dependencies merge. The destructure check works in that position only
because it reads the qualified name the AUTHOR WROTE in a pattern rather than
consulting type knowledge. A construction site has no such tell.

**Every violation in the tree was one shape.** Four types, all protocol types a
library expects its CALLER to build: `http/turn`, `http/done`, `shape/filled`,
`list/cursor`. So the migration was not the mechanical one predicted — it
changes a public API. http gains `reply` and `stop`.

The naming went through a correction worth keeping. `keep`/`stop` was proposed
and cleared, and reading the call sites killed it: `turn` WRAPS `done`, so
`keep(response, stop(v))` reads self-contradictory. `reply`/`stop` respects the
nesting. A naming that survives the type declarations can still fail at the
call site, which is where it should have been tested first.

**A hazard the migration nearly shipped.**
tests/golden/micro/field_name_shared_with_an_import built a cursor directly
only to get a foreign type with an `at` field; its subject is the getter-group
collision. Routed through `list/cycle` it printed "1 1" — the same value the
LOCAL type held — so the fixture could no longer tell the two reads apart, and
it still passed. The local moved to `here 9` so it prints "9 1". A migration
that quietly weakens a fixture does not announce itself.

**An advisory the gavel subsumes.** The laundered fixture forges a foreign
reason so its own err can be rescued under a borrowed name. That no longer
compiles, so the advisory cannot fire. Its test pins the refusal instead of
being deleted; the advisory stays live where the reason is genuinely foreign.


## 2026-08-20 (later) — the fuzzer's second find, and a list line falsified

The 30,000-child crossover campaign with both output streams compared
found one more crash: src/parser.rs:1191, "branches are non-empty" —
a block-if constant whose else opens a block and holds nothing
reaches the branch-shape check with zero lines, and building the
diagnostic's SPAN panicked before the diagnostic could speak. Reduced
to four lines. The fix gives the empty branch its own refusal ("a
branch needs a body: the expression it answers with") before the span
assumption. The shape is reachable only through kanso check on a
library file — play and run both preempt it with earlier rules — so
its red-green pin lives as a rust test beside the corpus, and the
unpinned list carries the mechanism.

Writing that fixture falsified a documented impossibility: the
unpinned list held the inline-constant message as unreachable ("the
lexer folds an indented line into its head unless that head opens
one") — but a constant head that OPENS a block (`two = if`) keeps its
branch lines, and the message fires, including under the corpus's own
staged-run mode. It is pinned now and its list line deleted; the list
shrank from five residents to four. Filed alongside, not ruled here:
whether refusing a block-if constant in a library is the intended
semantics or an accident wearing a confusing message — the lexer
licenses the shape, the checker refuses it, and the words describe a
different construct. Campaign totals to date: 50,800 mutants and
children, two compiler crashes found, both fixed same-day, zero
engine divergences among 1,229 executed survivors.

## 2026-08-20 (wave three) — the wider gene pool pays twice more

The third crossover wave — 40,000 children spliced from all 752 kso
files in the tree, the runtime goldens, book samples and hako
fixtures joining the pool for the first time — ran against the
compiler hardened by its own two previous findings. Raw yield: 16
check panics and 59 divergences. Triage cut it honestly: all sixteen
panics were ONE site (the statement grouper's set-arm was an
unreachable! — a field write in a play constant's block parses fine
and arrived there; the fix is the same refusal a fn body already
gives, pinned as a rust test since staged runs resolve names first
and hide the path). Of the 59 divergences, 44 were entropy — the
replays ran unseeded and children that inherited math/random diverge
run to run, a harness lesson now baked into the filter (seed it, and
demand self-stability before blaming an engine) — 4 were the licensed
int64 gap doing its documented job, 9 were the known cross-stream
buffering interleave, and TWO were real: the engines refused a
non-description in a group with different words, native's naming
`&` — an operator the surface retired. Native now says what the
oracle and the browser say: a group joins descriptions.

Campaign ledger to date: 90,800 generated programs, three compiler
crashes found and fixed same-day, one stale diagnostic unified, zero
true engine divergences among 2,297 executed survivors. The
fuzzer's finds have all been in the seams between features — an
empty else, a set outside build, an if the wrong width — the shapes
no fixture author writes because no intention produces them.

## 2026-08-20 (wave four) — the seams hold

The closing wave inverted the strategy: instead of splicing whole
fixtures, 50,000 children were assembled FROM the seam fragments
where all three crashes had lived — block headers, walls, sets,
orphan elses, mixed indents — deliberately mis-nested, replays seeded
from the start. Zero panics, zero signals, zero divergences; the
generator was hostile enough that only three children compiled at
all, and those three ran identically on both engines.

That closes the campaign at 140,800 generated programs across four
generator families — byte mutation, seeded sampling, corpus
splicing, seam templating — plus 384 million utf-8 cases. Yield:
three compiler crashes found and fixed same-day, one hidden
differential violation closed, one stale diagnostic unified, four
harness lessons banked (compare both streams; seed the replays;
demand self-stability; check mergeable before theorizing). The
block machinery that produced every crash now withstands a directed
assault on exactly itself, which is what hardened means when it is
earned rather than asserted.

## 2026-08-20 — permanent buffers are counted where they are allocated and freed

`k_buf_perm` mallocs a buffer for a list that outlives its beat. It counted
`allocs` and `alloc_bytes` and never `bytes_malloc`, while the permreg flush
counted `bytes_freed` for the same buffers. The escape vein pinned the
contradiction plainly — `bytes_malloc=0` against `bytes_freed=3000`, three
thousand frees of buffers the counters said were never allocated.

The grow path carried the larger share. When a list outgrows a perm buffer the
old one is released outright, adjusting `perm_live` and freeing the storage
without counting the free at all. With both counted, escape reads 12,000
against 12,000: nine thousand frees had been invisible, three times the
undercount the defect was filed with.

A third one sits beside them and no vein can witness it. `perm_live`'s decrement
sat inside the `k_stats_on` guard where its increment sat outside, so with
counters off the figure only ever grew and `perm_peak` reported everything a
program had allocated rather than its high-water mark. Every vein runs with
counters on, which is why nothing caught it and why it is written down here.

**Every counter that rose, and why it rose.** These are corrections, not more
work: the allocations always happened and the programs are byte-for-byte
unchanged. `basket_bytes_malloc` 16 to 30. `escape` 0 to 12,000. In the lazy
tier, `a_pushed_call_keeps_the_sweep_bytes_malloc` 0 to 2,400,
`an_escaped_list_gives_its_buffer_back_bytes_malloc` 0 to 400, and one small
buffer count each for `early_exit_bytes_malloc`, `fold_push_shape_bytes_malloc`,
`fused_map_shape_bytes_malloc`, `fused_reducer_bytes_malloc`,
`fused_select_shape_bytes_malloc`, `fused_tally_bytes_malloc`,
`piped_reducer_bytes_malloc`, `skip_shape_bytes_malloc`,
`sort_shape_bytes_malloc`, `take_shape_bytes_malloc` and
`tally_shape_bytes_malloc`. Each of those reads one above its `bytes_freed`,
because the last buffer a program holds outlives its exit and no flush runs
there.

Decode, encode, oneshot, scan and wide are byte-identical. Welfare reads 84.56
against a floor of 84.56 — it reads neither counter, so an accounting fix buys
no points and costs none. `instructions` and `machine_code` fail on this host
before the change as well as after, checked with the diff stashed; they are the
host-divergent pair.

The direction gate is right to ask about a rising counter and cannot tell a
correction from a regression, which is what this entry is for.

## 2026-08-20 — the flagship's own claim under test: kq versus jq

The fuzz campaign's closing act opened a product-level lane: kq's
README claims byte-identity with jq -S, and a claim that strong is a
differential harness waiting to be written. Three thousand adversarial
JSON documents (deep nesting, unicode, subnormals, huge exponents,
ensure_ascii escapes) ran through both binaries. Yield: two findings.

The big one is fixed in this commit and it was std/json's, not kq's:
the decoder handed each \u escape to from_code alone, so a VALID
surrogate pair — the encoding every ensure_ascii writer emits for
every character past the basic plane — was refused as "not a unicode
scalar value", a bare string err leaked from the character machinery
with no position. 1,579 of 3,000 documents died on it. str_unicode
now joins pairs; either half alone is a typed json/parse_failure with
a position and honest words. Pinned three ways: a json_test round
trip, a micro golden both engines answer byte-identically, and a
runtime golden for the lone half. The emitted vein moved and is
regenerated with its sentence: surrogate support costs six defines,
103 calls, 681 lines of decoder IR — correctness buying code size,
stated not hidden. Allocation counters flat; welfare holds at its
floor, 84.56.

The second finding stays open in kq's lane: float exponent formatting
diverges from jq (kanso renders 1e-07 where jq -S prints 1E-7 —
case and zero-padding both), so the byte-identity claim is false in
exponent form. Whether kq grows a jq-shaped formatter or the README
narrows its claim is a product decision for the kq lane; 492 of
3,000 documents witness it, corpus kept. kq's vendored json copies
also need the surrogate fix at the next sibling sync — until then kq
still refuses escaped emoji.

## 2026-08-20 — the sweep learns direction, and pays for the surrogate fix

The surrogate fix could not hold the welfare floor as first written. Its
call chain — six functions from `str_unicode` down to `str_low` — moved
the front end's fixpoint on lib/json from 31 rounds to 39, and rounds
are a welfare term: the score fell 0.61 below the floor, which the
2026-08-03 ruling says no reason may buy. The round trace showed where
the cost lived: rounds six through eleven each moved one or two
functions, information creeping through the string machinery's cycle
one hop per round, because a round visits functions in declaration
order and only reads what earlier visits in the same round already
wrote.

So the sweep learned an order. Functions now visit callee-first — a
post-order walk of the call graph — and alternate direction on
following rounds, callers-first on the even sweeps. Returns flow
callee-to-caller and params flow caller-to-callee, so a single static
order can serve only one of the two flows; alternating serves both,
and the first sweep goes callee-first so leaf returns land before any
caller asks. Measured on lib/json, with the surrogate fix in place:
declaration order 35 rounds and 27,874 visits; callee-first alone 45
and 22,179; caller-first alone 32 and 28,322; callee-first-then-
alternate 28 and 23,384. The winner beats declaration order on both
axes and beats the pre-fix baseline too — 31 rounds and 26,474 visits
— so the fix rides in with the front end doing less work than before
it existed. The five compile-golden samples split as a trade the sum
accepts: the module sample's visits fell 4,448 to 3,031, guards fell a
little, and the tiny recursion sample paid one round. Emitted text is
byte-identical across every sample, which is the monotone fixpoint's
order-independence made visible.

The fix itself also slimmed: `str_code` and `str_scalar` folded into
`str_unicode`'s branch, five functions where the first draft had six.
One draft went further and died in the goldens: merging the `\u` check
and the low-half range check into one condition forces `str_hex4` past
the string's end on a lone trailing half, because `and` evaluates its
arms in parallel — the staging across functions is load-bearing, and
the lone-surrogate golden is what caught it. Welfare lands at 84.81,
0.25 above the old floor, and the floor is set there. `compile_peak_bytes` on
lib/json rose 819,217 to 872,591 — the order change shifts when
the analyser's tables grow, and the term's 0.17-point cost is inside
the rise the visits paid for. The machine-code vein prices the fix at
1,296 bytes of .text on each decode binary — jsonbench 78,594 to
79,890, oneshot 95,826 to 97,122, the other five flat — which is the
surrogate machinery the linker kept.

## 2026-08-20 — the cell keeps what it holds

The trend gate printed its UNPRICED header and swallowed the one
counter name it existed to print. The interpreter printed the name.
That is the differential law's tripwire, and pulling on it found
memory corruption: adding a probe binding to the same function turned
the silent empty string into a SIGSEGV in `k_b_at`, dereferencing a
small integer where a list pointer belonged, on origin/main's compiler
as much as any branch — a long-standing native bug that had been
rendering as quietly wrong output.

The reduction went from the 400-line gate to a 190-line program with
no inputs at all, and the load-bearing pieces told the story before
the debugger did: the crash needed a never-forced `list/reject`
passed along and dropped, records whose lazy interpolated fields were
never printed, and enough allocation in one scope to cross the cohort
threshold. Every road pointed at unforced thunks crossing a beat
close. The mechanism, in runtime.c: `k_is_heap` answers false for
`K_THUNK`, so the evacuation that runs when a cohort closes — sized
by `k_copy_size`, walked by `k_deep_copy`, gated by
`k_slots_survive` — could not see inside a thunk. The malloc'd cell
itself survives every rewind, which is exactly what hid it: the cell
outlives the close while its captured args and forced result still
point into the rewound arena, and the next force or walk reads reused
memory. Whether that surfaces as a wrong empty string or a crash is
only a question of who reused the block first.

The fix teaches all four sites the same sentence: a thunk's cell is
never copied — every holder resolves through the same thunk, so
identity is the contract — and its slots evacuate like any other
survivor's interior, written back in place. `k_copy_size` and
`k_deep_copy` walk args and result with a cycle guard (knots tie
back), the two rewind fast paths stop treating a thunk-valued result
as nothing to save, and `k_slots_survive` declines to share a node
holding a thunk slot, which routes it to the walk that can answer.
Pinned as `a_thunk_capture_survives_the_cohort`, a micro golden
distilled from the gate: red with a stack-banner crash on the old
runtime, byte-identical across engines on this one. The lazy tier's
mem corpus never caught it because its fixtures force what they
build; the fixture that survives a close unforced is the one the
corpus was missing.

The veins price it, and two of them said something. Text: every row
rises by exactly 544 bytes, the walk itself. Instructions: deepbench
falls 26.7M (-3.2%) and widebench 737K (-0.86%) — the two rows with
meaningful evacuation volume (8,723 and 272 evac_allocs; the peer
session measured the correlation) — so the optimiser re-rolled the
edited walkers in their favour, while encodebench pays 2.23M (+0.023%)
for the per-slot thunk check across billions of slot checks and
nineteen evacuations. Welfare lands at 84.84, 0.03 above the floor,
and the floor is set there. The corpus gap stands with a number on
it now: thunk_allocs is zero on all seven benchmarks, so the path
this fix repairs is priced at zero by construction — the same shape
as the 95x and 998x memory fixes that scored nothing — and the
memory-frontier lane owes a benchmark that holds an unforced thunk
across a cohort close.

## 2026-08-20 — the corpus learns the pending-cell shape

Every benchmark read zero on all six thunk counters — not because
nobody had tried a lazy benchmark, but because the strictness analyser
is good: a record field read unconditionally anywhere downstream is
proven demanded, the thunk is erased, and two honest drafts of this
benchmark read zero and looked like broken fixtures. What defeats the
analyser is dispatch on a runtime value — a binding passed to a
two-clause consumer that stores it in one clause and drops it in the
other — because demand through a runtime value is what no analysis
can prove. That recipe, plus an io bind per iteration so there are
beats to cross at all, is pendbench: 200 records built under a bind
loop, each keeping a pending field, half forced by a value-dependent
walk at the end, 200 thunks allocated and 200 alive at exit.

Two ratchet rows land with it, split by what each pin can honestly
hold. The pend counters pin the LAZY TIER'S PRESENCE: a mutation that
makes the demand analyser claim everything is demanded zeroes all six
thunk counters and turns the gate red — before this row that erasure
would have moved nothing anywhere. The evacuation walk itself is
pinned by the micro golden, not by these counters: isolating the four
#972 sites one at a time against the fixture showed only the two
walker cases load-bearing (the rewind fast paths and the sharing
refusal never fire on it), and pendbench's counters are byte-identical
with and without the whole fix — its accumulator rides the kept-carry
path, which never rewinds under a live capture. So the second row
mutates the k_copy_size thunk walk away and proves the GOLDEN goes
red. A pin that claims more than it holds is the green that stops
anybody looking; these two claim exactly what they were watched to
hold. The machine-code and instructions veins gain a pendbench row,
stamped from the runner in this PR, and the trend gate enrolls the
pend counters with the thunk family in its direction tables.


## 2026-08-20 — the shorthands JSON already has, paid for by dead code

A second differential wave (2,000 fresh seeded documents, the durable
generator at ~/dev/jqfuzz/generate.py) found zero refusals — the
surrogate class is dead — and exactly two byte-divergence families
against jq -S. One is the float-formatting policy already filed as the
kq lane's product question. The other was new and free of any policy
tension: the encoder wrote \u0008 and \u000c where jq, python and
every other serializer write \b and \f. Two arms in esc_byte's
jump table close it, pinned by test_encode_shorthands, red first.

The arms cost 0.03 of welfare and the floor refuses any fall, which
is how the deletion was found: hex4, hex_char and hex_digits had no
caller anywhere in the library — the string-building duplicate of the
byte-side hex machinery — and the two constants among them were frozen
CAFs every decode paid for. Deleting the trio moves perm_allocs from
6 to 4 in the decode counters, takes front-end visits on lib/json to
23,224 — below where they stood before the feature — and lands
welfare at 84.85, floor set there. The suite panel reads 20 tests.

## 2026-08-20 — GAVELED: two definitions with one unfolding are one value

Clay ruled the equality collision: cycles compare by bisimulation no
matter which door built them, and the equality value itself is lazy —
his words were that everything cascades like the collapse of a wave
function, equality is lazy, the next thing that uses it is lazy, and
so forth until IO makes everything collapse. This supersedes the
2026-08-18 knot-equality refusal: a knot's cycle now joins the same
assumed-equal walk that build-block cycles have had since the cyclic-==
work, because a reader holding two rings cannot tell which construct
tied them, and == should not be able to either. The one refusal that
survives is a cell demanded mid-construction — the blackhole — which
is not yet a value at all.

The implementation is two sites and a subtraction. In the native
engine the thunk-pair revisit returns equal instead of dying, the
same assumption the record case makes a few lines below. In the
interpreter — which the wasm engine routes through — the revisit
returns true and the whole Option plumbing that carried "refused"
from the walk to the caller deletes, along with the caller's error.
`equality_refuses_a_value_that_names_itself` retires from the runtime
corpus and `a_knot_compares_by_its_unfolding` replaces it in the
micro corpus, byte-identical across the three engines: `[x]`-x equals
`[y]`-y, the one-ring equals the two-ring, and a ring differing in a
field does not.

The cascade half of the ruling found a real gap the moment it was
pinned. The demand analyser only defers a binding whose right side is
"expensive", and a bare comparison was not, so `verdict = x == y`
compiled strict and the walk ran with nobody asking — the mem fixture
written to pin zero evaluations read two on its first run, which is
the failing spec doing its job. Comparisons now count as expensive on
their operator: their cost is their operands' shape, which no syntax
shows — two knots bisimulate, two lazy sequences may never end — and
the deferral gate still requires conditionally-demanded use, so the
ordinary scrutinized comparison compiles as strict as ever. Nothing
else moved: every counter gate is flat, the compile goldens are
byte-identical, welfare holds at its floor, and the new pin
`an_unasked_equality_stays_a_cell` reads one cell built, zero forces,
zero evaluations.

One finding filed rather than fixed: pinning the knot version of that
fixture caught the interpreter and the native engine disagreeing on
whether an undemanded knot CAF counts as a thunk allocation (native
says yes at startup, the oracle never creates the cell). Semantic
counters — forces and evals — agree at zero. The fixture pins the
agreeing pair; the counting question is open.

The browser differential then earned its keep before the PR merged:
its own harness — a kanso program — died with "an if condition is
true or false, got true", which is a cell rendered by the error
message. The tail-position `if` lowering and the guard lowering both
tested the raw condition where the ordinary path has always forced
first; the gap predates this change, and lazy comparisons are simply
the first cells to reach it, riding a pass-through arm into a
consumer's tail `if`. Both sites force now — `maybe_force` still
emits nothing where the set proves no thunk — and the shape is pinned
as `a_deferred_condition_forces_at_the_tail`, red before the fix,
byte-identical across the engines after.

## 2026-08-21 — the herd is not a type

Clay declined fixed-length list types, arguing it himself: positions
that mean something are pets and pets get names — records; a herd's
count is a fact about the herd, not the type; integer-position
insisters want an integer-keyed map. Recorded in pending-gavels
beside the no-positional-products clause it composes with, with the
numerics counter-case noted so nobody reopens it sideways. Opened in
the same dialog and NOT yet ruled: whether `[]T` keeps the name
"list" — the fork and its stakes are in pending-gavels, and nothing
list-naming-adjacent should be built until the word comes. The word
came within the hour, in the parallel session: "list seems like the
name. you've convinced me." Ruled, recorded, never re-asked — the
book owes one sentence saying contiguous, constant-time index.
## 2026-08-21 — what render's cycle guard costs, measured

Task #258 carries Clay's condition on the in-walk cycle guard — "if it's
more efficient" — and until now that was an assertion nobody had priced.
Render is the one walk in the language that already knows when it has met
a node before, so it is also the only place the price can be read off
directly.

The instrument holds total work constant and varies only shape: a value
nested `d` deep, rendered `r` times, with `d * r` fixed at two million
nodes and the rendered text the same total length in every row. Anything
that moves is a function of depth alone. The guard was then isolated the
way the log's own rule asks — one element, one state: the four scan loops
in `k_render_path` were made to iterate zero times, leaving the push and
the pop in place, so the number below is the scan and not the
bookkeeping. The disabled build proves it is live by failing to detect a
knot at all, running out of stack where it used to answer `<cycle>`.

    depth   guarded   scan off   the guard   share of render
       10     0.29       0.28        ~0.01        at noise
       30     0.29       0.29         0.00        at noise
      100     0.33       0.29         0.04            12%
      250     0.39       0.29         0.10            26%
      500     0.48       0.31         0.17            35%
     1000     0.64       0.34         0.30            47%
     2000     0.94       0.39         0.55            59%

Seconds, best of three, this box, two million nodes per row. The guard
column doubles as depth doubles — 0.10, 0.17, 0.30, 0.55 — which is the
linear scan showing its cost per node as O(depth). The first two rows sit
inside the layout noise the log already records at about three per cent
and are reported as noise rather than as zero.

Two conclusions follow, and they point different ways.

For JSON the condition is met with room to spare. Documents nest tens
deep, not thousands, and at that shape the guard is unmeasurable against
a no-op build. Nothing about lib/json's use of a guarded walk needs to be
argued on cost.

As a general capability it is not met. A pre-pass that checks for cycles
before walking costs one extra traversal — the scan-off column — where
the in-walk linear scan costs the guard column, and the two cross at
roughly depth 1,400. Past that the two-pass shape a general capability
was meant to beat is simply faster. What that asks for is a different
structure under the same shape: `k_copy_seen`'s generation-stamped
pointer map already probes in constant time and is proven in the copy
pass, and a
path guard built on a set with real removal rather than a linear array
is O(1) per node and beats the pre-pass at every depth. That is what the
capability should be built on if it is built.

Nothing shipped from this. It is a measurement against a condition, and
it says the condition holds for the case that motivated the ruling and
fails for the case the ruling widened to.

## 2026-08-23 — a spec that could not run measured the host

`cargo test` on a fresh clone died in `bind_chain_depth`, and the words it
died with were `kanso runs: Os { code: 2, kind: NotFound }`. The spec asks
`/usr/bin/time` for a child's peak resident size; a container without that
binary reports the missing stopwatch as the interpreter failing to start. CI
has never seen it, because both the ubuntu and the macos images ship time(1),
so the only people who meet it are the ones running the suite somewhere new.

The number comes from the kernel now. `wait4` hands back a child's rusage at
the same call that reaps it, `ru_maxrss` is the high-water mark, and the one
host difference left is the unit — kilobytes on linux, bytes on the BSDs. libc
joins the dev-dependencies for that and nothing else; it was already in the
lock under rustyline.

Repairing the instrument turned up the reason to care. The spec's claim is
that a chain of effects ten times longer costs no more memory, and on this
host it reads 5.3 MB at a thousand links and 5.5 MB at ten thousand — which is
also what it would read if the measurement were dead. A spec whose pass and
whose failure look alike is worth what it costs to run. So the same instrument
now reads a shape that does nest, over the same tenfold ratio and against the
same threshold, and answers in the other direction:

    links        chain      nesting
      400        5.3 MB       6.9 MB
    4,000        5.3 MB      21.5 MB

Release build, best of three, this box. The nesting column is 4.1 KB a frame
and straight-line in depth. In a debug build the frames are fatter and the
same two rows read 19.3 MB and 116 MB, so the ratio the spec asserts holds in
both profiles while the constant does not.

Getting there took two false starts, both instructive. The first control was
a non-tail recursion adding a literal — `down (n - 1) + 1` — and it read flat
at a million deep, because the accumulator rewrite had already turned it into
a loop. The second was a string doubled twenty-two times, which read 11 MB to
341 MB and proved the instrument live but measured a live set rather than a
stack. The control that survives is a recursion whose leftover work is a call
to another function, which no rewrite reaches.

## 2026-08-23 — the accumulator rewrite reads an operand it can prove

The also-open list carried TRMC v2 — license the leftover operand by inferred
set rather than by literal, so `n * fact (n - 1)` loops — with a cost attached:
it needs `Inference` threaded out of `check::check`, or the compile golden pays
for a second inference run it does not use. Neither was necessary. The proof is
already in the shape the pass generates.

The rewrite's wrapper ascribes every counter position `int`, which is what
keeps non-integer arguments out of the loop entirely; they fall through to the
original arms as they always did. Add one requirement — every recursive call
hands each counter position arithmetic over counters — and the integer property
carries down every level by induction. An operand built from counters and
literals is then an integer at every depth, and it is pure: a name, a literal
and `+`/`-`/`*` reach no call and no effect, so computing it before the descent
rather than after moves nothing a program can observe. The pass reads all of
that off the syntax it already walks. `src/trmc.rs` grew fifty lines and gained
no dependency.

The license is asked for only when an operand is not a literal, so no group the
narrow rule already rewrote can stop being rewritten. What it now reaches:
`n * fact (n - 1)`, `n * n + r (n - 1)`, `n * 2 + w (n - 1)`. What it declines,
and why: a float operand, because reassociating floating-point addition changes
the answer; a call in the leftover work or in a counter position, because the
pass reads one group and cannot see through a call; double recursion like fib,
because there is no single descent to thread.

MEASURED, old binary against new over a battery of nine shapes on both engines:
every answer byte-identical where both engines answered — `fib`, a non-integer
base arm, a float operand, a call in a counter position, the two literal shapes
the narrow rule already had. The change is visible only past the depth where
the old code ran out of frames:

    weigh 100000        native answered, interpreter refused  ->  both answer
    n * n + r (n-1)     native answered, interpreter refused  ->  both answer
    n + k (dec n)       native answered, interpreter refused  ->  unchanged

The interpreter refuses unlicensed recursion at ten thousand frames and native
takes whatever the operating system gave it, so that middle band is a real
disagreement between the oracle and the engine that ships, on a shape anybody
would write. `an_operand_that_varies_still_loops` pins it in the micro corpus:
`weigh 100000` answers 5000050000 on all three engines in under a tenth of a
second.

Every counter gate is flat — decode, encode, escape, basket, oneshot, pend,
scan, wide, emitted code, machine code — both compile goldens are byte-identical
and welfare holds at its floor of 84.85. The rewrite fires on shapes no
benchmark contains.

Two fixtures had to move, and the reason is worth stating because it is the
failure mode this rule exists to catch. `tests/golden/runtime/deep_recursion`
and the book's appendix A sample both pinned the stack-exhaustion diagnostic
with `n + weigh (n - 1)` at a million deep — a shape the rewrite now turns into
a loop, which would have left that diagnostic with no fixture and nobody the
wiser. Both now use a float operand, which the rewrite cannot reach for a
reason a reader can check, and both carry a comment saying so. The appendix
paragraph that taught the old boundary teaches the new one.

OPEN, surfaced by this change rather than caused by it: `fact 20000` now
answers on the interpreter, and native reports `integer overflow (int64 native
build; spec is arbitrary precision)`. The engines have always differed there;
until now the interpreter ran out of frames first and the disagreement could
not be reached.

## 2026-08-23 — the last python leaves, and the one that stays says why

`scripts/stale_a_panel.py` was the repo's only `.py` file and the book gate ran
it on every build: it plants a marker word in two panels of ch04 so
`book_panels --write` can be watched putting them back. It is kanso now, and
the port was checked the way a port should be — byte-identical output against
the python on a real chapter, the complaint path exercised on a chapter that
carries neither panel title, and the gate watched going red with the planting
removed before it was watched going green with it back.

Writing it found its own bug, which is the reason the missing-title check runs
before any planting rather than inside the walk. An arm that answered with an
effect chain handed the next fold step an effect where a string belonged, and
the run died with `split takes two strings` — the arm's own complaint never
printed. A gate program is a program, and this is the class of mistake that
only turns up in one.

STATUS.md claimed the repo has no python in it. Six mutation scripts under
`scripts/ratchet/mutations` still carry a `python3` heredoc, and they are
staying: each damages a compiler source file before that worktree builds, and
the `target/` the worktree links to is shared across rows, so the binary in it
is whatever the previous row's mutated source produced. A tool that breaks the
compiler cannot be written in the language that compiler compiles. Written down
in STATUS.md so it is not re-opened as an oversight.

Two smaller things fell out. `tests/golden/wasm_gaps.txt` carried four
paragraphs describing knot-rendering divergences whose entries are gone because
the engines agree now — the file's own rule calls a gap that closed and stayed
written down a lie about the engine, and a paragraph is the same lie with a
longer fuse. And the wasm spec refuses to run against an artifact older than
the compiler's sources, which caught `docs/kanso.wasm` after a formatting-only
edit: panic messages carry line numbers, so reformatting moves the bytes. Two
rebuilds from one source hash identically, so the artifact is reproducible and
the staleness guard is reading real drift rather than build noise.

## 2026-08-23 — the gate that watches the page could not see the history

The page-drift check counts log entries written since docs/compiler.html last
changed and fails past a budget of three. It reported `0/3` on every pull
request while the page fell twenty-two entries behind, and the reason is one
line in the workflow, two steps above it:

    git fetch origin main --depth 1

The job checks out with `fetch-depth: 0`, so the history was there; that fetch
truncates it for every step after it. In a truncated history the shallow
boundary commit looks like the one that created every file, so
`git log -1 -- docs/compiler.html` answers with the tip, and the diff from the
tip to HEAD contains no log entries, so the count is zero.

MEASURED, on main's tip in a fresh clone of this repository:

    full history            the gate fails, 22 entries ahead of the page
    after `--depth 1`       `page drift 0/3`, exit 0

Two fixes, because either alone leaves the class open. The workflow's two
fetches drop `--depth 1` — nothing in that job wanted a shallow one, and the
checkout had already fetched everything. And the gate now asks
`git rev-parse --is-shallow-repository` first and refuses to answer on a
truncated clone, watched refusing before it was trusted. A gate that cannot see
must not report success, which is the same rule as never trusting a spec you
have not seen fail.

This is what the ratchet exists to catch and could not: its rows prove a gate's
own script goes red on a defect in the tree, and this defect was in the
workflow around the script. The gate's script was fine. The clone it read was
not.

Found while checking why PR #985 has sat red for two days. That one is a
different story and a real one: with its goldens regenerated — the decoder's
emitted lines 11,593 to 11,603, the front end's rounds on lib/json 28 to 30,
visits 23,224 to 23,345, peak 871,649 to 878,422 bytes — welfare reads 84.69
against a floor of 84.85. It is not a stale golden. By the weights as written
that change costs the project 0.16 points, and the branch owes either the
reason it is worth it or the compile cost back.

## 2026-08-23 — a sweep of the refusals, one divergence re-found and two messages fixed

The `stale_a_panel` port died with `split takes two strings`, which named
neither what it got nor where. That message is one of about twenty runtime
refusals written separately for each engine, and the coverage ratchet does not
reach them: `scripts/diagnostic_coverage` scans `Diagnostic::new(` literals,
which are the check-time diagnostics, and a `RuntimeError` is not one.

So the twelve most reachable were driven on both engines with a wrong argument
hidden behind a call, since a literal is refused before the program runs.
Eleven agree word for word — split, chars, bytes, join, slice, push, length,
char_code, from_code, utf8, and to_int on an int, which answers rather than
refusing.

MEASURED, the twelfth:

    text/to_float ["a"]
    native:  to_float takes a string, bytes, or number, not ["a"]
    oracle:  error[endpoint]: unhandled err reached the entry:
             "bytes are not a number"

Two engines, two answers, and not only the wording: native refuses at runtime
where the oracle answers an err VALUE, which is a different channel. The cause
is structural and already known — the interpreter has no distinct bytes value,
so a list goes through `bytes_to_str`, and native's `K_BYTES` tag refuses
anything that was never bytes. It was measured on 2026-08-02 and recorded, and
the record went into the archive where nothing on a live list mentioned it
again. It is in pending-gavels now, under also-open, with both ways out and
what each costs.

The sweep also showed why nothing caught it. `scripts/diagnostic_differential`
already drives every std function with a wrong argument on both engines, and
its one wrong value is a record — chosen because a record is wrong for
everything and cannot be a literal. A record reaches the same refusal on both
engines. Only a list reaches the one that differs, and a list is a legitimate
argument to enough of the surface that probing with one needs care: `list/cycle`
would not return. Widening the probe waits on the ruling that makes to_float's
answer knowable.

SHIPPED from the sweep, because it needed no ruling: `to_int` and `to_float`
both accept the bytes a file read hands back, and neither said so — "to_int
takes a string", "to_float takes a string or int". Both now name every kind
they take and what arrived instead, the way `length` has all along, in both
engines and byte-identical. Two fixtures pin them, watched red first. The
arguments are a float and a `none`, which render without a module name, so the
direct run and the run through an import say the same words and one golden
covers both.

The machine-code gate priced it: rendering the refused value costs 48 bytes of
`.text` in four of the eight benchmarks and nothing in the other four, where the
linker had already dropped the path. `bench/text_golden.txt` is regenerated on
that reading. Every allocation counter is flat, both compile goldens are
byte-identical, and welfare holds at 84.85.

## 2026-08-23 — the bytes fork is six functions, and four of them answer

The entry above records `text/to_float ["a"]` diverging and cites the archive's
prediction that a ruling would settle `append`, `find2`, `find2_below` and
`utf8` at the same time. Those four are measured now, driven with a list on
both engines, and the prediction was right and understated.

    text/append ["a"] "x"           native refuses    oracle: ["a" 120]
    text/append [65 66] "x"         native refuses    oracle: [65 66 120]
    text/find2 [65 66] 1 65 66      native refuses    oracle: 1
    text/find2_below [65 66] ...    native refuses    oracle: 1
    text/utf8 ["a"]                 native: an err    oracle: a refusal
    text/to_float ["a"]             native refuses    oracle: an err

Four of the six are worse than a wording difference. The oracle ANSWERS where
native refuses, so a program written against the oracle runs and the same
program compiled dies — which is the differential law's hardest case, and it
points the wrong way: the oracle is meant to be the engine that can express
whatever native runs, and here it is the one that accepts more.

`text/utf8 [65 66]` agrees, answering `AB` on both, because a list of small
ints is bytes to each of them. What the two disagree about is everything a
list can be that bytes cannot.

Nothing is fixed here, and nothing can be until the fork is ruled: either a
bare list of small ints IS bytes, and native widens, or the interpreter gains a
real bytes value. The table is in pending-gavels with both costs, and it is on
the task list as Clay's.

## 2026-08-23 — the accumulator rewrite gets a differential of its own

Widening TRMC's license this morning meant trusting a reassociation nothing was
checking. The pass rewrites `n * fact (n - 1)` into a tail-calling helper
threading an accumulator, and a reassociation bug does not fail — it answers a
number. No counter can see it: the rewrite changes the shape a recursion runs
in and not one allocation.

So the shapes are written twice. `f` is the plain form the rewrite reaches;
`g` is the same arithmetic with the leftover operand passed through a function,
which the license refuses to read through, so that group descends the way it
always did. Twenty shapes — two operators over five operands over two base
values — at four depths each, and the two forms must answer identically on both
engines. It runs in 0.6 seconds, which is why it sits with the other
differentials rather than in a nightly.

The last line reads the instrument rather than the compiler: a sum twenty
thousand deep, taken from the INTERPRETER's output, because that engine refuses
unlicensed recursion at ten thousand frames and native would answer it either
way on the operating system's stack. Delete the pass and that line dies.

Watched red twice before it was trusted. With the identity for `*` changed from
1 to 2, every product shape answers double and the gate names the lines. With
the pass returning before it looks at anything, the deep sum stops answering on
the interpreter and the gate says the comparison proves nothing.

The depths are per operator, and the reason is native's own limit rather than
the rewrite's: an int is arbitrary precision in the spec and an int64 in a
native build, so a product thirty-seven deep overflows there while the
interpreter answers. Every value the gate asks for stays inside int64 on both
engines, which is what lets one output be compared against the other. The first
run of the gate found that boundary by falling over it.

`accumulator_rewrite_deleted` is the row that proves it, and it uses awk rather
than the python heredoc its neighbours use — a line inserted after a matched
line is one awk expression, and the six python mutations stay python for the
bootstrapping reason recorded in STATUS.md rather than for the editing.

## 2026-08-23 — the differential guards the license, not just the rewrite

The gate from the entry above compares shapes the license accepts. Three more
were added that it must keep refusing: a float operand over an integer base,
`0.1`, `n * 0.1` and `n / 2`. Today neither copy is rewritten and they agree
trivially, which is the point — the guard is against a future widening.

Watched biting. With `int_arithmetic` widened by one line to accept a float
literal, the gate goes red and names the line:

    1.5000000000000002 1.5

Five terms of `n * 0.1` summed one way and the other. That is the license's
entire argument in one line of output, and it is now checked rather than
asserted.

The first attempt at these guards proved nothing, and the reason is worth
keeping. Their base arms answered `0.0`, and `classify` requires an integer
literal base before it looks at anything else — so the groups were refused for
their base rather than their operand, and widening the operand rule left them
refused. A guard that cannot fail is the failure this repo already has a rule
about; the bases are integers now, and the widening was run again to watch the
gate go red for the reason intended.

## 2026-08-23 — the os package, built

Gavel of 2026-08-17: the stdlib apes Go, `os` takes the filesystem, the
environment, the arguments and the processes, `io` keeps the abstract read and
write surface, `MkdirAll` goes to `os`, and any boundary case Go does not
answer goes to the language committee and never back to Clay. It has sat
unbuilt on the also-open list since.

Moved: `exit_status`, `process`, `args`, `env`, `exit`, `exists`, `is_dir`,
`list_dir`, `make_dir`, `read_file`, `run`, `start`, `kill`, `write_file`.
Stayed: `stdin`, `write`, `write_err`.

That last line is the boundary case, and this is the committee answering it.
Go's standard streams are files in `os` and the writing is done from `fmt`;
kanso has neither files nor a `fmt`, so what would move is three verbs and what
would be left is a module named for a surface with nothing behind it. They stay
in `io`, which is also what the gavel's own words ask for — "io keeps the
abstract read/write surface" describes a module that still has one.

The sweep: 69 `.kso` files rewritten, 332 call sites moved, 262 left; seven
Rust tests carrying kanso programs in string literals; the `include_str!` table
in `src/lib.rs`; and two places that knew the type's name — `deliberate_exit`
in `src/main.rs` and `k_exit_status` in `src/runtime.c` both matched
`io/exit_status` and now match `os/exit_status`, which is what keeps `os/exit 2`
an exit rather than an unhandled err.

Four things the sweep got wrong and the gates caught, each worth naming:

  - A blanket re-sort of import blocks sorted `tests/golden/errors/import_order`,
    the fixture whose whole job is to be out of order. The error corpus said so
    on the next run.
  - The imports in `.rs` string literals needed the same treatment as the files,
    and a raw string is easier to rewrite than an escaped one.
  - `bench/make_jsonbench` writes a program, and the program it writes carried
    `import "std/io"` with nothing left in it that says `io/`. The decoder's
    checksum gate reads that program.
  - `scripts/effects_differential` emits one fixed import header for every
    probe, and its own comment says a probe carries exactly the imports it uses,
    because an unused import is an error and an error compares equal on both
    engines — which reads as agreement. Eight probes went that way. The header
    is read off the probe's body now.

kq speaks the moved names and is the gating downstream job. Its branch is named
`claude/go-to-town-m0dicm`, the same as this one, which is how
`.github/clone-sibling.sh` checks the two together, and against this compiler
its unit tests, twelve jq goldens, three cost goldens, scale gate and
published-numbers stamp are all green. vse and kanso-json use none of the moved
names and need no branch.

Every counter gate is flat, both compile goldens byte-identical, welfare at
84.85, the book's panels regenerated, and the browser differential reads 317
programs with 0 disagreements.

## 2026-08-23 — the names that moved say where they went

A program written before the split says `io/read_file` and is told `unknown
name`, which is true and useless: the name is right and the module moved. The
refusal now names the destination and what stayed:

    error[name]: unknown name `io/read_file` — it moved to `os/read_file`,
    and `std/io` keeps the reading and writing

Fourteen names, matched only under the `io/` prefix, so every other unknown
name reads exactly as it did. Pinned at
`tests/golden/errors/a_name_that_moved_to_os`, and the message before the
change is on the record two entries up — the plain `unknown name` was what
this repo's own migration met first.

## 2026-08-23 — the perf check this branch owed, against main's own compiler

Every change carries a perf check, and this branch moved the compiler three
times — the accumulator rewrite, two refusal messages, and the library split.
The counter gates say the decoder's emitted IR is byte-identical and both
compile goldens held, but neither watches a clock, and the machine-code gate
did move: the decoder's `.text` grew 48 bytes because `to_float`'s refusal
renders the value it refused.

So main's compiler was built beside this one and the two were interleaved on
one box. That is a relative measurement and nothing else: this container has
been compiling all day, and the published figures are a sitting on a quiet
machine, so nothing here can re-sit them. What it can say is whether this
branch moved anything.

    front end, `check lib/json`, same input both sides, 12 rounds
      main   21.6 ms best, 21.6–26.2 spread
      here   21.2 ms best, 21.2–26.7 spread

    decode, jsonbench, 6 rounds
      main   332 ms best, 332–379 spread
      here   336 ms best, 336–403 spread

Both differences sit inside the per-run spread, and the decode difference is
1.2% against the 3% this log already records for randomised layout on one
tree. No published number moves. The 48 bytes are real and pinned in
`bench/text_golden.txt`; they do not show up in the time.

The front-end measurement is the interesting one, because the library split
adds a module for the resolver to find whenever a program imports `std/os`.
It costs nothing measurable on a program that imports it, which is what the
byte-identical compile goldens already implied and this confirms with a clock.

## 2026-08-23 — the site served whatever copy of the engine was committed

`docs/kanso.wasm` is what the playground runs, and the pages workflow shipped
the committed file. It builds the compiler, but only to run the fingerprinter;
nothing rebuilt the engine. So a merge that changed the compiler without a
hand-rebuild published a playground older than the page describing it, and no
gate could see it: the specs rebuild the artifact *before* they test it, which
proves this commit's source has a third engine and says nothing about the file
the site serves.

It is easy to be wrong about, which is the argument. This branch tripped the
spec's own staleness guard three times in one day — twice after a
formatting-only edit, because panic messages carry line numbers and moving them
moves the bytes.

The pages build rebuilds the engine before jekyll copies `docs` now, so the
site serves the compiler it was built from. The committed copy stays for a
checkout to run and for the browser differential to load, and the specs go on
rebuilding it, which is the check that the source has a third engine at all.

## 2026-08-23 — `kanso build myapp` from the directory above it

Exercising the verbs after the library split — the point being that a stdlib
change can break a whole verb where no corpus looks — turned up one that has
nothing to do with the split. A build is named for its program (#984), so
`kanso build greeter` run beside `greeter/` wants to write a file where the
directory is:

    /usr/bin/ld: cannot open output file greeter: Is a directory
    clang: error: linker command failed with exit code 1
    error: clang failed on greeter.ll

Three lines that name neither the cause nor the way out, and the way out is one
line. The build refuses before it writes anything now, and says both:

    error: this build is named `greeter`, and a directory of that name is
    here — build it from inside (`cd greeter && kanso build .`), or build it
    from somewhere the name is free

The spec checks the refusal, that no `.ll` is left behind, that the linker's
words never reach the user — and that the route it recommends actually builds
and runs, which is what makes printing it worth anything. Watched red against
the old path first, where it read the linker's complaint instead.

Nothing in this repo trips it: the benchmarks build from the root, where
`jsonbench` names a file rather than a directory. It is the single-module
project — `myapp/` built from beside it — that always hit it.

## 2026-08-23 — three veins moved, and one of this morning's numbers was backwards

CI caught what a stale build hid here. The cost-goldens job went red three
pushes running on `emitted`, `machine code` and `work`, while all three passed
locally — because the benchmark binaries on this box predated the fix to
`bench/make_jsonbench`, whose generated entry now imports `std/os`. Rebuilt from
nothing, every one of them moves, and they move the same way:

    emitted lines   11,603 -> 11,588
    escapebench     49,650 -> 49,458 bytes of .text
    pendbench       73,458 -> 73,362
    jsonbench    2,860,478,794 -> 2,860,478,381 retired instructions
    deepbench      806,938,332 ->   806,934,626
    (encodebench, oneshot, basket, widebench, escapebench, pendbench alike)

Every number falls, and one cause explains all of them: a program that imports
`std/io` used to drag the filesystem, the environment, the arguments and the
processes in with it, and now pulls a module with three names in it. Less code
is emitted, less machine code is linked, and a few hundred fewer instructions
run before main. A fall is a win to bank, so all three goldens are regenerated
here. Welfare does not move — a few hundred instructions in three billion is far
below anything a saturating term can see — so there is nothing to `--set`.

CORRECTION to this morning's entry on PR #985, which recorded its emitted-line
move as 11,593 → 11,603, a rise. It is the other way round: the golden is
11,603 and that branch produces 11,593, which is a FALL of ten lines and a win
rather than a regression. What does not change is the finding that mattered:
with its goldens regenerated honestly, #985 reads welfare 84.69 against a floor
of 84.85, and the term that pays is compile cost — front-end rounds on lib/json
28 → 30, visits 23,224 → 23,345, peak 871,649 → 878,422 bytes. That branch owes
the reason it is worth it or the compile cost back, and the emitted-line
direction was never the argument.

The lesson for this box: a counter gate reads what is on disk, and what was on
disk was built before the change. `sh scripts/gates/build_benchmarks.sh` after
`rm -f` on the binaries is what makes a local green mean anything.

## 2026-08-23 (later) — the instructions vein belongs to the runner, and this box is not it

The entry above regenerated three veins together. Two of them were right and
one was measured in the wrong place, and CI said so on the next push: `emitted`
and `machine code` went green against the new numbers, `work` stayed red.

The two that held are real. Building main in a worktree on this same box
reproduces its emitted golden to the line — 11,603 — and this branch gives
11,588, so the fall is the branch's and not the box's. Its cause is visible in
the IR: `import "std/io"` carried 957 module-level globals into the decoder and
`import "std/os"` carries 945, with three blank lines between them. Defines,
calls and branches are identical either way.

Retired instructions do not follow, because a global nobody reads retires
nothing. What this box measured as a fall of about four hundred instructions
across all eight benchmarks is the box: glibc here is 2.39-0ubuntu8.7 and the
runner's is 2.39-0ubuntu8.8, one Ubuntu revision apart on the same upstream
release. That is worth about 400 instructions before main, and several thousand
where memcpy carries the work — widebench 993 and deepbench 3,680.

Measured where the vein lives, the branch's actual move is small:

    jsonbench     2,860,478,794   unmoved
    encodebench   9,727,148,124   unmoved
    oneshot          46,596,968   unmoved
    basket           57,400,154   unmoved
    widebench        84,816,701 ->    84,816,675   -26
    deepbench       806,938,332 ->   806,938,306   -26
    escapebench     258,568,120 ->   258,568,077   -43
    pendbench       988,706,663 ->   988,706,559  -104

So the two figures the entry above published for this vein are withdrawn.
jsonbench did not go to 2,860,478,381 and deepbench did not go to 806,934,626;
neither number moved at all. Welfare is unchanged either way, which is the one
claim that survives intact — a hundred instructions in a billion is far below
what a saturating term can resolve, and there is nothing to `--set`.

The file already said not to do this. Its header has warned since it was
written that a row must never be read against a number measured somewhere else,
and the warning did not stop me, because what I had in front of me was a diff
and a diff invites a paste. So the rule is checkable now: the golden names the
host that measured it in a `measured-on` line, and
`scripts/gates/instructions_host.sh` compares that line against the host it is
running on. `scripts/gates/instructions.sh` runs it first, so off the runner
the refusal costs milliseconds and prints no numbers at all, which is the
point — there is nothing to copy. On the runner nothing changes.

It is its own script rather than a block inside the big one so the ratchet can
prove it honestly. A row whose gate is `instructions.sh` would go red in a
scratch worktree whether or not the mutation landed, because there are no
benchmark binaries there to measure, and a row that is red either way is
evidence of nothing. The small gate needs the golden and `ldd`, so it is green
unmutated and red under `instructions_host_unpinned`, which moves the claim
rather than the box. Both directions were watched.

The machine-code vein has the same shape and got the same treatment, because
the trap is the class rather than the instance. Its rows are `.text` sizes,
which are what the toolchain made of the source, so they belong to the clang
that emitted them as surely as retired instructions belong to the glibc that
ran them. `bench/text_golden.txt` names `clang=18.1.3` and
`scripts/gates/machine_code.sh` checks it before measuring. Nothing has gone
wrong there — this box and the runner share a clang, which is why CI accepted
the `.text` numbers regenerated here — and the point is that nothing had to.

One script over both goldens, `scripts/gates/measured_on.sh`, reading whichever
facts the `measured-on` line names. The granularities differ and the difference
is deliberate: glibc carries its Ubuntu revision because two revisions of one
upstream release demonstrably moved the rows, and clang carries only the
upstream version, because what selects codegen is the release and nothing here
shows a package revision moving a byte. A fact pinned tighter than the evidence
reds the gate on changes that are not changes.

Valgrind's version is in neither, though it belongs in the instructions one on
the merits. Pinning it would make that check unprovable on a host with no
valgrind to ask, and the nightly ratchet runner is one. A valgrind bump moves
the whole vein at once the way any toolchain bump does, which the header
already covers.

A toolchain bump will trip this, and should: every row moves with the image, no
row has regressed, and the refusal says so and names both hosts.

## 2026-08-23 (later still) — the compile-memory band has been hiding main's own drift

Looking for other veins with the provenance problem turned up a different one
in `bench/compile_memory_golden.txt`. Its peak-bytes row is not diffed exactly.
CI asserts only that reality is within two per cent of it, and the header gives
the reason: peak bytes is measured by the compiler's own allocator and is a
property of the host, with linux and macos disagreeing by 56 bytes on the same
input.

Two per cent of 871,649 is 17,432 bytes. The divergence it was written to
absorb is 56. Everything between the two is drift nobody sees, and there is
drift in there now:

    golden                                     871,649
    main, measured here, three runs identical  872,025
    this branch, measured here                 872,035
    this branch, measured on the runner        872,061   (two runs identical)

The number is deterministic per host — three runs on this box give the same
digits, and two CI runs give the same digits. This branch is worth ten of those
bytes. The other 376 are main's, accumulated since whenever the golden was last
written, and every gate has been green the whole way.

That matters more than the bytes, because welfare reads `compile_peak_bytes`
out of this golden as the CURRENT value of a term rather than measuring it. So
the compile-memory term has been scored against a figure the compiler left
behind, and the floor was ratcheted to 84.85 while it was.

Correcting it costs the floor. With 872,061 in the file the score still prints
84.85 — the term moves from 0.167 to 0.168 points, below the second decimal —
and `scripts/welfare` exits 1, because the true value now sits under a floor
that was set against the stale reading.

**This is Clay's, and it is on the list.** The options, as I read them:

1. Regenerate the golden and `--set` the floor with the reason, which banks a
   fall rather than a rise. That is the one thing `--set` has never been used
   for, and the doctrine is explicit that moving the floor while leaving the
   weights alone declares the objective wrong without saying so. Against that:
   nothing here is a code change, and the floor was set against a misreading.
2. Pay the 376 bytes back out of the front end and regenerate at whatever it
   then reads, which keeps the floor honest and costs real work on a term the
   weights say matters least.
3. Tighten the band to something near the divergence it documents — say 128
   bytes rather than 17,432 — so the next drift is caught the week it happens.
   That is orthogonal to 1 and 2 and looks right regardless of them, and the
   `measured-on` line makes it affordable, since the gate can now refuse off
   the reference host instead of widening to tolerate it.

Nothing is changed here. This branch leaves the golden alone: its own ten bytes
are inside any reading of it, and the 376 predate it.
## 2026-08-23 — the python that crept back is out, and a gate watches now

The 2026-08-09 entry declared the repo python-free. Within three days it was
not: #854's ratchet mutations carried six `python3` heredocs and #862's write-
path gate added `scripts/stale_a_panel.py`, and a `bench/kq_race.sh` racing an
apps/kq this repo no longer holds had survived the original sweep entirely.
Nothing watched the claim, so nothing went red.

Three moves, each verified differentially against the python it replaces.

The panel staler is kanso (`scripts/stale_a_panel/`), byte-identical on ch04
against the python, loud on a missing title or marker, and watched both ways
through book_check: misname its panel and the check dies on `missing panel
title`; restore it and the write path rewrites both staled panels back.

The six mutation heredocs are POSIX awk in the same .sh files. Anchors travel
through ENVIRON so no byte is reinterpreted, replacement is first-occurrence
like the python's `replace(..., 1)`, and a missing anchor dies with the same
"moved; this mutation needs rewriting" message. awk rather than kanso because
a mutation runs in a fresh worktree before any build — a helper needing
target/release/kanso would make the harness depend on the binary it is about
to mutate. All six produce byte-identical mutated sources and identical exits
against the heredocs they replace.

kq_race.sh is deleted, not ported: the archive records apps/kq removed with
kanso-lang/kq as its sole home, and the script builds a path that is not
there.

The gate is `scripts/gates/python_free.sh`, a python-free CI job, and a
ratchet row: no tracked .py file, no python3 call outside design/'s history
and the one mutation whose job is to introduce one. Watched red three ways —
the stale racer before its deletion, a python3 line appended to book_check
(the row's mutation), a tracked creep.py — and green on the clean tree.
## 2026-08-23 — one ledger: pending decisions live in pending-gavels, and nowhere else

Clay ruled it in the developer chat, verbatim intent: "110% UNIFY those
into a clear single source of truth. compiler-log was supposed to just
be the history of actual decisions, whereas we need something like
'pending-gavels' for keeping track of anything that requires my personal
decision because it's about the 'UX' of the language, not the
implementation details."

What had happened: four surfaces claimed the same authority and
disagreed. STATUS.md carried a full-text "Waiting on Clay" section that
forked per branch — main's copy showed one item waiting, the working
branch's showed four. design/pending-gavels.md said "every decision
waiting on Clay, in one place" while filing list-as-bytes under "not
blocking" as STATUS.md called it blocking. GAVELS.md sat orphaned at the
root — nothing referenced it — holding nine July letters that never
closed, plus a ruled ledger duplicating the log. And sessions were
citing decisions to Clay by their own private task-list numbers ("#2
list-as-bytes"), which resolve nowhere outside the session that minted
them.

The unification, per the ruling:

- **design/pending-gavels.md is the ledger.** Charter at the top: UX
  forks only, entries leave on ruling (the file's own 2026-08-15
  precedent, drifted from since), STATUS.md indexes but never carries
  text, entries are cited by heading, edits ride promptly-merged PRs.
- **This log is history and nothing else.** Rulings land here; nothing
  pending lives here.
- **The ruled entries left the ledger** — gavels 1 (surface), 3+5, 6,
  8, 15, 16, 17, 18+19, 20b, 21, 22, 23, 24, fixed-length lists,
  the name of `[]T` — their rulings are above under their dates and
  their full text is in the file's git history.
- **GAVELS.md is deleted.** Its ruled ledger (A1–A5, R5, T, W, B, BB,
  X, nullary-BB) duplicates rulings recorded here and in the archive;
  its unclosed letters are triaged into the ledger: Y closed 2026-08-19
  (`>>` keeps its glyph), F parked 2026-08-19, H shipped as entropy-by-
  default with KANSO_SEED pinning, and C, D, G, Z, AA sit under a
  "stale — revalidate against the post-24 language" heading rather than
  pending, with Z marked presumed-declined by the 2026-08-15 err gavel.
  Full text in git history.
- **The four live decisions** — list-as-bytes (upgraded to blocking,
  which the measurements say it is), the undemanded knot, the
  compile-memory band, and `>>` under run-time effect failure — are the
  ledger's blocking section, carried over verbatim from the branch
  STATUS.md that held the freshest text.

CLAUDE.md's design-flow line now names the ledger; the vague "AND a
memory file" is gone.

## 2026-08-23 — gavel: an undemanded knot allocates nothing, on any engine

Clay ruled the first entry in the unified ledger, in the developer chat.
The principle: work defers until it is actually presented to IO — until
it can affect the real world — and eager evaluation exists only as a
resource-optimization heuristic inside that contract, never as a
semantic difference an engine may expose.

The shape that forced the question compiles clean and cannot be caught
at compile time (an unreferenced knot is already `error[unused]`; this
one is referenced in a dispatch arm the run never takes):

    x = [x]

    pub play = picked 1

    fn picked 1
      io/write "one\n"

    fn picked _
      io/write "{length (list/to_list x)}\n"

Native reads `thunk_allocs=1, thunk_live_exit=1` — `k_caf_init` builds
every knotted constant before main. The oracle reads zero. Reproduced
today on both engines before ruling.

The ruling: the oracle is right. A knotted constant defers like every
other binding; an undemanded knot allocates nothing, and `thunk_allocs`
stays in the engine-shared differential set counting demanded work
only. The disagreement closes by changing native — the startup freeze
goes — not by re-scoping the counter or splitting it.

On the hot-loop cost that motivated the freeze, Clay rejected the
premise that deferral means a perpetual conditional: "of course you
need that check. but it shouldn't really be a 'check' like a
conditional. instead, you just make a code change once it's evaluated.
… imagine you have a stored lambda. when you run it, it says 'compute
this expensive thing, then replace the existing lambda with a new one
that just returns this'. then the next call doesn't need to 'check'
anything. it just runs." That is update-in-place — the machinery the
runtime already uses for ordinary thunks — an indirection rewritten at
first evaluation, not a branch paid on every read. Implementation is
the implementer's; if measurement finds a real hot-loop regression
even in the update form, the number comes back to the ledger before
any freeze returns.

Unblocked: the fixture pinning an undemanded knot at zero on both
engines, and the .mem/golden regeneration that lands with the native
change. The entry leaves the ledger with this commit.

## 2026-08-23 — gavel: a list is never bytes, and acceptance is declared

Clay ruled the second ledger entry in the same sitting: "in general I
like consistency," with the committee heard on the counterarguments
before the gavel. The steelman for widening had three legs — bytes are
just small ints (the data-is-data lens), the language has no bytes
literal so `[104 105]` is the only spelling a user can write down, and
`text/utf8 [65 66]` answers "AB" on both engines today, so full
strictness would break the one case the engines agree on. The first leg
dies on the evidence: `["a" 120]` — the oracle's answer for
`text/append ["a"] "x"` — is not data-is-data, it is `bytes_to_str`
accepting whatever list arrives. The other two legs are real and became
the ruling's riders.

The ruling: the interpreter gains a real bytes value, and a list is
never ambiently bytes on any engine. The four cases where the oracle
answered become refusals matching native's, and the fixture family
(`append`, `find2`, `find2_below`, `utf8`, `to_float`) can finally pin.

Two riders, named with the gavel:

- **Acceptance is declared, not coerced.** Where a function genuinely
  wants a list of small ints as byte input — utf8 is the live case —
  that is a visible, per-function acceptance in the library, identical
  on both engines. Whether utf8 keeps its list acceptance is a library
  decision made in the migration, not an engine property. This is the
  typeset-acceptance idiom already pending under the AA entry, applied
  early.
- **The constructor ships in the same change.** A list→bytes function
  with a loud refusal on anything outside 0–255, so byte data stays
  writable now that the coercion is gone. `text/bytes` covers strings;
  this covers numbers.

The entry leaves the ledger with this commit. Unblocked: the six-case
fixture table, and the interpreter's bytes representation work, which
is the implementer's.

## 2026-08-23 — the startup freeze goes, and an undemanded knot builds nothing

Implementing the gavel above. `k_caf_init` used to seed every knotted
constant's cell and run every builder before main, so a knot the program never
demands was built anyway. Each constant now seeds and builds its own cell on
the first read, and `k_caf_init` is left holding only the math-id handshake.

The ready flag is set BEFORE the builder runs, which is the same discipline
`k_caf_init` had when it seeded every cell before running any builder: a
constant that mentions itself re-enters the reader and has to find the
blackhole rather than the zeroed global, which is an integer zero and reads as
one. That seeding is what keeps the cycle finite, and the demanded knot still
answers `1` on both engines.

Measured on the ruling's own program, with the counters the differential
shares:

    undemanded      oracle 0 allocs   native 1 -> 0
    demanded        oracle 0 allocs   native 1 -> 1

The ruled disagreement closes. It cost nothing and paid something: the one
fixture in the mem corpus whose numbers move,
`an_unasked_equality_stays_a_cell`, falls from six allocations to two and from
four evacuations to none, because a constant nobody asks for is no longer
built and so never has to be evacuated as a survivor. The freeze had been
buying eagerness nobody wanted and paying for it in the beat.

One branch, taken once per constant. Clay's preferred shape is update in place
— rewrite the indirection at first evaluation so later reads check nothing —
and it stays the better form if this ever costs anything measurable. Nothing
in this corpus says it does.

Across the benchmark veins the shape repeats. Three of the eight `.text` rows
rise by the branch — encodebench and widebench 16 bytes, escapebench 32, one
per knotted constant — and five do not move at all, jsonbench among them,
because the decoder links no knot and so the hottest path in the project is
untouched. Three counter goldens fall: widebench loses five permanent
allocations and six allocations outright, scanbench two permanent and two
evacuations, encodebench two and two. Constants that used to be built before
main are not built at all when nobody asks. Welfare holds at 84.85.

Sixty-four bytes of `.text` is what update in place would take back, and this
is the number it has to beat.

A CORRECTION about how this was measured, because it nearly became a false
report. `kanso run` compiles and runs; the oracle is `run --interp`. Measuring
the ruling's program with `run` twice and calling one of them the oracle
produced two identical rows and the conclusion that the gavel's premise did not
reproduce. The premise reproduces exactly. The lesson is the same one three
goldens now carry in a `measured-on` line, arriving this time through a verb
rather than a host: a number means nothing without the thing that produced it.

The price came back from the runner. Four rows of
`bench/instructions_golden.txt` rise, each by about what taking the ready-flag
branch costs: encodebench +17,931, deepbench +155,986, escapebench +5,993,
widebench +358. deepbench's is the largest and the smallest — 0.019% of 807
million — because it reads its knotted constants inside the hottest loop it
has, so the ready-flag test is paid once per read instead of once per program.
jsonbench, oneshot, basket and pendbench do not move at all. These numbers are
the runner's: the vein carries `measured-on glibc=2.39-0ubuntu8.8` and this
container is one revision off, so `scripts/gates/measured_on.sh` refuses to
hand over a diff here and CI's job log is the only place they can come from.

`encode_sh_buf` rises 96 bytes. A per-capacity histogram of freshly-allocated
buffers, split by beat depth, says what it is: exactly one more five-element
buffer, allocated inside the beat. Every other capacity class is
byte-identical, `buf_reuse` does not move, and no five-element buffer is allocated
before main under either codegen. Beside it on the same benchmark, two
permanent allocations and two evacuation copies go away — `perm_allocs` 12 to
10, `evac_allocs` 19 to 17, `evac_bytes` 624 to 576. Two constants that used
to be frozen into malloc'd storage before main, and copied there out of the
arena, are not frozen at all now; what is left is 96 bytes of arena the beat
reclaims on its next rewind, in place of two allocations that lived until exit
and forty-eight bytes of copying.

Left open, and going to the ledger as its own entry: the DEMANDED knot still
disagrees. Native reports `thunk_allocs=1` where the oracle reports `0`,
because the oracle's `knotted` builds its cell without touching the counter.
That predates this change and survives it, and which engine is right is a
question about what the counter counts rather than a defect to pick a side on.

## 2026-08-24 — bytes are a value on every engine

Implementing the bytes gavel. The interpreter had no bytes: `text/bytes`
answered a list of integers and every consumer ran whatever list arrived
through `bytes_to_str`, which is how `text/append ["a"] "x"` came to answer
`["a" 120]` where native refused. `Value::Bytes(Rc<Vec<u8>>)` exists now, and
the six rows the ledger measured agree word for word on both engines:

    text/append ["a"] "x"        append takes bytes and a string, bytes, or byte
    text/append [65 66] "x"      append takes bytes and a string, bytes, or byte
    text/find2 [65 66] 1 65 66   find2 takes bytes
    text/find2_below [65 66] …   find2_below takes bytes
    text/utf8 ["a"]              err "utf8 takes byte values (0-255)"
    text/to_float ["a"]          to_float takes a string, bytes, or number, not ["a"]

Four of those were the oracle ANSWERING where native refused, which is a
program that runs under the interpreter and dies compiled. The browser engine
comes along for free: `rt_builtin` calls the interpreter's `call_builtin`, so
one implementation serves two of the three engines.

The two riders shipped with it. utf8 keeps its list acceptance, spelled the
same on both engines — there is no bytes literal, `[104 105]` is the only
spelling a program can write down, and `text/utf8 [65 66]` was the one case
the engines already agreed on. And `text/to_bytes` is the constructor, loud
outside 0-255 rather than keeping the low byte: `text/bytes` covers strings,
this covers numbers. The one place the low byte is still taken is
`text/append`'s single-number form, because that is what the compiled engine
has always done (`x.payload & 0xff`) and matching it is the differential law.
Whether either engine should refuse there instead is a separate question.

`==` still crosses. Native has compared a byte string against a list of its
numbers since it was written (`k_bytes_eq_list`), so the interpreter does too;
making both refuse is a change to native's semantics that the gavel did not
order.

What it cost: `front_end_visits` on lib/json 23,224 -> 23,250, and the
decoder's emitted lines 11,588 -> 11,595. Both are the price of a public
function in std/text, which lib/json imports. Measured separately: a plain
`pub fn zzz x / x` in std/text costs 13 visits by itself, so roughly half of
the 26 is the function existing and half is the builtin call in its body. No
allocation counter moves, no `.text` row moves, and welfare's floor moved to
whatever it cost — which welfare's own header says is what happens to a change
that makes the engines agree.

The goldens: six refusal fixtures under tests/golden/runtime, each watched
red against the old oracle first — four of them ANSWERED there, which is the
bug — and two micro fixtures for the surface that works and for the
constructor's refusal. The error corpus moved three line numbers, and the book
three more, because std/text grew five lines above `to_int` and an err trace
names the line it was born on.

## 2026-08-24 — a type field wakes its readers, not the whole program

The bytes gavel cost lib/json 26 front-end visits, and looking at where they
were spent found a much larger number sitting beside them. `KANSO_PHASES=1`
on lib/json:

    round 1: 365 moved of 407 visited
    round 2:  45 moved of 407 visited
    round 3:  52 moved of 407 visited
    round 4:   7 moved of 407 visited

Four full sweeps of every function, the last one to let seven of them move.
The fixpoint has had dirty-tracking since it was written — a function's
returns wake its readers and nobody else — and one line was defeating it. When
a declared type's field set grew, inference set `all_dirty`, and the next round
walked the program. It had to: nothing recorded which functions could care.

They are static. `type_fields` is read in exactly one place, `bind_pattern`'s
`Pattern::Ctor` arm, so the functions that can be affected are the ones whose
patterns destructure that type — in the head or anywhere in the body. The index
is built once, before the first round, and a field growing now wakes those and
nothing else.

The rounds it saved were paid back by rounds it cost: information travels one
hop per round, and a round that walks forty functions carries it less far than
one that walks four hundred. So a change moves its readers in the CURRENT round
as well as the next. The sweep alternates direction, so about half of them are
still ahead of the cursor and take the new answer immediately; the rest are
behind it and are simply not walked again.

    lib/json          rounds 28 -> 40, visits 23,224 -> 17,786
    the module sample rounds  6 ->  6, visits  3,031 ->  2,403
    the five samples  visits    133 ->    115

`front_end_rounds` 28 -> 40 is the cost and it is real; 5,438 fewer expression
visits is what it buys, and the visit is what carries the work — a round is a
loop over a work list that is usually short now. Welfare weighs both and comes
out ahead: 84.85 to 84.87, banked.

The clock does not show it, and the entry would be dishonest without saying
so. Interleaved on this container, three runs each, `infer` reads 2.80-3.04 ms
on the branch against 2.82-4.06 ms on main — inference is about a fifth of a
15 ms front end and the spread here is wider than the effect. The visit count
is the instrument that can see it, which is the whole reason the compile
goldens count work rather than time.

The index costs memory to hold: `compile_peak_bytes` on lib/json reads 876,930
here against main's 872,035 on the same box, three runs identical each way.
That is inside the two per cent the gate allows and outside what welfare can
see, because welfare reads the golden's number rather than measuring — which is
the ledger entry that has been waiting on Clay since yesterday, and this change
adds 4,895 bytes to what it is hiding.

The answers do not move. Every engine, the error corpus, the diagnostics
differential and the browser differential are unchanged — the only goldens that
move are the ones that count the compiler's own work.

## 2026-08-24 — the highest-ranked idea on the memory board was priced against a number that is gone

`design/memory-frontier-research.md` has ranked copy-or-pin first since
2026-08-07, on a measurement: half of every allocation the one-shot shelf made
was the copy-out before a rewind — 63,967 evacuation allocations of 128,528,
1,991,456 bytes. Rechecked today, one-shot reads `evac_allocs=3`,
`evac_bytes=96`. #868 took it from 63,967 to 5 and #977 to 3. The measured half
the idea was going to delete had been deleted by something else, and the memo
did not know, because a status table records what an idea IS rather than
whether its premise still holds.

Where evacuation lives now, across the eight shelves: wide 264 allocations for
1,032,336 bytes, pending 2,658 for 498,976, scan 36 for 8,800, and everything
else under six hundred bytes. So the idea gets reposed rather than retired —
and the instrument that priced it the first time can price it again. The
evacuation path was instrumented to record each survivor's source address and
copied size.

Wide is four copies. Four nodes of 256,016 bytes — a 16,000-element list
buffer, 16 + 16 x 16000 — carry 99.2% of the megabyte. `bench/wide.json` is a
16,000-element list, so that is its top-level buffer evacuated as the streaming
loop's carried accumulator, once per rewind. Two of the four report the same
source address, which says only that the arena reused it — the addresses are
bump-allocated and a rewind hands the same bytes back. The other 260 survivors
are 8,272 bytes between them, median 32.

Pending is diffuse: 666 of 2,658 survivors are needed to reach 90% of half a
megabyte, nothing above four kilobytes, largest 3,216.

That is the answer the memo asked for and nobody had taken, and it is two
answers rather than one. A quarter-megabyte survivor occupies whole pages by
itself, so not copying it retains almost nothing — and it does not need general
page pinning, only a size threshold and storage that does not rewind. A
three-kilobyte survivor is threaded through the garbage, and pinning its page
keeps the garbage with it. The size distribution is the decision variable,
which is 5.2's survivor-ratio selection asked one level down.

Nothing is built. What changed is that the board now says what the shelves say.
