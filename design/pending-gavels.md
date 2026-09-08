# Pending gavels

The single source of truth for decisions awaiting Clay — ruled so on
2026-08-23, unifying what had forked across four files. An entry is here
because it is about the language a user meets: surface, semantics,
observable behavior. Implementation details do not come here; whoever
holds the file decides them and answers for the decision in the log.

The lifecycle, restored from this file's own precedent of 2026-08-15:
an entry lives here while open; the ruling is recorded in
design/compiler-log.md, which is the history and nothing else; and the
entry leaves this file in the same commit. Everything that ever left is
one `git log -p -- design/pending-gavels.md` away.

Rules of the ledger:

- **An entry cites its search, or it is invalid.** Before filing, search
  design/compiler-log.md, design/log/compiler-log-archive.md and every
  design/*.md for the question. The entry then says what the search found
  — the ruling that partly covers it, the experiment that answers its
  premise — or states plainly that it found nothing. An entry with no
  citation line is not a pending decision; it is an unsearched one, and
  it does not go to Clay.
- **An entry carries a recommendation.** Every question below says what
  the holder of the file would do and why, so a sitting can be a yes or a
  no rather than a fresh design conversation. Where the recommendation is
  to close the question, one word does it.
- **A gaveled item carries its citation forever.** Where an entry
  survives because only part of it was ruled, the ruling's marker stays
  in the entry. This rule exists because gavel 1b's marker has now
  fallen out of this file twice — once caught and restored, once in the
  `e3052383` rewrite that made this single ledger — and each loss made a
  settled question look open. The log is append-only and cannot lose a
  fact; this file is maintained by hand and has. When the two disagree,
  the log wins.
- STATUS.md may index this file. It does not carry decision text.
- Sessions cite entries by the headings below, never by a session task
  id — a task list is private to its session and its numbers resolve
  nowhere else.
- Edits to this file ride small, promptly-merged PRs, never a feature
  branch, so the ledger cannot fork.

The residual sweep of 2026-08-25 walked the log, the archive and every
design doc for questions that were asked and never answered. What it
found is below: every remaining question asked once, with a recommendation,
so the list can be ruled in batched sittings and end. The intent is that
this is the whole of it. Six candidates the sweep turned up
were already answered by the shipped code or by a later gavel, and those
went to the log rather than here.

## Blocking — a fixture, gate, or merge is waiting

(The sha256 digest question sat here briefly and was bounced on
2026-08-29: performance questions with no surface area are the
implementer's, per this file's own charter. The log carries the
research mandate it left with.)

### The compile term's workload is whatever lib/json imports, and a library edit halved it

**Cited:** the 2026-09-03 rebuild of the objective (three compile rows over
`kanso check lib/json`, `bench/objective_sources.txt`), the 2026-09-06 gavel
(one run program), and the log's 2026-09-07 entry "The scan, iterated"
(kanso#1291). Searched the log, the archive and design/ for a ruling on WHAT
the compile term compiles: none. The rows were pinned to lib/json when the
term was built because it was the one real library, and nothing said what
happens when lib/json's imports change.

**What happened.** lib/json's escape fold was its only use of std/list. The
iterated scan retired the fold, the import went with it, and `kanso check
lib/json` now compiles half the code it did: `compile_instructions` ~42.6M
-> ~19.6M, `compile_allocs` 25,862 -> 11,613, `compile_peak_bytes` 724,798 ->
375,222 on the container, welfare 51.95 -> about 56. The compiler is
byte-identical. By the objective's definition the rise is real; by what the
term is for — how fast the compiler is — it measures nothing, and a later
change that re-imports std/list into lib/json reads as a four-point fall.

**Options.**

- (a) As is. The term prices what lib/json costs to check, a library edit
  is allowed to move it, the floor rises to ~56 now and the re-import
  argues later. Recommendation: no.
- (b) A fixed corpus. The three compile gates and welfare check a package
  that names its imports once — lib/json plus every std module the
  benchmarks import (list, text, testing) — so a dropped import moves the
  rows by the import's own compile cost and nothing else, and a bare
  library edit reads as what it is. Rebased once, with its own --set, and
  the history chart marks the day. Recommendation: **yes**.
- (c) As (a), with this rise recorded as a re-basing in the welfare history
  rather than a gain. Cheaper than (b) and leaves the next import change
  to make the same argument. Recommendation: no.

**Until ruled:** the standing rule applies (a rise is held). kanso#1291 sets
the floor with CI's rows and its log entry says which part is the workload,
so the ruling can undo exactly that much.

### An infinite or nan float has no rendering on either engine

**Cited: this log, design/log/compiler-log-archive.md, this ledger and
every design/*.md, searched 2026-09-07 for infinity, inf and nan in the
rendering sense: nothing. The 2026-09-05 whole-float ruling (kanso#1285)
and the %g rule `render_float` in src/eval.rs mirrors -- exponent form at
X < -4 or X >= max(15, digits) -- speak only to finite values; the
whole-number arm tests `is_finite` and the arm after it does not.**

Found while probing the number renderer's copy at its edges. Ten
multiplications of 1e10 overflow a double, and rendering the result:

    t = 10000000000.0
    h = t * t * t * t * t * t * t * t * t * t
    print "{h * h * h * h}"

The interpreter panics -- `render_float` asks Rust's `{:e}` for the digits
and expects an `e` in the answer, and `inf` has none (src/eval.rs:3949,
"LowerExp has an e"). Native prints `1.797693134862316e+308`, the largest
double's digits for a value that is not a double's; for `inf - inf`, which
is nan, it prints `2.696539702293474e+308`. No golden in the corpus prints
either, so the differential law has never been asked. Both engines are
wrong, and the oracle's wrong is a crash, but what `"{x}"` should say for
such an x is a surface the user meets, which is why it is here and not
fixed.

**RECOMMENDATION.** C's spelling, which is also what the %g rule the
renderer already mirrors produces: `inf`, `-inf`, `nan`. Rust's `Display`
for f64 prints the same three, so the oracle would say it with one arm and
native with one branch in front of ryu. The alternative worth naming is
JavaScript's `Infinity` and `NaN`, which no other rule in the renderer
descends from. Whether `json/encode` may emit such a value at all is a
question for that library, and is not asked here; a differential fixture
over the three values, on all three engines, ships with whichever spelling
is ruled.

## Open, not blocking


### The book teaches the boundary language (queued P1, Clay 2026-08-26)

**RE-PREMISED AGAIN 2026-08-29 by the effects-are-types gavel, which
supersedes the three-chain-words form.** The call-site story the book
owes is now: `<t>effect` as a first-class passable outcome type;
`bind`, `annotate`, `rescue` as ordinary effect-first functions and the
sole eliminators; no automatic bind — a box where the unwrapped type is
expected is refused, and propagation is bind's contract. Half one (ch04
"nothing is asked of the signature") DOES NOT survive as written: its
short-circuit-at-the-call story describes the retired railway and needs
rewriting on explicit elimination. Half two lands when the typed-effect
surface is implemented, present tense as always. compiler.html entry 23
owes a rewrite or retirement in the same campaign.


### An assert hako

**Cited: the licence half is ruled — archive 2026-08-17, assertions are
ordinary foreign rescue. What is open is the surface shape only.**

A real assertion library in the rspec direction Clay sketched —
`(expect 1) . to (equal x)` — as its own small surface design, never
improvised inside a test fix. Its arms are foreign to every tested hako,
so the err license needs nothing special. Queued 2026-08-17.

**RECOMMENDATION: build it as its own design pass. The gate is lifted.**
The matcher surface reads failures, so its shape depended on how a
failure is spelled — that is ruled (three-forms gavel, 2026-08-26) and
built on all three engines (kanso#1116), so designing it now cannot mean
designing it twice. `rescue` is the word a matcher's own failure door
would use.


## Stale — the July campaign's unclosed letters (GAVELS.md, retired here)

EMPTY. Clay ruled the last five in one sitting on 2026-08-26 — C struck,
`done` minted for D, G struck on the July provenance measurement, Z
confirmed declined, AA explicit-cast only. Every letter A1–X, BB, C, D, G,
Z and AA now has a ruling in the log or the archive; the section stays as a
header so a reader looking for the campaign finds where it went.

## Parked — on the record, no action

- `<<` labels: walls cover staircases; revive on real DAG demand.
- Labeled nameless patterns: parked 2026-08-19 — needs a fresh look
  against the post-24 language, not pending. Group headers stay behind
  it.
- dot-absorbs-`>>`: argued no — erases the visible then/bind split.
- Postfix index on `)`: `(sort xs)[1]` stays illegal; bind-then-index.
- `;` inline separator: the borrow if inline groups are ever demanded.
- `&` as bitwise: orthogonal, someday.
- Convention over configuration, for the framework campaign (Clay,
  2026-09-06): reflection only in the safe direction — code to name,
  never name to code. Two candidate primitives: a function value that
  knows its qualified name (`name_of`), and a module's exports as a
  map. A router built from them indexes a map with an IO string and
  gets a function or `none`; no string ever becomes code. The most
  kanso-native shape: the path parse is a decode into a typeset of
  declared routes, and the route table is a dispatch group. Parked
  beside `serve`; not pending. DEFERRED BY CLAY (2026-09-06): not
  before the language as currently specified is finished — the
  effects-are-types migration, the fused operators, whole-cohort
  block-born, the growable partial, the suffix contracts, the
  consolidated benchmark, the clang bump, the book rewrite — and then
  optimized and debugged as far as it can be taken. No session starts
  this on its own initiative; Clay reopens it.
- `serve` / processes: the executor-loop primitive; next design
  campaign — three investigations already terminate there. The July
  reification form (an err becoming an inert Failure record at the
  supervisory boundary) died with gavel 1; the campaign starts from
  the three combinators.
- Hako tag-signing and checksum policy: parked in design/hako.md until
  something is worth attacking. The lock already carries a sha.
- Monorepo hakos (several modules per repo): the path shape allows it;
  the lock-granularity decision waits for a real case.
- Survivor cap 4× block threshold: the multiplier is a judgment call;
  the principle (the dance's transient stays at threshold scale) is in
  the log.

**THE MECHANISM IS NAMED NOW, AND IT IS A TERM ALREADY RULED ON.** callgrind's
call graph: `std::rt::lang_start_internal` calls `pthread_getattr_np`, which
parses `/proc/self/maps` with `getline` and `sscanf` to place Rust's stack
guard. Splitting each profile into that parse and the program:

    binary                      row          maps parse   the program
    9fcc6686dc47 baseline       42,344,081      112,580    41,878,959
    45c6dbed10bb +64 KiB .bss   42,346,211      114,710    41,878,959
    2a4e10fb2116 100 fns        42,345,904      112,586    41,880,776
    5e73453bcc7b 200 fns        42,343,660      110,317    41,880,801

The `.bss` probe adds no code and the compiler's work is **identical to the
instruction**. All 2,130 of the row's move is the parse.

kanso#1234 found this term and the ruling of 2026-09-03 was NO EXCLUSION, so
**nothing here asks to exclude it and nothing has been changed.** The new fact
is its size: 0.27% of the row and 100% of its binary-to-binary drift, with
`std::rt::lang_start::{{closure}}` sitting still through a change that moved the
published row by 2,130. The ruling was made when the term was known to exist and
not known to be the whole of the drift, and this entry is where that goes.
