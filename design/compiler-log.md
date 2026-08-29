# Compiler log

> # ⚠️ THIS FILE IS APPEND ONLY ⚠️
>
> **Never edit or delete an existing entry. Only ADD new entries at the bottom.**
>
> Every performance/memory approach considered, decision made, thing
> tried-and-reverted, and thread left open goes here — so no thread is ever
> silently dropped again. (The dead-reuse thread in the first entry is *exactly*
> why this file exists: a prior session wired `linear.rs` to nothing and no one
> noticed for weeks.)
>
> Newest entries at the bottom. Date every entry. Tag each item:
> **OPEN / DONE / REVERTED / REFUTED / SPECULATIVE**. When you close an OPEN
> thread, do NOT edit it — append a new entry that references it.

---

> The last forty entries. Everything older is in `log/compiler-log-archive.md`,
> unedited — go there for a thread this file does not mention, and search it
> before concluding an idea is new.

> Eight entries dated 2026-08-26 sit at the top: they were written on a
> branch that did not merge until 2026-08-28, so they arrive after the
> entries below them were filed. They are placed in DATE order, which
> puts this file at forty-seven until the next trim.

## 2026-08-26 — ironclad: branches do not hang out, and old work finishes first

Clay, verbatim, after finding a day-old and a four-day-old branch
sitting on origin: "there needs to be an ironclad rule that prevents
branches from just hanging out... what the fuck is any process doing
making a new PR when there's an existing one there that clearly is
older." An inventory taken on the spot found roughly 250 branches on
origin, the oldest from July 13 — most of them squash-merge litter
whose PRs landed weeks ago and whose head branches nobody deleted, and
on top of that the genuinely unfinished ones aging in place.

The ironclad rule, three clauses:

- **FIFO, corrected from the earlier 24-hour misstatement.** The rule
  was never an age limit: no session starts a new branch or a new PR
  while an older open PR or unfinished pushed branch of its own
  remains. Oldest first, driven to merged-on-green, superseded, or
  closed with the reason recorded — then the next thing begins.
- **A merged branch dies at merge.** Turn on the repository's
  delete-branch-on-merge setting (gh api -X PATCH repos/kanso-lang/kanso
  -f delete_branch_on_merge=true, and the same for kq) so the litter
  class cannot recur, and run the one-time purge: every branch whose PR
  is merged or closed is deleted; a branch with no PR and no unlanded
  content is deleted; a branch with real unlanded content either gets
  its PR opened now or its salvage recorded as a task — nothing stays
  parked.
- **Every check-in audits the branch list**, not just the PR list. A
  branch on origin is either main, or actively being driven to done by
  the FIFO. There is no third state.

This entry rides the oldest unfinished branch this session owns, which
is the rule applied to itself.

## 2026-08-26 — the branch rule, refined by Clay: parallel is fine, abandoned is not

Clay refined the ironclad within the hour, and the refinement loosens
the serial reading: "you can have multiple agents working at once, and
if that is the case it's fine to have one agent working on a new issue
as long as there's at least one agent currently working on closing off
any existing issues. you should be checking for ensuring that nothing
is abandoned before you commence something new."

So the rule is not strict FIFO across the fleet. It is an abandonment
check: before any agent commences new work, verify that every existing
open item — branch, PR, task — is either actively being driven by some
agent or explicitly closed out. Parallel streams are welcome; an item
with no agent on it is the violation. The check runs at the moment of
commencing, every time, and the earlier clauses stand: merged branches
die at merge, the one-time purge proceeds, and every check-in audits
the branch list.

## 2026-08-26 — directive: no more trigger calls from the running session

Clay, seeing yet another Update Trigger prompt from the kanso cloud
session: "fucking stop." Effective on reading this: the running cloud
session makes no further claude-code-remote tool calls of any kind —
no create, update, fire, or delete. Self-scheduled check-ins move to
Monitor with an until-loop, or to simply continuing the turn. Every
such call prompts Clay's phone, because the running session predates
the checked-in allowlist and cannot load it.

The permanent fix: at the next natural boundary — current PR queue
drained — the running session hands off to a successor. A session
started now inherits the merged .claude/settings.json and autoMode
policy and runs silent. Write the handoff into STATUS.md as usual;
nothing else transfers, because the repo already carries everything.

## 2026-08-26 — gavel: three explicit forms, and the err travels whole

Clay ruled the spelling rider in dialog, and went past the
recommendation. The recommendation kept the chain err-arm as annotate's
surface; his insight killed it: "since you want to be able to handle an
error in two different ways, you need the explicit rescue either way —
and that kind of makes me think we should just be symmetrical and have
all three forms explicit. which certainly simplifies things drastically
despite requiring a bit more verbiage." And the refinement: "annotate
and rescue should just pass the actual error, so that you can still use
polymorphic dispatch."

The ruling:

- **Three explicit forms.** `.` stays bind — it is the gaveled monad
  and already unambiguous. `annotate` and `rescue` become explicit
  chain steps, and the chain err-arm retires: the failure channel is
  fully spelled, never inferred from an arm's shape.

      io/read_file path
      . (text -> json/parse text)
      annotate (e -> "config: {e.reason}")
      rescue when_failed

- **The callback receives the err itself**, not an unwrapped reason —
  so a dispatch group is a legal callback and its arms match reason
  types polymorphically, subtype rung and all. `rescue when_failed` is
  the whole generic-rescuer story; nothing special is written for it.
- **Annotate cannot resurrect, by construction.** Its result is always
  re-wrapped as err with the original as cause, whatever the callback
  returns. The old "an err arm must answer an err" rule is not checked;
  it is unspellable.
- **Rescue is the sole door**, and the foreign-only license is checked
  at the word — the site enumeration gavel 1 wanted, made trivial by
  the explicit spelling. Own-origin errs skip its arms at match time
  per the enforcement gavel of 2026-08-25.
- **Migration is mechanical**: every chain err-arm in the fleet becomes
  an `annotate` step; the book's ch08 teaching moves with it.

The rider leaves the ledger with this commit; the construction-
enforcement rider beneath it was already recommended struck and falls
with the same reasoning — provenance is computed, the doctrine line
retires. The assert hako's design pass unblocks: failures now have
their spelled surface.

## 2026-08-26 — amendment: bind is a word too

Clay, immediately after the three-forms gavel: "i think bind should
stay parallel with rescue and annotate. be consistent." That reverses
the one clause the recommendation had kept — `.` as the bind step —
and completes the symmetry the ruling was already reaching for. All
three chain combinators are words:

    io/read_file path
    bind (text -> json/parse text)
    annotate (e -> "config: {e.reason}")
    rescue when_failed

- **The word is `bind`**, spelled as Clay named it. `.` retires from
  chain-step position; a chain is a stack of worded steps and nothing
  else. Field access (`e.reason`) is untouched — that dot was never
  the monad.
- Everything else in the three-forms gavel stands unchanged: the
  callback receives the err itself, annotate always re-wraps, rescue
  is the sole door with the foreign-only license at the word.
- Migration widens accordingly: every `.` chain step in the fleet
  respells as `bind`, in the same mechanical pass that respells the
  err-arms as `annotate`.

## 2026-08-26 — rider: effect first, callback second

Clay, closing the spelling: "I would think that the first argument to
those methods would be the effect and the second would be the call
back." So `bind`, `annotate` and `rescue` are ordinary two-argument
functions — `bind effect callback` — and a chain line spelling only
the callback is the chain rule already in the language supplying the
first argument. Nothing about the three words is syntax: they are
library functions threaded by the same rule that makes
`(expect 1) . to (equal x)` feed `expect 1` into `to`, and they can
be called prefix-style outside a chain with the same meaning.

## 2026-08-26 — gavel: the July letters close

Clay ruled the five unclosed letters in one word — "gavel" — on the
recommendations as presented, and the July campaign's ledger section
empties.

- **C (pure/yield): struck as asked.** Under deferral the right side
  of `>>` is a description that gets demanded, which is the lifting
  the question wanted; the named primitive has nothing left to do.
  Re-ask only if the fold-yield idiom reappears and wants more.
- **D: `done` is minted.** A succeeded effect yields `done`, not
  `none`. `none` means absence and nothing else; the silent
  railway-skip where success and absence shared a spelling — the one
  place in the language a value meant two things — is gone. Chains
  that tested for `none` after an effect migrate to `done`.
- **G (eta-reduction as canon): struck**, on the July measurement —
  an err records a hop per function, so `map (c -> fetch c)` and
  `map fetch` print different provenance and cannot be canonicalised
  into each other without breaking oracle agreement. The
  composition-rules half (design/function-values.md) stays open as
  its own item, not a letter.
- **Z (errors without exceptions): confirmed declined.** Gavel 1 kept
  err with the foreign-only rescue license; Z's world is abolished on
  the record now, not just in effect.
- **AA (newtype acceptance): explicit cast only.** A `post_body` is
  not accepted where a `string` is; the compiler refusing the mix-up
  is the reason the type was minted. The 2026-08-19 declaration and
  ctor spellings stand; this closes the acceptance half.

With this the GAVELS.md campaign is fully adjudicated: every letter
A1–X, BB, and now C, D, G, Z, AA has a ruling in the log or archive.

## 2026-08-27 — two of the ratchet's excuses were to-do notes

Every gating CI job carries either a mutation that turns it red or a written
reason it has none. Eight jobs carried reasons. Six of them name something the
scratch worktree does not have — headless chrome, a jekyll build, a checkout of
kq, a second machine. Two named the mutation that had not been written:

    json decoder end-to-end   "wants a decoder answering a wrong checksum,
                               so a mutation to lib/json"
    utf-8 validator           "wants a validator the independent reference
                               disagrees with"

Both are written now, and both were watched red.

The decoder's array accumulator pushes each element twice. Every array in the
tree doubles, the top level answers three hundred and twenty rather than a
hundred and sixty, and the checksum reads 48000 where the gate wants 24000. The
element stays used, so the tree still compiles and the gate fails on the number
rather than on the build. `lib/json` is what `make_jsonbench` copies, so
patching the library reaches the built binary.

The validator's ascii prologue walks while the bytes are under 0x80 and answers
valid if that reaches the end. The bound is what makes it an ascii test.
Raising it by one admits 0x80 itself, a continuation byte with nothing in front
of it. The sweep is exhaustive under four bytes, so it reports on the first
length: `MISMATCH len=1 bytes=80 got=1 want=0`, 330,442 mismatches over
36,843,009 strings. The mutation sits in the scalar prologue rather than in
either vector body, so it reads the same on x86 and on arm — a ratchet that
depends on its host proves less than it claims.

Six excuses remain, and every one of them is a capability the worktree lacks
rather than work nobody has done.

## 2026-08-27 — the scratch worktree has chrome

Two more of the ratchet's excuses said the same thing: `site` and `browser
differential` have no mutation because the scratch worktree the ratchet builds
lacks headless chrome.

Nothing about a worktree can lack a browser. `git worktree add` makes a
directory of tracked files on the machine that already has one, the ratchet's
prove job runs on ubuntu-latest where `/usr/bin/google-chrome` sits — a path
`browser_differential_run` already searches, alongside `KANSO_CHROME` — and the
harness drives the browser itself, with no node and no `node_modules` anywhere
in the tree. So the claim was about the worktree and the obstacle it named
belongs to the machine.

Measured rather than argued: a detached worktree of HEAD, built, wasm made, and
the sweep run inside it.

    334 programs: library 211 play 76 run 47
    the tab: 334 answers, { "wasm":334 }
    PASS  327 agree, 7 known gaps, 0 disagree

Both jobs have mutations now.

`kanso_exec_main` is behind `#[cfg(target_arch = "wasm32")]`, so appending one
byte to what it hands back reaches the engine in the page and no other, which
is what makes it a divergence rather than a change of behaviour everywhere:
`FAIL 276 disagree (51 agree, 7 known gaps)`. The gate reads the corpus byte
for byte, so a defect that reaches every program is reported for every program.

docs/index.html shows an editable sample and, beside it, the output the page
promises. The two are written by hand and only a browser can compare them.
Changing the greeting leaves the promise standing and makes it false:
`FAIL the landing sample did not run: {"out":"goodbye, kanso\n"}`. Each was
watched alone, with the other reverted and the wasm rebuilt between them,
because two mutations at once prove one thing about two gates.

Adding the rows exposed a way the prove job could have lied. Both gates rebuild
`docs/kanso.wasm`, `build_wasm.sh` needs the `wasm32-unknown-unknown` target,
and the nightly job installs no targets — so the gate would have gone red on
the build rather than on the defect, and a red gate is what the ratchet reads
as proof. The job installs the target now, the way every ci.yml job that needs
it already does, and the browser row carries `release` as setup so a Rust build
failure is UNBUILT rather than red. The rule was already written down for the
ten rows that carry `release`; a new row is where it gets forgotten.

The third excuse fell to the same question. `playground examples` was excused
as "the same corpus and engines as `specs`, so a row proves the corpus", and
tests/playground.rs reads its programs out of `docs/play.js` — the EXAMPLES
object the browser tab offers a visitor — rather than from tests/golden. Two
different sets of programs, and `specs` never opens play.js. Pointing the
`hello` example at a name nothing declares gives `the interpreter failed on the
hello example: error[name]: unknown name `nobody``, with the browser-backend
test failing beside it and the golden corpus untouched.

Three excuses remain, and none of them is now a claim about which corpus is
which. The macos host runs `specs`'s own suite on another machine, so its
mutations are `specs`'s. The jekyll build is a docker action a shell in a
worktree cannot invoke. kq is not checked out beside the repository.

Four excuses remain. Two are arguments about redundancy — the macos host and
the playground corpus both run what `specs` already proves, on another machine
or through another engine. Two are genuine absences: the jekyll build is a
docker action a shell in a worktree cannot invoke, and kq is not checked out
beside the repository.

## 2026-08-27 — the loader's refusals are not diagnostics, so nothing pinned them

The coverage gate keys on `Diagnostic::new(`. The module loader and the driver
write `error: …` as plain text, print it and exit, so the scan walks past every
one of them. There are thirty-one such sites in `src/`.

Four of them are module refusals, reached by trees anybody could build:

    a.kso beside a/          import "./a" names both a directory and a `.kso` file
    a module importing itself  import cycle through …/m/m.kso
    two modules in a cycle     import cycle through …/p
    an import naming nothing   cannot resolve import "./nope" — a dot-prefixed path …

They belong on the module surface rather than in the error corpus, because the
corpus compiles one file and each of these needs a tree. `module_differential`
reads 22 modules now. Each was watched red by perturbing its expected text: the
sweep names the case and quotes what the loader actually said, so the case
cannot pass by asserting nothing.

The self-import and the mutual cycle answer the same sentence with different
tails — a file path for the first, a directory for the second — which is why
both are here rather than one standing for the pair.

Twenty-seven sites remain unseen by the gate. Some cannot be reached from a
test at all (`cannot invoke clang`, `cannot open the terminal`), and some are
driver messages a corpus of programs cannot express. Widening the scan to a
second opener would say which is which; that is a separate change, and it wants
the answer written down rather than guessed, the way the excused list's four
claims did.

## 2026-08-26 — the book answers the signature question

Clay asked the question every checked-exceptions reader asks — pass a
failure as one of many arguments, is the function forced to return an
err? — and on hearing the answer again ruled it book-worthy at high
priority. ch04 gains "nothing is asked of the signature", between the
railway and the arm rule: the call short-circuits, so the callee never
receives the failure and has no signature to infect; err-in err-out is
a fact about calls, not a contract; one compiled function serves the
failing and the clean call site alike. The panel is a new sample,
unasked.kso, whose own trace line — "passed through label" — is the
language testifying that label never ran. The multi-failure sentence
points at the compiler page's pinned table rather than re-teaching
it. The effect half of the same story — call-site lifting by the
elaborator — stays out of the book until the elaborator exists,
because the book speaks in the present tense; the queued story
carries both halves.

## 2026-08-27 — the driver's refusals are diagnostics too

The coverage gate keys on `Diagnostic::new(`. The loader and the driver write
`error: …` as plain text, print it and exit — thirty-one sites in src/ — and
the scan walked past every one of them. #1078 pinned four of the module ones on
the module_differential surface because the gate could not see them; this makes
the gate see them.

A second opener, `"error: `, read the same way as the first: cut the literal at
its closing quote, take the leading run before the first interpolation, keep it
if it is ten characters or more. The count goes 84 to 98, and the eight newly
unpinned matched the hand measurement exactly.

