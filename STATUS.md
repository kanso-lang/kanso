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

**Eighteen questions are ready to rule, and every one of them carries a
recommendation.** That is the point of the 2026-08-25 sweep: Clay's ask was
*"the goal here would be to not have anything left to gavel. I feel like I keep
gathering things and then you ask me the same question 10 to 20 more times."*
So each entry now cites the search behind it and proposes an answer, and a
sitting can be a yes or a no rather than a fresh design conversation.

**Thirteen open, not blocking** — whether a `compile_instructions` move that
cannot be work needs a spend attribution, asked after three such moves in two
days; whether `read_file` is text or bytes, which
is one reader with no way to say which you meant and two engines that answer
differently; the two
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

**kanso#1089** — clang is installed, and that was never the question. Went
green on 19 checks, then #1088 landed and made it conflict on the log and the
excused list; main is merged in, the log entries reordered by merge date, and
CI is running again on the merge commit.

**kanso#1090** — two ways the page's endpoint got an err wrong. It could not
read an `os/exit_status`, so a program that exited deliberately printed
`unhandled err reached the executor` at its reader and answered 1 whatever code
it named; and a wall whose right side answered a plain value made it say "main
is not an io" where the other two engines say "`>>` sequences two effect
descriptions". Both pinned on all three engines, both watched red at their own
sources.

## What landed on 2026-08-27

    kanso 0d293915  #1088  the excuse with no reason, and the sitting it cost
    kanso f5a721b5  #1087  a build body dropped the effects written in it
    kanso dc353927  #1086  the last excuse, and a ratio measured further out
    kanso 7a9cf9a9  #1085  a file that is there, readable, and not text
    kanso 7479b308  #1084  the digest gate's excuse was wrong the same way
    kanso 04e93b07  #1083  what a digest costs, measured
    kanso e90d13e0  #1082  the page can read its own arguments now
    kanso 2d22ad11  #1081  the kq row's excuse was wrong about the mechanism
    kanso 6f9ad496  #1080  three things a page does that no program asked it to

**The through-line is a claim written from reading and never tested.** Six
excuses were audited across two lists. Four described the wrong obstacle — the
kq row's dependencies (#1081), the digest job's build step (#1084), the macOS
job's mutations (#1086), clang's installation (#1089). One reasoned correctly
about one walker when there were two, which is how an effect written in a
`build` body came to be dropped in silence on both engines (#1087) — the day's
one real bug from that seam. One was sound and had simply never been written
down (#1088).

The same shape produced the rest. `read_file` reported a file that was there
and readable as absent, because the ErrorKind was thrown away (#1085). The page
answered `unknown builtin` for `io/stdin` and `os/args` while `time/now`,
declared identically, reached the executor (#1082). And a digest's peak arena
turned out linear in the input at about 6.5 kB per byte hashed (#1083), which
is the blocking question above.

## Next

**A deliberate exit is not the only thing no corpus can express.**
tests/golden/runtime asserts every program in it exits 1 and tests/golden/micro
asserts every program in it exits 0, so any behaviour whose observable end is
some other status has no home and is invisible to the three-engine walk. Worth
knowing what else is in that shadow.

**`main is not an io` at src/wasm_rt.rs:1132 is answered.** It was never the
wasm twin of the driver message #1079 pinned in `tests/a_plan_needs_an_io.rs` —
this file said so for two days and the two are different messages on different
paths. It is `exec_slot`'s catch-all, and a program CAN reach it: a bare name
on the right of a wall (`x = 2` then `io/write "one" >> x`) slips past
`never_describes`, which refuses a literal or a direct call and not a name.
Native and the oracle said "`>>` sequences two effect descriptions"; the page
said "main is not an io". Fixed and pinned in the same PR as the deliberate
exit.

**Whether the CHECK should catch that instead** is the live question.
`tests/wall_takes_effects.rs` already extended this refusal from literals to
calls, on the stated ground that "a call to a function that can never answer an
effect is the same case one step out, and the fixpoint already knows which
calls those are". A bare name is that case one step further out, and the
fixpoint is keyed by (name, arity) so arity zero is the same lookup. Measure it
before writing it down: it changes what the language refuses, and the runtime
guard is needed either way for a piped call, a non-`Ident` head and a
parameter.
