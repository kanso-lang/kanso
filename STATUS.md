# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

The decisions live in design/pending-gavels.md — the single ledger; this file
only indexes it. **Blocking right now: one — "What a digest costs, and whether it stays
written in kanso".** `sha256/hex` holds the whole message; peak arena is linear
in the input at about six and a half thousand bytes per byte hashed, and the
asset-digests job's headroom falls as `docs/kanso.wasm` grows.

**Fourteen questions are waiting, and every one of them carries a
recommendation** — one blocking, thirteen open but not blocking. The July
letters are closed: Clay ruled the last five in one sitting on 2026-08-26, and
that ruling reached main only on 2026-08-28, on a branch that had been sitting
unmerged. Counted from the ledger rather than carried forward. That is the point of the 2026-08-25 sweep: Clay's ask was
*"the goal here would be to not have anything left to gavel. I feel like I keep
gathering things and then you ask me the same question 10 to 20 more times."*
So each entry now cites the search behind it and proposes an answer, and a
sitting can be a yes or a no rather than a fresh design conversation.

**The thirteen open, not blocking** — whether a `compile_instructions` move that
cannot be work needs a spend attribution, asked after three such moves in two
days; whether `read_file` is text or bytes, which
is one reader with no way to say which you meant and two engines that answer
differently; which claim owns a qualified name when a module declares one of
its imports' names, whose filing pointed at a gavel that has since fallen; what
a record prints as when its module is imported;
`--explain-copies`; the assert hako's surface; the
bare-call cross-module tie whose interim ruling is still live and still called
interim; whether a dependency's render arms join the root group; `first coll n`;
where `std/` comes from; block-born as a dataflow property; and the
ten-thousand-frame guard, which is a standing offer rather than a question.

**The July letters are closed.** C struck, `done` minted for D, G struck on the
July provenance measurement, Z confirmed declined, AA explicit-cast only. Every
letter A1–X, BB, C, D, G, Z and AA has a ruling in the log or the archive.

Six candidates the sweep turned up were **already answered by shipped code**
and went to the log rather than the ledger, so they cannot reach him again: the
lambda-parens rule, `next`'s signature, cyclic rendering and equality, `run`
versus `play`, the three small July spellings, and the write-once marker.

Three rules now guard the file. An entry **cites its search** of the log, the
archive, every design doc **and the tests**, or it is invalid — an unsearched
question does not go to Clay. The third source was added on 2026-08-27, after
two questions turned up that the 2026-08-25 sweep had missed because each was
recorded only in a spec: `module_differential`'s known-defect ledger carried
one, and an `#[ignore]` reason in `tests/entry_file.rs` carried the other,
whose log entry ends "That is a gavel." A sweep that reads only prose cannot
see a question a test is holding. And an entry **carries a recommendation**,
because a question with no proposed answer turns one sitting into ten.

## In flight

Nothing. Every branch this session opened is merged and verified on
`origin/main`, and neither kanso nor kq has an open pull request.

**One thing is blocked on access rather than on a decision.** The ironclad
branch directive of 2026-08-26 reached main on 2026-08-28 in #1112, having sat
unmerged on the branch that carried it. Its audit clause ran on 2026-08-29 and
found 324 branches on origin, growing at about the merge rate; the twenty most
recently touched are all squash-merged pull requests whose content is on main,
so nothing unfinished is hiding there. The purge and
`delete_branch_on_merge=true` both need push-delete rights this container does
not have — the git proxy refuses a delete refspec, the GitHub MCP has no
delete-branch tool, and there is no `gh`. Setting the flag first is the half
worth doing: it stops the class recurring.