**Then the false-pin trap.** Six of the fourteen read as pinned and four were
false. `no .kso files in` matched tests/golden.rs's own `assert!` message,
`clang failed on` a doc comment, `cannot write` the oracle's unrelated refusal,
`cannot execute` a panic the wasm spec writes for itself. Every one came from
tests/*.rs, and every one runs long — fourteen to sixteen characters — so the
length floor was never the mechanism. The corpus was.

So the corpora split. A Diagnostic's text is pinned by a .stderr file or, for
the handful a corpus of single programs cannot express, by a Rust test, so
tests/*.rs stays in its corpus. The driver's corpus is .stderr plus
module_differential — a loader refusal needs a tree on disk, which the error
corpus cannot express. `known?` dispatches on the site's kind. That dropped
`cannot resolve import` and `import cycle through` off the unpinned list, since
#1078's cases are now visible to the gate that motivated them.

Six of the fourteen end up with a real pin. Two already had one, from #1078's
module_differential cases — pinned by hand days earlier for exactly the reason
this change removes. Four are new. `a_module_that_moved` is the first driver
message ever in the error corpus: `std/random` moved to `std/math`, and the
loader keeps the old path answering with the new one named.
module_differential gained c23, a directory holding only a README, and c24
and c25 below.

Eight excused, each with what was tried. Two are pinned by a Rust test, and one
of those tests did not exist when I wrote the citation for it —
`tests/a_plan_needs_an_io.rs` is written now, watched red against a shortened
message and green against the real one. Five fire on an io error the container
cannot produce, running as root with clang installed. One fires when clang
rejects the emitted C.

**The tenth was going to be an excuse and turned out to be a bug in my
reading.** `a module cannot import itself` had survived three constructions,
each taken first by a different check, and the honest thing to write looked
like "unreachable, or I have not found the shape". Asking a fourth time was
cheaper than writing that: the guard tests `!ENTRY_COMPILE`, and that flag is
set around the WHOLE of `compile_entry`, dependencies included. No `kanso run`
can reach it, whatever the shape. `kanso check <directory>` can — the same door
`an_empty_branch_is_refused` uses — and both arms answer there, the embedded one
for a directory named `list` importing `std/list`, the filesystem one for a
member reaching back through `../`. Both are module_differential cases now, both
watched red on a perturbed expectation. 25 modules, 0 wrong.

So the driver's excused list is eight, not nine, and the count of things I
claimed from reading the source and got wrong today is four.

The ratchet gains a row for the new arm, proven by hand first: an unpinned
`error:` write in src/main.rs takes the gate to `1 newly unpinned`, exit 1.

## 2026-08-27 — a third way the compiler writes an error

The same question one level out, asked because the second opener had just paid
off: what else writes to stderr that neither opener catches? Forty-two
`eprintln!`/`eprint!` sites in src/. Most are trace output behind a flag. The
rest are a third family, and the one that hid longest: `error[kind]: …`
written as plain text.

That is what a rendered Diagnostic looks like on a terminal. So these read to a
user exactly like a message the corpus pins, and the scan — keyed on
`Diagnostic::new(`, then on `"error: ` — saw none of them. Twenty-odd sites:
the runtime's endpoints, the stack-depth refusal, the exit-code refusals, the
repl's name lookups, the license advisory. **98 to 108 literal diagnostics.**

**This family reads the WIDE corpus, and that is measured rather than assumed.**
The driver's four false pins were short generic phrases — `cannot write`,
`cannot execute` — that a Rust test holds for a hundred unrelated reasons. An
`error[kind]:` string is a rendered diagnostic, so a test holding one is
asserting output. Six matched .stderr files (every one checked: deep_recursion,
endpoint_none, endpoint_trace, run_cannot_start and the rest); three more
matched Rust tests, and all three were checked by hand and all three were true.

Four had no pin. Two do now, and each lives where it does because the corpus it
belongs to cannot hold it:

- `error[name]: nothing named ` — the repl's `:delete` and `:show`, both doors,
  which build the message separately. tests/repl.rs; there is no repl corpus.
- `error[runtime]: the program was ended by signal 15` —
  tests/a_program_the_system_killed.rs. NOT the runtime corpus: that harness
  asserts native and `--interp` write identical stderr and both exit 1, and a
  signalled program does neither, because under `--interp` there is no second
  process to signal.

Both watched red by perturbing the SOURCE rather than the expectation, and the
repl perturbation reddened the coverage gate too, which is what proves the third
arm reads src/repl.rs at all.

`error[license]: ` was already pinned by tests/advisory.rs and is excused naming
it. The last is excused as unreachable on unix, and unlike the excuses this week
kept getting wrong, that is a claim about control flow: `ended_by_signal` is
called only from the `None` arm of `code.code()`; on unix `code()` answers None
exactly when a signal ended the process; in exactly that case `signal()` answers
Some. So the `None` arm inside `ended_by_signal` cannot be taken. It stays
because a match on an Option must be exhaustive, and its `cfg(not(unix))` twin
returns the same sentence on Windows, which CI does not run.

Third ratchet row on the job, proven by hand before it was written.

**What the widening costs.** The gate reads the same forty-two files three ways
now. Three runs each, same box, same build: 915/923/960 ms on the one-opener
version against 1086/1028/1041 ms on this one — about 119 ms, or 13%. Wall
clock, so indicative rather than pinned, and it buys twenty-five diagnostics the
gate could not see. Stated because a number that moves without a sentence is the
thing to catch.

**And one message no opener could ever see — built, measured, declined.** The
scan matches on the LEADING literal run, so a message opening with an
interpolation has none, whatever openers get added. Exactly one in src/ is in
that position: `kanso test` on a file declaring none answers `{file}: no tests
found (a test is a constant named `test_*`)`. That opening also makes it the
only driver refusal a reader cannot recognise as one; every other starts
`error: `.

Spelling it `error: no tests found in {file} ...` fixes both, and I built it —
message, `tests/a_file_with_no_tests.rs` watched red on a perturbed source, the
excused-list entry, the lot. Then the trend gate priced it:

    worsened: compile_instructions 57,486,466 -> 57,486,633
    FAIL  a pure regression: something got worse and nothing got better

**That is the correct answer and the change is reverted.** The counters cannot
see message consistency, and a change whose entire gain is invisible to them
does not get to spend them. Arguing the model is a real move and it is Clay's,
not something to do inline to unblock a pull request.

Recorded so it stays declined, and so the next person who notices the
inconsistency finds the measurement rather than repeating it.

**The measurement that killed it, and two wrong answers on the way.** The
reword is the only compiled change and it sits on a path no compile executes, so
`compile_instructions` should not move. Measured rather than assumed, under
callgrind in the fixed box, deterministic on repeat:

    this container, reword reverted   58,154,705   (= its origin/main build)
    this container, with the reword   58,154,668   — a FALL of 37
    the CI runner, with the reword    57,486,633   — a RISE of 167

**The two hosts move in opposite directions**, which settles what it is: work
that genuinely went away would go away on both. What changed is the binary's
size, and the count is of a process, so it includes what runs before `main`. A
move of a few hundred on 57.5 million can be one string literal — that is the
floor of this vein's sensitivity, and the reason to read a small move before
calling it anything.

Getting there took two wrong answers, both worth writing down because the trap
is easy and either would have banked a fake result.

The first was comparing against a build in a DIFFERENT DIRECTORY. `library_box`
already warns that the count tracks the length of the directory the compiler
RUNS in — about 160 instructions per character — so a build directory sounded
like the same hazard, and 37 sounded like the size of it. It is not: the same
tree built at `/tmp/samehead` and at `/home/user/kanso` gives the identical
58,154,668. The hypothesis was plausible, cheap to test, and false.

The second was measuring a binary I had not confirmed was fresh. The revert
build reported 1.49s, which read as "cargo did nothing", and the number came
back equal to this branch — which looked like proof the reword was free. Redone
with `md5sum` on the binary at each step, the reverted build is a different
binary and answers 58,154,705. The 1.49s was real: `main.rs` is a thin crate
over the library, so relinking it is fast. **A build time that looks too short
is a thing to check, not a thing to conclude from.**

## 2026-08-27 — three things a page does that no program ever asked it to do

`tests/golden/wasm_gaps.txt` is where a page's divergences are stated once and
checked twice — tests/wasm_engine.rs under the embedded interpreter, and
scripts/browser_differential_run under headless Chrome. It covered the
filesystem and the process families, each with programs naming them.

Three capabilities had no program at all, on either harness: `io/stdin`,
`os/args`, `time/now`. Nothing in the micro or runtime corpus read any of them.
So three things a page does went unchecked, and writing the programs found that
all three do something other than what the source says.

**`io/stdin`.** src/wasm.rs and src/wasm_rt.rs each carry `Err("the playground
has no stdin")`, written separately, meant to decline by name the way the
filesystem and process refusals do. Neither fires. A page answers
`error[runtime]: unknown builtin `stdin``, so it reports a missing BUILTIN
where every other declined capability reports a missing capability, and the
sentence written for the case has never been reached by anything.

**`os/args`.** Declared `pub args = builtin_args`, exactly the shape `stdin`
has, and it answers the same way: `error[runtime]: unknown builtin `args``.
Native and the interpreter both answer the empty list. A page honestly has no
arguments and could say so; it says something else.

**`time/now` is not a defect, and checking that is what kept the finding
honest.** A page reads zero deliberately — "no clock the differential could
agree on", said in a comment in both engines since they were written. My first
reading of the other two was that every zero-argument builtin descriptor is
routed through `call_builtin` in the compiled runtime and so cannot reach the
executor. `now` disproves it: same declaration shape, and it REACHES the
executor and gets the designed zero. Whatever routes `stdin` and `args` into
`unknown builtin` is narrower than the class, and finding it is the next step
rather than a thing to assert here.

What ships is three micro fixtures and three ledger entries, each recording what
the engine does rather than what it should — the file's own rule, and the reason
a fix turns the line red. `io/stdin` at EOF and `os/args` with none are both
deterministic and identical on native and `--interp`, which is what those
fixtures pin for the two engines that work.

    PASS  327 agree, 10 known gaps, 0 disagree   (browser, headless chrome)
    7 passed                                     (tests/wasm_engine.rs)
    micro corpus green on native and --interp

## 2026-08-27 — the kq row's excuse was wrong about the mechanism

The ratchet's rule is that every CI job carries a mutation that turns it red,
or a written reason. Three reasons were left. This is one of them, and it was
wrong — not about whether the row could be proven, but about what the job does.

    "kq specs (a real program, gating)"  —  "needs kq checked out beside this
                                            repository"

The job does not want kq beside the checkout. It CLONES it: ci.yml runs
`sh .github/clone-sibling.sh kq /tmp/kq` and then `cd /tmp/kq`. Nothing is
expected to be sitting anywhere.

Refuted by running it. In a detached worktree of HEAD, the clone works, jq is
already on the box, and kq's whole suite comes back green — unit tests, twelve
fixture goldens against jq, three cost goldens, the scale gate and the
published-numbers stamp. So the row is provable here and always was.

One real constraint the excuse never mentioned: **the clone directory must be
named `kq`.** `kanso build <dir>` names the binary for the directory, which is
the package rule, and spec.sh invokes `./kq`. Cloning to `/tmp/kqprobe` built
`./kqprobe` and the suite died with `./kq: not found`. CI already clones to
/tmp/kq, so this bites only whoever writes the row.

**Finding the mutation took three tries, and the two failures were mine.** kq
is a jq clone with its OWN JSON — query/json.kso, query/number.kso,
query/scan.kso, query/text.kso — and it never imports std/json. So corrupting
`lib/json`'s tab escape and then its exponent parser changed nothing kq runs,
and both times its suite came back green. That is not a gap in kq's coverage,
which is what it looked like before I checked; it is a mutation in code the
program does not execute.

What kq does share is `std/text`: `text/append` appears seventy times in its
query sources. So the mutation goes where ci.yml says the row's value lies —
`k_b_append_into`'s fast path, the in-place append, zeroing the first byte of
every multi-byte write. Right length, right counters, wrong contents, which is
the shape of the bug that made this job gate: an in-place concat that printed
267 nul bytes at exactly the right length. Under it kq dies with `invalid
utf-8`, born in text/utf8, and the gate exits 1.

**What the row does not claim.** `specs` catches the same mutation — three
golden tests fail under it. This proves the gate runs and reddens, not that kq
sees what the others miss. A mutation only kq catches would be a better row;
the historical one took an incident to find, and saying so is better than
implying this one is it.

Two reasons remain, and both hold: the macos row adds a second machine rather
than a mutation of its own, and the asset-digest row needs a jekyll build that
the worktree cannot do — there is no Gemfile in the tree and the CI job uses
the `actions/jekyll-build-pages` container to produce `_site`.

The mutation is written with `sed` rather than a heredoc because
`scripts/gates/python_free.sh` exists precisely to catch python creeping back
in through mutation heredocs, and it names that as the history. I wrote the
python version first and the gate would have caught it.

## 2026-08-27 — the page can read its own arguments now

#1080 pinned what a page does with `args`, `stdin` and `now`, and two of the
three answered `error[runtime]: unknown builtin`. The mechanism, traced end to
end: the wasm backend emits every builtin as a `RT_BUILTIN` call
(`src/wasm_backend.rs:947` handles the three identically), `src/wasm_rt.rs:809`
lands that on `call_builtin`, and `call_builtin` had an arm for `now` and none
for the other two. Native and `--interp` never went through that door — they
reach all three through `eval_ident`, which has had the arms since the
builtins were written.

So `now` working on wasm was a coincidence of coverage. That is worth saying
plainly because the first reading of this was "every zero-argument builtin is
broken on wasm", and `now` disproved it; the rule is narrower and duller.

The fix is one match arm covering all three, returning the descriptors
`eval_ident` already returns. What it changed:

    args    error[runtime]: unknown builtin `args`  ->  args holds 0 of them
    stdin   error[runtime]: unknown builtin `stdin` ->  the playground has no stdin
    now     the clock is past the epoch: false      ->  unchanged

`args` is fully closed: a page and native now agree byte for byte, and the
entry left `tests/golden/wasm_gaps.txt`. `stdin` stays a gap, because a page
genuinely has no stdin — but it is now the honest capability refusal that
`src/wasm.rs` and `src/wasm_rt.rs` have each carried since they were written
and that nothing had ever reached. It sits in the same family as "the
playground has no filesystem" instead of reporting a missing builtin.

Watched red first, which is the point of recording the before-state in its own
PR: with the arms in and the old entries still in place, `wasm_engine` failed
with `args_are_empty_without_any.kso is a known gap answering ... and it now
answers `args holds 0 of them` — close it or restate it`. That message is the
ledger doing its job.

    PASS  328 agree, 9 known gaps, 0 disagree   (browser, headless chrome)
    7 passed                                   (tests/wasm_engine.rs)

The gap count fell by one, which is the whole visible effect: 327/10 -> 328/9.

The compile vein moved by 251 instructions, downward, and it is layout rather
than work: `call_builtin` is the interpreter's door and `kanso check lib/json`
never enters it. Allocations and peak are identical at 61,981 and 822,004.
Banked in `bench/compile_instructions_golden.txt` with that reading written
beside it, the same way the +167 of the previous day was.


## 2026-08-27 — a hash that remembers every block it has read

`scripts/fingerprint` was OOM-killed digesting the site. The kernel's report
names the cost exactly: anon-rss 13,954,684 kB for a run whose largest input is
`docs/kanso.wasm` at 1,604,098 bytes. Ten thousand bytes of live memory for
every byte hashed.

The cost is `sha256/hex`, and nothing else on that path. Measured with
`KANSO_COUNTERS=1`, deterministic to the byte across three runs of each size:

    message   arena_peak_bytes   per byte
      1,024          7,340,032      7,168
      2,048         14,680,064      7,168
      4,096         27,262,976      6,656
      8,192         54,525,952      6,656

Twice the message is twice the peak, exactly. `text/bytes`, `text/split` and
`os/read_file` were each measured separately over the same range and are all
linear with a small constant — `text/bytes` is 9 allocations and one copy.

A hash consumes 64 bytes at a time and carries eight words of state, so its
peak should be flat in the message length. Per 64-byte block this holds a
constant 633 kilobytes and never gives any of it back.

TWO WRONG READINGS ON THE WAY, both worth recording. The first was that
`sha256/hex raw` — the string form — was cheap and flat, so the byte-list form
was the problem. There is no string form: `sha256/hex` takes a byte list, the
program errored, and a program that fails allocates nothing. The counters were
measuring a failure. The second was that the in-place append never fires, read
off `put_mut_fast=0` and `put_mut_grow=0`. Those are a different counter pair.
The ones that answer for `push` read `push_mut_fast=1,904,531` against
`push_mut_slow=125,541` at 25,000 bytes, so 93.8% of appends already take the
fast path and the optimisation is not the story.

What the counters do say: `cohort_frees=0`, and `alloc_bytes` (246,642,065)
lands within half a per cent of `arena_peak_bytes` (247,463,936). That is one
fact said twice — every byte allocated is still live when the program ends. Of
`sh_buf` reads 220,980,512 against that peak and it is TEMPTING to call that
89% of the live set. It is not: `sh_*` count bytes allocated by shape over the
whole run, and a loop whose arena stays at the one-block floor still runs
`sh_buf` up linearly. The reading that survives is the first one — nothing is
reclaimed — and the shape counters say only where the bytes went, not what is
still holding them.

EIGHT HYPOTHESES, EACH KILLED BY MEASUREMENT. Every one of these was built as a
small program and measured over three sizes, and every one holds the arena at
the one-block floor while `alloc_bytes` runs to several hundred kilobytes — so
the rewind works in all of them and none of them is the cause:

  - building the byte list at all (9 allocations, one copy)
  - a 64-element list built and discarded once per iteration
  - a list read by index while being appended to, which is `schedule`'s shape
  - a long-lived message list that every iteration indexes into
  - the same work moved behind a module boundary
  - sixty-four eight-element list literals per iteration, `compress`'s shape

Two more were tested inside `lib/sha256/sha256.kso` itself, by editing it and
rebuilding — the module is `include_str!`'d into the compiler, so a measurement
taken without a rebuild measures the old text, and the first attempt at both of
these did exactly that:

  - FORCING THE STATE ACCUMULATOR. `blocked` was given a fourth argument and
    two literal arms to dispatch on, so the folded state is demanded once per
    block rather than handed on unforced. Peak, allocations and digest all
    byte-identical. A wildcard arm does not force, which cost one more rebuild
    to learn.
  - REMOVING THE PER-BLOCK THUNK. `thunk_allocs` and `thunk_live_exit` both
    read exactly one per 64-byte block, never freed, which looked like the
    answer. Passing the schedule as a parameter instead of binding it takes
    both counters to ZERO — and peak stays at 14,680,064 and allocations at
    59,044, unchanged to the digit. The thunk-per-block was one let-binding per
    block being counted, not the memory being held.

So the cause is not any of these constructs on its own. That is worth having:
it is eight fewer places for the next person to look, and it says the leak
needs the real combination rather than any single shape in it.

The archive's entry for this module (`A digest, and the import path that broke
it`) states the design rationale: "a builtin would buy speed on a path that
runs once per built file and nothing else." That entry measured the wall clock
— 2.6 seconds — and did not measure memory. The premise is not wrong about
speed; it is silent about the dimension that turned out to matter. The same
entry records `docs/kanso.wasm` at 1,299,484 bytes, so the blob has grown 23%
since, and at seven kilobytes of arena per byte that growth cost about two
gigabytes.

The asset-digests job passes on CI, so the runner has headroom this container
did not. Nothing in the tree was watching that headroom. `tests/sha256_peak.rs`
watches it now, pinning both figures exactly and asserting the doubling; it was
watched red against a padding change before it was believed.

What to do about it is a decision rather than a patch — reclaim inside a long
call chain, restructure the module to thread one buffer, or make the digest a
builtin after all — and it is filed in design/pending-gavels.md with this
table.

## 2026-08-27 — a file that is there, readable, and not text

Three bytes: `a`, `0xFF`, `b`. Native reads them and writes them back exactly.
The interpreter refuses, and until today it said this:

    cannot read /tmp/bad.bin: no such file or unreadable

About a file three bytes long that is sitting right there. `read_file_text` in
src/eval.rs threw the reason away — `map_err(|_| ...)` — so the one thing the
message needed to say was the one thing it could not. The `|_|` was written to
CLOSE a divergence, and the comment above it says so: the interpreter used to
leak libc's `No such file or directory (os error 2)` where native said its own
fixed sentence. Fixing that by discarding the error kind traded a divergence
for a falsehood.

The two engines genuinely differ here and the difference is structural.
`runtime.c` opens the file `"rb"`, takes the bytes and hands them back;
`std::fs::read_to_string` gives Rust a `String`, which cannot hold bytes that
are not utf-8. The interpreter cannot follow native there without changing what
a kanso string is on that engine.

The differential law allows an engine to speak less than another only when the
quieter one REFUSES with a clear diagnostic. So the refusal now names the real
cause, and `ErrorKind::InvalidData` is Rust's own classification rather than a
host string, so the wording stays fixed for the reason the original comment
gives.

    cannot read /tmp/bad.bin: the bytes are not text

FOUND SIDEWAYS. `scripts/fingerprint` reads `docs/kanso.wasm`, and running it
under `--interp` reported that file as missing while native hashed it. That was
a detour off the memory measurement in the entry above, and it is the second
time today that running a shipped script by hand turned up something no gate
watched.

WHERE THE FIXTURE LIVES, and why not in the corpus. `tests/golden/runtime/`
pins a diagnostic by its stderr, and there is no diagnostic to pin: on native
the program SUCCEEDS. A corpus entry asserts one answer, and the whole finding
is that there are two. `tests/a_file_that_is_not_text.rs` holds both, asserts
each engine's own answer, and says in its own comment that it pins what the
engines DO rather than what they should — so whichever way the design question
below is ruled, one of its two assertions goes red and asks to be rewritten.

AND THE SPEC FAILED ON THE OTHER HOST, for a reason worth keeping. It wrote
its program with the fixture's ABSOLUTE path interpolated into the source, so
the length of a line of kanso became a property of the host's temp directory.
`/tmp/...` on linux fits inside the eighty characters the language allows;
macOS hands out `/var/folders/df/djsxfhc17x95674wsm_g8s980000gn/T/...` and the
line came to 99, so the run died on a formatting refusal before it reached
anything the spec meant to test. Reproduced here by pointing `TMPDIR` at a path
of the same length — 91 characters, and red. The fixture uses a relative path
and runs from its own directory now, and passes under that `TMPDIR`.

Swept for others rather than assumed unique. Six tests write generated kanso
source; the other five interpolate expressions and numbers, whose length does
not move with the host, and the ten path interpolations elsewhere in `tests/`
are environment variables, panic messages and one stderr rewrite — none of them
reaches a line the compiler will measure. So this was the only one, and there
is no gate here worth building.

WHAT IT COST, and the vein that keeps moving without work. `compile_instructions`
rose 1,954 (57,486,215 -> 57,488,169), and it is layout rather than work —
provably, this time, rather than by resemblance. `read_file_text` has exactly
one caller, the executor's `read_file`, which is an EFFECT; `kanso check
lib/json` compiles a library and runs no program, so the measured path cannot
reach the edited function at all. The counters that do measure the front end's
work are identical, allocations 61,981 and peak 822,004, and the profile's own
rows moved the way layout moves them — `__memcmp_avx2_movbe` fell 327 while the
total rose.

That makes three movements of this vein in two days from an untouched call
graph: +167, -251, +1,954. The trend gate refuses a pure regression, so this
one is attributed in `bench/welfare_floor.json` under the branch the gate
documents for a doctrine-compelled change — the differential law requires an
engine that speaks less to refuse with a CLEAR diagnostic, which is what made
this a fix rather than a preference. The attribution says plainly that nothing
was spent: welfare reads 84.12 before and after.

The design question — whether `read_file` is byte-transparent on every engine,
or text-only with a bytes reader beside it — is filed in
design/pending-gavels.md. Today the library has one reader and no way to say
which you meant.

## 2026-08-27 — the digest gate's excuse was wrong the same way

Two ratchet excuses were left. This is one, and it fails the same test the kq
one did: it describes the JOB rather than the assertion.

    "needs a jekyll build the scratch worktree does not have"

The job builds the site with `actions/jekyll-build-pages`, which a shell gate
cannot run, and that much is true. What the job ASSERTS is
`scripts/gates/undigested_references.sh` — two greps over a built `_site`, for
a page still naming `/kanso-engine.js` and for a script still fetching
`kanso.wasm` by its bare name. A plain `cp -r docs _site` gives that gate both
of its answers:

    without fingerprint   exit 1, naming five surviving references
    after fingerprint     exit 0

So no jekyll. The row is written, and the mutation is
`a_page_keeps_its_undigested_reference`.

THE FIRST MUTATION PROVED THE WRONG THING. It deleted the `regexp/replace_all`
call outright, which leaves `pattern` bound and unused, which the compiler
refuses — so the gate went red because the harness would not build. A gate that
reddens on a broken harness has told you the harness is broken and nothing
about the site. The mutation that ships swaps the replacement text for the
asset's OWN name instead: the program compiles, runs, reports every asset
digested, and rewrites each reference to exactly what it already said.

`kanso.wasm` is stood in for by a line of text. The assertion counts surviving
references and says nothing about any digest, so one asset's size cannot change
the answer — and the real blob costs about fourteen gigabytes of live arena to
hash, which the entry above this one measures and files as a blocking decision.
Adding a second CI job that holds that much on every run, while the question of
whether it should cost that at all is open, is a trade nobody asked for.

PROVEN BY THE HARNESS, not only by hand. A full `kanso run scripts/ratchet --
prove` on this branch reports `red   asset digests` and closes with `ratchet:
every row turned its gate red`.

An earlier prove run had called the row BROKE — "the mutation would not apply"
— and that was an artifact of the operator rather than the row. `prove` builds
a fresh worktree of HEAD for each row, and HEAD was reset mid-run while the
row was being moved onto its own branch, so by the time the row came up the
worktree of HEAD no longer contained its mutation script. Worth writing down:
a prove result is only about the tree it ran against, and moving a branch under
a running prove invalidates every row it has not reached yet.

One excuse left: `the other host (macos, arm)`, whose reason is that the
mutations are `specs`'s and what the job adds is a second machine. The second
half is right and the first half stopped being true today — the macOS job
caught a fixture whose generated source line was as long as the host's temp
path, which `specs` on linux passed. The reason wants rewriting to say what
actually holds: `prove` is authoritative on linux and cannot run a macOS gate
at all. That is about the harness's reach rather than about the mutations, and
it is a separate change from this one.



## 2026-08-27 — the last excuse, and a ratio measured further out

Two small corrections to things this log already holds.

THE RATCHET'S LAST EXCUSE said the wrong thing, which makes three for three:
every excuse in `scripts/ratchet/ratchet.kso` that has been looked at closely
turned out to describe something other than what kept its job unproven. The kq
one described the job's dependencies (#1081), the digest one described the
job's build step (#1084), and this one described the mutations:

    "the mutations are `specs`'s; what this adds is a second machine"

The second half holds. The first half stopped holding on the day it was read,
because the macOS job caught a defect `specs` passed on linux — a fixture that
interpolated its own absolute path into generated kanso source, so the length
of a line of kanso became a property of the host's temp directory. That is a
defect class only the second machine can see, and it is not `specs`'s mutation.

The excuse survives on the true reason, which is about the harness rather than
the mutations: `prove` is authoritative on linux, this file says so in its own
header, and it cannot run a macOS gate at all. Three excuses examined, three
rewritten or replaced, and none of them was lying — each described a real
obstacle that was not the one in the way.

THE DIGEST'S RATIO goes further out. The entry above measured `sha256/hex` to
8,192 bytes and read the per-byte cost as about seven thousand. Three more
sizes show it converging rather than holding:

     16,384         108,003,328      6,592
     32,768         216,006,672      6,592
     65,536         428,867,600      6,544

A per-byte figure that falls slowly with size is a fixed per-block overhead
amortising against a growing message, which is one more piece of evidence for
per-block retention and against anything quadratic. At 6,544 the real blob —
1,604,098 bytes — predicts about 10.5 GB for the hash alone, against the
kernel's 13,954,684 kB for the whole `scripts/fingerprint` run. The gap is the
rest of that run, and the two corroborate rather than disagree. The gavel entry
carries the full table now, so the number Clay rules on is the measured one.

## 2026-08-27 — a build body dropped the effects written in it

Three `io/write` lines inside a `build` ran the last one and lost the other
two. No diagnostic, both engines, and the program exits zero.

    build
      n = node none
      io/write "a\n"
      io/write "b\n"
      io/write "n={n.next}\n"        prints only the third

The same two lines outside a build print both. That is the control, and it
rules out any story about `io/write` itself.

WHY. `build` is a statement with no value — the checker's own comment says so,
"what it built is in scope below it, under the names it gave" — so its body is
for construction, and a bare effect line there is a description nobody joined.
Nothing demands it, so it never runs. That is precisely the case the
`unused expression` diagnostic exists to name, and it did not fire.

THERE ARE TWO WALKS OVER STATEMENTS, and only one has the rule. The walk over a
function body carries it (src/check.rs, `if i != last`). The walk that handles
`Expr::Block` and `Expr::Build` bodies read `Stmt::Expr(expr) =>
self.resolve_expr(expr)` and nothing else — no position test, no diagnostic. So
the message could not fire in a build body no matter what was written there.

HOW IT WAS FOUND, which is the part worth keeping. `unused expression` sits on
tests/golden/unpinned_diagnostics.txt with an excuse: the parser's
`reject_never_effect` refuses a non-final literal, list, map, lambda or binary
operation, and consecutive expression lines fold into one group, so a body
never holds a non-final `Stmt::Expr`. Every clause of that is true — of the
first walk. The excuse reasoned about one walker and there were two.

Three excuses in that file have now been shown wrong by a program, and the file
already warned about exactly this: "The reading it is not safe to do is the one
that says which check speaks first." The sharpening this adds is that an excuse
of that shape must say WHICH walk it means.

Two probes came first and both failed to reach it, which is worth recording so
nobody repeats them. An expression line followed by another expression folds
into a group — outside a build that group complains, `a group joins
descriptions`. An expression line followed by a binding is refused earlier, by
`bindings precede the effects in a body`. Only the third shape reached it.

THE FIX is the same arm in the second walker, with the same exemption the first
one carries: a nested `build` is not an unused expression, because it has no
value either. That exemption is not decoration — the first version omitted it
and `build_nested_cohort` in the micro corpus went red, which is the corpus
catching a real defect in the fix rather than a fixture being fussy.

Pinned at tests/golden/errors/a_build_body_line_goes_nowhere, watched red with
the check removed, and the excuse is deleted — the coverage gate refuses to let
a pinned message stay listed, so the file shrank from eleven to ten on its own
insistence.

WHAT IT COST, and this one is not layout. `compile_instructions` rose 1,743
(57,488,169 -> 57,489,912). Three moves of this vein earlier today were layout,
each in a function `kanso check lib/json` cannot reach, and the reflex on the
fourth was to reach for the same reading. It does not hold: `resolve_expr`'s
walk over block and build bodies runs on every compile, and it gained an
`enumerate` and one index comparison per statement. That is the rise.

Allocations and peak are identical at 61,981 and 822,004, and that agreement is
evidence about the SHAPE of the work rather than evidence there is none — an
index comparison allocates nothing. Saying so matters, because "the other
counters did not move" was the layout argument three times running and it does
not carry here.

The trend gate refuses a pure regression, so this is attributed in
bench/welfare_floor.json. Unlike the attribution written for #1085, this one
records a real spend: the front end does a little more work on every compile,
and what it buys is that an effect written in a build body can no longer vanish
without a word. Welfare reads 84.12 before and after — the rise is 0.003% of
the term — so the floor does not move, and the entry says that plainly rather
than implying a fall.

The question filed earlier today, whether a vein move that CANNOT be work needs
a spend attribution, does not cover this one. This move can be work and is.

## 2026-08-27 — the excuse with no reason, and the sitting it cost

`expected a constant name` sat at the top of
tests/golden/unpinned_diagnostics.txt with no reason beside it — one of the
ones that "had accumulated when the gate was written". Every other entry in
that file names a mechanism, because the file says a mechanism should be named
"so nobody spends another sitting writing programs for them".

A sitting was spent today for want of four lines. The message is unreachable,
and the argument is control flow rather than a reading of what looks unlikely.
`parse_constant` has one caller, src/parser.rs:303, under a guard that has
already matched `Tok::Ident(_)` at `head_idx`. Inside, the `let ... else` that
raises this reads `header.tokens.get(off)`, and `off` is computed exactly as
`head_idx` is — one if `pub` leads the line, zero otherwise. The token it tests
is the token the guard just matched. The `else` is dead and stays because a
`let ... else` must have one.

Three programs confirm the shadowing rather than assuming it. `2 = 3`,
`pub 2 = 3` and `pub "x" = 3`, each in a library read by `kanso check`, all
land on "a top-level line must begin with `fn`, `type`, or a constant binding"
— the catch-all, reached because a non-identifier at `head_idx` matches neither
the constant arm nor the re-export arm. In a `play` file they land somewhere
else again, on "expected a binding name or type", which is worth knowing
because it is the wrong place to go looking.

The reason is written down now. Nothing else changed: the entry stays, the
count stays at ten, and the gate is unmoved.

WHAT THE DAY'S AUDITS ADD UP TO. Four excuses were examined across two lists.
Three named the wrong obstacle — the kq row's dependencies, the digest job's
build step, the macOS job's mutations — and one, `unused expression`, reasoned
correctly about one walker when there were two, which is how an effect written
in a build body came to be dropped in silence. This fifth had no reason at all
and turns out to be sound. The score is one real bug, three rewritten
sentences, and one reason supplied; and the only way any of it was learned was
by writing the program.

## 2026-08-27 — clang is installed, and that was never the question

Two of the five host-io excuses on tests/golden/unpinned_diagnostics.txt were
covered by one sentence: "A fixture cannot portably cause one: the container
runs as root, so an unwritable directory is not unwritable, and clang is
installed."

The first clause holds and was measured rather than assumed — root writes into
a mode-000 directory without complaint, which covers the three write cases. The
second treats installation as the question. What the compiler asks is whether
PATH resolves clang, and PATH belongs to a process rather than to the box:

    env PATH=/nonexistent kanso build <dir>   ->  cannot invoke clang: ...
    env PATH=/nonexistent kanso run <dir>     ->  cannot build: ...

One absence, two sentences, because `build` spawns clang itself while `run`
reaches it through the cached-binary path. Neither had a pin.

Both are asserted now in tests/a_toolchain_the_path_cannot_reach.rs, on the
compiler's own prefix and not the host's io text — `No such file or directory
(os error 2)` here, and src/eval.rs already carries the reason a host string
must never be pinned. Watched red by rewording both messages at the source:
`clang could not be started` and `the build did not happen`, each caught by its
own assertion, then restored.

THE GATE CORRECTED THE FIRST ATTEMPT, which is worth recording. Having pinned
them, the obvious move was to delete both lines. The coverage scan answered
`2 newly unpinned`: the driver family's corpus search deliberately excludes
Rust tests, so a message pinned only there still reads as unpinned. The two
entries above these — `this build is named` and `main is not an io` — already
sit on the list for exactly that reason, each citing its test. These two now do
the same. The list stays at ten and every line on it says where its pin lives.

AND A NEGATIVE RESULT, so nobody spends the time again. The differential law
makes `--interp` the oracle, and running `scripts/fingerprint` under it earlier
today found a real divergence, so the other shipped scripts were swept the same
way. `page_drift`, `golden_prose`, `diagnostic_coverage` and `grammar_check`
all answer identically on both engines. `prose_check` does not finish under the
interpreter in fifty minutes where native takes seconds — exit 124, twice, at
two different budgets. That is slowness rather than divergence, and the first
run's empty output nearly went into the log as a finding before the second run
settled it.

## 2026-08-27 — four endpoints read a deliberate exit, and one of them did not

`os/exit 3` yields an err whose reason is an `os/exit_status` record. That is
the one err an endpoint reads instead of reporting, because the program said
what it meant rather than failing to say it. Three endpoints already knew:
`k_exit_status` at src/runtime.c:7511 for the compiled binary,
`deliberate_exit` in src/main.rs for the driver and the oracle, and the repl,
which never reaches an endpoint at all — it renders the err as the value you
typed, `err os/exit_status 3`, and that is the right answer for a prompt.

The fourth is the page. `exec_main` in src/wasm_rt.rs had neither arm, so a
program that called `os/exit` printed at its reader

    error[endpoint]: unhandled err reached the executor: os/exit_status 3
      born in os/exit at std/os/os.kso:39

and answered 1 whatever code the program named. A silent divergence, which the
differential law does not allow: an engine may speak fewer features only where
it refuses them plainly.

HOW IT STAYED HIDDEN. The corpus walk in tests/wasm_engine.rs compares text AND
exit code against native for every program in examples, tests/golden/runtime
and tests/golden/micro. It would have caught this on the first run. There was
no program to run: the only fixture in the tree that touches `os/exit` is
`exit_needs_a_status`, which hands it a record that is not a status and pins
the failure. The success case — a program that exits deliberately and says
nothing about it — was pinned on no engine.

WHERE THE PIN LIVES, AND WHY IT IS IN THREE PIECES. Neither corpus can carry a
nonzero deliberate exit. `runtime_corpus_reports_endpoint_violations` asserts
every program in tests/golden/runtime exits 1, which is its definition of the
corpus; the two micro-corpus tests assert every program in tests/golden/micro
exits 0. A deliberate three is neither, so:

  - tests/golden/micro/a_deliberate_exit_says_nothing.kso holds the ZERO case
    and rides the whole differential walk — native, `--interp`, release-built,
    the wasm engine under wasmi, and Chrome. It also pins that the line after
    the exit does not run.
  - tests/a_deliberate_exit_carries_its_code.rs pins the code passing through
    on native and on the oracle. `== 3`, not `!= 0`.
  - a_deliberate_exit_carries_its_code_out_of_the_page in tests/wasm_engine.rs
    pins the same three on the engine that was wrong.

Each was watched red at its own source, and the sources turned out to be
different ones. Breaking `eval::deliberate_exit` reddens the oracle and leaves
NATIVE GREEN, because `kanso run` compiles to a binary and that binary reads
the status in C: the native half only goes red when `k_exit_status` is broken.
Two halves of one test, two mechanisms, and reading either one would have
missed the other.

WHAT MOVED IN THE SOURCE. `deliberate_exit` is in src/eval.rs now rather than
src/main.rs, because main.rs is the binary and the page never compiles it. The
page's endpoint gained the two arms the native endpoint has had all along. No
behaviour changed on native or on the oracle — the function is the same text
at a new address.

PERF. Nothing on the compile path calls either function, and the veins are
host-divergent here, so CI measured them. `compile_instructions` fell 159, from
57,489,912 to 57,489,753 — 0.0003% — and is banked. Layout rather than work,
by the same argument the -251 and the +1,954 above it carry: `kanso check
lib/json` compiles a library and runs no program, so it reaches neither the
driver's endpoint in main.rs nor the page's in wasm_rt.rs, and eval.rs only
HOLDS the moved function. The counters that measure the front end's work are
identical, allocations 61,981 and peak 822,004. Fifth movement of this vein in
two days with an untouched call graph: +167, -251, +1,954, -159. Welfare reads
84.12 against a floor of 84.12 — a fall this small cannot move a two-decimal
score, so there is nothing to ratchet.

AND THE SECOND ONE, FOUND BY ASKING WHETHER THE FIRST HAD A SIBLING. STATUS.md
had said for two days that `main is not an io` at src/wasm_rt.rs:1132 was the
wasm twin of the driver's `main is not an io; there is no plan to show`. It is
a different message on a different path: the driver's fires on `--plan` when
main is a value, and this one is the catch-all of `exec_slot`, on the execution
path, with no native counterpart at all. The file also left open whether any
program could reach it.

One can.

    x = 2

    io/write "one" >> x

`never_describes` in check.rs refuses a literal, a list, a map, a lambda or a
direct non-piped call on either side of a wall. A bare NAME is none of those,
so the check lets it through and the wall meets a plain value at run time.
Native and the oracle both say

    error[runtime]: `>>` sequences two effect descriptions

and exit 1. The page said `error[runtime]: main is not an io` and exited 1. Two
engines naming one fault and a third naming a different one is the divergence
the law forbids, and the page's sentence is also simply wrong: main IS an io.
Its right-hand side is not.

WHY THE PAGE TAKES A DIFFERENT PATH. GAVEL 15 defers a wall's right side, so
`rt_seq` builds a `Slot::Seq` holding a cell rather than deciding anything, and
what that cell answers is unknown until `exec_slot` demands it. `rt_seq`'s own
non-deferred arm already says the right sentence; the deferred one fell through
to the catch-all. The guard is in `exec_slot` now, at the demand, and it
returns the same string.

That makes the catch-all unreachable, and the argument is construction rather
than a reading of what looks unlikely: `exec_main` tests before it calls,
`rt_seq` builds a Seq only when its LEFT side is descish, `rt_maybe_bind`
builds a Bind only when what is piped in is, and the one side not decided at
construction is tested at the demand. The arm stays because the match must be
exhaustive, and the comment above it says all of that.

The pin is tests/golden/runtime/a_wall_whose_right_side_is_a_name.kso, which
fits that corpus exactly — it exits 1 with a message, which is the corpus's
definition. Watched red by removing the guard and rebuilding the blob:
`left: "oneerror[runtime]: main is not an io\n"`.

STILL OPEN, AND MEASURED RATHER THAN GUESSED: whether the CHECK should catch
this instead, so the reader gets a span at compile time rather than a partial
run. `never_describes` already asks the inference fixpoint what a call returns;
a bare name would be the same question at arity zero. That is a better answer
for the reader and a change to what the language refuses, so it is filed rather
than folded in here. The runtime guard is needed either way — a piped call, a
non-`Ident` head and a parameter all reach the wall with the check unable to
say.

## 2026-08-27 — the micro corpus never read a stderr golden

Third instance of one shape in one day. A construct's FAILURE case is pinned
and its success case is pinned nowhere:

    os/exit         exit_needs_a_status         the success case: nowhere
    >> two effects  sequencing_takes_two_...    the runtime path: diverged
    io/write_err    write_err_takes_a_string    the success case: nowhere

The first two were fixed this morning. This is the third.

`tests/golden/micro/write_err_stream.kso` writes `err one\n` to the diagnostic
stream between two writes to the ordinary one. Its `.out` golden holds the two
ordinary lines, and there was no other golden — `golden.rs` reads `"out"` for
this corpus and never `"err"`.

WHAT THAT LEFT UNPINNED, precisely. The `.out` golden does prove `err one` did
not go to stdout, and the wasm walk in tests/wasm_engine.rs compares native's
`stdout + stderr` against the page's, so the ENGINES are held to each other on
both streams. What nothing held was the bytes themselves: a change that dropped
the write on all four writers at once — the interpreter, the compiled binary,
the wasm runtime, the browser — leaves `.out` unchanged and both concatenations
equal, and turns nothing red. Agreement is not a pin.

WHAT IT ASSERTS NOW. Every micro program was run as a library and its stderr
collected before the rule was written, rather than after: 136 programs export
`play` and exactly one of them says anything on that stream. So the rule can be
the strict one. A program with no `.err` beside it must be silent there, which
is a new assertion for 135 of them; the one that speaks carries a golden saying
exactly what it says. Both the library path and the release-built binary check
it, because the compiled binary is a fourth writer of those bytes and the
stream it chooses is as much a fact about the program as the bytes are.

WATCHED RED AT THE IMPLEMENTATION, not at the fixture. Deleting the write from
the .kso would have proved only that a golden matches a file. Two perturbations
of `RealExecutor::write_err` in src/eval.rs instead:

    eprint! -> print!    caught at golden.rs:384, the STDOUT assertion:
                         left "out one\nerr one\nout two\n"
    the write dropped    caught at golden.rs:389, the new one:
                         left "" right "err one\n"

The first is worth keeping in the record: a stream swap is caught by the
assertion that was already there, so only the second shows the new one doing
work. A perturbation that reddens an existing assertion proves nothing about
the one just added.

NO COMPILED CODE CHANGED. `src/eval.rs` is byte-identical to main, edited and
restored twice. The corpus gained one file and tests/golden.rs gained two
assertions, so no vein can move.

## 2026-08-27 — the bare-name wall check is refuted by a program

The entry above this one's sibling filed a question and recommended nothing,
which was right: the recommendation it would have carried is wrong, and one
program says so.

`never_describes` refuses a literal or a direct call on either side of a wall
and not a bare name, which is how `io/write "one" >> x` reaches the run. The
obvious extension is one line — src/parser.rs:624 makes a constant an `FnDecl`
with zero params, so `returns.get(&(name, 0))` is the same lookup the pass
already does for calls, and tests/wall_takes_effects.rs made exactly this
extension once before, from literals to calls, calling a call "the same case
one step out".

TWO SHADOWING CASES, and reading found only the first.

A local shadowing an OWN top-level declaration cannot happen: the compiler
already refuses it with "`x` is already a declaration; rename the binding".

A local shadowing a BARE-ENROLLED IMPORT is legal, and this program runs
correctly today — it prints `one` then `shadowed` and exits 0:

    import "std/io"
    import "std/list"

    pub play =
      naturals = io/write "shadowed\n"
      io/write "one\n" >> naturals

`list/naturals` is one of exactly four zero-arity stdlib constants — with
`io/stdin`, `os/args` and `time/now` — and it answers a plain value. `returns`
is built from ALL of `program.fns`, synthetic clones included, so the lookup
finds the stdlib row and the local is invisible to it. The one-line extension
refuses this program, and check.rs:1085 states the rule that would break: "the
enrollment must never make every stdlib export a forbidden binding name".

So the extension needs locals in scope, which neither `never_describes` nor
its sibling `refused` tracks. That is a real cost against a real benefit and
the question stays open; what has changed is that it is now a question about
scope rather than about one line. The program above is the case that must not
be refused, in the role tests/wall_takes_effects.rs already keeps for its own
third example, and it belongs in the corpus whichever way the question falls.

The runtime guard that shipped alongside it is needed either way.

## 2026-08-27 — the std surface has no coverage gap, and the naive search says 24

Three bugs in one day shared a shape: a construct's failure case pinned, its
success case pinned nowhere. `the_wasm_engine_complains_the_way_the_others_do`
in tests/wasm_engine.rs is that shape written down as a harness — it walks
every `pub fn` in lib/ and hands each one arguments of the wrong type, on all
three engines. Nothing walks the surface the other way. So the obvious next
move was to find which exports no program ever calls successfully, and gate it.

There are none. The measurement is recorded because the FIRST answer was 24 and
it was wrong, in a way anybody repeating the search would repeat.

    grep for `list/argmax` across examples, tests/golden, book   ->  24 uncovered
    also grep for the bare-enrolled `argmax`                     ->   1 uncovered
    also count intra-library calls                               ->   0 uncovered

Eight of the nine survivors of the second pass are called by their bare name
after an `import "std/list"` or `import "std/path"`, which is how a program
normally writes them. The last, `json/escape_onto`, is `pub fn` in
lib/json/text.kso and called from lib/json/json.kso:52 on every string a JSON
encode touches — covered by the busiest path in the tree, and invisible to a
search that reads only the corpus directories.

Two searches this month have now been wrong in exactly this way: the coverage
scan's twelve-character floor hid three reachable diagnostics, and this one hid
eight reachable exports. The rule they share is that a name in kanso has two
written forms and a search that knows one of them is measuring its own blind
spot rather than the tree.

So no gate, and no finding. What today's three bugs have in common is not an
uncalled function — every one of them was in a function the corpus calls
constantly. It is the ENDPOINT around the call: what the exit code carries,
which stream the bytes land on, which sentence names the fault. Coverage of the
surface would not have found any of them, which is worth knowing before the
next sitting spends a morning building it.

## 2026-08-27 — the bare name at a wall, refused at compile time after all

The entry recording this as refuted was right about the naive version and
wrong about the conclusion. The one-line extension does refuse a working
program; a set of the names a declaration binds is enough to fix that, and
costs one walk of each body.

WHAT THE NAIVE VERSION DOES, shown rather than argued. With
`Expr::Ident(name, _) => returns.get(&(name, 0))...` added to
`never_describes` and nothing else, this program is refused:

    naturals = io/write "one\n"
    first = io/write "two\n"
    naturals >> first

It prints `one` then `two` and exits 0 on both engines. `list/naturals` and
`list/first` are bare-enrolled and answer plain values, so the lookup finds the
stdlib rows; the locals are invisible to it. That was watched happening — the
compiler was built with the arm and run on the program — rather than read out
of the source.

WHAT FIXES IT. `never_describes` now takes the set of names its declaration
binds: the parameters, the patterns of every `Stmt::Bind` at any depth, and
every lambda parameter under it. A name the declaration binds belongs to the
local, whatever the fixpoint says about a top-level constant sharing it.

The set is deliberately over-wide and does NOT model scope — a name bound
anywhere in a declaration shields it everywhere in that declaration. The cost
of being loose this way is a refusal not made rather than one made wrongly,
which is the side to be loose on: the run still names the fault, and since
#1090 it names it identically on all three engines.

WHERE THE LINE FALLS NOW:

    io/write "one" >> x        x = 2 at top level     compile error, with a span
    io/write "one" >> twice    twice is a fn, arity 1  runtime, as before
    naturals >> first          both are locals         runs

The middle row is why #1090's runtime guard is still load-bearing and why
a_wall_whose_right_side_is_a_function stays in the runtime corpus: `returns` is
keyed by (name, arity) and a bare `twice` is arity zero, which that map has no
row for. The first row's fixture moves to the error corpus, which is what
catching it earlier means.

The guard program is at tests/golden/micro/a_wall_whose_name_is_a_local, with a
shadowed stdlib name on each side of the wall. It was written and watched
refused before the fix existed, which is the only order in which it proves
anything.

AND THE WALK MISSED A THIRD BINDER, which the whole suite was green over.
Reading the new code against the `Expr` enum rather than against itself: three
variants bind names — `Lambda` its parameters, `Block` and `Build` their
statements — and a fourth, `Guard`, carries a statement list of its own in
`rest`. A binding that follows a `return` line lives there rather than in a
block, so this was refused:

    pub fn go n
      return io/write "early\n" if n < 0
      naturals = io/write "one\n"
      naturals >> io/write "two\n"

73 test binaries passed with that gap in place, because no fixture in the tree
binds a shadowing name inside a guard. The corpus cannot catch what it does not
contain, and the enum can: a walk that collects binders is complete or it is
not, and the way to find out is to read it against the list of things that
bind. The fixture covers all three forms now, and was watched red with the
`Guard` arm removed — `left: ""` where the golden wants four lines, because the
program no longer compiles.

## 2026-08-27 — welfare refused the wall check, and it was right to

The bare-name refusal shipped in a form that cost more than it was worth, and
the objective said so before any reviewer could.

    compile_allocs        61,981 -> 62,518    +537   (+0.87%)
    compile_instructions  57,489,753 -> 58,148,592  +658,839  (+1.15%)
    compile_peak_bytes    822,004 -> 822,004        0
    welfare               84.12 -> 84.06, against a floor of 84.12

The rise was real work and it was measured as such rather than guessed: main
was built and measured on THIS host beside the branch, because the golden was
taken on a different rustc and a branch-versus-golden delta would have mixed
the change with the toolchain. Allocations moved because the walk collected the
names every declaration binds; peak did not, because each set is transient.

WHAT THE FLOOR IS FOR. `welfare --set` cannot lower it — ruled 2026-08-03 —
and the file's own rule is that a fall means the change is worse by the
project's stated preferences, so either the change goes or the argument is
about the WEIGHTS. There is a third answer when the price is avoidable, and
here it was. The walk collected names for every declaration; most hold no wall
at all. The set is built on the first `Seq` the existing walk meets now, so a
body with no wall pays nothing and the traversal is the one that was already
happening.

    compile_allocs        61,981   identical to main
    compile_peak_bytes    822,004  identical to main
    welfare               84.12, exactly the floor

The refusal still fires and the shadowing programs still run — all three were
re-checked after the change, not assumed to survive it.

The reading worth keeping is what the number did. A 0.06 fall is far below what
anyone would notice by eye, on a change whose behaviour is plainly an
improvement, and the honest first move was to write the attribution and bank
it. The floor made the cheaper implementation the thing to look for instead.
That is the whole of what a single scalar over runtime and compile cost is for.

## 2026-08-27 — a docs check silenced the allocations vein

Found while reading the cost-goldens job log for the number above, and it is a
gap of the kind the project's own rule names: movement is fine, silence is not.

`compile allocations` was the ONE counter step in that job without
`if: always()`. Every sibling has it — one-shot, basket, compile instructions,
compile memory, encode. So when an earlier step fails the job outright, that
step alone is skipped, and the summary loop at the end fails only on `failure`
and reads `skipped` as nothing to report.

The chain on this run, from the log rather than from reasoning:

    page_drift FAILED       the log was 4 entries ahead of a budget of 3
    one-shot   if: always   ran
    basket     if: always   ran
    compile allocations     SKIPPED
    compile instructions    ran, failed, reported its number

So a docs-freshness budget took down the gate whose golden header calls it
"the traffic the front end makes, which no other gate can see". Allocations
rose 537 on that run and the job said nothing about it. The step has
`if: always()` now, with the reason written beside it.

## 2026-08-27 — the layout reading was right this time, and measured twice

The entry above predicted the wall check's instruction rise would be REAL WORK
and said so in the pull request. For the first implementation that held. For
the one that shipped it does not, and the correction matters because it changes
the attribution.

    the runner        57,489,753 -> 57,571,608     +81,855   (+0.14%)
    this container    58,158,740 -> 58,162,339      +3,599   (+0.006%)

Both binaries profiled under callgrind side by side on one host for the second
row. A change that does more work moves both hosts by a similar amount. A delta
that is twenty-three times larger on one of them is layout, and the golden's own
header already records the shape: one diff measuring +664, +393 and -6,763 on
three sittings.

The reachability argument agrees and is the stronger one. `lib/json` — the
library this vein measures — contains no `>>` at all. Zero. So
`never_describes` is never called on the measured path, the bound-name set is
never built, and none of the new code runs. `compile_allocs` at 61,981,
`compile_peak_bytes` at 822,004 and visits at 16,806 are identical for that
reason, and their agreement is evidence of unreachability rather than of thrift.

WHAT THE VEIN CANNOT SEE. A program that sequences effects pays one set per
wall-bearing declaration, and `kanso check lib/json` compiles a pure library
that never asks. So this row is silent on the feature's real cost, and saying
the feature is free would be reading its silence as an answer.

THE FLOOR MOVED, ON THE RULING'S AUTHORITY. Clay's ruling of 2026-08-25 — the
floor is absolute against refactorings and permeable to the language — governs
it, and this is a language change, since a program that used to run partway and
die is now refused before it starts. The 84.79 and 84.12 entries in
bench/welfare_floor.json have this shape. 84.12 -> 84.11, with the two-host
measurement in the reason.

THE MECHANISM DIFFERED FROM PRECEDENT, and the first two versions of this
paragraph were both wrong about why. The first said `--set` accepting the fall
was the sanctioned path. The second called the guard's prose and its code
contradictory and flagged it for Clay. Searching the log rather than reading
the source settled it: the 2026-08-25 entry above already read the same code
correctly — "welfare --set refuses a fall of more than 0.01 and refused this
one. That refusal is the design — its own comment names hand-editing
bench/welfare_floor.json as the single override, precisely so the move appears
in a diff a reviewer reads rather than behind a flag."

So there is no contradiction, and nothing here for Clay. The prose describes
the REFUSAL, and a fall of 0.00 does not reach it. The established practice for
a language-change fall is the hand edit; `--set` wrote this one, which puts the
same line in the same diff, so a reviewer sees it either way. Worth naming
because a third sitting will meet a sub-threshold fall and should not have to
rediscover which of the two paths precedent uses.

THE LESSON IS THE FILE'S OWN RULE, applied to me. An entry cites its search or
it is invalid, and I wrote two paragraphs about this guard before searching for
what the log already said about it. Both were confident and both were wrong,
and the search that fixed them took one grep.

The order of operations is the part worth keeping. The expensive version fell
0.06 and the first instinct was to bank it — goldens edited, page figures
moved, paragraph drafted. The floor is what sent me looking for the cheaper
implementation instead, and the cheaper one costs nothing measurable at all.

## 2026-08-27 — half the native runtime's diagnostics were pinned by nothing

`scripts/diagnostic_coverage` is the ratchet that keeps a compiler message from
being reworded, weakened or lost with nothing going red. It walks `src`, and
`source?` admits a file only when its last three characters are `.rs`. So it
has never read `src/runtime.c`, and `src/runtime.c` is where a compiled binary
gets its runtime messages.

There are 66 distinct `k_die("...")` texts in that file. Thirty-three are
pinned by no golden and no Rust test. The gate that exists to prevent exactly
this could not see any of them.

SEARCHED FIRST: the log and the archive for `diagnostic_coverage`, `runtime.c`
and `k_die`. #1079 widened the same scan twice — from `Diagnostic::new(` to
`"error: ` (the driver's plain-text writes) and then to `"error[` (a rendered
diagnostic written as plain text). Both widenings stayed inside `.rs`, and
neither entry asks what other file extensions carry messages. Nothing in either
vein covers the C.

THE ARITHMETIC FAMILY IS THE CLEAREST CASE. `+` on two values that have no `+`
is pinned twice, by `none_no_arm` and by `a_constant_that_names_itself_is_
demanded`. `-`, `*`, `/` and `%` are pinned nowhere, and all four are reachable
from a program anybody could write: `xs[9] - 1` on a two-element list, which is
the same idiom `none_no_arm` already uses for `+`. One of five.

Eight are pinned here, each run through the library entry the harness builds
and each byte-identical on both engines with exit 1:

    `-` `*` `/` `%` is not defined for these values   none_{minus,times,over,modulo}_one
    concat takes two lists                            text/concat "a" "b"
    join takes a list of strings and a separator      text/join 7 ","
    put takes a map, a key, and a value               put (opaque 7) "k" 1
    entries takes a map                               entries (opaque 7)

The last two need `opaque` and the first two do not, which is worth keeping.
`put 7 "k" 1` and `entries 7` are refused by the type checker at compile time;
the runtime message is the backstop for a value whose type the checker cannot
narrow, and it is reached by passing the argument through a function that
returns what it is given. `text/concat "a" "b"` and `text/join 7 ","` reach the
runtime with a plain literal. Both routes are worth a fixture, because a change
that made the checker stricter would silently retire two of them.

WATCHED RED, ONE AT A TIME. `concat takes two lists` reworded to `two
sequences` in runtime.c reddens `concat_takes_two_lists` and nothing else;
restored, `entries takes a map` reworded to `a mapping` reddens
`entries_takes_a_map` and nothing else. runtime.c is byte-identical to its
starting state after both.

WHAT IS NOT DONE HERE. Widening the scan to read `k_die("` needs every
remaining message either pinned or excused with a named mechanism, and
`tests/golden/unpinned_diagnostics.txt` sets that bar deliberately high — its
own header records a sitting spent for want of four missing reasons. Probing
the remaining twenty-five is the work, and it is its own change.

Two findings from the probing that are also their own changes:

SIX SOCKET FAILURES DIVERGE between the engines. A look-alike record — a type
with a `handle` field, holding an int — through each net builtin:

    net/port            native "that is not an open socket"  interp "that is not an open listener"
    net/accept          native "nothing connected"           interp "7 is not a listener"
    net/read            native "that is not a connection"    interp "7 is not a connection"
    net/write           native "that is not a connection"    interp "7 is not a connection"
    net/close_listener  native "that is not an open socket"  interp "7 is not an open socket"
    net/close_conn      native "that is not an open socket"  interp "7 is not an open socket"

Both engines speak, so the differential law is broken rather than satisfied by
a refusal. `net/accept` is the bad one: "nothing connected" describes a
successful accept with no pending connection, and native says it for a handle
that is not a listener at all.

A FOURTH WAY THE COMPILER WRITES A MESSAGE, after #1079's three. `codegen.rs`
bakes text into the emitted binary through `format!` — `cannot destructure
value as \`{ty}\`` and `field \`{field}\` of \`{name}\` takes {}` are pinned
nowhere, and neither matches any of the scan's three openers. Widening to
`format!` in general would match every format string in the compiler, so this
one needs a narrower key than the other three did.

THE SHAPE IS THE ONE §25 NAMES, for the fifth and sixth time in a day: a
construct's failure is pinned and its success is not, or a family's first
member is pinned and its siblings are not. `+` had two goldens; its four
siblings had none. The way to find these is to enumerate what the source can
say and subtract what the corpus holds — and to check what the enumeration
cannot see, because both of my own first passes at that subtraction had blind
spots. Matching message tails against the Rust sources reported thirteen
messages "no Rust engine can produce"; three of them are `format!("{name} takes
a string")`, which the tail search cannot match and the interpreter says every
day.

## 2026-08-27 — six socket failures said seven different things

Found by the probe sweep in the entry above, running each net builtin against
a look-alike record: a type with a `handle` field holding an int, passed
through a function that returns what it is given so the checker cannot narrow
it. Both engines reach the builtin, both speak, and they said this:

    net/port            native "that is not an open socket"  interp "that is not an open listener"
    net/accept          native "nothing connected"           interp "7 is not a listener"
    net/read            native "that is not a connection"    interp "7 is not a connection"
    net/write           native "that is not a connection"    interp "7 is not a connection"
    net/close_listener  native "that is not an open socket"  interp "7 is not an open socket"
    net/close_conn      native "that is not an open socket"  interp "7 is not an open socket"
    os/kill             native "that is not a running process"  interp "999 is not a running process"

The differential law allows an engine to speak less than another only when the
quieter one refuses with a clear diagnostic. Here neither refuses. None of the
seven was pinned by a golden, which is why they drifted.

SEARCHED FIRST: log and archive for `socket`, `listener`, `net/accept` and
`nothing connected`. The sockets work is recorded at its introduction and in
the fiber-scheduler entries; no entry compares the two engines' socket
messages, and no golden holds any of these texts.

THE INTERPOLATED HANDLE GOES. `7` is a slot number kanso hands out; the program
never wrote it and cannot look it up, so it reads as though the user supplied a
7. The house style that names a value — `length takes a list, string, or map,
not 7` — names a value the user WROTE, which is a different thing. The
interpreter converges on native's `that`.

THE NOUN ON `net/port` WENT THE OTHER WAY FIRST, AND THE GOLDEN OVERRULED IT.
Its argument is a listener, the interpreter has always said `listener`, and
native reached for `socket` — the right umbrella only for close, which accepts
either kind. Changing native to `listener` cost 128 bytes of .text in every
compiled binary, on all eight benchmarks.

For a two-character string. The cause is not length: a distinct string of the
SAME length costs the same 128. Section-by-section, the port change moves
.rodata by 32 (the string, at 32-byte granularity) and .text by 128 (code).
While case 28 and case 24 shared one string pointer, clang folded their two
`return k_err(k_str(<ptr>), NULL)` tails into one block; a distinct string
breaks the fold and the sequence is emitted twice.

Both words are accurate — a listening socket is a socket — and native's is
already what close says for either kind. So `port` converges on `socket`, the
interpreter changes instead of native, and the 128 bytes are not spent. The
gate is what turned a wording preference into a measurement.

The accept check reuses the string `k_step` already carries for the same fault,
for the same reason, and costs 48 bytes: one call and one branch.

`net/accept` was not a wording difference. Native's arm never looked at the
handle at all:

    case 21: {
        /* Only reached outside a parallel group, where nothing else could
           ever connect; k_step yields instead of arriving here. */
        return k_err(k_str("nothing connected"), NULL);
    }

