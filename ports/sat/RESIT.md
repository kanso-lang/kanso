# Re-sit: sat

## Result

check.sh: pass (54 unit tests, 59 cases on three engines, oracle agreed on
15 fixtures and 60 random formulas), engines agree: yes.
Lines changed by the migration: 146 (`diff -r -x resit ports-orig/sat
ports/sat | grep -c '^[<>]'`), in 12 files. No expected-output file changed.

Scratch programs for the re-test are in `resit/t/`, the size sweeps for F2
and F20 in `resit/f2/` and `resit/f20/` (built with `kanso build --release`
and timed), and the full check log in `resit/check.log`. The journal's own
bugs were rebuilt release from `bugs/` and timed the same way.

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906) | yes | 23 calls in 9 files |
| ``error[import]: `list` is not imported here — a module's files share their declarations, not their imports`` | yes | 5 files gained `import "std/list"`; 2 lost an `import "std/text"` that became unused (`error[unused]`, also clear) |
| ``error[exhaustive]: this can be a none and `list/concat` has no arm for it — resolve it here, or give `list/concat` a `none` arm`` (`text/slice` past the end answers `none`, #1890) | partly. It names the `none`, but every one of these ranges is already bounded, and no guard satisfies the check (see N1). I wrote a two-arm `ranged` helper (none → `[]`) in solver and dimacs, and used `list/to_list (list/drop args 1)` for the two argument tails in cli | 9 |
| ``error[formatting]: a line holds at most 80 characters`` after wrapping a slice in `ranged` | yes, though the hint about list literals did not apply (N4) | 2 |
| ``error[opacity]: `dimacs/formula` is foreign — only `dimacs` builds a `formula`; ask it for one through a pub function`` in a test (#1883) | yes. The test now destructures the parse result instead of building the expected record | 1 test |
| ``error[name]: unknown name `set_at` `` (and `vec_of`, `cleaned`, `budget`, `luby_of`, `neg`, `option`, `options`, `unusable`, `value_lines`) in test files: a test file sees only pub names (#1883) | no. "unknown name" does not say the name exists and is private (N3) | 11 functions and 2 types made `pub`, one new `pub fn parsed_options`; 3 test files rewritten in part |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a list has no indexed update | missing-feature | major | core-lib | still | `put [1 2 3] 2 9` → ``error[type]: `put` takes a map here, not a list`` (now refused at check rather than at run time) |
| F2 | native map writes cost O(size) when the key exists | performance | major | core-lib | still | `bugs/map_replace_linear` at 1,000 / 10,000 / 40,000 keys, release: 0.23 s / 2.31 s / 9.45 s; interpreter at 40,000: 0.27 s |
| F3 | a read before a write falls off the in-place path | performance | blocker | core-lib | fixed | `bugs/bound_read_defeats_inplace_put`, release: `named` 0.010 s (was 51 s), `inline` 0.010 s |
| F4 | the book's `pub play =` entry does not run | confusing-semantics | minor | tooling | still | the book's own `samples/ch04/missing.kso` under `kanso run` → "`f04c.kso` is a library — nothing to run. it exports `play`: import the module from an entry file…", while ch04 prints `kanso run missing.kso` above its output. A file of functions plus bare statements also answers "is a library" under `run` and works only under `kanso play`. The two messages now agree with each other |
| F5 | no float literal can say 1e100 | missing-feature | minor | language | fixed | `big = 1e100` prints `1.0e+100` (`resit/t/f05.kso`; ruling #1889) |
| F6 | a list read needs `i < 1 or length xs < i` beside it | confusing-semantics | major | language | still | `resit/t/f06.kso` with `return acc if length xs < i` → ``this can be a none and `twice` has no arm for it``; with the `i < 1 or` half (`f06b.kso`) it prints 12 |
| F7 | the none check sees calls but not names or operators | confusing-semantics | major | language | still | `neg trail[q]` refused (`f07a`); `p = trail[q]` then `neg p` accepted (`f07d`); `m[k] + 1` passes check and fails at run (`f07b`: ``error[runtime]: `+` is not defined for these values`` / `failed in bump at f07b.kso:2`); `0 < n` with `n` an err still lets the err reach the entry (`f07c`) |
| F8 | a keyed read below a guard does not bind | engine-bug | major | language | still | `bugs/keyed_read_after_guard` → ``error[name]: unknown name `a` `` on run and `--interp` |
| F9 | no list patterns in parameters | missing-feature | minor | language | still | `fn header_of [p cnf v c]` → `error[syntax]: expected a parameter pattern` (`f09`) |
| F10 | a long expression cannot wrap at an operator | aesthetics | minor | language | ruled-no | 2026-10-13 gavel "a bracketed list or map may span lines" (#1882): "Operator chains, `if` and guards are not covered, and they are still broken by naming their parts" |
| F11 | no record update | missing-feature | major | language | fixed | `merge t { "qhead":2 }` prints `f11/trail 2 5` (`f11`; ruling #1886) |
| F12 | a local in one file collides with a declaration in another | refactoring-hazard | major | language | still | `resit/t/f12`: `moved = x * 2` in b.kso beside `pub fn moved` in a.kso → ``error[name]: `moved` is already a declaration; rename the binding`` |
| F13 | imported names collide with your own types and functions | diagnostic | major | language | partly | a module's own `fn step` beside `import "std/list"` now calls the function (`resit/t/f13s` prints `4 3` on run, interp and release). A module's own `type tally` still fails: ``error[arity]: no 4-argument arm of `tally` (arms take 1)`` (`f13m`), with no mention of std/list. Under `kanso play`, native still turns `step 3` into `list/step 3` (bug below) |
| F14 | a pattern on a foreign record works bare and fails qualified | diagnostic | minor | language | still | `resit/t/f14`: `(d/formula clauses vars)` → ``error[opacity]: `d/formula` is foreign — its structure does not cross an import``; `(formula clauses vars)` prints `[[3]] 3` |
| F15 | no comparator sort | stdlib-gap | minor | core-lib | partly | `list/sort_by xs key` exists (#1914) and sorts by a key (`f15`). minisat's key is the pair `(size > 2, activity)`, and `list/sort [[2.5 1] [1.0 7]]` still stops: `comparison requires two values of one comparable type`, so a two-part key still needs the hand-written merge sort |
| F16 | no multi-line string or list literal | missing-feature | nit | language | fixed | `resit/t/f16.kso`: a `"""` block and a four-line list literal print as written (#1882) |
| F17 | list slicing and concatenation live in std/text | aesthetics | nit | core-lib | partly | `list/concat` moved to std/list (#1906); slicing a list is still `text/slice`, so solver files still import std/text for lists |
| F18 | a passed literal `none` is refused as a sentinel | confusing-semantics | nit | language | still | `begin [] none` → ``this can be a none and `begin` has no arm for it — … give `begin` a `none` arm`` (`f18b`) |
| F19 | a binding above a guard runs when the guard returns | confusing-semantics | major | language | still | `resit/t/f19.kso`: a 3-billion-step binding above `return 1 if n < 10`, native: 8.09 s; the same without the binding (`f19b`): 0.05 s. The 2026-10-13 gavel (#1928) makes an unread *argument* lazy, not a binding |
| F20 | a collection grown inside a record is quadratic natively | performance | blocker | core-lib | still | `resit/f20`, release, `boxed` at 4,000 / 8,000 / 16,000 rows: 0.005 s / 0.74 s / 4.26 s; `flat` at 8,000: 0.005 s. The knee moved past 4,000; the growth past it did not change |
| F21 | no assertion; a debugging err spreads to every caller | missing-feature | major | language | still | `resit/t/f21.kso`: one `return err …` in `set_at` → ``this can be an err and `scan` has no arm for it`` at the caller. No assert exists |
| F22 | runtime errors carry no location | diagnostic | major | language | fixed | `resit/t/f22.kso` → `error[runtime]: slice takes a list or string` / `failed in cut at f22.kso:4`, on native and interp (#1975, #1987) |
| F23 | `kanso test` resolves names differently from `kanso run` | engine-bug | major | tooling | fixed | `bugs/function_named_like_imported_type`: `kanso test grow` → `2 passed, 0 failed` (#1883: a test file is an outside consumer) |
| F24 | arm order across several parameters is hard to satisfy | diagnostic | minor | language | ruled-no | 2026-10-13 gavel "overload arms stay in specificity order" (#1888) |
| F25 | indexing and `put` cannot be extended to your own type | missing-feature | major | language | still | `fn put (vec items) i x` → ``error[name]: the name `put` is already taken`` (`f25`) |
| F26 | the native runtime spends most of a solve copying | performance | major | core-lib | partly | release vs interp today: pigeonhole 6 0.30 s vs 6.53 s (journal: 0.42 s vs 7.1 s); random 120/511 0.29 s vs 4.95 s (journal: 0.87 s vs 5.6 s); 10-queens 0.06 s vs 0.61 s. Faster, but the program still needs its hand-built vector; no profile was taken |
| F27 | readability parentheses are an error | aesthetics | nit | language | ruled-no | 2026-10-13 gavel "parentheses are required where `and` meets `or`, and refused wherever else they group nothing" (#1887) |
| F28 | a persistent tree loop falls off a cliff | performance | major | core-lib | fixed | `bugs/persistent_tree_cliff` at 100,000 steps, release: 0.45 s (was 39 s) |

Counts: fixed 7, partly 5, still 13, ruled-no 3, moot 0, worse 0.

## New friction

### N1: an in-range slice in pure code needs a helper to discard a `none` that cannot happen
- kind: confusing-semantics
- severity: major
- scope: core-lib
- where: solver/vec.kso `with_slot`, solver/analyze.kso `swap`, solver/reduce.kso `merge_by`, solver/watch.kso `rewatched`, dimacs/dimacs.kso
- wanted: `list/concat front (text/slice xs (i + 1) (length xs))` below `return push xs x if length xs < i`, which bounds the range.
- wrote:
  ```
  fn ranged none
    []

  fn ranged xs
    xs
  ```
  and `list/concat front (ranged (text/slice xs (i + 1) (length xs)))` at 7
  sites. No guard satisfies the check (`return [] if i < 1 or length xs < i`
  above the slice is still refused). `text/slice!` answers a box, which pure
  code cannot open, and `text/to_span` answers `none` too.
- why it matters: every slice in the solver had a range computed from the
  list's own length. The helper says nothing true about the program, and if
  a range ever were wrong, it would hand back `[]` silently, which is the
  silent data loss #1890 set out to stop.

### N2: the none check covers `list/concat` but not `push`
- kind: diagnostic
- severity: minor
- scope: language
- where: solver/analyze.kso `swap`, solver/watch.kso `rewatched`
- wanted: the check to refuse `push (text/slice c 1 (a - 1)) y` the way it refuses `list/concat out (text/slice …)`.
- wrote: nothing; it passed check. `resit/t/n2.kso`:
  `push (text/slice [1 2] 5 9) 3` passes check and stops at run time with
  `error[runtime]: push takes a list and a value`.
- why it matters: of four slices in `swap`, the checker flagged two and let
  two through, so the migration cannot rely on the checker to list every
  site that can now see a `none`.

### N3: a test file reading a private name is told the name is unknown
- kind: diagnostic
- severity: major
- scope: tooling
- where: solver/solver_test.kso, solver/vec_test.kso, cli/cli_test.kso
- wanted: a message saying `set_at` is private and a test sees only pub names.
- wrote: ``error[name]: unknown name `set_at` ``, 23 times. The fix was to
  make 11 helpers and 2 types pub and add `pub fn parsed_options`, widening
  the module's surface only so its tests could reach it, and to rewrite the
  tests that built `options` and `unusable` records into destructuring
  helpers. The rule itself is a ruling (#1883, 2026-10-12 "a test file is an
  outside consumer of its module"); the journal's "What worked well" had
  counted testing private helpers as a strength.
- why it matters: the solver's unit tests were mostly of private helpers
  (literal negation, the Luby sequence, the vector). Under the ruling they
  test pub surface that exists only for them.

### N4: the 80-column hint suggests spanning a list that is not the problem
- kind: diagnostic
- severity: nit
- scope: tooling
- where: solver/watch.kso:86, cli/cli_test.kso:30
- wanted: a hint that fits a long call chain.
- wrote: "a line holds at most 80 characters — this one has 87; a list or map
  that long spans lines, one element to a line", on a line whose only list
  literal was `[other l]`. The fix was a named binding.
- why it matters: small; the hint points at the one construct on the line
  that cannot be the cause.

## Bugs

- `bugs/play_local_step_becomes_list_step/step.kso` (new): under `kanso
  play`, a file that imports std/list and defines `fn step x` prints
  `list/step 3` natively and `4` on `--interp`. The engines disagree. Inside
  a module directory every engine prints 4.
- The journal's bugs, re-run: `keyed_read_after_guard` still fails (F8);
  `map_replace_linear` still linear (F2); `accumulator_in_record_quadratic`
  still quadratic past about 4,000 rows (F20);
  `bound_read_defeats_inplace_put` fixed (F3); `persistent_tree_cliff` fixed
  (F28); `function_named_like_imported_type` fixed (F23).

## Comfort

On language and core-lib, the original agent would start with several of its
complaints answered. It could write `1e100`, update a record with `merge`,
spread a usage string and a long list over lines, and read the file and line
of a runtime failure. The two worst performance cliffs, a map read before a
write and a long persistent-tree loop, are gone, so the first map-based
version of the solver would no longer be a thousand times slower natively.
But the shape of the work is the same. A list still has no indexed update,
so the hand-built vector stays. The none check still accepts a name and
refuses a call. A binding above a guard still runs. A keyed read below a
guard still does not bind. A record that carries a growing list is still
quadratic past a few thousand elements, and map overwrites still slow with
the map's size. The new `none` from `text/slice` added a helper to every
bounded slice. On tooling, tests must now go through the pub surface, which
cost this port a dozen extra pub names and a confusing "unknown name". The
journal's summary called the cost model "the hard part"; today it is less
hard, but the agent would still end up threading eight positional parameters
and routing every array access through `slot` and `set_at`.
