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

### Where the box wraps under the pure-fallibility rider: at every err-carrying answer, or at the `!` name

**Cited:** the archive's "rider: pure fallibility is boxed too" (2026-08-31),
"gavel: effects are types, and the words are the only doors" (2026-08-29),
"gavel: the suffix contracts are refusals, as ruled in July" (2026-09-03),
and the live log's "the effect type is spellable" (2026-09-10), which sized
the rider. None of them says what a non-`!` declaration that raises `err` in
one arm and answers a value in another yields, or where its value arm is
lifted into the box.

**The question.** The rider says any operation whose answer includes an err
yields `<t>effect`. Read literally, a declaration like json's `hex_digit`,
`if (…) (c - 55) (err "invalid hex digit")`, yields `<int>effect`: the
compiler lifts its value arm into the box at the declaration's boundary,
every caller opens it with `.>`, and by infer's reading 3,801 declarations in
the tree answer that way, 738 of lib's 770, because a declaration that hands
a fallible answer through is fallible too. Measured on 2026-09-10 with the
checker made to read it so: the first layer of refusals alone is 108 sites in
lib (list 49, regexp 25, sha256 25, json 3, http 3, path 3), and each bind
written for one moves the refusal a caller outward until the 738 are chained.
The library becomes bind chains from `parse_value` down, and every bind is a
closure and a box the arena pays for.

The insist alone is smaller, and it has the same problem in a sharper form.
With only `foo[k]!` answering a box, which is the case the rider was raised
on, the checker refuses 143 sites in the tree (lib 28, scripts 61, the test
corpus 54). The readers are `==`, `+` and `-` in sha256's compress and
regexp's scanner, where the index is in range by construction. The lenient
form looked like the answer for those, and it is not: under the 2026-09-07
exhaustiveness ruling `choice s[5] s[6] s[7]` is refused three times, since
each index can be a none and `choice` has no arm for one, and `walked
parts[i] s` the same. So an in-range read has no spelling left. `xs[i]` is a
none at every group; `xs[i]!` is a box at every operator; what remains is
`xs[i]! .> (v -> …)`, a box and a closure per element in the kernels, or a
`none` arm on every group an element reaches. Of the 723 `]!` sites in the
tree, 13 open their answer with a word today; the other 710 hand it to an
operator, a group or a field. The bind shape was priced on native with the
box built: a loop reading two million in-range elements costs 9 allocations
and 21 ms as `acc + xs[i]!`, and 8,000,014 allocations (four an element: the
box, the closure, the bind and the rewrap), 352 MB and 65 ms as
`xs[i]! .> (v -> go (acc + v))`, the same sum both ways.

**Options.**

1. The literal rider: every err-carrying answer is a box, lifted at the
   declaration boundary. The cost is the paragraph above.
2. Fallibility is spelled in the name, which is how the suffix contract
   already reads it in one direction: a `!` name answers a box; a name
   without `!` answers data and may not raise, absence being `none`, as
   `foo[k]` already is. `to_int` answers none on bad text and `to_int!` the
   box. The 71 lib declarations that raise or insist themselves choose a
   spelling each; the other 667 are untouched. The railway retires the same
   day, since nothing outside a box carries an err any more.
3. As 2, with `err reason` itself answering a box, so a non-`!` name may
   still raise and its callers must open it. Wherever that is written it
   costs what 1 costs.

4. The index keeps a third spelling for the in-range read, and the ledger
   does not propose one: it is a language question. What the kernels need
   is a read that is neither a none nor a box, which is what `xs[i]!` was
   before the rider.

**Recommendation:** 2 for names, and for the index a ruling on 4 before the
insist lands, because without it the insist PR respells sha256 and regexp
into bind chains or none arms and pays for it in every cost vein. The insist
answers a box on all three engines in the worktree today (the oracle and
native print the same four lines on the fixture); the respell of the 710
sites is what waits.

## Open, not blocking

### The book teaches the boundary language (queued P1, Clay 2026-08-26)

**RE-PREMISED AGAIN 2026-08-29 by the effects-are-types gavel, which
supersedes the three-chain-words form.** The call-site story the book
owes is now: `<t>effect` as a first-class passable outcome type;
`bind`, `annotate`, `rescue` as ordinary effect-first functions and the
sole eliminators; no automatic bind — a box where the unwrapped type is
expected is refused, and propagation is bind's contract.

**BOTH HALVES WAIT ON THE IMPLEMENTATION. Measured 2026-09-08 on
`2abcedaf`.** Since 2026-08-29 this entry has said that half one — ch04's
"nothing is asked of the signature" — does not survive as written and can
be rewritten ahead of the surface. Two probes say otherwise. A lambda
handed a failing argument does not run its body, and the call answers the
failure: that is the railway ch04 teaches, live, word for word. And
`<int>effect` is not spellable: the canonical-spacing rule refuses the form
outright, and no checker ever sees a type, because there is no `effect` type
in the tree for one to see. The gavel retired the design. The engines still
run the railway, and the book is present tense, so a rewritten ch04 would
describe a language nobody can run.

What has landed is the smaller part, and the book already has it: `bind`,
`annotate` and `rescue` ship on all three engines (kanso#1116), and ch05
teaches them as ordinary two-argument functions taking the effect first.
Missing is the type — passing a box, and the refusal of a box where the
unwrapped value is expected.

**RECOMMENDATION: hold until `<t>effect` exists, then run the campaign
once.** ch04, ch05's framing and compiler.html entry 23 move together in
that pass. Nothing here is a question for Clay; the entry stays as the
record of what the book owes and what it is waiting for.

### The maps parse is 100% of the compile row's binary-to-binary drift

**Cited: the ruling of 2026-09-03 (NO EXCLUSION; the toggle dropped, sorts
plus `setarch` shipped instead, kanso#1234) and the archive entry that
measured this — "the mechanism, named and accounted to the instruction",
which closes by saying the fact goes to the gavel rather than into a gate.
Nothing in a design doc or a spec speaks to it.**

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
published row by 2,130. The ruling was made when the term was known to exist
and not known to be the whole of the drift, which is the question this entry
asks: does the 2026-09-03 ruling stand on the new number?

**RECOMMENDATION: it stands, and this entry closes on a word.** The ruling
was that the row counts what the binary costs to start, term and all, and
0.27% is not a reason to reopen a decision made on principle. What the
number does change is how a session reads a 2,130 move on the row: as the
loader's, until `lang_start::{{closure}}` says otherwise.

**FILED WITHOUT A HEADING until 2026-09-08**, appended under Parked, where
the ledger's own navigation could not see it — sessions cite entries by
heading, STATUS.md indexes by heading, and neither could reach this one.

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