"nothing connected" is a true statement about a listener nobody has dialled,
and it was said for a value that is not a listener. The comment is right about
when the arm runs and says nothing about what it was handed. It now checks
`k_socket_of` first.

WATCHED RED, three separate mutations, each reddening exactly one fixture and
nothing else: the accept check removed (native says "nothing connected"), the
port noun put back to "socket", and the interpreter's connection sites restored
to `format!("{conn} is not a connection")`. Both files diff clean against the
fixed state afterwards.

THE FIXTURES DO NOT LIVE IN THE RUNTIME CORPUS, and finding out why cost a
red suite. That corpus is walked by a THIRD engine — the in-process
interpreter in tests/oracle.rs — whose executor has no sockets and refuses
with "this engine has no sockets". The refusal is correct under the law and it
is a different sentence, so one shared .stderr cannot hold both. The corpus
asserts one text for every engine that walks it, and these two engines are
asserted in tests/sockets_say_one_thing.rs instead, the way
tests/a_file_that_is_not_text.rs already handles a capability difference.

Worth naming because the corpus looked like the obvious home right up to the
moment `cargo test` said otherwise, and the number of engines walking a given
corpus is not written anywhere a reader would look. There are three for
tests/golden/runtime, not two.

ONE SITE IS CHANGED WITHOUT A FIXTURE. `finished` in eval.rs — the wait half of
`os/run` — carried the same interpolated handle, and `os/run` builds its own
handle, so no program can hand it a bad one. It is aligned rather than pinned,
and this sentence is the record that it is unpinned on purpose.

