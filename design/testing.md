# Testing — the settled design

Ruled by Clay 2026-08-19 ("simpler, like go test or test::unit"),
shaped by the foreign-assert insight of 2026-08-17, drafted as the
first artifact of the refinement phase. The whole design is three
sentences: a test is a boolean constant. The runner is a foreign
party. A failure's reason, once a licensed party separates it from
its err, is ordinary data.

## The mechanism, unchanged

A test is a constant whose name begins `test_`, in a `_test.kso`
file, evaluated by `kanso test`. True passes. False fails. An err
propagates to the harness, which reports `FAILED (returned err …)` —
the harness is not the package, so its reading of the failure is the
licensed foreign rescue, needing no rule and no exemption.

    test_decode_int = decode "42" == 42

That is go test's shape with less ceremony than Go's, and it stays.
There is no assertion DSL: `==` is the assertion, the boolean is the
verdict, and the suite reads as a table of facts. Nothing below adds
a second way to write any test that can already be written.

## The testing hako

One small std hako, `testing`, owning the pieces a bare boolean
cannot say. It is an ordinary hako — every tested package's errs are
foreign to it, so its arms are licensed by the rules as they stand,
with no builtin holes and no file gating.

    pub fn failed? (err _)          -> true
    pub fn failed? _                -> false

    pub fn when_failed (err reason) k  -> k reason
    pub fn when_failed _ _             -> false

`failed?` answers whether a value is a failure. `when_failed` is the
piece the old design said was owed — asserting WHICH failure: it
rescues the err (licensed: the raiser is foreign to testing), hands
the caller the bare REASON RECORD, and answers whatever the
continuation answers; on a non-failure it answers false, so a test
that expected a failure and got a value fails honestly.

The continuation is where the tested package reads its own failure,
and it breaks no rule doing so: the rule bans arms MATCHING YOUR OWN
ERR, and the continuation never sees an err — it receives a reason
record that a licensed foreign party already separated from the
failure. Clay ruled the round-trip explicitly: "you ensured it would
bubble up to the caller, and it did. if the caller wants to pass it
back to you, so be it." 1b's per-field pub covers the field reads.

    test_error_position =
      when_failed (decode "[1, nope]") (r -> r.position == 5)

Dispatch on the reason's TYPE works the same way, because a reason
record is a plain value — arms on reason types are not arms on errs:

    fn defect_reason _:defect   -> true
    fn defect_reason _          -> false

    test_must_wraps_defect =
      when_failed (must (decode "nope")) defect_reason

## What this retires

- **The `failed?` builtin and its `_test.kso` file gate.** The hako's
  `failed?` is ordinary code with the same name and type; the builtin,
  its gating machinery, and its infectiousness hole all delete. The
  design sheds a special case rather than gaining one.
- **json_test's endangered assertions survive the projection
  migration.** `failure_position`/`failure_reason` (deleted by the
  1b migration) are replaced at the two call sites by `when_failed`
  reading `r.position` directly; `defect?`'s err arm becomes
  `defect_reason`'s type arm, and the advisory goes quiet without
  widening anything.

## What this defers

The 2026-07-28 far-queue sketch (one `describe`, one `context` level,
JustBeforeEach-style refinement) is SUPERSEDED for now by the
simpler ruling. Its one real content — shared setup without
repetition — is carried by ordinary bindings in the test file until
real suites demonstrate the need for more. If that day comes, the
two-deep constraint recorded there remains the right cage for it.

Effectful tests (a test that must run a plan) are out of scope for
this slice: tests are values, the wire belongs to programs. When io
testing is wanted it arrives as its own design against the boundary
language, not as a widening of `test_`.

## A collision the committee pass caught, and its resolution

The July record seeds every PUB dispatch group's receivable-err set
with its own hako ("anyone may hand a package its own failure back").
Under gavel 24's clause 1 — no arm may match an own-origin err — that
seeding would statically refuse EVERY pub bare-err arm, including
`when_failed`'s, and with it every generic foreign rescuer Clay
explicitly blessed. The seeding served the old return-channel rule;
it cannot survive the new one.

Resolution, derived from the ruling's own sentence ("your own
failures only bubble"): clause 1 is DISPATCH SEMANTICS, not only a
static check — **an arm cannot see an own-origin err**. At match
time an err whose origin hako equals the arm's hako simply does not
match; infectiousness then carries it onward, so it keeps bubbling,
which is the doctrine executing itself. The static refusal remains
for what the computed provenance set proves WITHOUT self-seeding
(arms naming own reason types, provably-own flows); the pub seed
retires. For `when_failed` this means: a testing-raised err reaching
it skips both arms and propagates, so the harness reports the failure
— exactly right. Veto window Clay's, as with every derivation. Filed
2026-08-25 in design/pending-gavels.md as **An arm cannot see an
own-origin err — semantics, or an advisory?**, since the window never
closed and the shipped behaviour is an advisory rather than either.

## Refinement-phase stitches, logged while drafting

- A lambda cannot carry arms, so type-dispatching a reason inside
  `when_failed` needs a named local group (`defect_reason`) — two
  lines of ceremony the arm-bundle syntax would erase if lambdas ever
  learn patterns. Noted, not proposed.
- `when_failed` answering false on success conflates "did not fail"
  with "failed the wrong way" in a suite's failure report. The
  harness prints the test's value either way, so the distinction is
  visible in the output, but a two-arm report would say it sooner.