## What landed on 2026-08-27 and 2026-08-28

    kanso fa6b10d0  #1100  the scan reads eval.rs: 175 diagnostics became 242
    kanso d887d913  #1099  `if` and the guard say one sentence on all three
    kanso 0c958475  #1098  `cannot destructure` diverged on three engines
    kanso 7c173bd4  #1101  the page said "this value" about two things
    kanso d2177f3a  #1102  six val sites, not four: a constructor and an
                           interpolation too
    kanso aec580f8  #1104  the walk could not say how much it had walked
    kanso a705979b  #1105  the page refused before three sites could explain
    kanso 3a7e76c5  #1103  two refusals named each other, and a reader had
                           nowhere to go
    kanso 61af582a  #1106  the page answered `1 + d` with `d`
    kanso 9441e3f3  #1107  three more of the same family
    kanso d45c4f17  #1108  the scan reads the browser engine: 242 became 262
    kanso ad851b12  #1109  the page catches up: §27, the answer not the sentence
    kanso 5f53290d  #1110  the log holds forty entries again

**The through-line is one accessor.** `val` in `src/wasm_rt.rs` answers a value
for a value and a handle for a closure and refuses everything else in its own
words. Any site that opens with `match val(h)` and owns a sentence about what
it was handed therefore never reaches that sentence, and any site that means to
CARRY a description refuses it instead. Both halves were live, and the second
half had a case in it that was not a sentence at all: `1 + d` handed the
operand's own handle back, so the expression evaluated to `d` and the page
printed `<io>` where the other two engines refuse.

That one is the finding worth carrying. Every diagnostic gate this project has
compares refusals, and the program does not refuse; the error corpus pins what
a program writes to stderr, and it writes to stdout. The three-engine walk,
which compares what each engine PRINTS, is the only instrument that could have
seen it — and it could only see it because #1104 had just taught the walk to
account for every program in the corpus.

**The method that found the rest.** Following the failing program found four
sites. Sweeping the accessor's own call sites — reading each one and asking
what a program could hand it — found six, then a refusing family, then a
carrying one. Where the sweep was wrong it was wrong in a specific way: twice a
site was written off as unreachable on the strength of a probe that ran a
DIFFERENT program. `(opaque d).nope` is a name error and never reaches the
runtime; `(opaque d).n`, where some record declares `n`, reaches `rt_no_field`.
A site is unreachable only when a program written to reach IT fails to.

**And the scan finally reads the engine a reader meets first.** `src/wasm_rt.rs`
writes 36 `die(` sites spelling 24 sentences, on the engine the website's
playground runs, and no gate had read a word of it. Two openers took the count
from 242 to 262, and both carry a ratchet mutation watched turning the gate
red. #1105 had claimed in a comment that the scan already watched two
hand-copied sentences for drift; it did not, and could not have — the copies
are gone now anyway, dissolved by #1106's placeholder.

## Next, and the handoff

**This session hands off here.** The directive of 2026-08-26 — merged to main
only on 2026-08-28, in #1112, having sat unmerged on the branch that carried
it — says the running cloud session makes no further claude-code-remote tool
calls, because it predates the checked-in allowlist and every one of them
prompts Clay's phone. It also says the handoff happens at the natural boundary,
current PR queue drained, and that a session started fresh inherits the merged
`.claude/settings.json` and runs silent. The queue is drained. Nothing else
transfers, because the repo carries it.

One catch worth knowing: creating the successor is itself a
claude-code-remote call, so the running session cannot start it. Clay does.

**The biggest buildable thing is now the err spelling, and it is ruled.**
Gaveled 2026-08-26 and on main since #1112:

    io/read_file path
    bind     (text -> json/parse text)
    annotate (e -> "config: {e.reason}")
    rescue   when_failed

Three words, all ordinary two-argument functions — `bind effect callback` —
threaded by the chain rule already in the language. The callback receives the
err itself, so a dispatch group is a legal callback. Annotate cannot resurrect
by construction. Rescue is the sole door, its foreign-only license checked at
the word. `.` retires from chain-step position; field access is untouched.

