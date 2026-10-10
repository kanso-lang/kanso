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

### May four port fixes lower the welfare floor?

**Cited:** CLAUDE.md's "The welfare number only goes up", whose 2026-09-13
exception lets a change that builds a ruled part of the language lower the
floor without asking, and keeps every other fall with Clay. Searched the live
log, the archive and the ledger for a ruling on fixes that make the native
engines finish programs the interpreter already finishes: none. The nearest
case is kanso#1830, which lowered the floor under the exception because the
program crashed.

**The question.** Four pull requests fix native-only failures that port
journals reported. Each is green apart from the floor, and each costs welfare
through compile rows, not through any runtime vein.

- **kanso#1822, a read named before the write keeps the write in place.**
  `v = m[i]` followed by `put m i (v + 1)` copied the map on every put. sat's
  reduction falls from 27.5 s to 0.010 s natively. Welfare 90.25888 to
  90.25499, down 0.00389.
- **kanso#1824, a loop entered from a tail cycle gives its garbage back.** The
  lox port's 50,000-pass loop peaked at 3.0 GB natively and now peaks at
  77 MB (lox F18). Welfare 90.25811 to 90.25621, down 0.00190.
- **kanso#1820, a map's view takes keys out of order without moving itself.**
  `list/tally` over 40,000 distinct strings falls from 4.95 s to 0.21 s, and
  xsv's cardinality count and ugit's diff hit the same cliff. With main merged
  in, welfare 90.25153 to 90.25054, down 0.00099. That is inside the 0.001
  band the gate allows for disagreement between hosts, so CI passes it, but
  every term here is deterministic and the fall is real.
- **kanso#1825, a fold that joins onto its accumulator writes in place.**
  The mustache port held 2.77 GB rendering 20,000 pieces (mustache F14).
  Welfare 90.25811 to 90.25519, down 0.00292.

None of the four changes what a program prints. The figures are each
branch's own score against its own floor, and the four falls add to about a
hundredth of a point.

1. **Lower the floor for all four**, and widen the exception to cover any
   change that lets a native engine finish a program the interpreter
   finishes, so the next one does not wait.
2. **Lower it for these four only.**
3. **Hold them** until each pays for its compile cost elsewhere.

**Recommendation: 1.** Two of the four are programs that run out of memory
natively and finish on the interpreter, which is a divergence between engines
of the kind the differential law forbids. The other two turn runs of 27 s and
5 s into 10 ms and 0.2 s. Each fall is a few thousandths of a point.

## Open, not blocking

### May a list built from data hold `tie` references?

**Cited:** the live log's "gavel: a field may hold a list of `tie` references"
(2026-10-10), which admits a list or map literal and whose Owes asks the graph
fixture to drop its chain of records; "a `tie` node holds its references in a
list", which built it; `tests/golden/micro/a_graph_tied_from_a_map_of_edges`.
Searched the log, the archive, the ledger and the tests for a ruling on a list
of references a call builds: none.

**The question.** The graph fixture reads each town's roads from a map, so the
number of references a node holds is data. A literal cannot hold a number of
elements decided at run time, so under the ruling's text the fixture still
needs its chain of `road` records. The natural spelling is

    town name (list/map (roads_of edges[name]) ref)

which hands `ref` to `list/map`. The check refuses that today, because what a
function from std does with `ref` is out of its sight. `list/map` stores each
answer and reads none of them, so the same argument that admitted the literal
would admit this call.

1. **Admit `list/map xs ref`, and `list/map xs (k -> ref k)`, written as a
   field.** The check names `list/map` as a function that stores what its
   function answers, and the graph fixture drops its chain.
2. **Leave it at literals.** A node whose links come from data keeps the chain.

**Recommendation: 1.** A graph read from data is the case `tie` exists for,
and `list/map` is the one call that builds such a list without reading what it
holds.

### Does `build` still earn its place beside `tie`?

**Cited:** the live log's "gavel: a data-sized cycle is tied with `list/tie`,
and `list/tie!` insists" (2026-10-04), which leaves this open; `docs/book/ch03.html`,
whose "two records that point at each other" teaches `build` as the way to make
a cycle; `docs/book/appa.html`'s `error[build]` family; the build-hole gavel of
2026-09-16; and the mem vein's `build_cycle.kso`. Searched the log, the archive
and the ledger for any ruling on retiring `build`: none.

**The question.** `tie` makes any cycle `build` makes. With literal data the
ada-and-bob ring is

    pair = list/tie! ["ada" "bob"] (n ref -> person n (ref (other n)))

and the compiler can see the whole graph, so a broken link is refused at check
the way an unfilled hole is. If that holds, `build` has nothing left that only
it can do, and kanso has two ways to write a cycle. Clay, 2026-10-04: "it would
seem that you don't need build anymore because you can use tie with inline data
structures... the only issue is then I'm not sure if you can get the static
analysis to tell you at compile time if there's a problem. in that case build
would be useful."

1. **Retire `build`** once `tie` is built and its compile-time check on literal
   data is pinned. Chapter 3 teaches the cycle with `tie`, the `error[build]`
   family and the hole `_` leave the language, and the build goldens move to
   `tie`. One way to write a cycle.
2. **Keep both.** `build` for a fixed handful of nodes written by hand, `tie`
   for nodes counted from data. Two spellings, each with a case the other
   handles less directly.

**Recommendation: 1, decided after `tie` lands.** The only argument for keeping
`build` is earlier error detection, and with literal data `tie` gives the same
detection. The ruling should wait for the build so it rests on the check
working rather than on the expectation that it will.

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
