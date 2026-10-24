# Re-sit: mal

## Result

check.sh: pass. All 34 fixtures agree on interp, dev and release, and
`kanso test lisp` passes 22 of 22. Log: `resit/check.log`.

Engines agree: yes, on every fixture. One reduction outside the port's code
makes them disagree (Bugs, B1).

Lines changed by the migration: 127 (`diff -r` against ports-orig/mal,
counting `<` and `>` lines, `resit/`, `bugs/` and `build/` left out).

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation`` (#1906) | yes | 13 (core.kso 12, repl.kso 1) |
| ``error[exhaustive]: this can be a none and `list/fold` has no arm for it — resolve it here, or give `list/fold` a `none` arm`` (text/slice past the end answers none, #1890) | partly. It points at the caller of `tail`, not at the `text/slice` inside `tail` that can answer none. Every site was a tail-of-list or rest-of-argv slice that cannot actually miss. One helper, `sliced none empty`, fixed all of them | 9 slices (tail, bind_rest, apply, swap!, reader piece, run_file, input_lines, decode_key, cases_of) |
| `error[formatting]: a line holds at most 80 characters` after wrapping a slice in the helper | yes | 2 |
| ``error[exhaustive]: this can be an err and `ev` has no arm for it`` (a name bound to a raising call now carries its err, #1884) | yes | 1 (`run_source`: dispatch `read_str`'s err before `ev`) |
| ``error[exhaustive]: this can be a none and `lisp/agrees?` has no arm for it`` with no source line under it (N3) | partly. The line number led to `c.expected`, and the none was born in `cases_of` | 1 |
| ``error[name]: unknown name `boot` `` in lisp_test.kso (a test file is an outside consumer, gavel 2026-10-12 on #1883) | no. It says "unknown name", not "a test sees only pub names". The names were reported in three rounds: constants first, then names in helper functions, then a run-time failure (B1) | 12 names made `pub` across 6 files (`boot`, `new_atom`, `set_atom`, `deref_atom`, `encode_key`, `decode_key`, `read_str`, `pr_str`, `run_source`, `error_text`, types `finished` and `threw`) |
| ``error[opacity]: `lisp/kw` is foreign — only `lisp` builds a `kw` `` | yes | 2 (the tests now build a keyword with `decode_key "ka"`) |
| ``error[naming]: `round_trip` answers only true or false: name it `round_trip?` `` | yes | 3 |
| ``error[runtime]: no overload of `outcome_text` matches these arguments`` under `kanso test` | no. The arm named the private type `finished`, which the test file cannot see, and nothing refused it (B1) | 1 (made `finished` and `threw` pub) |

No expected-output file changed.

## Journal

Scope tags: language, core-lib, domain-lib, tooling.

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a single file with declarations cannot be run, and the book says it can | tooling | minor | tooling | still | appendix B's own sample `samples/appb/input.kso`, copied to `resit/f1_input.kso` and run as the book says: ``error: `f1_input.kso` is a library — nothing to run. it exports `play` ``. `kanso play` refuses the `pub play` form too. One part improved: the bare-statement form's refusal now ends "run its definitions beside their statements with `kanso play`", and `kanso play resit/f1_bare.kso` prints `3 items` |
| F2 | natively, a map `put` copies the whole map unless provably unique | performance | major | core-lib | fixed | `resit/f2_put.kso` (the journal's bug at n = 160000): 0.06 s native, where 16000 took 33 to 50 s. `resit/f2_put_record.kso` (map in a record field, through a helper, n = 160000): 0.20 s native |
| F3 | inlining a variable can make a program illegal, and naming it again makes it legal | confusing-semantics | major | language | partly | the two spellings now agree (#1884, "a name bound to a raising call carries its err into the check"): `resit/f3_inline.kso` and `resit/f3_named.kso` both give ``this can be an err and `wrap` has no arm for it``. The wanted code, an err riding through `wrap` with no arm, is still refused. That is the 2026-09-15 ruling "a bare err halts where it lands", so this half is ruled |
| F4 | no way to write a long string or a long list across lines | missing-feature | minor | language | fixed | `resit/f4_long.kso`: a spanning list literal (gavel #1882) and a `"""` text block holding a three-line mal macro both run and agree on interp and native |
| F5 | one namespace for a module, its imports and the ambient builtins | refactoring-hazard | major | language | partly | `resit/f5`. Fixed: a local type `hit` beside `import "std/regexp"` (log 2026-10-18, "A module's own type beats an imported type's twin"), and a test helper no longer joins the module (#1883). Still: a local type `tally` beside `import "std/list"` gives ``no 2-argument arm of `tally` (arms take 1)`` (B2). Still: a lambda parameter `more` in b.kso beside `fn more` in a.kso gives ``error[name]: `more` is already a declaration``. Still: a field pattern `(holder entries)` gives ``error[name]: `entries` is already a declaration`` |
| F6 | a parameter named after a marker is silently a literal pattern | confusing-semantics | major | language | still | `kanso play bugs/marker_name_as_pattern_binder.kso`: ``error[runtime]: no overload of `total` matches these arguments`` on both engines. It now names the line (`failed in the entry at ...:17`), but nothing refuses the arm before the run |
| F7 | every list index is a none the checker wants handled | aesthetics | minor | language | ruled-no | `resit/f7_index.kso`: `shown items[i]` after a bound check still gives ``this can be a none and `shown` has no arm for it``. The pure strict index the entry asked for was declined by "gavel, reversed the same day: `!` answers a box, at the index and at the name" (2026-09-16) |
| F8 | native memory grows with every step of the interpreter loop | performance | major | language | fixed | port built with `--release --counters`. `(sum2 200000 0)`: `arena_peak_bytes=1048576 beat_iters=10800501`. `(fib 25)`: `arena_peak_bytes=1048576 held_peak_bytes=2192`, where the journal had 796,917,760 |
| F9 | no reference cells, so environments and atoms are rebuilt by hand | missing-feature | major | language | still | no cell type in appendix B or the library, and no ruling on one in the log. `list/tie` builds a cycle once and cannot mutate it. The port still threads a `world` record through every step |
| F10 | adding a field to a record means editing every construction | refactoring-hazard | minor | language | fixed | `resit/f10_merge.kso`: `merge w { "out":[] }` prints `[] 1 2` on both engines (gavel #1886) |
| F11 | arm order across several parameters is hard to predict | diagnostic | minor | language | still | `resit/f11_order.kso` (the journal's wanted `pr_str` group): ``overloads of `pr_str` appear most-specific first: literal, then concrete type, then generic`` at two arms, still without naming the arm each must move above. The rule itself stays (#1888, "overload arms stay in specificity order") |
| F12 | io/stdin is all of stdin, so the REPL is not interactive | stdlib-gap | major | core-lib (basic io) | fixed | `resit/f12`: a prompt loop on `io/read_line` (#1914) answers `(+ 1 2)` at second 69 and `(+ 3 4)` at second 71, two seconds apart, natively and on the interpreter. The port itself still reads `io/stdin`; moving it was not needed to pass |
| F13 | the runtime's "no overload matches" names no argument | diagnostic | minor | language | partly | `resit/f13`: ``error[runtime]: no overload of `m/ret` matches these arguments`` / `failed in m/start at f13/m/m.kso:11`. The calling line is named now (#1975, #1987); the argument's shape still is not |
| F14 | "this can be an err" with no path back to the err | diagnostic | minor | language | still | `resit/f14_trail.kso`: the err is born in `deep` two calls down, and the refusal names only ``settle`` at the outer call. The `e:err` versus `(err r)` surprise does not reproduce in `resit/f14_spell.kso` (prints `true`), and the journal could not reduce it either |
| F15 | a long test constant has to grow a throwaway binding | aesthetics | nit | language | ruled-no | `resit/f15`: the two-line form gives `a single-expression constant is written inline`. The #1882 gavel says operator chains "are still broken by naming their parts" |
| F16 | braces in a mal string are interpolation in a kanso string | diagnostic | nit | language | still | `resit/f16_brace.kso`: ``error[syntax]: unexpected character `\` `` with the caret on the escaped quote after the brace |
| F17 | the binary is named after the directory, with no way to rename it | tooling | nit | tooling | still | a project `mal` with a module `mal`: ``error: this build is named `mal`, and a directory of that name is here — build it from inside``. The message is clearer; there is still no `-o` |
| F18 | `== false` is refused, and mal's false? needs exactly that | aesthetics | nit | language | still | `resit/f18_false.kso`: ``comparing to `false` asks a question the value already answers — write `not` the value``. `resit/f18_not.kso`: ``an if condition is true or false, got f18_not/nil`` for `not nil`, a program with no `if` |

Counts: fixed 5, partly 3, still 8, ruled-no 2, moot 0, worse 0.

## New friction

### N1: a tail slice that cannot miss still answers none to the checker
- kind: aesthetics
- severity: minor
- scope: core-lib
- where: lisp/eval.kso `tail`, and eight other slices
- wanted: `text/slice xs 2 (length xs)` as the rest of a list
- wrote:
  ```
  fn tail xs
    sliced (text/slice xs 2 (length xs)) []

  fn sliced none empty
    empty

  fn sliced part _
    part
  ```
- why it matters: a range that ends at `length xs` and starts at 2 is either
  the tail or, for an empty list, the inverted range that appendix B says is
  the empty result. It can never be none (`resit/slice_edge.kso` prints `[]`
  for `text/slice [1 2 3] 4 3`). The checker reports the none at every caller
  of `tail`, not at the slice. `text/slice!` answers a box, and a span is
  built for strings, so the port needs a helper with a none arm. A list's
  slice is also still spelled `text/slice`, while #1906 moved concat to
  `list/` because "a function lives with the type it works on".

### N2: a test sees only pub names, so the unit tests widened the module's surface
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: lisp/lisp_test.kso and the six files it tests
- wanted: unit tests of the reader, printer and atom store, as written on
  2026-10-10
- wrote: twelve declarations made `pub` (`boot`, `new_atom`, `set_atom`,
  `deref_atom`, `encode_key`, `decode_key`, `read_str`, `pr_str`,
  `run_source`, `error_text`, `finished`, `threw`), and `kw "a"` replaced by
  `decode_key "ka"` because a test cannot build a private type
- why it matters: the ruling (#1883) is deliberate, and this is its cost. The
  module's only real export is `main`, so everything a unit test touches had
  to become public. The refusals came in three rounds: unknown names in test
  constants, then unknown names in the test's helper functions, then a
  run-time "no overload" for a pattern on a private type (B1). The first
  message says "unknown name" and does not mention that a test sees only pub
  names.

### N3: a none found through the entry is reported without its source line, twice
- kind: diagnostic
- severity: minor
- scope: language
- where: lisp/repl.kso:271 (`agrees? c.expected`), reduced in `resit/n5`
- wanted: the usual excerpt and caret, at the place the none is born
- wrote: nothing; `kanso check resit/n5` prints
  ```
  error[exhaustive]: this can be a none and `m/agrees?` has no arm for it — resolve it here, or give `m/agrees?` a `none` arm (in m.kso)
    --> n5/m/m.kso:9:14
  ```
  twice, with no source line under either copy. `kanso check resit/n5/m`
  says `ok`.
- why it matters: the none was stored into a record field by a `text/slice`
  one line earlier (in the port, in a different function), and the report
  names the field read. With no excerpt and the qualified name of the
  module's own function, the message reads like something from another
  module.

### N4: the map literal's spacing rule does not say what it wants
- kind: diagnostic
- severity: nit
- scope: language
- where: `resit/f10_merge.kso`
- wanted: `merge w {"out": []}`, the spelling in the #1886 gavel's text
- wrote: `merge w { "out":[] }`, found in appendix B
- why it matters: the refusal is `canonical form requires exactly one space
  here` at two carets, without the canonical spelling. The gavel's own
  example is in the refused form.

## Bugs

- **B1** `bugs/pattern_names_unseen_type/`: a pattern naming a private type
  of another module checks clean. The interpreter prints `got nothing` (the
  arm never matches); native stops with `error: native backend: unknown type
  `held``, an internal message with no file or line. Expected: `error[name]`
  at the pattern on every engine, as a call to the same name gets. In a test
  file the same shape is a run-time "no overload matches".
- **B2** `bugs/type_named_like_imported_function/`: a module's own two-field
  type `tally` beside `import "std/list"` (which exports a one-argument
  function `tally`) is refused with ``no 2-argument arm of `tally` (arms take
  1)``. Should print `2 1`; renaming the type makes it do so. This contradicts
  the 2026-10-12 entry "a name the author declared wins over a builtin and an
  import", which fixed the reverse case (a function named like an imported
  type).

The three bugs in the original `bugs/` stand as filed: `map_put_bound_is_
quadratic.kso` is fixed (F2), `marker_name_as_pattern_binder.kso` still
fails (F6), and `text_concat_refuses_strings.kso` is moot, since
`text/concat` is gone (#1906).

## Comfort

On the language and core library, a new start would be much easier than the
journal's. The three largest problems are gone: a map threaded through a
loop no longer copies (F2), the interpreter loop runs in a flat megabyte
natively (F8), and a long list or block of source text can be written as it
reads (F4). `merge` replaces the six one-field rebuild helpers (F10), and
`io/read_line` would make the REPL usable at a terminal (F12). The journal's
first debugging session would still happen, because a parameter named `done`
is still a silent literal pattern (F6), and an interpreter still has no
mutable cell, so the `world` record and the `knot` trick would be the same
design (F9). The namespace is quieter but not quiet: imports no longer leak
most bare names, but a type named like an imported function still collides
(B2), and a lambda parameter cannot share a name with a function in another
file of the module. The new costs are small: a none helper around every tail
slice (N1), dispatching an err before handing it to the evaluator, and a
dozen `pub` markings for the unit tests (N2). The first contact with the
toolchain is still the journal's loop: appendix B's own sample is refused by
`kanso run` (F1). On balance, the summary's top three (F8, F2, F6) are now
two fixed and one standing, and the port migrated in about 130 changed
lines without a change to any expected output.