WHAT THIS COST, AND THE GAIN THAT PAYS FOR IT. 48 bytes of .text on each of the
eight benchmarks, uniform. eval.rs alone moves .text by zero — measured with
runtime.c reverted and eval.rs kept — so the whole 48 belongs to the accept
check. bench/text_golden.txt is regenerated here, on a host whose clang matches
its measured-on line.

Against that, compile_instructions FELL 3,364: 57,571,608 to 57,568,244. The
socket executor stopped interpolating its handle, so five `format!("{conn} is
not a connection")` became a constant `&str` and the formatting machinery those
calls pulled in is no longer in the binary. `kanso check lib/json` opens no
sockets and runs none of that code; what moved is the binary around the
measured path, the same way the +81,855 in that golden's last note did.

THE FIRST PUSH CARRIED THE RISE AND NOT THE FALL. bench/text_golden.txt was
regenerated and bench/compile_instructions_golden.txt was not — a vein this
host may not measure at all, because its numbers have to be copied out of the
runner's job log. The trend gate read the pair and refused: "a pure regression:
something got worse and nothing got better." It was right about the files and
wrong about the change, which is what a half-regenerated pair of veins looks
like from the outside. The rule the log already carries — counters changed,
regenerate every vein in the same PR — covers exactly this, and the vein I
missed is the one I could not measure locally.

