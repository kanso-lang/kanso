# Status

What I am doing, and what is waiting on Clay. Rewritten at every stopping
point; if it disagrees with the task list, the task list is right and this file
is stale — say so.

## Waiting on Clay

The decisions live in design/pending-gavels.md — the single ledger; this file
only indexes it. **Blocking right now: nothing.**

**Eighteen questions are ready to rule, and every one of them carries a
recommendation.** That is the point of the 2026-08-25 sweep: Clay's ask was
*"the goal here would be to not have anything left to gavel. I feel like I keep
gathering things and then you ask me the same question 10 to 20 more times."*
So each entry now cites the search behind it and proposes an answer, and a
sitting can be a yes or a no rather than a fresh design conversation.

**Eleven open, not blocking** — the two
surviving err-gavel riders (the annotate/rescue spelling, and construction
enforcement); which claim owns a qualified name when a module declares one of
its imports' names, whose filing pointed at a gavel that has since fallen; what
a record prints as when its module is imported;
`--explain-copies`; the assert hako's surface; the
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

**A stack of three, bottom to top: #1077, #1078, #1079.** Each merges on green
and the two above it are rebased; /tmp/restack.sh and task #71 hold the
procedure and the traps.

They are one arc, and it is about a gate that could not see what it was gating.
The diagnostic-coverage scan keyed on `Diagnostic::new(`. The compiler writes
errors three ways, and the other two were outside it entirely: the loader and
driver's plain `error: ` text, and `error[kind]: ` written as plain text, which
is what a RENDERED diagnostic looks like. **84 → 109 literal
diagnostics.** #1077 turned two ratchet excuses into mutations; #1078 pinned
four loader refusals by hand on the module surface, because the gate could not
reach them; #1079 makes the gate reach them.

## What landed on 2026-08-26 and 27

    kanso 32d29a05  #1075  two of the ratchet's excuses were to-do notes
    kanso 82a90a07  #1074  the page owes an entry: a gate's exceptions are claims
    kanso 56fd2ef9  #1072  the check a retired convention left behind
    kanso 480ab566  #1071  three diagnostics the coverage gate could not see

**The through-line is a claim written from reading the source and never
tested**, and four of them were wrong. The coverage gate's twelve-character
floor hid three reachable, unpinned messages. An excuse claiming an indented
top-level line is always taken by another rule holds only when a declaration
precedes it. `check_unused_private` walked every declared function and type on
every compile asking whether the name began with an underscore, which the lexer
had stopped allowing — it could not answer yes, and the deletion cost 7,495
front-end instructions. Three ratchet excuses were readings rather than
measurements; running the browser sweep in a detached worktree refuted two of
them, and `tests/playground.rs` reading `docs/play.js` refuted the third.

**Also landed**: the qualified-name clone measured (#1063), eight programs'
emitted code counted (#1064), one cell of five with two wrong answers on the way
(#1066), four acceptance tests that had quietly started passing (#1067), a file
module that could be imported and could never import (#1068), and two questions
the residual sweep could not see because each lived in a test (#1069).

## Next

**The wasm engine's own refusals are pinned nowhere** (task #77), found by the
same method one surface further out: it writes `Err("...")` strings, which none
of the three openers reaches. `the playground has no stdin` and the wasm twin of
`main is not an io` appear nowhere but inside the compiled `docs/kanso.wasm`.
Whether the answer is a fourth opener or spelling them like the others is an
open question the build should settle by looking at what the browser shows.

**The read half of gavel 1b has shipped** — a field the wrong type declares is
refused at compile time now (`src/check.rs:1449`), not left to run time. This
file called it the largest remaining piece for two days after it landed.
