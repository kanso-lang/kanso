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

### The welfare model cannot see the yield hole, because the corpus was written around it

**Searched** design/compiler-log.md (the 2026-09-03 entry names this exact
fix and defers it), design/log/compiler-log-archive.md (2026-07-28 and
2026-07-29 on `desc_yield`'s missing arms and the corpus gap they left) and
every design/*.md. Nothing rules on what follows.

`desc_yield` answered a chain's yield from a table keyed on the head's bare
name. Eight std effect wrappers were absent from it and a loop past any of
them ran on the grow-only arena. The fix carries the yield per declaration in
the inference fixpoint. It works: the `os/read_file!` twin of
`tests/golden/read_beat/reading.kso` goes from `beat_iters=1` to 201.

It costs the front end **+0.2578%** — 42,239,175 to 42,348,055 retired, same
host, same binary layout question as ever, of which about 66,000 is
attributable work and the rest is layout (`demand::analyze` and the parser
move, and this change touches neither). Every runtime counter is
byte-identical. So welfare falls about **0.008**, which is eight times the
gate's tolerance.

**Why this is not the ordinary case the floor rule already answers.** The rule
says a fall means either the change goes or the weights are wrong. Here the
weights are fine and the corpus is blind. The one program in it that reads a
file at runtime is jsonbench, and `bench/make_jsonbench` wrote its main out
into a `fed` pair *specifically to route around this hole* — the reason is in
that file's own comment, with the numbers: 2 arena blocks against 248, 2 MB
peak against 260 MB. This branch deletes the workaround and writes the natural
`os/read_file! "bench/large.json" . go`, and **every counter is unchanged**.
That is the proof: the corpus measures the workaround, so the fix it buys back
is worth exactly zero to the model.

The correctness carve-out in `scripts/welfare/welfare.kso` does not cover
this. It is for the differential law — engines disagreeing — and all three
engines agreed before and after. This is a performance hole, which is
precisely what welfare is supposed to price.

**Recommendation: ship it and move the floor, recording this entry as the
reason.** The alternative readings, both worse: keep the hole (a natural
program pays 130x the peak, and the next wrapper anyone adds pays it too), or
add a benchmark whose only purpose is to make this change score — which is
writing the test after the answer.

What is genuinely open is whether "the corpus is blind here" is an admissible
reason to move a floor at all, given the rule was written to stop exactly that
sentence being used loosely. If it is not, the answer is a corpus change first
and this fix second, and that ordering is the ruling to make.

**ADDED 2026-09-04, and it may change what the fall means.** The compile row
was re-sat during this branch. `compile_instructions` is keyed per silicon and
welfare reads the FIRST row of `bench/compile_instructions_by_cpu.txt` as a
bare number. Measured, with nothing else changed:

    41930035   welfare 73.05
    41845704   welfare 73.06

**What moves between those two numbers is BINARY LAYOUT, not silicon.** The
first draft of this entry said it was a chip change, because the run that
refused sat on Zen 4 where the previous sitting was Zen 3. The next run
corrected it: Zen 3 counted 41,845,704 as well. Same chip, different binary —
sha 42283602b2c8 against sha 0e081d4c2c96 — and **-84,331 for a source change
the container measures at +34**. The two AMD models happen to agree to the
instruction on this binary, which is what hid it for one run.

So a relink bought 0.01 of welfare, the same size as the fall this entry is
about, for 34 instructions of real front-end work. The by-cpu file's header
already names layout as a term — a docs-only pull request once moved this row
5,081 — and this is sixteen times that, against a floor that ratchets.

The question, and it is not about this branch: **should a term whose movement
is dominated by binary layout set a ratcheted floor at all?** The per-chip key
was built to separate the chip from the change and it does that. It does not
separate the LINK from the change, and nothing in the tree does. Shapes I can
see, none of them chosen here: welfare reads each row against that row's own
first recorded value, so the term measures movement rather than magnitude; or
the compile term stays in the report and leaves the ratchet, which is close to
the exclusion argument ruled against on 2026-09-03 and therefore not free; or
the row is measured against a binary pinned some other way, which is a larger
change than either.

Nothing has been changed on any of it. The row was re-sat because CI refused
and the refusal named the line to paste; that is the documented path and it is
all that was done.

**ADDED 2026-09-04, second sitting on the same question — a third chip landed
on this binary and counted the same number.** CI refused again, this time on
Intel Emerald Rapids (0x6/0xcf), and it counted 41,845,704: what Zen 4 and Zen
3 both counted on sha 0e081d4c2c96. Three silicon keys, one binary, one value
to the instruction. The row is added, which is the documented path, and again
that is all that was done.

It bears on the shape above, so it is filed under the same heading. Walking
every recorded state of `bench/compile_instructions_by_cpu.txt`, one state has
two chips carrying different values — `f6e24e91`, the commit that introduced
the key. `compile_sample`'s binary sha landed in that same commit, so the
readings that argued for keying by silicon are the readings whose binaries
nobody wrote down, and the file's header describes their SOURCES as identical
rather than their binaries. The layout term measured above is -84,365 against
the roughly 5,124 that the key was built on.

**This does not say the key is wrong, and it must not be read as saying so.**
The row is no more a function of the binary than of the chip: Zen 4 read
41,844,180 on this same sha, so something moves the count within one chip and
one binary. Three agreeing chips say the chip term is small on this binary and
cannot speak for another. Removing a measured guard on that would be trading
evidence for an inference.

So the question the shapes above are already waiting on gains a second half:
if welfare is to stop reading a layout-dominated magnitude, is the per-chip key
still buying anything, or is the thing that actually wants keying the BINARY?
Answering the second without the first would be re-keying a term that may not
belong in the ratchet at all. Nothing here proposes either.

**ADDED 2026-09-04, and it corrects the note above while strengthening the one
before it.** The layout term is measured now. Seven binaries on one chip, same
procedure as the gate minus its container stop:

    sha           .text     .data  .bss    instructions
    9fcc6686dc47  2550854   2640   312     42,344,081   baseline
    82ec0846958a  2550854   2640   312     42,344,081   dead pub fn, linker dropped it
    5d50f9d9721d  2550854   2640   312     42,344,081   no_mangle fn, dropped too
    8663815286be  2550854   2640   1336    42,344,081   +1 KiB .bss
    09a6c2fab6b8  2550854   2640   312     42,344,093   +64 KiB .rodata
    7fc53be7987e  2550854   2640   4408    42,346,211   +4 KiB .bss
    3c1e1cff9e3b  2550854   2640   65848   42,346,211   +64 KiB .bss

**2,130 instructions bought by moving a static nobody executes.** That is the
half of this entry that was argued rather than shown, and it is shown now: the
objective ratchets a number that a link can move by the size of the changes the
vein exists to catch.

**AND IT WITHDRAWS THE SUGGESTION THAT THE KEY MIGHT BE UNNECESSARY.** That
rested on the header's claim that cargo does not build the same bytes twice. On
this host it does — two from-scratch builds of an unchanged tree into different
target directories both read sha `9fcc6686dc47`. Nothing under `docs/` or
`design/` reaches an `include_str!`, so the docs-only pull request that moved
this row 5,081 ran on an identical binary, and that 5,081 is the chip or the
mode flip below. The per-silicon key keeps the evidence it was built on, and the
second question in the note above is withdrawn rather than left standing.

**CORRECTED SAME DAY, and the correction strengthens the entry.** The `.text`
term was never tested above — every probe there left `.text` unmoved because the
linker dropped each dead function. Keeping one (reached through an environment
variable the gate never sets) gives ten binaries and six values:

    sha           .text     .data  .bss    instructions
    9fcc6686dc47  2550854   2640   312     42,344,081   baseline, three runs
    82ec0846958a  2550854   2640   312     42,344,081   dead pub fn, dropped
    5d50f9d9721d  2550854   2640   312     42,344,081   no_mangle fn, dropped
    8663815286be  2550854   2640   1336    42,344,081   +1 KiB .bss
    09a6c2fab6b8  2550854   2640   312     42,344,093   +64 KiB .rodata
    5e73453bcc7b  2550950   2640   312     42,343,660   200 unreached fns
    2152c689dc78  2566982   2640   312     42,345,628   400 unreached fns
    2a4e10fb2116  2550950   2640   312     42,345,904   100 unreached fns
    7fc53be7987e  2550854   2640   4408    42,346,211   +4 KiB .bss
    3c1e1cff9e3b  2550854   2640   65848   42,346,211   +64 KiB .bss

The baseline read 42,344,081 on three separate runs, so each value is a property
of its binary. Two of these binaries share `.text`, `.data` and `.bss` to the
byte and read 2,244 apart, which retracts "a relink alone is not the term" from
the note above; six values retract "bimodal" with it.

**The span is 2,551 instructions for source changes that do no work, and it goes
both ways** — `5e73453bcc7b` reads 421 BELOW the baseline for two hundred
functions no execution reaches. A ratchet reading this row would bank that as a
win and then refuse the next change that gave it back. That is the entry's
question with a number on it.

**THE MODE FLIP IS THE THING TO NOTICE.** Seven binaries gave two values, and
which one a binary lands on is decided by whether `.bss` crosses a boundary
between one page and four. It matches what this vein has been reporting from the
other end for a fortnight — two clusters 5,064 apart on one unchanged binary,
the 508 lattice, Zen 4's pair on one sha. A ratchet reading a bimodal quantity
banks the low mode and then refuses the high one, which is a gate that fires on
the linker's luck. Whether that is worth a shape of its own, or is the same
question as the first half, is part of what is being asked here. Nothing has
been changed on any of it.


## Open, not blocking


### The book teaches the boundary language (queued P1, Clay 2026-08-26)

**RE-PREMISED AGAIN 2026-08-29 by the effects-are-types gavel, which
supersedes the three-chain-words form.** The call-site story the book
owes is now: `<t>effect` as a first-class passable outcome type;
`bind`, `annotate`, `rescue` as ordinary effect-first functions and the
sole eliminators; no automatic bind — a box where the unwrapped type is
expected is refused, and propagation is bind's contract. Half one (ch04
"nothing is asked of the signature") DOES NOT survive as written: its
short-circuit-at-the-call story describes the retired railway and needs
rewriting on explicit elimination. Half two lands when the typed-effect
surface is implemented, present tense as always. compiler.html entry 23
owes a rewrite or retirement in the same campaign.


### An assert hako

**Cited: the licence half is ruled — archive 2026-08-17, assertions are
ordinary foreign rescue. What is open is the surface shape only.**

A real assertion library in the rspec direction Clay sketched —
`(expect 1) . to (equal x)` — as its own small surface design, never
improvised inside a test fix. Its arms are foreign to every tested hako,
so the err license needs nothing special. Queued 2026-08-17.

**RECOMMENDATION: build it as its own design pass. The gate is lifted.**
The matcher surface reads failures, so its shape depended on how a
failure is spelled — that is ruled (three-forms gavel, 2026-08-26) and
built on all three engines (kanso#1116), so designing it now cannot mean
designing it twice. `rescue` is the word a matcher's own failure door
would use.


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