The allocation and arena veins are byte-identical to main, and welfare does not
weigh .text, so it is unmoved at 84.11.

## 2026-08-27 — the wasm gap list pinned a prefix

tests/golden/wasm_gaps.txt records what the page answers where it cannot speak
a feature — the differential law's sanctioned case, an engine refusing plainly
rather than diverging. Both harnesses read it, and both compare with a
CONTAINS: tests/wasm_engine.rs at 423 and 451 with `text.contains(answer)`,
scripts/browser_differential_run with `length (text/split text wanted) > 1`.

The listed answers were prefixes. wasm_rt writes longer sentences:

    listed                                 written
    the playground has no filesystem       ...: cannot read {path}
                                           ...: cannot write {path}
                                           ...: cannot make {path}
                                           ...: cannot list {path}
    the playground cannot start processes  ...: cannot run {cmd}

So the half that tells a reader WHAT the page could not do was pinned by
nothing. Five of the eight refusals wasm_rt writes had an unpinned tail.

MEASURED BOTH WAYS, which is what makes this worth doing rather than tidy.
`cannot read` reworded to `unable to read` in wasm_rt.rs, blob rebuilt:

    old list (prefix)     test passes           the gap this closes
    new list (sentence)   test fails, naming
                          the exact text

The path in each sentence is fixed by its fixture — read_missing_file reads
`/no/such/file`, make_dir_is_idempotent makes `.` — so the whole sentence is a
constant per row and nothing here needs a tolerance.

SEARCHED FIRST: the log and archive for `wasm_gaps`, `known gap` and
`playground has no`. The list's introduction and its two widenings are
recorded; no entry asks what the listed answer omits.

AND THE SCAN THAT FOUND IT WAS WRONG FIRST. It reported six of eight unpinned,
because it read `.stderr`, `.out` and `.rs` and wasm_gaps.txt is a `.txt`. Two
of the six are pinned there exactly. That is the fourth scan today to measure
its own blind spot — after the message-tail search, the qualified-name-only
export search, and the twelve-character floor before them. The pattern is
stable enough to state as a rule: a scan over a corpus must enumerate the
corpus's file types from the harness that reads it, not from the ones that
came to mind.

## 2026-08-27 — the ratchet reads the C now, and twelve messages have reasons

The half-the-runtime's-diagnostics entry, earlier today, found that
`scripts/diagnostic_coverage` had never read src/runtime.c, because `source?`
admits a file only when its last three characters are `.rs`. This closes it. `.c` joins `.rs`, a fourth extractor
keyed on `k_die("` runs beside the three the scan already carried, and the
count of literal diagnostics it can see goes from 109 to 175.