**All three words work on all three engines** as of 2026-08-29 (kanso#1116).
`tests/golden/chainwords/` holds four cases, pinned on both engines against
one golden, and three mutations were watched turning the right case red.

`annotate` needed no new description field: the runtime builds its wrapper
closure itself, over the callback and the site, and hands it to rescue's node.
`k_closure` was already there for that shape.

**The page was diverging in silence and nothing in the tree could see it.**
It compiled `rescue` and then propagated the failure the other two engines
catch, because the wasm backend has no node for the words and the names fell
through its generic call path rather than its `unsupported call` arm. Every
chainwords case needs a filesystem to make an effect fail, and a page has
none, so no program the page could run reached the words.
`tests/golden/micro/a_settled_failure_meets_the_three_words.kso` closes that:
it reaches them through a subject that has already failed, with no filesystem,
so all three engines run it. It went red on the wasm harness on its first run,
and the page was built rather than left refusing. The general lesson is worth
carrying: the engine with no world is the one that most needs a case, and a
fixture that fails an effect cannot be that case.

One narrow hole is written down rather than assumed absent: the page's
`as_desc` answers None for a rescue or annotate slot, so a worded step other
than `bind` inside a `join` is refused there rather than scheduled. Nothing in
the corpus reaches it.

`bind`, `rescue` and `annotate` are reserved names now. Two fixtures in the
corpus had defined functions by two of those names and were renamed, which is
a fair measure of how ordinary the words are.

**`rescue` was new capability, not a respelling** — the finding that reshapes
the rest. An execution-time failure could not be handled inside a chain at
all: a chain step over an effect wraps its expression in a one-parameter
closure, and handing a failure to a closure returns the failure rather than
entering the body, so the callback was never called whatever its shape.
`docs/book/samples/ch05/fallback.kso` is that program and ch05 teaches it as
the design. So the migration's err-arm half is empty for a stronger reason
than the greps below gave: the chain err-arm was never a working surface, and
the one site that looks like one is the book's counter-example.

**What is left is one pass over the lambda steps.** 485 loose-dot steps in
fleet code with comments and strings stripped: 346 are `. (lambda)`, the
monadic steps that respell as `bind`, and 139 are `. named_fn`, the threading
form the effect-first rider preserves. (An earlier count of 309 counted
differently and is superseded.) Most of the 346 are in `scripts`, which is the
gates, so a botched pass there takes CI down rather than a user program.

**It cannot start until Clay rules on the line grammar.** The gavel's sample
is dotless — `bind (text -> ...)` — while the same paragraph says the first
argument comes from the rule that makes `(expect 1) . to (equal x)` work, and
that rule is the dot. Dropping the dot collides with the rule that an indented
line under an argument-taking statement is one more argument (`src/lexer.rs`),
which the leading `.` is currently the only thing distinguishing — so the
dotless form needs the parser to know the three words, contradicting "nothing
about them is syntax". Filed.

**Two things the gavel's own sample needs and does not have.** Its lambda form
reads `e.reason`, and an err has no such reader — nor could a lambda use one,
since every operation on an err propagates it. A group callback destructures
instead, which is the gavel's primary story. Whether an err gains readers that
get past its own infectiousness, the way `wrap_err`'s second argument does, is
open.

**`done` is the second, smaller one.** Letter D was ruled in the same sitting:
a succeeded effect yields `done`, not `none`. `none` means absence and nothing
else. Chains that tested for `none` after an effect migrate.

**What needs Clay, and cannot start without him:**

  - the one blocking question, what a digest costs, filed with its measurement
  - `delete_branch_on_merge=true` and the 324-branch purge (task #109), both
    checked from here on two days and both refused by the tooling
  - the chain-line grammar above, which blocks the 346-site respelling
  - whether an err gains readers a lambda callback can use

**Sixteen questions wait in `design/pending-gavels.md`** — one blocking,
fifteen open — each with a recommendation, counted from the ledger rather
than carried forward. Two are new today: the chain-line grammar and whether
an err gains readers a callback can use.

**The rules that carry forward**, each earned twice:

  - a scan over a corpus enumerates the corpus's file types from the harness
    that reads it, never from the ones that came to mind
  - a claim that a site cannot be reached is only as good as the program
    written to reach THAT site — `(opaque d).nope` is a name error and proves
    nothing about `(opaque d).n`
  - a branch with a ruling on it is not a record until it merges. Four gavels
    sat unread for up to thirty-two hours, and this session reported the tree
    settled while they did.
