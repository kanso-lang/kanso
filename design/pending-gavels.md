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

## Open, not blocking

### What spelling does "cyclic structures sized by data" need?

**Cited:** the archive's "block-born is the whole cohort" (2026-08-29), whose
words are *cyclic structures sized by data (a graph parsed from input, N
linked nodes from a map) gain a spelling*; the live log's build-hole gavel of
2026-09-16, which took back the two shapes that reached that purpose, on
reasoning this entry does not ask to undo; the 2026-09-09 entry building the
cohort as kanso#1359, which named birth through a call as the next widening
and left it to the implementer; and the live log's 2026-09-18 entry "birth
through a call, measured", which is the measurement STATUS.md's cohort row
has owed since 2026-09-16 and which is what raises this question rather than
answering it.

**The question.** The gavel's purpose needs a program to make N nodes, where
N comes from data, and tie them to each other. Five probes against a release
build of `30fb1abe` say that today it cannot, and they fail for five different
reasons:

1. a hole outside a `build` block is refused — so the maker cannot be a
   function;
2. a value a call returns is not block-born, so its field cannot be filled —
   this is the one that widening `born_of` would fix;
3. a hole cannot escape the block it was written in, because it must be
   filled before that block freezes — so the maker cannot be a `build` of its
   own either;
4. a lambda lexically inside the block is outside it for the hole rule, which
   closes the `map` shape the purpose actually takes;
5. `ns[0].field = ...` is a syntax error before any analysis runs, because a
   fill's target parses as a bare name — so N nodes need N names.

Two nodes named by hand still work, and `tests/golden/mem/build_cycle.kso`
pins that. What is missing is only the sizing.

**Recommendation, and it is a question about the spelling rather than a
build.** Widening `born_of` to see through a call is real, separable and worth
doing on its own terms, and it fixes exactly the second of those five. It does
not reach the purpose, so building it and calling the row closed would be
wrong. What the purpose needs is a decision about which of these to open:

- **a build block that iterates** — a form binding one name per element of a
  list, so N nodes get N births without N names in the source. This is the
  smallest change that reaches the words of the gavel, and it leaves the hole
  rule exactly as the build-hole gavel left it.
- **a hole that survives a call**, which means a function whose answer carries
  an unfilled field and a caller obliged to fill it. That is a second effect
  in the type, and a much larger language change.
- **the purpose retired**, with the gavel's sentence about data-sized cycles
  struck and the alias and the field of a born node left as what the cohort
  gained.

The holder of this file would open the first. It is the one that keeps every
rule the 2026-09-16 gavel established and adds a binder rather than an escape
hatch. But which of the three is Clay's, because the gavel's own words are
what is at stake.

**Worked examples, added 2026-09-30 at Clay's request.** Neither program
compiles today; each is a sketch of one spelling. Both build the same graph
from a map of edges, `a -> b`, `b -> c`, `c -> a`, and follow three links from
`a` back to `a`.

*An iterating `build` block.*

```
type node
  id
  next

links = { "a":"b", "b":"c", "c":"a" }

pub play =
  build
    nodes = { id: node id _ | id in keys links }
    for id in keys links
      nodes[id].next = nodes[links[id]]
  print "{nodes["a"].next.next.next.id}"
```

What it keeps: a hole is written only inside a `build` block, it is filled
before the block freezes, and it cannot leave the block through a call or a
lambda. What it changes: the number of births comes from data, since the map
line makes one per key; and a fill's target may be an indexed name,
`nodes[id].next`, which lifts the fifth of the five refusals above. "Filled
exactly once" can then no longer be proved by counting names. It holds when
the `for` walks the same keys that made the births, which the checker can
see; anything else is checked when the block freezes.

*A knot on a local binding.*

```
type node
  id
  next

links = { "a":"b", "b":"c", "c":"a" }

fn graph links
  nodes = list/to_h (list/map (keys links) (id -> [id (node id nodes[links[id]])]))
  nodes

pub play = print "{(graph links)["a"].next.next.next.id}"
```

What it keeps: there are no holes and no `build` block. The machinery is the
one a top-level constant knot uses today (`ring = { "a":ring }`): the
self-mention is stored as a cell, forced when it is read, and equality compares
by unfolding. What it changes: a local binding may name itself, which only a
top-level constant may do today, and the knot's size comes from data. A link
to an id nobody declared forces to `none` when it is read rather than failing
where the knot is made. A local that has to force itself while it is still
being built, such as `nodes = length nodes`, needs the same refusal a demanded
top-level knot gets.

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