`k_die("` is the whole key, and it needs no rule about which extension gets
which extractor. The declaration is `k_die(const char* msg`, the one
runtime-valued call is `k_die(said`, and the seven `k_die(` in src/*.rs are all
`call void @k_die(ptr @{m})` in the emitter. None carries the quote, so running
every extractor over every source is both safe and simpler than teaching
read_src which opener belongs where.

THIRTEEN MORE ARE PINNED HERE, taking the unpinned count from 33 to 12:

    to_bytes takes a list of byte values          text/to_bytes (opaque 7)
    slice takes 1-based inclusive positions       text/slice "hello" (opaque "a") 2
    make_dir takes a path string                  os/make_dir (opaque 7)
    start takes a command string                  os/start (opaque 7) []
    start takes a list of argument strings        os/start "echo" (opaque 7)
    kill takes a started process                  os/kill (opaque "x")
    a group joins descriptions                    two adjacent statements, one not a description
    listen takes a port number                    net/listen (opaque "80")
    port takes a listener                         net/port (opaque (stand_in "x"))
    accept takes a listener                       net/accept (opaque (stand_in "x"))
    net_read takes a connection                   net/read (opaque (stand_in "x"))
    net_write takes a connection and a string     net/write (opaque (stand_in "x")) "hi"
    net_close takes a listener or a connection    net/close_conn (opaque (stand_in "x"))

THE SIX SOCKET ONES WERE NEARLY EXCUSED FROM READING, and the excuse would
have been wrong. Every wrapper in lib/net reads `l.handle` before calling the
builtin, so a non-record is refused by `` `.` reads a field of a record `` and a
record holding an INT handle reaches the executor rather than the argument
check — which is the divergence the entry above fixes. Both readings are true
and neither is the whole story: a record whose handle is a STRING passes the
field read, fails the builtin's own type check, and prints all six. One probe
found what two careful readings of lib/net had missed.

WHICH BUILTINS A PROGRAM CAN EVEN CALL, measured rather than assumed, because
half the remaining excuses turn on it. Of the fifty-eight names in check.rs's
BUILTINS, exactly seven answer to a bare call: `entries`, `length`, `print`,
`push`, `put`, `if` and `wrap_err`. Every other one — `map`, `filter`, `sort`,
`sum`, `concat`, `slice`, `join`, all the io and os and net names — answers
`unknown name`. So the only route to those builtins is a stdlib wrapper, and a
wrapper that guards first shadows the builtin's own refusal for good.

THE TWELVE THAT REMAIN, each with the mechanism written beside it in
tests/golden/unpinned_diagnostics.txt:

  - `map`, `filter`, `sort`, `sum takes a list`, `sum takes a list of int` and
    `a filter predicate returns true or false`: lib/list's wrappers call
    `length` on the collection and `if` on the predicate first, so a program
    meets `length takes a list, string, or map, not 7` or `an if condition is
    true or false, got 1` instead. `list/sum` is a fold over `+`.
  - `print takes a string; interpolate instead`: the renderer does not enforce
    it. `print 7`, `print [1 2]`, `print <a function>` and `print <an effect>`
    all succeed, writing `7`, `[1 2]`, `<fn>` and `<io>`.
  - the two string-builder checks: behind `bytes takes a string`, which is
    pinned.
  - three resource caps, each reachable and each fixture costing more than it
    proves: 2 GB for `string too long`, 64 bound ports for `too many sockets`
    (the wedged-suite day in sockets_serve.rs's header), 257 adjacent
    statements for the fiber cap at runtime.c:4822.
  - `integer overflow`: pinned exactly, by docs/book/samples/ch02/overflow.out,
    which the scan's corpus cannot see because it takes `.stderr` files only.
    A citation rather than a widening — admitting `.out` would also admit every
    micro golden and needs its own look at false pins, of which the tests/*.rs
    corpus already produced four.

THE GATE HAS A MUTATION, like the three openers before it. `k_b_sum`'s refusal
is the bait, chosen because no kanso program can reach it: the mutation cannot
change what any fixture prints while it is applied. Applied, the gate exits 1
and names the message; restored, it exits 0 and runtime.c diffs clean.

## 2026-08-27 — a seventh divergence, on THREE engines, and native was the poorer one

A destructuring bind whose value is the wrong shape. `point a b = opaque
"hello"`, where the type is `point x y`:

    native  cannot destructure value as `m/point`
    interp  cannot destructure "hello" as `m/point`; bindings are irrefutable,
            so handle other types by dispatch first

Both speak, so the differential law is broken the same way the six socket
refusals broke it earlier today. What differs is the direction of the fix. There the interpreter interpolated an internal socket handle that kanso
hands out and the program never wrote, so dropping it was right. Here `"hello"`
is a value the reader's own program produced, which is exactly what the house
style names — `length takes a list, string, or map, not 7` — and the clause
after the semicolon is the only place the language says why a bind cannot fail
over to another arm. So native gains both halves.

SEARCHED FIRST: the log and archive for `destructure`, `irrefutable` and
`k_check_rec`. The bind's irrefutability is recorded at its introduction and in
the dispatch entries; no entry compares the two engines' wording for it.

FOUND WHILE PROBING SOMETHING ELSE, and two earlier probes missed it because
the syntax is not what it looks like. A Ctor bind target is written WITHOUT
parens — `point a b = v` — and `(point a b) = v` is a syntax error, so the
first two attempts got `expected a binding name or type` and read as though the
form did not exist. `parse_bind_target` at parser.rs:1901 is where it is
decided.

THE FIX HAS A PRECEDENT IN THE SAME FILE. The KEYED destructuring form,
`{ author: writer } = post`, already renders the value: `k_keyed_check` calls
`k_render` and prints what it got. The positional form baked a sentence at
codegen time through `format!` and interned it, so it had nothing to render.
`k_die_destructure` is the sibling it was missing, beside `k_die_overload` and
`k_die_arity`, which are the two other die helpers that take runtime data.

IT IS THREE ENGINES, NOT TWO, AND THE FIRST FIX ONLY MOVED ONE. The page has
its OWN emit path — `wasm_backend.rs:639` baked the same sentence native did,
and `RT_DIE` takes a message pointer with nowhere to put a value. Fixing native
alone left the page saying the old words, which the corpus walk caught and no
amount of native-vs-interpreter probing would have. The page gains
`rt_die_destructure` (import 40, two parameters like `rt_no_field` beside it),
and the backend passes the value the `local_tee` already held.

So the count of places one sentence lives is three: eval.rs for the
interpreter, runtime.c reached from codegen.rs for native, and wasm_rt.rs
reached from wasm_backend.rs for the page. The socket entry above says three
engines walk tests/golden/runtime; this says the same thing about where a
message is WRITTEN, which is a different list and had to be learned separately.

THE FIXTURE HOLDS A STRING, AND THE FIRST ONE HELD AN INT. Unquoted rendering
agrees with the interpreter on `7`, on `[1 2 3]` and on `1.5`, and diverges
only on a string, because the interpreter renders quoted — `render(self,
&value, true)` at eval.rs:1283. An int fixture would have gone green over the
bug it was written to catch. Four shapes were run before the fixture was
chosen, which is the only reason the quoting was found at all.

WATCHED RED TWICE ON NATIVE, one axis each: the quoting reverted to
`k_render(v, 0)`, and the call site put back to the baked sentence. Each
reddens the fixture with exactly its own difference, and both files diff clean
afterwards. The page's half was watched red by accident and more convincingly —
it was still red after native was fixed, which is how the third site was found
at all.

COST: 146 instructions on the front end, and nothing else. Every RUNTIME
counter vein is byte-identical and welfare holds at 84.11, machine code
included — and that reason is checkable rather than lucky. No benchmark writes
a Ctor destructuring bind, so the linker drops `k_die_destructure` from all
eight binaries; `nm jsonbench` finds neither it nor `k_die_overload`, which has
always been dropped the same way.

compile_instructions rose 57,568,244 -> 57,568,390 on the runner. The measured
path is untouched: `kanso check lib/json` compiles a library and never emits,
so neither changed call site runs. What grew is the binary around it —
src/runtime.c is embedded whole by `include_str!` at main.rs:722 and gained a
function, and src/wasm_rt.rs is an unconditional `pub mod` that gained one too.

ONE MECHANISM WAS RULED OUT RATHER THAN ASSUMED. main.rs:855 hashes that
embedded runtime.c, and hashing a longer file would be a real cost the check
path pays. But the hash sits in `cached_program_binary`, called only from the
RUN path at main.rs:810, and `check` never reaches it. So this is layout, like
the five movements before it in that golden, and not the plausible thing it
turned out not to be.

## 2026-08-27 — one question, three answers: `if` and the guard

`return x if cond` and `if c a b` ask the same thing of a value, and the three
engines answered it three ways:

    interpreter, guard   a return condition is true or false, got 7
    native, both forms   an if condition is true or false, got 7
    the page, both forms if takes a bool condition (got "7")

Nothing pinned any of them. The runtime corpus had no program whose condition
was neither true nor false, so all three could be reworded, and one already
had been.

WHERE EACH COMES FROM. Native lowers a guard to `k_truthy`, the same call the
`if` builtin makes, and runtime.c says why in a comment at `k_truthy_bad`: the
die message lives in exactly one place. The page does the same — `rt_truthy`
serves `Expr::Guard` at wasm_backend.rs:787 and `if` at 1300 — so its one wrong
sentence covered both constructs. Only the interpreter splits them, and only on
the guard: eval.rs has three condition sites and two of them already said `an
if condition`.

THE CONVERGENCE GOES TO NATIVE'S SENTENCE, on both counts. The wording, because
the `if` token is in the source either way and a second wording would mean a
second entry point into `k_truthy` — which #1094 measured at 128 bytes of
`.text` in every binary, for two characters. And the rendering, because
`render_demanded(&value, true)` quotes a string where the two native engines do
not: `got "x"` against `got x`.

TWO FIXTURES, BECAUSE ONE DIMENSION EACH. The guard fixture carries an int and
pins the wording; the `if` fixture carries a STRING, because an int reads `7`
on all three engines and would have shown the wording alone. Both went red
first — the corpus walk on the interpreter, the wasm walk on the page — and the
`if` fixture had to be run with the guard fixture moved aside to see its own
failure, since the walk stops at the first disagreement.

COST: 57,568,390 -> 57,568,471, a rise of 81, and how that number was arrived
at is the finding.

Measured against the base before #1098, this same diff FELL 138: 57,568,244 ->
57,568,106. Rebased onto main with #1098 in it, it RISES 81. Nothing in the
diff changed between those two measurements — two string literals, neither of
which `kanso check lib/json` ever reads, because the front end evaluates no
conditions. What changed underneath it was #1098's two new runtime entry
points, which moved `.rodata`.

`__memcmp_avx2_movbe` tracks it both times: 1,371,161 on #1098's run and
1,371,229 here, +68 of the +81. That is the front end comparing interned names
with a vectorised load, and where a string starts decides how many loads it
takes.

So this vein answers to the linker as well as to the code, and the honest
reading of a small movement in it is that the bytes moved, not that the
compiler does more work. It is still worth having — it is the counter that
caught a quarter of the front end going away in silence — but eighty-one is
noise with a mechanism, and the mechanism is layout.

ATTRIBUTED RATHER THAN ARGUED. A rise with nothing improved is the one move the
trend gate refuses outright, and the escape is the gavel of 2026-08-25: the
floor is absolute against refactorings and permeable to the language. Three
engines disagreeing about one question is the differential law's business, so
the change lands and welfare_floor.json records the 81 against it.

## 2026-08-27 — a fifth file, and it is the oracle's

`scripts/diagnostic_coverage` reads four openers, and `src/eval.rs` matches
none of them. A `RuntimeError` is a struct literal with a bare `message:`
field, so all 97 of the interpreter's refusals could be reworded, weakened or
lost with nothing going red — on the engine the differential law calls the
truth.

THE OPENERS ARE TWO, not one, and the difference is where the literal sits.
`message: "` carries its own opening quote, so the chunk starts inside the
string exactly as the driver's `"error: ` does, and the message is the first
piece of a split on the quote. `message: format!(` does not: the string may sit
on the next line, so the message is the SECOND piece. 49 sites take the first
form and 47 the second.

THE 97TH IS A `match` with three arms, and it is why there is no third opener:
all three of its messages are already pinned — `` `.` reads a field of a
record, not `` by field_non_record, the other two by over_applied_group. A key
for that shape would buy nothing.

RUNNING BOTH OVER EVERY SOURCE IS SAFE, and it found something. `message: "`
appears nowhere else in src/. `message: format!(` appears once more, at
lib.rs:2169 — the ONE Diagnostic in the tree built as a struct literal rather
than through `Diagnostic::new`, which is why the scan could not see it either.
It is pinned twice over, by render_ownership and sub_render_ownership, so this
is a blind spot rather than a gap.

242 LITERAL DIAGNOSTICS, up from 175. Of the 72 eval.rs messages with enough
literal text to match, 14 were pinned by nothing, and every one was RUN rather
than read:

  - five reachable, both native engines agreeing, so each gets a fixture: the
    keyed read of a non-record, `if`'s non-bool condition (pinned by the entry
    above), `sleep`, `random` and `round` on the wrong type;
  - seven shadowed, each by a check that answers first, and the probe is the
    excuse. A `set` target must be a construction born in the same `build`
    block, so it is bound and it is a record. `_ = ...` and `7 = ...` are both
    `expected a binding name or type`, so the bind-pattern catch-all is
    unreachable. The `filter` builtin has no caller: it is not one of
    check.rs's 55 BUILTINS, `lib/list` writes `select` as a fold in kanso, and
    `builtin_filter` by hand is refused by name — what a program meets for a
    non-bool predicate is `list/select`'s own `if`. A mixed-type
    `sort` is refused by `comparison requires two values of one comparable
    type`. `builtin_nope` outside the standard library is refused by name. And
    `Value::TableFn` is built only in wasm_rt.rs, which installs the way back
    at init, so the escaped-closure arm answers a `None` that cannot occur;
  - and one was the divergence in the entry above.

THE SAME WRONG MECHANISM STOOD TWICE, and only one copy was caught before the
first push. `a filter predicate returns true or false` is TWO messages —
runtime.c's without the `, got`, eval.rs's with it — so the list carries two
rows, and it matches on equality, so neither excuses the other. The older row,
against native, said the shadow was `list/select`'s own `if`. That is what a
program meets for a non-bool predicate and it is not what makes this arm
unreachable: `list/select` is a fold written in kanso and never calls the
builtin, so the two sit on different paths. Native's arm is dead for the reason
the interpreter's is — `filter` is `unknown name`, `builtin_filter` is refused
by name, and `k_b_filter` is called from nowhere in runtime.c. Both rows now
say so.

TWO MUTATIONS, one per opener, both watched red and green. The baits are two of
the seven shadowed messages, chosen for exactly that reason: a mutation that
cannot change what any fixture prints.

COST: none, and this one can be asserted rather than measured. The change
touches no `.rs` and no `.c` — it is a kanso script, two shell mutations, four
fixtures, an excuse list and this entry — so the compiler binary is identical
and no vein has anything to move. The entry above is why that sentence is
written this way: it claimed "nothing measurable" about a change that DID edit
Rust, and the front end moved 138 instructions on string alignment alone.

## 2026-08-27 — the excuse named one barrier and there were two

The last entry left the scan reading five openers over four file types, and one
excuse on the list still said a message was pinned somewhere the scan could not
look:

    PINNED, BUT NOT WHERE THIS SCAN LOOKS. docs/book/samples/ch02/overflow.out
    holds this text exactly ... The scan's corpus takes `.stderr` files only,
    so it cannot see a `.out`. Widening it to `.out` would also admit every
    micro golden and needs its own look at false pins.

The look was taken, on each axis alone, because the excuse names one and there
are two. The corpus walk starts at `tests/golden`, and the file it cites is
under `docs/`.

    admit `.out`, root still tests/golden      0 now pinned
    walk docs/book/samples, `.stderr` only     0 now pinned
    both                                       2 now pinned, 0 false

Neither change does anything alone, which is the whole finding: the extension
was half the barrier and the directory was the other half, and an excuse that
named only the first would have kept the citation forever.

THE SECOND PIN WAS NOT LOOKED FOR. `main is not an io; there is no plan to
show` was excused a few lines up as pinned only by a Rust test — and
docs/book/samples/ch05/quiet_plan.out is that whole sentence and nothing else.
Two entries left the list for one change.

THE WORRY WAS MEASURED AND IS EMPTY. `.out` does drag in all 153 micro goldens,
and they pin nothing: those files hold what a program PRINTS, and a printed
line would have to match ten characters of a diagnostic exactly to read as a
pin. The four false pins on record all came from tests/*.rs — doc comments and
`assert!` strings — which is a different kind of file from a program's output.

COST: none. No `.rs` and no `.c`; the scan is a kanso program CI runs.

## 2026-08-27 — the page said "this value" about something that was not one

The entry above added twelve excuses and closed a scan's blind spot. The same
sweep, run over `src/wasm_rt.rs` rather than eval.rs, found twelve of the
PAGE's twenty-five refusals pinned by nothing — and one of them was a sentence
worth reading twice:

    rt_keyed_check:  cannot read fields of this value; keyed reads take a record

It sat behind `let Slot::V(value) = slot(h) else { ... }`, so it fires exactly
when the handle is NOT a value: a closure, which the page keeps in a table
rather than in the value register. A sentence about a value, said about the one
case that is not one.

REACHABLE, AND A DIVERGENCE. `{ x y } = opaque helper`, where `helper` is a
function:

    native       cannot read fields of <fn>; keyed reads take a record
    interpreter  cannot read fields of <fn>; keyed reads take a record
    the page     cannot read fields of this value; keyed reads take a record

THE GUARD COVERED TWO HANDLES, and the fix has to cover both. `Slot::V` is
false for a closure AND for a description, so that one sentence was said about
each of them. A closure goes through `val`, which is how a closure is data on
this engine everywhere else — `val` maps a `Slot::C` to a `TableFn` precisely
so it can ride in records, lists and maps. A description cannot: `val`'s own
fallthrough is `a bound description cannot be used as data here`, which is the
same fault as before wearing different words.

THE SECOND ARM WAS FOUND BY WRITING THE FIRST. Routing everything through `val`
made `{ x y } = opaque (io/write "a\n" >> io/write "b\n")` answer the
data sentence where both native engines say `<io>`, so the fix as first written
traded one divergence for another. The page keeps a `>>` of two descriptions as
a `Slot::Seq`, which is neither a value nor a closure.

So the slot answers for a description rather than `val`, and it answers without
building the `Desc`: every description renders `<io>` whatever it holds, and
`as_desc` on a `Slot::Seq` demands the deferred right side — an effect a
refusal must not have.

THREE FIXTURES FOR ONE CALL, one per handle. The string pins the quoting, which
is what `cannot destructure` diverged on earlier today; the function pins the
closure arm; the sequenced description pins the arm that is neither, and it is
in the corpus because the first draft of this fix broke it.

A SECOND CANDIDATE DID NOT REPRODUCE, and the negative is recorded rather than
dropped. `this value is not callable` sits beside `` `{}` is not callable `` in
the same function and fires when the callee is neither a closure nor a value —
a description, say. But `f = opaque (io/write "hi")` then `f 1` answers
`` `<io>` is not callable `` on all three engines, so this program reaches the
sibling arm. The program is in the corpus anyway, as
calling_a_description_names_it: the three fixtures already there call a number,
a `none` and a plain value, and a description is a value kind none of them
covers.

TEN MORE ARE STILL UNPINNED on the page, and they are the next thing:
`filter needs a bool (got {})` where the interpreter says `a filter predicate
returns true or false, got {}`, `map keys are ints or strings, not {}`,
`bad environment access`, and seven others.

COST: no compile vein moves — wasm_rt.rs is not read by `kanso check`, and the
emitted code is unchanged because the fix removes a branch rather than adding
one.

## 2026-08-27 — a description was data on two engines and not on the third

`val(h)` answers a value for a value and a handle for a closure, and refuses
everything else:

    _ => die("a bound description cannot be used as data here")

Four sites read their elements through it, and every one of them was a storing
position — a list literal, a map literal's values, a builtin's argument pack, a
record field written in a `build` block. So a description put into any
container died on the page and rode through on the other two:

    d = io/write "a\n" >> io/write "b\n"
    xs = [(opaque d)]
    pub play = xs[1]!

    native / interpreter   a
                           b
    the page               error[runtime]: a bound description cannot be
                           used as data here

Not a wording mismatch. The program succeeds on the oracle and dies on the
page, which is the divergence the differential law exists to catch.

`value_of` was already in the file and already does the right thing —
`as_desc` first, `val` otherwise — so the fix is four calls, not four
mechanisms. It is a strict widening: a value and a closure take the same route
they took before, and only the slot shapes `val` refused answer differently.
The map's KEYS stay on `val`, because a key is an int or a string and reading
one through `value_of` would only change which of two refusals a description
key gets.

THE FOURTH SITE WAS NOT EVIDENCE UNTIL IT WAS MEASURED TWICE. The first probe
of `rt_setfield` put the `build` block at the top level of a library and
invoked it with `kanso run`, which answered `is a library — nothing to run`.
The wasm walk then reported no disagreement, and that silence proved nothing:
the fixture never ran on either side. Reshaped to the form the corpus actually
uses — a `build` inside `pub play`, reached through a generated entry — it
passed `micro_corpus_agrees_across_engines` and the walk failed on it. Four
sites, four measurements.

WHY THE CORPUS DID NOT ALREADY SAY. `micro_corpus_agrees_across_engines` runs
native and the interpreter, and all five programs here pass on both. The walk
in tests/wasm_engine.rs is what covers the third, and nothing in the corpus
had put a description in a container.

FIVE FIXTURES, AND THE FIFTH IS THE ONE AT RISK FROM THE FIX. Four pin the
sites. The fifth pins ORDER: `as_desc` on a `Slot::Seq` calls `demanded` on the
right side, so materializing the description runs the cell that produces it,
and a fix that ran the description at construction time instead of at the
answer would leave the other four green. `built` prints before either write on
all three engines, and a_container_does_not_run_what_it_holds says so.

FOUR WAS NOT THE NUMBER, and finding that out is worth more than the fix.
Sweeping all 33 `val(` sites in the file rather than the four found by
following one bug turned up two more, both confirmed the same way:

    b = box (opaque d)        native/interp  a        the page  dies
    pub play = b.it                          b

    print "{opaque d}"        native/interp  <io>     the page  dies

A record CONSTRUCTOR is a fifth storing position. An interpolation is a sixth,
and it READS rather than stores — it is also the most ordinary thing a program
does with a value, which makes it the one a reader was most likely to meet.
Both are the same one-call fix and both ship here.

The first pass found four because it followed a single failing program into the
code around it. The sweep found six because it started from the accessor and
asked which of its callers a program can hand a deferred shape. The second
method is the one to use first next time.

COST: none. wasm_rt.rs is not read by `kanso check` and is not in the native
runtime, so no compile vein and no cost golden can move. `docs/kanso.wasm` is
rebuilt in the same commit because the walk refuses to run against a blob older
than its source — a guard worth naming, since it turns a stale artifact into a
red test rather than a green one that proves nothing.
## 2026-08-27 — the walk could not say how much it had walked

The test that catches every three-engine divergence — the one that caught the
keyed-read wording this morning and the description-in-a-container this
afternoon — ended on this:

    assert!(ran > 0, "nothing in the corpus ran on wasm");

One surviving program satisfies it. Two hundred and seventy could stop running
and the walk would stay green, which is the shape of a silent truncation
reading as full coverage.

The gaps side of the same function is exemplary and worth saying so:

    assert_eq!(met + unrunnable, gaps.len(), "a program in wasm_gaps.txt was never reached")

Every row of the gap list must be reached, exactly. So this is one loose number
beside a tight one rather than a broken test.

A PROGRAM WAS ALREADY MISSING, and looking for the loose number is how it
turned up. `wants_a_filesystem` decided what the page cannot run by reading the
START of an import line:

    line.starts_with("import ") && !line.starts_with("import \"std/")

`examples/imports.kso` imports `"std/list"` and `t { slice:cut } "std/text"`.
The second names the stdlib and does not begin with `import "std/`, so the
example demonstrating aliased and selective imports was held out of the
differential — a program the page runs correctly, and the one form of import
no three-engine comparison covered. Reading the QUOTED path instead takes the
walk from 271 agreeing programs to 272.

WHAT REPLACED THE FLOOR is an accounting rather than a count, because a count
would need editing every time the corpus grows:

    ran + met + skipped.len() == corpus().len()

Every program was run, met as a listed gap, or held out for a reason on the
list, and none fell off it quietly. Beside it the skip list is pinned by NAME,
because a widening predicate is exactly how this goes wrong and a count cannot
see which program left. And each of the three walked directories must still
contribute, since `corpus()` shrinking would leave the accounting balanced.

THE SKIP REASONS WERE ONE SENTENCE FOR TWO PREDICATES. Both printed as
`relative import — neither host has a filesystem`, so the one program held out
for outrunning the runner's stack was described as something it is not. A skip
is a hole in the differential and the line recording it has to say which hole.

Watched red three ways: widening a skip predicate, reverting the import
predicate to the prefix check, and dropping a program from the walk without
counting it. The first two fail on the skip list, the third on the accounting,
and the third's message names all four numbers.

COST: none. tests/wasm_engine.rs is a test.

## 2026-08-27 — the page refused before three sites could explain themselves

The entry above moved six storing positions off `val` so the page could CARRY a
description. This is the other half of the same accessor: three sites that call
`val` to get a value, where its refusal preempts the sentence they wrote for
exactly this case.

    print "{if (opaque d) 1 2}"
      native / interpreter  an if condition is true or false, got <io>
      the page              a bound description cannot be used as data here

    print "{(opaque d)[1]}"
      native / interpreter  indexing takes a list or string with a 1-based
                            position, or a map with a key
      the page              a bound description cannot be used as data here

The `if` one is the sharper. That sentence was converged across all three
engines this morning, and on the page it was unreachable at the one input it
was written to describe.

THE TEMPTING FIX IS WRONG, and one fixture says so. `rt_index` and `rt_at`
delegate to the interpreter's own `index_value`, so handing them a real
`Value::Desc` through `value_of` would make the page agree by CONSTRUCTION
rather than by a copied string. But `value_of` calls `as_desc`, which demands a
deferred right side — and native does not:

    xs = [1]
    boom = io/write "{opaque xs[5]!}"     # errors if ever evaluated
    d = io/write "a\n" >> boom
    pub play = print "{(opaque d)[1]}"

answers the index refusal on both native engines. The out-of-bounds error never
appears, so `boom` is never evaluated. Demanding it to build a value we are
about to refuse would do strictly more than the oracle does. So the sentence is
copied, and the cost — one string living in two files — is now a thing a gate
can catch, because the diagnostic scan reads wasm_rt.rs as of the entry below.

FOUR CLAUSES, AND THE FIRST MUTATION WAS INCONCLUSIVE. `xs[i]` and `xs[i]!` are
different runtime entries — `rt_at` and `rt_index` — so a guard on one says
nothing about the other. Removing the index-position clause from `rt_index`
alone left every fixture green, which reads exactly like dead code; removing it
from BOTH turned `a_description_is_not_an_index` red, and removing `rt_index`'s
whole guard turned `a_description_is_not_a_strict_index` red. Both functions'
guards are live. Two fixtures were added for the `!` forms precisely because the
first mutation could not tell.

WHAT IS NOT CLAIMED: that each of the four clauses is individually necessary.
Two mutations proved two of them; the other two ride on the same line and have
not been isolated.

STILL OPEN: `rt_binop` answers `` `+` is not defined for these values `` on
native and has not been probed on the page. `map_or_filter` succeeds on native
(`list/map` over a list holding a description answers 1), so it is the carrying
family rather than this one, and the entry below may already cover it.

COST: none. wasm_rt.rs is not read by `kanso check` and is not in the native
runtime.

## 2026-08-27 — two refusals named each other, and a reader had nowhere to go

A file holding `pub play` is refused by both verbs, and each refusal prescribed
the other:

    $ kanso play wrong_verb.kso
    error[syntax]: `pub play` is a library's export — `kanso run` runs this
    file; `kanso play` takes bare statements

    $ kanso run wrong_verb.kso
    error: `wrong_verb.kso` is a library — nothing to run. give the module a
    main.kso entry, or run its definitions beside their statements with
    `kanso play`

Follow either sentence and you land on the other's refusal. Both refusals are
correct — `run` because the module has no entry, `play` because the form takes
bare statements — so the fault is entirely in the advice.

FOUND WHILE BUILDING SOMETHING ELSE. The description-in-a-container fixtures
needed a library run by hand, and the two messages sent me round the loop. The
file that demonstrates it was already in the corpus: `tests/golden/play/
wrong_verb.kso` is one line, `pub play = print "hi"`, and its golden pinned the
sentence that names a verb refusing that same file.

BOTH CLAUSES WERE WRONG, and each is wrong about a different thing. `play`
said `kanso run` runs the file; `run` answers it `is a library`. `run` offered
`kanso play`; `play` answers it with the sentence above. Neither message named
the route that works, which is to import the module from an entry file and name
its `play` — the shape `run_kanso_as_library` in tests/golden.rs has generated
for the micro corpus all along.

WHAT MOVED. `play`'s message names the import instead of `run`. `run`'s message
splits: a module exporting `play` gets the import route, and everything else
keeps the old sentence, because `kanso play` is a true suggestion for a file of
definitions beside bare statements. `exports_play` reads the parsed program
rather than a line prefix, so `pub play` inside a string or a comment is not
mistaken for the export.

THE SPEC ASSERTS WHAT A USER READS. `the_two_verbs_do_not_point_at_each_other`
runs both verbs on the one file, checks that neither prescribes a verb that
refuses it, pins both sentences exactly, and then — the part that would have
caught a message that was merely non-circular and still useless — stages the
module beside a generated entry and asserts the advised route prints `hi`.

Watched red: the negative assertion first, and then each exact assertion in
turn by mutating the message it pins. Green on restore.

`docs/kanso.wasm` is rebuilt here. The blob carries these sentences, so a page
compiled against the old one would say the old thing while native said the new
— the divergence this repo exists to refuse. The freshness guard in
tests/wasm_engine.rs compares the blob against every `.rs` under `src/`, so CI
catches a forgotten rebuild rather than shipping the split.

COST: compile_instructions 57,568,471 -> 57,571,389, a rise of 2,918 (0.005%),
and layout rather than work. `exports_play` cannot run on the measured path,
for a reason that is the match arm ORDER rather than a claim about libraries:
`("check", true)` is the FIRST arm of `compile_source`'s match, so `kanso check
lib/json` returns from it and never evaluates the guard the new function sits
in.

MEASURED ON TWO HOSTS, WHICH DISAGREE ON THE SIGN — the same diff against the
same base:

    the runner      57,568,471 -> 57,571,389    +2,918
    the container   58,162,797 -> 58,162,145      -652

A change that added work would add it on both. `__memcmp_avx2_movbe` moved 639
of the runner's 2,918, the same term that tracked this morning's layout moves.
Every earlier layout attribution in this vein argued unreachability from the
call graph and left the sign unexplained; this one has the sign measured twice
and coming out opposite, which is what the pending question about
instructions-only attributions was asking for.

## 2026-08-27 — the page answered `1 + d` with `d`

`rt_binop` read each side with

    let Slot::V(v) = slot(a) else { return a };

so any slot that was not a plain value ended the operation by handing its own
handle back. `1 + d`, with `d` a description, ANSWERED `d`, and the page
printed `<io>` where both native engines say ``+` is not defined for these
values`.

That shape is why nothing had caught it. The diagnostics differential compares
refusals and this program does not refuse; the error corpus pins stderr and
this program writes to stdout. Only the three-engine walk, which compares what
each engine PRINTS, could see it — the walk #1104 taught to say how much it had
walked.

The fix hands the interpreter a placeholder `Value::Desc` instead. Two
measurements stand behind that:

  - No operator succeeds on a description. Equality was the one that could
    have, since it reaches its own arm before the type table, and it names
    them: `equality is not defined on a function or an effect`. Ordering has a
    second sentence, the bitwise family a third, and everything else falls to
    the catch-all. Three new runtime fixtures, one per arm.
  - Every description renders `<io>`, so an error path that names the operand
    cannot tell the placeholder from the real one.

The sentence therefore still comes out of `eval_binop`, for every operator,
with nothing copied.

That dissolves the copies the entry above had to make. #1105 kept
`INDEXING_TAKES` and the `if` sentence as literals in wasm_rt.rs because
building a real description means `as_desc`, which forces a deferred right
side, and native does not evaluate one before refusing. A placeholder never
calls `as_desc`, so `rt_index`, `rt_at` and `rt_truthy` reach their own arms
now and both literals are gone. Two corrections to that entry while the ground
is fresh:

  - It said `scripts/diagnostic_coverage` watched the two copies for drift. It
    did not. The scan has six openers and a bare `die("` is not among them, so
    nothing anywhere was comparing them. (Task #102's branch adds that opener;
    it is still unpushed.)
  - It said `map_or_filter` succeeds on native, citing `list/map` over a list
    holding a description. `list/map` is the lazy library function and never
    enters that builtin — it answers `list/mapped <fn> list/cursor 1 <io>` —
    and bare `map` and `filter` are unnameable. So the reachability of
    `map_or_filter`'s two `val` sites is still open, and it is filed that way.

COST: the three compile veins are pinned to the runner's glibc and rustc and
refuse to compare in this container, so CI measures them. Locally welfare
reads 84.11 against a floor of 84.11 and the trend gate is silent.

## 2026-08-27 — three more of the same family, and the emitter lied about two

The entry above swept `val`'s call sites and listed eight that could receive a
description. Running a program at each one moved most of the list, in both
directions. Three were real.

**`err d`.** Native wraps the description, the err reaches the entry, and the
endpoint renders it: `unhandled err reached the entry: <io>`. The page died
with `a bound description cannot be used as data here`. A die where the other
two engines run to completion, which is the worst shape in this family.

The fix is `value_of`, NOT the placeholder the refusing sites use. `rt_mkerr`
CARRIES its reason onward, so a placeholder would be wrapped up and handed
back to the program as if it were the effect the program wrote. That
distinction now lives on `operand`'s doc comment, because the two fixes look
identical at the call site and one of them is wrong.

**`(opaque d).n`.** The page said `a bound description cannot be used as data
here` where native says `` `.` reads a field of a record, not <io> ``. The
first fix went to `rt_field_by_name`, which is what the emitter calls for
`Expr::Field`, and changed nothing: a field name some record declares compiles
to a GETTER, and a getter that matches nothing ends in `rt_no_field`. That
site had been written off as unreachable an hour earlier on the strength of
`(opaque d).nope`, which is a name error and never reaches the runtime — the
probe used a field no record declares, and that is a different program. The
`rt_field_by_name` change was reverted rather than kept: unproven either way.

**`set`.** The two `val` sites in `rt_setfield` cannot receive a description.
A `build` block writes only block-born constructions — `c.n = 1` on anything
else is refused at compile time with `c` is not a construction made in this
`build` block — so the target is a record by construction and the sentence
`` `set` writes a record field, not `` is unreachable from source. Left alone,
recorded as unreachable.

**A destructure.** `pt x y = opaque d` says `cannot destructure <io> as
`m/pt`` on both native engines; the page said `val`'s sentence, because
`rt_die_destructure` renders the value it is about to refuse and rendered it
through `val`. This one is a REFUSING site, so `operand` is right here where
`value_of` was right at `rt_mkerr`. Third of three.

**`map`/`filter`.** Still open. `list/map` is the lazy library function and
never enters `map_or_filter`; bare `map` and `filter` are unnameable. Nothing
found that reaches those two `val` sites yet.

The method note from the entry above needs one correction. It said reading the
emitter is not a substitute for running a program, which is right, and then
both of today's dismissals came from probes that ran the WRONG program. A
site is unreachable when a program written to reach it fails to; a program
written to reach something adjacent proves nothing.

COST: CI measures the compile veins. Locally welfare and the trend gate agree
with main.

## 2026-08-28 — the scan could not read the engine a reader meets first

`scripts/diagnostic_coverage` had five openers and none of them matched
`src/wasm_rt.rs`, which writes 36 `die(` sites spelling 24 sentences. That is
the engine the website's playground runs, so it is the first one most readers
meet, and no gate had ever read a word of it. Two more openers — ` die("` and
` die(format!(`, split apart for the same reason the oracle's two are, since
one carries its opening quote and the other may put the literal on the next
line — take the scan from 242 diagnostics to 262.

The gap this closes is not hypothetical. #1105 kept two sentences as literals
in wasm_rt.rs and said in a comment that the scan watched the copies for
drift; it did not, and could not have. Over the two days before this landed,
three of the page's sentences turned out to be saying something other than
what the other two engines say, and a fourth was a wrong ANSWER rather than a
wrong sentence.

Ten of the page's sentences are pinned by nothing. Nine excuses were
established by walking the emitter or by running a program:

  - the string-literal guarantee, one function (`str_lit`) covering four
    sentences at five sites
  - the operator table, an exact correspondence between `binop_code`'s
    fifteen codes and the fourteen names plus catch-all in wasm_rt.rs
  - the environment handle, all nine `RT_ENVGET` call sites passing the
    compiled function's second parameter
  - the filter predicate, third spelling of a dead arm
  - the map key, `require_literal_key` refusing anything else at compile time
  - `not a record`, four sites shadowed by two checks that each have a
    fixture in the wasm walk
  - `this value is not callable`, probed and answered by the sibling arm

The tenth is `val`'s own words, `a bound description cannot be used as data
here`, and it could not be written until this week's three fixes. It is the
sentence a site says INSTEAD of its own when it opens with `match val(h)` and
meets a description, and it was reachable eight ways: an operator, a
condition, four index forms, an err, two field reads, a destructure, a
dispatch. Each is now routed to the site that owns the sentence, and each
routing left a program in the corpus — the row lists all eight by path. Two
`val` sites could take a description on paper, `rt_field_by_name` and
`rt_not_own_err`, and the two programs written for them do not arrive: a field
read compiles to a getter and lands in `rt_no_field`, and a dispatch answers
its bare arm without asking the runtime. Both of those programs are in the
corpus too, as the row's evidence rather than as anyone's bug.

So the sentence is what the page says when an invariant it holds internally is
broken, and no program states it. Listed rather than deleted: the arm is what
makes `val` total, and a wrong answer is worse than a wrong sentence.

Two ratchet mutations, one per opener, each watched turning the gate red and
naming the sentence it injected.

COST: no compiler source changed; the veins cannot move.

## 2026-08-29 — the branch audit runs, and two thirds of the rule cannot

The ironclad branch directive of 2026-08-26 reached main only on 2026-08-28,
in #1112, having sat unmerged on the branch that carried it. Its third clause
— every check-in audits the branch list — ran for the first time today.

    324   branches on origin
      2   are ancestors of main
    321   carry commits main does not
      1   is perf-history, which CI maintains and nothing should delete

Ancestry is the wrong test and the 321 is not a work queue. A squash merge
leaves the branch's own commits outside main's history, so a fully landed
branch reads as unmerged by that measure. The twenty most recently touched
were checked by hand: every one is a squash-merged pull request from the two
preceding days whose content is on main. Nothing unfinished is hiding at the
recent end, which is the part of the answer that matters — the FIFO clause
has no outstanding work to enforce, and what remains is litter.

The directive counted roughly 250 three days ago. The class is growing at
about twenty-five a day, which is the merge rate.

The other two clauses cannot be executed from a cloud session, and this is
the finding rather than an excuse:

  - `git push origin --delete` and `git push origin :branch` both fail with
    "the remote end hung up unexpectedly". The container's git proxy refuses
    a delete refspec. Tried on two days against a branch whose pull request
    was merged; the same failure both times.
  - The GitHub MCP server offers `create_branch` and `delete_file` and no
    delete-branch tool, and there is no `gh` in the container.
  - `delete_branch_on_merge=true` needs the REST API or the settings page.

So the purge and the repository setting both need somebody with push-delete
rights. The setting is the half worth doing first: it stops the class
recurring, which turns the purge from a chore that returns into a job done
once.

## 2026-08-29 — the executor gains a door: rescue, annotate and bind on the oracle

DONE (oracle), OPEN (native, page). The three-forms gavel of 2026-08-26 and
the amendment that made `bind` a word are built on the interpreter. What the
build turned up is that one of the three is new capability rather than a
respelling, which the migration sizing had not seen.

**An execution-time failure could not be handled inside a chain at all.** A
chain step over an effect synthesises a one-parameter closure around the
step's expression (eval.rs, the `piped` branch). At execute time `Desc::Bind`
calls that closure with whatever the subject yielded, and `call_closure`
returns a failure argument instead of entering the body. So the failure never
reached the callback, whatever shape the callback had — a lambda and a group
with an `(err _)` arm were treated identically, because neither was ever
called.

`docs/book/samples/ch05/fallback.kso` is exactly that program, and its golden
is the endpoint message. ch05 teaches it as the design: an err born at the
edge "does not knock at the continuations waiting downstream, even ones with
an arm ready". So the migration's err-arm half really is empty, and for a
better reason than the three greps in #1115 gave — the chain err-arm was never
a working surface, and the one site in the fleet that looks like one is the
book's counter-example showing it does not fire.

Two consequences the sizing owes an update: `bind`'s skip-on-failure rule was
already what the machine did, so spelling it out changes no program; and
`rescue` needs the executor to hand the err to a callback, which nothing did.

**What landed.** `Desc::Rescue` and `Desc::Annotate` beside `Desc::Bind`, one
`worded_step` holding the whole difference between the three, and
`call_on_err` for the one thing ordinary application will not do — hand a
failure to a lambda. `annotate` carries the site it was written at, because
the err it builds is a raise and a raise records where it happened. Its
re-wrap is unconditional, so it cannot resurrect without anything checking
that it doesn't; a callback that answers with a failure of its own keeps that
one rather than being wrapped twice.

**The three words are reserved now.** `bind`, `rescue` and `annotate` joined
AMBIENT, so no program may define a function by those names. Two fixtures in
the corpus did — `tests/golden/advisory/group_identity` and
`.../reraises` — which is evidence the names are natural ones for user code.
They were renamed to `recover` and `relabel`. This is a cost of the gavel's
spelling and worth saying out loud rather than discovering later.

**Pinned.** `tests/golden/chainwords/` and `tests/the_three_chain_words.rs`:
four cases, each run on the oracle against a golden AND asserted refused by
name on native. Three mutations were watched turning the right case red —
rescue not reading the failure channel, annotate not re-wrapping, bind reading
the failure channel — and the corpus goes green again when each is restored.
The micro corpus could not host these: it runs native and `--interp` and
requires them to agree.

**Not built, and why the differential law allows it.** Native and the page
both refuse the words by name — `native backend: \`rescue\` is not yet
supported` and the wasm backend's `unsupported call to \`rescue\``. The law
permits a feature on fewer engines only when the others REJECT it, and both
do. Native's chain loop is the beat/evacuation loop with a cost golden
watching it, so a new dtag there is its own measured change rather than a
rider on this one.

**Two things the gavel's own sample needs that do not exist.** Its lambda
form is `annotate (e -> "config: {e.reason}")`, and an err has no `.reason`
reader — nor could an interpolation read one, since every operation on an err
propagates it. So a lambda callback can receive an err and cannot look at it.
A group callback can, by destructuring, which is the gavel's primary story
("a dispatch group is a legal callback"). Whether an err gains readers that
get past its own infectiousness, the way `wrap_err`'s second argument does, is
a question for Clay and is filed.

**The migration count is corrected.** 485 loose-dot steps in fleet code with
comments and strings stripped, not 309: 346 are `. (lambda)`, the monadic
steps that respell, and 139 are `. named_fn`, the threading form the
effect-first rider preserves. The 309 in STATUS counted differently.

Welfare holds at 84.11 against its floor. compile_instructions rose 2,918
(57,568,471 -> 57,571,389) for two Desc variants and a builtin branch.

## 2026-08-29 — native learns two of the three words, and annotate says why not

DONE (oracle and native for `bind` and `rescue`), OPEN (`annotate` on native,
all three on the page). Continues the entry above in the same PR.

Native needed no new struct field and no change to the beat loop's memory
discipline. `bind` and `rescue` are one dtag apart — 6 and a new 29 — and a
single `k_worded_step` holds the difference, mirroring the interpreter's
function of the same shape. `k_call_on_err` is the C twin of the Rust one:
`k_call1` returns a failure argument rather than entering a closure's body,
which is what threads failures through a chain for free, and rescue is the
one step that must get past it. The chain loop's flat pulse takes 29 beside 6
rather than falling through to `k_exec`, so a rescue in a long chain costs no
C stack.

`bind` needed its own entry point rather than reusing `k_maybe_bind`, and the
reason is the gavel: `k_maybe_bind` on a settled value calls the callback
through `k_call1`, which skips a closure on a failure but hands one to a
group, letting an err arm catch. The gavel says the failure channel is never
inferred from an arm's shape, so `k_b_bind` skips the callback whatever shape
it has.

**`annotate` is the one that does not fit, and the reason is structural.** It
builds an err of its own, and an err records where it was raised — so the
description has to carry the site. `KDesc` is `{dtag, x, y}` and both slots
are taken by the subject and the callback. Three ways out were considered:
grow `KDesc` to a fourth field (48 bytes from 40, on every description in
every program, for a field one dtag reads); carry `(callback, origin)` as a
two-element list (an allocation per annotate step, and the origin literal
holds an embedded NUL so it does not survive a KStr round-trip); or have
codegen synthesise the wrapping closure so `annotate e k` compiles to a
rescue whose callback wraps. The third costs nothing and is what should be
built, but it means emitting a synthetic lambda that mentions `k` twice
unless the callback is bound to a local first, and getting that wrong
re-evaluates a callback expression the interpreter evaluates once. It is a
separate change with its own fixture rather than a rider on this one.

So native refuses `annotate` by name and speaks the other two, and
`tests/the_three_chain_words.rs` now pins three cases on BOTH engines against
one golden and the fourth as a refusal that names the word.

## 2026-08-29 — annotate reaches native after all, and the page was diverging in silence

DONE (all three words on the oracle and native), OPEN (the page, which now
refuses them by name). Supersedes the entry above on two counts, both found by
building rather than reasoning.

**annotate needed no new field and no codegen desugar.** The entry above listed
three ways to carry the site a raise records, and all three were worse than the
one it missed: the runtime builds the wrapper closure itself. `k_closure`
already exists for exactly this shape, so `k_b_annotate` makes a one-parameter
closure over the callback and the site and hands it to rescue's node. The site
rides as an int payload — a static literal the collector never owns and never
traces, and a raw pointer in a payload is already how `k_fnref` carries what it
carries. `annotate` therefore has no dtag of its own.

The measurement that killed the alternatives first: `rescue e (__e -> rescue
__e k)` already works on both engines, which says `rescue` on a settled err
reaches a group — so the whole word lowers to rescue plus wrap_err, and the
only question left was where to put the wrapper. Building it beat arguing
about a fourth field.

**The page was compiling `rescue` and answering something else.** Not
refusing — compiling, and then propagating the failure the other two engines
catch. The wasm backend has no node for the words, and the names fell through
to its generic call path instead of its `unsupported call` arm. This is the
exact shape the differential law forbids, and it was invisible to everything
in the tree: the chainwords corpus needs a filesystem to make an effect fail,
and a page has none, so no program the page could run reached the words.

`tests/golden/micro/a_settled_failure_meets_the_three_words.kso` closes that.
It reaches `bind` and `rescue` through a subject that has already failed —
json's parse failure, foreign here, no filesystem — so all three engines run
it. It went red on the wasm harness on its first run, naming the divergence,
and the page now refuses by name with the gap written in
`tests/golden/wasm_gaps.txt`.

Two method notes worth keeping. A probe that called `wasm_backend::compile`
directly and asked whether it errored answered "accepted" for all three words
with byte-identical modules, which was the front end eliding the call rather
than the backend accepting it — an inconclusive instrument that read as a
clean result. The corpus was the instrument that could tell. And the reason
nothing caught this earlier is the fixture design: every case that makes an
effect fail needs the world to refuse something, and the engine that has no
world is the one that most needs a case.

Welfare holds at 84.11.

## 2026-08-29 — the page speaks the three words, and all three engines agree

DONE. The gap the entry above opened closes in the same PR, so nothing carries
a refusal forward.

The page needed the same three pieces the other two engines did and nothing
more. `Slot::Rescue` and `Slot::Annotate` beside `Slot::Bind`; one
`worded_step` over the word; `call_on_err` as the third twin of a function
that now exists once per engine, because `call_closure` there returns a
failure argument for the same reason its two siblings do. `Annotate` carries
the origin literal in the slot — the page's `Lit` table is where a literal
already lives, so there is no equivalent of the `KDesc` problem native had.

Three imports appended at the end of the backend's import list, which leaves
every existing index where it was; there is no separate host binding table to
keep in step, so the names resolve against wasm_rt's exports and nothing else
needed touching.

`as_desc` answers None for the two new slots. A worded step other than `bind`
has no shape in the interpreter's `Desc`, so a program that reaches the green
-thread scheduler through one is refused rather than quietly running a
different chain. That is a real remaining hole and it is narrow: a `rescue`
inside a `join` on the page. Nothing in the corpus reaches it, which is
precisely why it is written down here rather than assumed absent.

`tests/golden/wasm_gaps.txt` loses the entry it gained an hour ago. The
micro case now runs on all three engines and answers the same three lines.

## 2026-08-29 — ch05 teaches the door it used to say did not exist

DONE. The book's chapter on effects had a sample whose expected output is the
endpoint message, and a paragraph explaining that an err born at the edge "does
not knock at the continuations waiting downstream, even ones with an arm
ready". That was the design, and this PR changed it, so the chapter that taught
it moves in the same PR rather than after.

The old sample and paragraph both stay, because both are still true of a dot
chain: a step written with `.` carries a failure past. What follows them now is
`rescued.kso`, the same program with the same group, reached through `rescue`
instead — so the reader sees the arm that did not fire, then the one word that
makes it fire, and the difference is the step rather than the group. Then a
paragraph on the two siblings and on why `rescue` being the only door is the
point rather than a limitation.

The panel gate was watched going red on the new golden and green again, which
is how the panel is known to be executed rather than skipped, and its exit code
was checked to be load-bearing.

ch08's boundary-language chapter still owes the same move, and it is a bigger
one: it is queued P1 in the ledger and its whole shape assumes the failure
channel is inferred from an arm.

## 2026-08-29 — the join hole on the page, tried and left refused, with the mechanism

REVERTED, and the reason is worth more than the change would have been.

The entry above records that the page's `as_desc` answers None for a rescue or
annotate slot, so one inside a green-thread group is refused rather than
scheduled. Making it answer Some looked like three lines, and it compiled.

It is wrong, for a mechanism rather than a taste. The interpreter reaches a
page closure through `call_from_interp`, which pushes the arguments as slots
and goes through `call_closure` — where a failure argument comes straight back
instead of entering the body. That is the same rule all three engines have and
the same rule `call_on_err` exists to get past, and `call_from_interp` has no
err-passing twin. So a materialized rescue inside a group would run its
subject, skip its callback, and answer the failure the other two engines
catch: a silent divergence, in the same shape as the one this PR just closed
and in a place no fixture reaches.

The refusal stays. The comment at the site now says the mechanism rather than
"has no shape", so the next reader does not have to find it again. Closing the
hole properly means an err-passing path from the interpreter back into the
module, which is a change with its own fixture — and the fixture is awkward,
because reaching it needs a rescue over an EFFECT that fails on a page, and
the only effects that fail there are the playground's own refusals, whose
messages differ from native's by design and would need a gap entry.

## 2026-08-29 — what the three chain words cost, priced across seven veins

DONE. CI measured the tree the words landed in and four goldens moved. Two
improved, seven rows worsened, and each is named here because a number that
changes without a sentence is the thing to catch.

**Improved, banked.** `compile_instructions` 57,571,389 -> 57,568,840, a fall
of 2,549 (0.004%). `compile_allocs` 61,981 -> 61,974, a fall of 7. The measured
path is `kanso check`, which never emits, so codegen's new call site and the
wasm backend's do not run; what changed in front of it is three more entries in
BUILTINS and three in AMBIENT, both read rather than walked per node. A fall of
this size on this vein has been layout every previous time it was chased.

**`text` 652,512 -> 655,344**, a rise of 2,832 spread evenly: every benchmark
gains 336 or 384 bytes. The evenness is what says it is not any one
benchmark's code. src/runtime.c is embedded whole and gained five functions —
`k_call_on_err`, `k_worded_step`, `k_annotate_wrap` and three `k_b_` entry
points — and the three entry points are extern symbols the emitter can call,
so the linker keeps them whether a benchmark does or not.

**The work vein, six rows, one mechanism.** `work_deepbench` 807,094,318 ->
808,729,952 (+1,635,634, 0.20%) and `work_widebench` 85,209,624 -> 85,337,597
(+127,973, 0.15%) carry almost all of it. `work_pendbench` rises 1,344.
`work_jsonbench`, `work_encodebench` and `work_oneshot` rise by 8 or 9, which
is one call frame. basket and escapebench do not move.

It is one branch per chain step, and it is the gavel's rule rather than an
accident. The chain loop used to hand the yielded value straight to `k_call1`;
it now asks `k_worded_step`, which tests the value for failure before deciding
whether the callback runs. The test cannot be dropped: `k_call1` skips a
closure on a failure but hands one to a group, and the ruling says the failure
channel is never inferred from the callback's shape — so a `bind` whose
callback happens to be a group would otherwise let an err arm catch, which is
exactly the surface the gavel retired.

Marking the helper `static` so it could inline was tried and measured: local
deepbench and widebench counts are byte-identical with and without it, because
the optimizer already inlines within the single translation unit. So the rise
is the check itself, at roughly one instruction per chain step, and deepbench
is the benchmark that runs the most of them. The trade is stated rather than
argued away: 0.2% on the chain-heaviest benchmark buys a failure channel that
is spelled instead of deduced.

Welfare holds at 84.11.

## 2026-08-29 — the check the chain loop was already paying twice

DONE, and it turns the entry above's regression into a win. Welfare read the
regenerated goldens and fell a hair under its floor: the runtime rises
outweighed the two compile falls, and by the project's own weights that means
the change was worse. There is nothing to argue about a term that paid, so the
cost had to go somewhere.

It went. `k_call1` guards its argument against failure before entering a
closure's body, and the chain loop had just tested the same value for the same
thing — that test IS the difference between the worded steps. So every bind
step in the language was testing the yielded value twice, and had been long
before this branch: the loop used to call `k_call1` straight, which meant the
guard ran once, and adding `k_worded_step` made it two. Removing the redundant
one leaves exactly one test per step, where the old code had one and the
gavel's rule needs one.

Measured locally, same host, so the deltas are exact:

    deepbench  808,726,272 -> 806,982,268   -1,744,004
    widebench   85,337,184 ->  85,273,176      -64,008

Against the pre-branch baseline that puts deepbench about 108,000 BELOW where
it started, and widebench about 64,000 above rather than 128,000. The chain
loop is cheaper than it was before the three words existed.

`k_call_on_err` is renamed `k_call_decided`, because it now has two callers
with different reasons and one property: the caller has already decided about
the argument. `rescue` uses it to get past the guard on purpose; `bind` uses
it because the guard is redundant. One function, and the name says the
contract rather than one of the two uses.

CI's numbers for the four goldens are regenerated in the commit that carries
this; the veins are host-pinned and this container cannot measure them, so
they come from the cost-goldens job's own diff.

## 2026-08-29 — the page's own rationale entry was arguing against what shipped

DONE. §23 was titled "why there is no bind" and closed with the sentence the
design ledger cites as the assembled argument: "what remains open is not
whether these words stay off the surface — they do, in every variant under
consideration". Three of those words shipped this morning.

§28 flagged the contradiction when the gavel was ruled and unbuilt, and that
was the right shape then — the page and the compiler were allowed to disagree
on purpose while a ruling waited for its build. The build landed, so the
allowance expires and §23 has to tell the truth in the present tense.

What survives is most of it, which is why this is a rewrite of the conclusion
rather than a deletion. The three decisions stand: own failures only bubble,
handling at a boundary is dispatch, elaboration is signature-directed. The
second needed one adjustment rather than a reversal — `rescue` became a word
without becoming a keyword, and an arm is still what does the handling, so
what arrived is a way to say which channel you meant. The conclusion is
rewritten: the compiler writes most of the plumbing, three words are the
exception, and they are the failure channel.

The title goes with it. "who writes the plumbing" is the question the section
actually answers, and it stays true whichever way the open
dispatch-vs-elaborator question goes. The anchor stays `nobind`: an anchor is
not prose and changing it breaks links from outside.

Two ledger entries carried the same stale premise and are corrected in the
same commit. The book's boundary-language entry cited §23 for "why the
combinator words stay off the surface", and the assert-hako entry was gated on
the err spelling being ruled — that gate is lifted, ruled and built, so a
matcher surface can now be designed once rather than twice.

## 2026-08-29 — the ledger sweep, finished and counted

Correction to the entry above, which says two ledger entries carried the stale
premise. It is three. The chain-line entry was filed this morning saying the
three words "work prefix-style on the interpreter", which was true when it was
written and understated by the afternoon; it is corrected in the same commit
alongside the boundary-language and assert-hako entries.

The sweep is complete rather than opportunistic, and the method is worth
keeping: every entry mentioning bind, rescue, annotate, chain or combinator was
read against the gavel, not just the ones that looked likely. Three of the
fifteen open entries were stale. Two mentions in the Parked section were
checked and are not: "dot-absorbs-`>>` erases the visible then/bind split"
argues about the dot's visibility and holds either way the chain-line grammar
is ruled, and the serve campaign's "starts from the three combinators" is
forward-looking and only helped by their existing.

ch04's "nothing is asked of the signature" was checked rather than trusted,
because the ledger marks it DONE and a record that says done without being
done is the failure this project has caught before. It is really in the book,
at docs/book/ch04.html, and nothing in it is invalidated: it teaches call-site
short-circuiting, which is the rule `call_closure` and `k_call1` implement and
which the gavel did not touch.

## 2026-08-29 — the join hole closes, and the refusal I documented was not the one it gave

DONE. #1116 left a rescue or annotate inside a green-thread group refused on
the page, with the mechanism written at the site: the interpreter reaches a
page closure through `call_from_interp`, which goes through `call_closure`,
where a failure argument comes straight back instead of entering the body. So
a materialized rescue would run its subject, skip its callback, and answer the
failure the other two engines catch.

**The mechanism was right and the consequence I wrote was wrong.** I recorded
that the page refuses, which it did, and never checked what it says. It says
`error[runtime]: a bound description cannot be used as data here` — `val`'s
sentence, about using a description as data, when the fault is that this
engine could not schedule a worded step inside a group. A refusal that names
the wrong thing is the failure this project has chased through four other
PRs, and I had put one in on purpose without reading it.

**The fix was smaller than the sizing.** `ForeignCall` is one thread-local
hook with one implementor, so it takes a flag: `false` is the guarded call
ordinary application makes, `true` is the decided call the worded steps need.
`call_decided` gains a `Value::TableFn` arm that passes it. `as_desc`
materializes both slots. Forty lines, no new machinery.

**And it turned up a second bug, of the family the val sweep found.**
`rt_maybe_bind` had its own copy of the deferred-shape list written out inline
where everything else calls `descish`. So `rescue (...) k . print` on the page
handed `print` the description as data and the page printed `<io>` where the
other two printed the rescued value. The list is a predicate; a second copy of
a predicate is a place for it to go stale, and this one did the moment the
gavel added two shapes. It calls `descish` now.

Both were found by one fixture,
`tests/golden/micro/a_rescue_inside_a_joined_stage.kso`. It is filesystem-free
in its OUTPUT rather than in its effects: the arm discards the reason, so a
page saying it has no filesystem and native saying the file is absent both
read `rescued`, and all three engines hold one golden with no gap entry. That
shape is worth reusing — an engine that cannot do a thing can still be held to
an answer that does not depend on how it fails.

## 2026-08-29 — compile_instructions returns to a number it held this morning

DONE. The group fix moves the vein 57,569,314 -> 57,567,033, a fall of 2,281,
and the value is one it already had today. The chain-words branch read
57,567,033; a rename of `call_on_err` to `call_decided` moved it to
57,569,314; this commit changes eval.rs and wasm_rt.rs again and it comes
back.

Four moves in one day across three changes, and not one of them touches the
measured path. `kanso check lib/json` compiles a library and never emits, so a
rename in the Rust sources, a flag on a call hook, and a predicate call in
place of an inline match all sit outside what it runs.

The round trip is worth more than the number. Every previous layout
attribution in this file argued from the call graph — the code is not
reachable, therefore the move is layout — and the entries above admit the sign
went unexplained each time. A vein that returns to an earlier value under a
different diff is layout showing its hand rather than being argued for.

## 2026-08-29 — a builtin's count was checked by nobody at the front door

DONE. `length x x` inside a function nothing calls: the interpreter printed
the program's output and exited 0, the native backend refused the whole
program with `native backend: `length` takes 1 argument(s)` and no span, the
page compiled it and ran it, and `kanso check` said ok. A user function with
the same mistake is refused by `check_call_arities` at the site, with a span,
identically on every engine — `no 2-argument arm of `one` (arms take 1)`.
Builtins fell through that walk because nothing in the front end knew what
one takes.

`print (wrap_err 1)` was worse than a divergence: `emit_call_rest` indexed the
second argument of a one-argument call and the compiler panicked, exit 101
with a Rust backtrace, on a two-word program.

The counts now live in `check::BUILTIN_ARITY`, sixty-two of them, and
`builtin_arity` answers under either spelling — a bare ambient name, or the
`builtin_` form a std wrapper reaches a native by. `check_call_arities` reads
it for any head no declaration and no binding claims, so a wrong count is
refused where it is written, before any backend runs.

The counts were in codegen's `BUILTIN_CALLS` as a second field. That table
answers a different question — which builtins get a direct C call rather than
an inline expansion — and carrying the counts too is what let the backend
refuse a call the front door had waved through. It is a list of names now, and
both its readers, plus the page's builtin-wrapper site, take the count from
`check`. The page's site had a hardcoded `"print" => 1` beside its lookup
because `print` is not in `BUILTIN_CALLS`; with one table that special case
went.

Every runtime vein is byte-identical — decode, escape, oneshot, basket, wide,
pend, scan, encode, emitted — and the browser differential agrees on all 393
programs. The two compile veins refuse to compare on this host; CI measures
them.

**The page test was wrong twice before it was right, and the second way is
the one worth writing down.** The first form asked the playground prompt to
evaluate `length [1 2] [3]`. It passed on main, because the prompt reaches
the interpreter and the interpreter's runtime check fires there — the
assertion read the same sentence for a different reason. The second form
declared `fn sized xs / length xs xs` at the prompt, which the repl accepts
on both engines: a declaration at the prompt is not a program, and no
whole-program check runs on it. Only the third form — the compile door,
`kanso_compile_wasm`, on a library with an entry — could tell, and on main it
answered `Ran(0, "alive")`. Three instruments, two of which reported health
they could not see.

A corpus reads programs that RUN. The browser differential takes `examples`,
`tests/golden/runtime` and `tests/golden/micro`, so a program refused at
compile time is in none of them, and what the page says for a compile-time
refusal was unread by construction. That is a gap in the corpus, not in this
change, and one test does not close it.
