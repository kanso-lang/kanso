# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

The decisions live in design/pending-gavels.md — the single ledger; this file
only indexes it. **Blocking right now: nothing.**

**Sixteen questions are ready to rule, and every one of them carries a
recommendation.** That is the point of the 2026-08-25 sweep: Clay's ask was
*"the goal here would be to not have anything left to gavel. I feel like I keep
gathering things and then you ask me the same question 10 to 20 more times."*
So each entry now cites the search behind it and proposes an answer, and a
sitting can be a yes or a no rather than a fresh design conversation.

**Eleven open, not blocking** — welfare's blindness to compile traffic; the two
surviving err-gavel riders (the annotate/rescue spelling, and construction
enforcement); whether an own-origin err arm is dispatch semantics or the
advisory that ships today; `--explain-copies`; the assert hako's surface; the
bare-call cross-module tie whose interim ruling is still live and still called
interim; whether a dependency's render arms join the root group; `first coll n`;
where `std/` comes from; block-born as a dataflow property; and the
ten-thousand-frame guard, which is a standing offer rather than a question.

**Five July letters** — C (pure/yield), D (what a succeeded effect yields), G
(eta-reduction as canon), Z (errors without exceptions), AA (newtype dispatch
acceptance). Three of the five are recommended for one word: strike, confirm
declined, strike.

Six candidates the sweep turned up were **already answered by shipped code**
and went to the log rather than the ledger, so they cannot reach him again: the
lambda-parens rule, `next`'s signature, cyclic rendering and equality, `run`
versus `play`, the three small July spellings, and the write-once marker.

Two rules now guard the file. An entry **cites its search** of the log, the
archive and every design doc, or it is invalid — an unsearched question does
not go to Clay. And an entry **carries a recommendation**, because a question
with no proposed answer turns one sitting into ten.

## In flight

**kanso #1027 — the history row is checked on the way out, and before a merge.**
The second half of the interpolation miscompilation: `docs/numbers.html` had
two trend series and two panel sections empty for months because five groups of
counter keys were being run together into one key each, and `site_smoke` could
not see it because its fixture is hand-written and supplies the keys itself.
`perf_record` now checks its output against the same lists it checks its inputs
against, and the step that builds the row runs on every pull request instead of
only after a merge. That promotes the perf-history job out of the ratchet's
unproven list.

## What landed on 2026-08-25

    kanso ca5dc614  #1026  four gavels, the archaeology, the residual list,
                           a miscompilation, and the band gavel built

**Four gavels reached the record** — no tolerance bands, a demanded knot counts,
a build hole is spelled `_`, and `>>` stops at the first run-time failure
(which was never open; July's B ruling had answered it).

**The band gavel is built.** `bench/compile_memory_golden.txt` is exact for the
host its `measured-on` line names, and the figure corrected to the runner's
864,300 — not the 872,061 the ruling recorded, which predated twelve merges.
Welfare 84.87 to 84.89, banked. The gate had carried no ratchet mutation at
all, which is how it could rot; it has two now.

**A miscompilation, found from a symptom.** An interpolation seeded by a
captured parameter shared one buffer across map iterations, so native printed
`["abc" "abc" "abc"]` where the oracle printed `["a" "b" "c"]`. The first fix
was too broad and cost pendbench 2% for no correctness; the parser already drew
the distinction, between a lambda applied where it stands and one handed to a
consumer. Final cost 727 front-end instructions.

**Three results recorded so they stay recorded**: the lexer's names cannot move
into the AST (declined, with the reason, on the compiler page); `valid_utf8` is
cancelled by the gavel that superseded the plan carrying it; and the interner's
churn is measured — wide in count, narrow in kind.

## Next

**The read half of gavel 1b** is the largest remaining piece, and it is sized
rather than guessed: a field the WRONG type declares is caught at run time, not
before, because `Set` in src/infer.rs is fourteen kind bits with no type
identity. The work is a per-expression record-type set, sourced at constructors
and carried by the fixpoint the value sets already use. It closes a correctness
gap older than the privacy question — the doctrine says this language refuses
before anything runs, and `has no field` does not.

**kanso#985** is the implementation of gavel 1b's json half, not a decision.
One log conflict from ready; `merge-tree` says everything else applies cleanly.
