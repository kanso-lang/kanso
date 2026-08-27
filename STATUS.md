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

**kanso#1091** — a micro program's stderr is part of what it does. `golden.rs`
read `"out"` for the micro corpus and never `"err"`, so `io/write_err`'s success
case was pinned on no engine. It also carries two negative results, below.

## What landed on 2026-08-27

    kanso 7be1ed9c  #1090  two ways the page's endpoint got an err wrong
    kanso 36dcd74e  #1089  clang is installed, and that was never the question
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

**Two ideas died by measurement today, and both are recorded** so they stay
dead. Neither was a guess that looked wrong; each was a recommendation about to
be written down.

**The bare-name wall check.** `never_describes` refuses a literal or a direct
call on either side of `>>` and not a bare name, so `io/write "one" >> x`
reaches the run. `tests/wall_takes_effects.rs` made this same extension once
before, from literals to calls, and the lookup for a name is the same one at
arity zero — so it read as a line of code. It is not. A local shadowing a
BARE-ENROLLED import is legal, and this prints `one` then `shadowed` and exits
0 today:

    naturals = io/write "shadowed\n"
    io/write "one\n" >> naturals

`returns` is built from all of `program.fns`, synthetic clones included, so the
lookup finds `list/naturals` and the local is invisible to it. check.rs:1085
states the rule that breaks: "the enrollment must never make every stdlib
export a forbidden binding name." The open question is whether to give
`never_describes` and its sibling `refused` a set of the names bound in the
function they are walking — cheaper than full scope and possibly enough. That
program is the case that must not be refused, and belongs in the corpus either
way. The runtime guard shipped in #1090 covers the behaviour meanwhile.

**A coverage gate over the std surface.** `the_wasm_engine_complains_the_way_the_others_do`
walks every `pub fn` in `lib/` and hands each the wrong arguments; nothing walks
the surface the other way, so gating the success half looked like the obvious
build. There is nothing to gate: 100 of 100 exports are reached. The first
answer was 24, and the arithmetic of being wrong is worth keeping —

    qualified name only, across the corpus dirs     24 uncovered
    plus the bare-enrolled form                      1 uncovered
    plus intra-library calls                         0 uncovered

Two searches this month have failed the same way: the coverage scan's
twelve-character floor hid three reachable diagnostics, and this hid eight
reachable exports. A name here has two written forms, and a search that knows
one is measuring its own blind spot.

**What today's bugs actually shared** was never an uncalled function. All three
lived in a function the corpus calls constantly, and what was wrong was the
ENDPOINT around the call: what the exit code carries, which stream the bytes
land on, which sentence names the fault. That is where to look next, and
surface coverage would have found none of them.
