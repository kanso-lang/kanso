# Re-sit: markdown

## Result

check.sh: pass (39 unit tests; 661 of 661 fixtures agree on all three
engines), engines agree: yes.
Lines changed by the migration: 89 (`diff -r -x resit -x bugs
ports-orig/markdown ports/markdown | grep -c '^[<>]'`), in 10 files. No
expected-output file changed.

Scratch programs are in `resit/t/`; `resit/f12/` is the journal's fold
reproduction migrated to today's compiler; `resit/f23/` is the port with the
F23 workaround taken out; `resit/big.md` is the 1.5 MB input for F24; the
performance bugs were rebuilt release from `bugs/` and timed;
`resit/check.log` is the check run.

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c` `` (#1887) | yes; the example shows the spelling, and `and` binding tighter means the fix never changes meaning | 5 |
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906) | yes | 2 |
| ``error[exhaustive]: this can be a none and `atx_content` has no arm for it`` and the same for `text/join`, `flatten`, `emphasize`, `shrink`, `list/concat`, `trim_blank_tail` (`text/slice` past the end answers `none`, #1890) | partly. Every flagged range is bounded by the list's own length; I wrapped 12 slices in a two-arm `ranged` helper (see sat N1), and re-split 4 lines for 80 columns (one became a call to the existing `rest_from`) | 12 |
| ``error[unused]: unused binding `hr` ``, ``error[dispatch]: overlapping overloads of `name_of` are illegal`` and ``overloads of `name_of` appear most-specific first`` in blocks_test.kso. A test file sees only pub names (#1883), so the private markers `hr`, `indented`, `para`, `quote` in `fn name_of hr` became binders | no. The messages are about binders and arm order; none says the type is private | 6 types made pub |
| ``error[name]: unknown name `atx_content` `` and 8 other functions; ``unknown name `ref_def` `` for a record the test builds (#1883) | no for the functions (see sat N3); for the record, a constructor `pub fn new_ref_def` | 9 functions made pub, 1 constructor |
| `test_blocks_in_order ... FAILED (returned false)` and two more: `fn name_of (fenced _ _ _ _)` and `(heading level)` in the test name types the test cannot see, so they never match, and `check` accepted them (raytracer F15) | no | 2 types made pub |
| ``error[formatting]: canonical order places type declarations before functions`` after placing `new_ref_def` | yes | 1 |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | the book's `pub play =` entry does not run | tooling | minor | tooling | still | the book's `samples/ch04/missing.kso` under `kanso run` → "is a library — nothing to run. it exports `play`…" (sat `resit/t/f04c.kso`) |
| F2 | no collection literal may span lines | missing-feature | major | language | fixed | `resit/t/f02.kso`: a three-entry map over five lines prints `& 3` (#1882) |
| F3 | string literals have no `\u` escape | missing-feature | minor | language | still | `"\u{2061}"` → ``error[syntax]: unknown escape `\u` `` (`f03.kso`) |
| F4 | a `_` arm does not cover none | confusing-semantics | minor | language | still | `skip_one cs[2] 5` with arms `" "` and `_` → ``this can be a none and `skip_one` has no arm for it`` (`f04.kso`) |
| F5 | an imported module's type names override my own | refactoring-hazard | major | language | fixed | `resit/t/f05`: `type skipped` beside `import "std/list"` builds the module's own record, `m/skipped 3` (2026-10-12 "a name the author declared wins over a builtin and an import"). A type named like a std/list *function* still loses (sat F13) |
| F6 | one namespace per module means a new type breaks other files' locals | refactoring-hazard | major | language | still | `resit/t/f06`: `type found` in a.kso, `found = length xs` in b.kso → ``error[name]: `found` is already a declaration; rename the binding``, still without naming the declaration |
| F7 | an err arm for an impossible argument spreads through every caller | confusing-semantics | major | language | partly | the wanted `length (text/split s piece) > 1` with a computed separator now checks and prints `yes` (`f07.kso`). Handing the split to a function of one's own still needs an err arm: `several? (text/split s piece)` → ``this can be an err and `several?` has no arm for it`` (`f07b.kso`), now reported at that call rather than far away |
| F8 | a `return … if x == none` guard does not narrow x | missing-feature | major | language | fixed | `resit/t/f08.kso`, the journal's `entity_here` shape, prints 3 |
| F9 | every function handed an index result needs its own none arm | aesthetics | minor | language | still | as F4 |
| F10 | no spelling of a list of held functions passes check in a module | engine-bug | minor | language | still | `kanso check bugs/partial_in_list` → `canonical form requires exactly one space here` at `[&a &b]` and at `. push &a` |
| F11 | `kanso check` on a module passes code that fails when imported | tooling | major | tooling | partly | diagnostics now carry the excerpt and caret, and each is printed once (`kanso check resit/t/f11/l` and `resit/t/f11/main.kso` each report one error, the library's own none at `l.kso:8`). Checking a library alone and confirming at the importer's call is the design (2026-10-12 gavel "arms travel with the value…": "a library is checked alone… and the program that passes the value confirms an arm exists or is refused at its own call site") |
| F12 | natively, list/fold corrupts the accumulator | engine-bug | blocker | core-lib | fixed | `resit/f12` (the journal's reproduction, migrated): interp, `kanso run` and release all print `1 1 5 6 7 7` |
| F13 | 80 columns with no way to continue a string or a boolean | aesthetics | minor | language | partly | a `"""` block holds the tag names (`f13.kso`). Long conditions are still broken by naming parts, as ruled (#1882: "Operator chains, `if` and guards are not covered") |
| F14 | clarifying parentheses are refused | aesthetics | nit | language | fixed | the wanted `… or (c == "/" and cs[e + 1] == ">")` is now the required spelling (#1887); the migration wrote exactly it |
| F15 | arm ordering across several parameters is hard to predict | confusing-semantics | minor | language | partly | the tie between `pair_up xs _ i none _` and `pair_up xs j i (delim a) (delim b)` is gone (`f15.kso` prints `none 1 3`). The ordering rule stands as ruled (#1888) |
| F16 | positional construction in alphabetical order swaps fields silently | refactoring-hazard | major | language | ruled-no | the `merge` gavel (#1886): "Construction by field name stays declined (2026-07-29, named arguments)" |
| F17 | no way to print from pure code while debugging | tooling | major | language | still | appendix B lists no trace or debug function; no ruling in the log |
| F18 | no case conversion or case folding in std/text | stdlib-gap | minor | core-lib | partly | `text/lowercase` and `text/uppercase` exist (#1914) and change a to z only: `f18.kso` prints `div ẞ STRAßE`. No case fold |
| F19 | no Unicode character categories | stdlib-gap | minor | core-lib | still | std/text exports nothing of the kind (`grep '^pub' lib/text/*.kso`) |
| F20 | no record update | missing-feature | minor | language | fixed | `merge` (#1886); sat `resit/t/f11.kso` |
| F21 | a list grown through any function call is copied natively | performance | major | core-lib | fixed | `bugs/push_through_call`, release, n = 4,000 / 8,000 / 16,000: direct, helper and nested all 0.005 to 0.007 s (journal: helper 12.8 s and 368 s, nested 64.7 s) |
| F22 | a list held in a record is copied on every push | performance | major | core-lib | partly | `bugs/record_field_push`, release, n = 4,000 / 8,000 / 16,000 / 32,000 / 64,000: 0.005 / 0.11 / 0.58 / 2.9 / 8.8 s. 64,000 now finishes (journal: killed, out of memory), but time still grows about fourfold per doubling past 8,000; interp at 16,000: 1.1 s |
| F23 | a call inside one arm of a dispatch crashes the native build | engine-bug | blocker | language | fixed | `resit/f23` restores the wanted `walked c 0 true (close_top st st.line)`: `a\n```\n```\n` renders correctly natively, and all 661 fixtures match on dev and release. The journal could no longer reproduce it either, so this is weak evidence |
| F24 | the native build is slower than the interpreter and uses far more memory | performance | major | core-lib | partly | 1.5 MB (spec examples x100): release 1.21 s, peak 756 MB; interp 22.8 s (journal: 4.7 s and 1,059 MB release, 25 s interp). Release is now the fast engine; its memory is still several times the interpreter's 84 MB |
| F25 | a one-expression test past 80 columns must bind a name | aesthetics | nit | language | still | raytracer `resit/t/f19m`: ``a single-expression constant is written inline`` |
| F26 | small things | diagnostic | nit | tooling | still | `fn code_text []` → `expected a parameter pattern` (`f26.kso`); `entries`/`values` still reserved as locals (raytracer `f27.kso`); a failing test still prints only `FAILED (returned false)`; `kanso build` still writes into the current directory with no output flag |

Counts: fixed 8, partly 7, still 10, ruled-no 1, moot 0, worse 0.

## New friction

### N1: a private type named in a test's pattern turns into a binder or never matches
- kind: confusing-semantics
- severity: major
- scope: language
- where: commonmark/blocks_test.kso `name_of`
- wanted: the journal's `fn name_of hr`, `fn name_of (fenced _ _ _ _)` and so on, testing the parser's block kinds.
- wrote: `pub` on `hr`, `indented`, `para`, `quote`, `html_block`, `listing`, `fenced` and `heading`.
- why it matters: under the 2026-10-12 rule a test file cannot see private
  types. A bare marker in an arm (`hr`) then reads as a binder, and the
  errors say "unused binding" and "overlapping overloads". A record pattern
  (`(fenced _ _ _ _)`) names a type that does not exist from the test's
  side, `check` accepts it (raytracer F15), and the tests fail with
  "returned false". Neither message says the type is private.

### N2: names carry an err into the check, but not a none
- kind: confusing-semantics
- severity: major
- scope: language
- where: commonmark/inline.kso `pair_up` (`after = text/slice …` then `list/concat right after`)
- wanted: the check to treat `after = text/slice xs (i + 1) (length xs)` followed by `list/concat [0] after` the way it treats the same slice written inline, which it refuses.
- wrote: nothing; it passed. `resit/t/n4.kso` checks clean and stops on
  every engine with `error[runtime]: concat takes two lists` / `failed in
  tail_of at n4.kso:6`.
- why it matters: 2026-10-12 (#1884) closed this gap for err. For none it is
  open, so in this port four slices bound to names were not flagged by the
  migration while their inline neighbours were.

### N3: the tests' reach into the module had to become pub surface
- kind: refactoring-hazard
- severity: minor
- scope: tooling
- where: commonmark/*_test.kso
- wanted: tests of `atx_content`, `split_lines`, `thematic?`, `caret_of`, `normalize_label` and other internals.
- wrote: `pub` on 9 functions and 8 types, and `pub fn new_ref_def` so a test
  can build a reference table. Ruled (#1883).
- why it matters: the module's pub surface now lists its parser internals;
  nothing marks which of them exist only for the tests.

## Bugs

No new reductions. The journal's bugs, re-run:
`fold_corrupts_state` fixed (F12); `push_through_call` fixed (F21);
`record_field_push` finishes now but is still superlinear (F22);
`partial_in_list` still refused (F10).

## Comfort

On language and core-lib, an agent starting this port today would not meet
the two things that cost the original the most. `list/fold` gives the same
answer natively as on the interpreter, and a push behind a helper or onto
another push no longer copies, so the hand-written loops and the moved
pushes would not be needed, and the release build is now the fast engine
(1.2 s against 22.8 s on 1.5 MB). The entity table could be a map literal
over many lines, a `== none` guard now narrows, an own type named like a
std/list type wins, and `merge` replaces the nine frame restatements. The
none rules are otherwise as they were: a wildcard does not take none, a
scanner still needs a none arm on every function an index reaches, and the
new `none` from `text/slice` added a helper to every bounded slice. The
module namespace still makes every type a reserved local in every file, and
there is still no way to print from pure code. On tooling, the tests had to
move onto pub names, and private markers in test patterns failed in ways
whose messages did not say why. The journal said the native faults took
half the session; today that half would mostly go to the checker's none
rules instead.
