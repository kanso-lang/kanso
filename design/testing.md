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

The continuation is where the tested package reads its own failure.
It receives a reason record that a foreign party already separated
from the failure, and since the 2026-09-15 ruling made a bare err
data it could also match the err itself with an `(err …)` arm of its
own; the round-trip through `testing` is the shape the suites use. Clay ruled the round-trip explicitly: "you ensured it would
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
failures only bubble"): clause 1 was made DISPATCH SEMANTICS on
2026-08-24 — an arm could not see an own-origin err at match time,
and the static refusal covered what provenance proved without the
seed. RETIRED 2026-09-15 by the ruling "the box is explicit, an err
is a value": a bare err is data and an `(err …)` arm matches it
wherever it is written, so both the match-time skip and the static
refusal are gone. What stands is the 2026-08-29 gavel's foreign-only
rescue licence, asked at the WORD: a `.?` written in the package that
raised the failure hands it on without entering its callback. For
`when_failed` nothing changes — the tested package's failures are
foreign to `testing`, so its arms see them as they always did.

## Refinement-phase stitches, logged while drafting

- A lambda cannot carry arms, so type-dispatching a reason inside
  `when_failed` needs a named local group — two lines of ceremony the
  arm-bundle syntax would erase if lambdas ever learn patterns. Noted,
  not proposed. json's suite spells it `a_defect?`: the naming rule
  wants the question mark on anything answering only true or false, so
  a bare `defect_reason` is refused.
- `when_failed` answering false on success conflates "did not fail"
  with "failed the wrong way" in a suite's failure report. The
  harness prints the test's value either way, so the distinction is
  visible in the output, but a two-arm report would say it sooner.
