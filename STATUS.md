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

**kanso#1098** — a destructuring bind of the wrong shape said three different
things on three engines, and native's was the poorest: it dropped both the
value the reader bound and the sentence saying why a bind cannot fail over to
another arm. Native and the page gain them.

## What landed on 2026-08-27

    kanso f1aecf5d  #1095  the ratchet reads the C now, and twelve have reasons
    kanso 43526dc7  #1097  the wasm gap list pinned a prefix
    kanso c0b1d066  #1094  six socket failures said seven different things
    kanso 080deb09  #1093  eight native runtime messages that nothing pinned
    kanso fe52d516  #1092  a bare name at a wall, refused at compile time
    kanso 54914c40  #1091  a micro program's stderr is part of what it does
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

## The sweep this file asked for, and what it found

The last version of this section proposed one more pass over
`tests/golden/runtime` for a construct whose SUCCESS case is pinned nowhere.
The pass ran. The answer was not a fifth instance — it was a gate that could
not see a whole file.

`scripts/diagnostic_coverage` is the ratchet that stops a compiler message
being reworded, weakened or lost with nothing going red. Its `source?` admits a
file only when its last three characters are `.rs`, so it had never read
`src/runtime.c` — where a COMPILED BINARY gets its runtime messages. **Thirty-
three of the sixty-six `k_die` texts there were pinned by nothing.**

The arithmetic family is the shape in miniature: `+` on two values that have no
`+` is pinned twice, and `-`, `*`, `/`, `%` were pinned nowhere, all four
reachable by the idiom the `+` fixture already uses. One of five.

Twenty-one are pinned now across #1093 and #1095, the scan reads `.c`, and the
twelve that remain carry a mechanism apiece rather than a shrug.

## Next

**Four findings from the sweep are still open, each its own change.**

A FIFTH FILE THE RATCHET CANNOT SEE, and it is the oracle's. `src/eval.rs`
raises 98 `RuntimeError`s, and a `RuntimeError` carries a bare `message:`
field — none of the scan's four openers. Reading the 72 with enough literal
text to match against the goldens leaves 14 that nothing pins. Every one was
then run rather than read, which sorted them:

  - five are reachable and the two native engines agree on them, so each wants
    a fixture: the keyed read of a non-record, `if`'s non-bool condition,
    `sleep`, `random` and `round` on a value of the wrong type;
  - seven are shadowed, each by a check that fires first, and the probe that
    established it is the excuse: a `set` target must be a construction born
    in the same `build` block, so it is bound and it is a record; a bind
    pattern is only ever a name, a constructor or a keyed read, because
    `_ = ...` and `7 = ...` are `expected a binding name or type`;
    `list/select` routes its predicate through `if`; a mixed-type `sort` is
    refused by `comparison requires two values of one comparable type`;
    `builtin_nope` outside the standard library is refused by name; and a
    foreign closure handle is built only by `wasm_rt.rs`, which installs the
    way back at init;
  - and one is a DIVERGENCE. `return x if cond` on a non-bool says three
    different things: the interpreter's `a return condition is true or false,
    got 7`, native's `an if condition is true or false, got 7`, and the page's
    `if takes a bool condition (got "7")` — a third wording, and quoted where
    the other two are not. The plain `if` builtin agrees across native and the
    interpreter, so it is the guard alone. Converging on native's sentence
    keeps `k_truthy` in one place, which is what runtime.c says it is for, and
    #1094 measured what a second wording costs: 128 bytes of `.text` in every
    binary, for two characters.

A FOURTH WAY THE COMPILER WRITES A MESSAGE, after the three #1079 found.
`codegen.rs` bakes text into the emitted binary through `format!`. #1098 fixed
one of the two — destructuring a value as a named type — and the other, the
one naming which types a field takes, turns out to be DEAD: `check_field_annotations`
refuses every field annotation, so nothing reaches the emitter's half. Deleting
it is not obviously right, because it is the emitter half of a feature the
checker currently declines rather than a mistake. Widening the scan to `format!`
in general would match every format string in the compiler, so a key for this
family still needs finding.

THE SCAN'S CORPUS TAKES `.stderr` ONLY. `integer overflow` is pinned exactly,
by `docs/book/samples/ch02/overflow.out`, and the gate cannot see it. Admitting
`.out` would also admit every micro golden, and the tests/*.rs corpus has
already produced four false pins, so it needs its own look before it lands.

THE OTHER ENGINES HAVE THE SAME FILE PROBLEM, and #1097 answers half of it.
The scan reads `.rs` and `.c` now, and `src/wasm_rt.rs`'s refusals are
`Err(String)` rather than any of the four openers, so it sees none of them.
What pins them instead is tests/golden/wasm_gaps.txt — which listed a PREFIX
where the page writes a sentence, leaving the half that names what the page
could not do free to be reworded. #1097 tightened every row to the whole
sentence, measured both ways: reword `cannot read` in wasm_rt.rs and the old
list passes while the new one fails. Whether the scan should read wasm_rt.rs
as well is still open.

AND THE PAGE WRITES MESSAGES OF ITS OWN, which #1098 found the hard way. It has
a whole emit path — wasm_backend.rs into wasm_rt.rs — parallel to codegen.rs
into runtime.c, so a sentence can live in THREE places and drift in any of
them. Fixing native's `cannot destructure` left the page saying the old words,
and only the corpus walk caught it.

**And the method is worth carrying forward more than the findings are.** Every
mechanism in the excuse list was established by running a program, because
three separate readings of the source were wrong on the way:

  - matching message tails against the Rust sources reported thirteen messages
    "no Rust engine can produce"; three were `format!("{name} takes a string")`,
    which the interpreter says every day;
  - the six `net_*` refusals looked shadowed by the wrappers' `.handle` read,
    and a record whose handle is a STRING reaches every one of them;
  - the runtime corpus looked like the obvious home for the socket specs until
    `cargo test` named a THIRD engine walking it, whose executor has no sockets
    and refuses differently.

  - the scan that found the wasm gap read `.stderr`, `.out` and `.rs`, and
    wasm_gaps.txt is a `.txt`, so it called two pinned messages unpinned.

A search that knows one written form of a thing is measuring its own blind
spot, and four of them did it in one day. The rule that follows: a scan over a
corpus enumerates the corpus's file types from the harness that reads it,
never from the ones that came to mind.
