# Re-sit: mustache

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the journal
re-test are in `resit/`, named after the entry they test.

## Result

- check.sh: pass. 26 unit tests, 35 cases on three engines each.
- Engines agree: yes on every case in check.sh. Two bugs found during the
  re-sit make engines disagree or fail to build (Bugs, below); neither is on
  a path the port now takes.
- Lines changed by the migration: 75 lines of `diff -r -x bugs -x resit -x build`
  against ports-orig/mustache, 16 of them in the test file.
- No expected-output file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| M1 | `` error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes | 5 in 2 files |
| M2 | `` error[exhaustive]: this can be an err and `sectioned` has no arm for it — dispatch on it here, or give `sectioned` an `(err _)` arm (in parse.kso) `` (a name bound to a raising call carries its err, #1884) | yes. The arm that forwards the err unchanged, `fn sectioned _ _ _ _ _ _ e@(err _)` / `e`, is the shape std/json uses; I took it from lib/json/text.kso. Binding with `.>` instead fails to build natively (Bugs). | 5 arms (`sectioned`, `blocked`, `parented`, `inverted`, `interpolate_rendered`) |
| M3 | `` error[exhaustive]: this can be a none and `list/count` has no arm for it — resolve it here, or give `list/count` a `none` arm `` (text/slice past the end answers none, #1890) | no. The second suggestion is impossible for a std function. Every one of these ranges lies inside its bytes. Two helpers with dead none arms, `inside` (answers `[]`) and `inside_text` (answers `""`), the same shape std/path uses for `whole`. | 4 |
| M4 | `` error[exhaustive]: this can be a none and `partials_from` has no arm for it `` at `partials_from (path/dirname template)` | no. `path/dirname` now answers `none` to its callers, because it ends in a bare `text/slice`. A dead arm, `here_if_none`, in the CLI. | 1 |
| M5 | `` error[opacity]: `mustache/text_node` is foreign — only `mustache` builds a `text_node` `` and the same for `var_node` and `context_miss` (a test file is an outside consumer, 2026-10-12 gavel) | it says what is wrong, not how to test it. The parse test compares outlines instead of trees; the strict-mode test reads `r.line` and `r.name`; `at?` reads fields instead of destructuring `parse_error`. | 3 tests, 1 helper |
| M6 | `` error[exhaustive]: this can be an err and `renders?` has no arm for it `` for data from `json/decode` | yes. `(err _)` arms on `renders?` and `renders_with?` answering false. | 2 arms |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | std names take over a module's own types without an import | engine-bug | major | language | partly | `bugs/std_name_leaks_without_import.kso` now prints `std_name_leaks_without_import/run 1 2 3` on both engines, and `resit/f01c.kso` (own `type cursor` beside `import "std/list"`) works. When the file imports the module that exports the name, the own declaration still loses: `resit/f01b.kso` (`type run` with `import "std/os"`) gets `` no 3-argument arm of `run` (arms take 2) ``, and `resit/f01d.kso` (`type env`) prints the record natively and `<io> 0` on the interpreter. `bugs/own_type_loses_to_imported_function.kso`. |
| F2 | a file's opening comment cannot stand apart | diagnostic | nit | language | fixed | `resit/f02`: a header comment, one blank line, then a commented `pub type` checks ok (gavel 2026-10-13, "a file may open with a header comment and one blank line"). |
| F3 | canonical arm order pulls arms out of a byte table | aesthetics | minor | language | ruled-no | #1888 (gavel 2026-10-13, "overload arms stay in specificity order"). `resit/f03_arm_order.kso` still gets `overloads of `act` appear most-specific first`. |
| F4 | a qualified marker in a pattern does not count as using its import | engine-bug | major | language | still | `bugs/qualified_marker_in_pattern.kso`: `` error[unused]: unused import "std/json" `` on both engines. |
| F5 | arms that can never overlap called a tie | engine-bug | minor | language | fixed | `bugs/disjoint_arms_called_a_tie.kso` prints `nothing loud a other 3` on both engines. |
| F6 | every mustache template in source needs its braces escaped | aesthetics | minor | language | still | `resit/f06_braces.kso`: `` `name` is not a literal: a map's keys are literals ``. A `"""` text block interpolates too (`resit/f06b.kso`, same error), so there is still no raw form. |
| F7 | the interpreter's concat refuses a list made by text/bytes | engine-bug | major | core-lib | fixed | `resit/f07_concat_bytes.kso`: `list/concat [1] (text/bytes "a")` prints `[1 97]` on both engines (log 2026-10-12, "concat reads the bytes text/bytes answers"). `push` onto `text/bytes` still fails; see kv's re-sit. |
| F8 | a maybe-none in two positions needs four arms | confusing-semantics | minor | language | still | `resit/f08_two_nones.kso`: `(none, _)` and `(_, none)` get `` these `placed` arms tie ``; `resit/f08b.kso`: the guard `return "inline" if first == none or resume == none` does not narrow, and four call positions get `this can be a none`. |
| F9 | errors in a library appear only when another module imports it | diagnostic | major | language | still | the journal's own reproduction on a scratch copy: with `sigil = if (sigil? cs[after]) cs[after] 0` and a `none` arm on `sigil?`, `kanso check mustache` prints `mustache: ok` and `kanso check .` reports two errors in parse.kso under `(module ./spec)`. |
| F10 | a pub type cannot be built outside its module | confusing-semantics | minor | language | ruled-no | 2026-08-20, "only a type's owner constructs one", restated in the #1886 gavel. The test-file rule made it bite harder (M5, N5). |
| F11 | a string passed where a list was meant ran without complaint | refactoring-hazard | minor | language | ruled-no | #1946 (gavel 2026-10-15, "a record field carries no type, and kanso has no type parameters"). |
| F12 | the build names the binary after the directory, no flag to change it | tooling | minor | tooling | still | `kanso build . -o x` answers `` `build` wants a file or directory ``; from a directory named mustache, `kanso build .` answers `this build is named `mustache`, and a directory of that name is here`. |
| F13 | every input of a three-file command needs its own function | confusing-semantics | minor | language | still | `resit/f13_three_inputs.kso`: the three `(file_not_found f)` arms get three `` these `rendered` arms tie `` errors. |
| F14 | interpolating into an accumulator is quadratic, far worse natively | performance | major | core-lib | fixed | `bugs/interpolation_accumulator_slow_native.kso` (40,000 pieces): 0.01 s natively, 1.87 s on the interpreter; the journal had 21.6 s natively. `resit/f14_interp_acc.kso`, release build: 0.01 s, 10 MB. Log 2026-10-10, "a fold that joins onto its accumulator writes in place". |
| F15 | adding a field means editing every construction | refactoring-hazard | minor | language | fixed | `resit/f15_merge.kso`: `merge acc { "cache":(put acc.cache key nodes) }` prints the updated `sink` on both engines (#1886). |
| F16 | a function named like a std type: engines call different things | engine-bug | major | language | fixed | `bugs/function_named_like_std_type.kso` prints `2` on both engines. Its sibling with an import is N6. |
| F17 | one none arm satisfies the check; the catch-all takes the rest | confusing-semantics | minor | language | still | `resit/f17_one_none_arm.kso` prints `caught <none>`. |
| F18 | `x = not (f y)` before a return guard refused as misplaced | engine-bug | minor | language | still | `bugs/not_call_binding_before_return.kso` and `resit/f18_not_call.kso`: `` a `return` sits with the bindings, before the effect chain ``. |
| F19 | the book's `pub play =` samples run under no verb | tooling | minor | tooling | still | ch05's `fused.kso` under `kanso run` answers `is a library — nothing to run`. See kv's re-sit, F3. |
| F20 | changing a callback's argument count is not checked | refactoring-hazard | major | language | still | `resit/f20`: a two-parameter function stored in a record and called with three arguments checks `ok`; the run stops with `` no overload of `m/two_args` matches these arguments ``, now naming `f20/m/m.kso:5`. |
| F21 | std/text has no case conversion | stdlib-gap | minor | core-lib | fixed | `resit/f21_case.kso` prints `HI THERE abc` (#1914). ASCII only, as the port's hand version was. |
| F22 | alphabetical fields put `close` before `open` | confusing-semantics | minor | language | ruled-no | construction by field name declined (2026-07-29, named arguments), restated in the #1886 gavel, which added `merge` for updates only. |
| F23 | a set of ten bytes is thirty-three lines of arms | aesthetics | nit | language | still | the ten `sigil?` arms are unchanged. #1889 added hex literals and left the byte literal unruled. |

Counts: fixed 7 (F2, F5, F7, F14, F15, F16, F21), partly 1 (F1), still 11
(F4, F6, F8, F9, F12, F13, F17, F18, F19, F20, F23), ruled-no 4 (F3, F10,
F11, F22), moot 0, worse 0.

## New friction

### N1: every helper that takes a parse result needs an arm that hands back the err
- kind: confusing-semantics
- severity: minor
- scope: language
- where: mustache/parse.kso (`sectioned`, `blocked`, `parented`, `inverted`), mustache/render.kso (`interpolate_rendered`)
- wanted: the 2026-10-10 code, where `inner = read_run ...` and then
  `sectioned cs cur until acc t s inner` passed a parse err through
  `sectioned` untouched.
- wrote: one more arm per helper,

      fn sectioned _ _ _ _ _ _ e@(err _)
        e

- why it matters: #1884 made the err visible at the binding, which is
  right, but the only thing these helpers do with it is give it back. Five
  arms of boilerplate in a 680-line engine. The other spelling, `.>`, fails
  to build natively (N2).

### N2: `.>` into an arm that takes a record apart fails to build
- kind: engine-bug
- severity: major
- scope: language
- where: bugs/bind_into_record_pattern_fails_to_build.kso
- wanted: `read_run cs next until acc .> (inner -> sectioned ... inner)`,
  which is what ch04 teaches for stepping past an err.
- wrote: the pass-through arms of N1.
- why it matters: a function that answers a record or an err, bound with
  `.>` to a function whose arm destructures the record, makes clang refuse
  the IR (`'%t1' defined with type '%parsed = type { i64, i64, i64 }' but expected '%KValue = type { i64, i64 }'`).
  The interpreter prints the right answer, so the engines disagree.

### N3: in-range slices need dead `none` arms
- kind: confusing-semantics
- severity: major
- scope: core-lib
- where: mustache/parse.kso (`inside`, `inside_text`)
- wanted: `list/count (text/slice cs a b) (c -> c == 10)` where `a` and `b`
  are positions the scanner found.
- wrote: `list/count (inside (text/slice cs a b)) (c -> c == 10)`, with
  `fn inside none` answering `[]`, and a second helper answering `""` for
  the one string slice.
- why it matters: the same as kv's N3. The dummy value would hide a real
  bug, and a typed slice needs its own helper because the dummy has to have
  the right kind.

### N4: `path/dirname` answers `none`
- kind: stdlib-gap
- severity: minor
- scope: domain-lib
- where: cli/cli.kso:105; lib/path/path.kso:57
- wanted: `partials_from (path/dirname template)`.
- wrote: `partials_from (here_if_none (path/dirname template))` with a dead
  `none` arm. `dirname` ends in `text/slice p 1 (past - 2)`, which can no
  longer miss but carries `none` out; `basename` in the same file wraps its
  slice in a dead arm (`fn whole none`) and `dirname` does not.

### N5: a test cannot build the tree it expects
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: mustache/mustache_test.kso:80 and :118
- wanted: `nodes == [(text_node "Hi ") (var_node false 1 ["who"])]`, the
  journal's own "what worked well" example.
- wrote: `shows? "Hi \{\{&who}}" "text \"Hi \"\nraw who"`, which compares the
  outline text and no longer checks the line number the node carries.
- why it matters: the 2026-10-12 test-file gavel removed the test the
  journal praised as the best thing about kanso's equality. The remaining
  way to compare a tree is through a printer the module happens to have.

### N6: a type the file declares loses to a function of a module the file imports
- kind: engine-bug
- severity: major
- scope: language
- where: bugs/own_type_loses_to_imported_function.kso
- wanted: `type env` and `env "x"` in a file that imports std/os.
- wrote: nothing in the port; the port's renames from F1 (`scope`,
  `stretch`, `scanner`) are still in place.
- why it matters: the interpreter builds `os/env "x"` and native builds the
  record, so the oracle disagrees, and with `type run` the check refuses the
  program. F1 was fixed only for files that do not import the module.
  kv's re-sit found the same rule broken for a function (`read_line`).

### N7: a foreign marker is refused qualified and accepted bare
- kind: confusing-semantics
- severity: minor
- scope: language
- where: resit/f13_three_inputs.kso
- wanted: `fn rendered (os/file_not_found f) _ _`, since the file imports std/os.
- wrote: `fn rendered (file_not_found f) _ _`, after
  `` `os/file_not_found` is foreign — its structure does not cross an import ``.
  The bare spelling, which reaches the type because the value carries its
  arms, is accepted. F4 is the same pair the other way round.

### N8: `kanso check` refuses a play file
- kind: tooling
- severity: minor
- scope: tooling
- where: resit/f20 (first written as one play file)
- wanted: `kanso check file.kso` on the file `kanso play` runs.
- wrote: a module and a `main.kso`, after `error[syntax]: a file with
  declarations is a library, and a library has no statements to run — `kanso play` runs declarations beside statements in one file`.
  A play file without declarations checks fine.

## Bugs

- `bugs/bind_into_record_pattern_fails_to_build.kso` (new): `half 4 .> shown`,
  where `half` answers a record or an err and `shown` destructures the
  record, fails native codegen with an LLVM type error; the interpreter
  prints `4/2`.
- `bugs/own_type_loses_to_imported_function.kso` (new): own `type env` in a
  file that imports std/os; the interpreter prints `<io> 0`, native prints
  the record.
- Fixed since the journal: `disjoint_arms_called_a_tie.kso`,
  `function_named_like_std_type.kso`, `std_name_leaks_without_import.kso`,
  `interpolation_accumulator_slow_native.kso`. `concat_rejects_bytes_in_interp.kso`
  no longer compiles because `text/concat` is gone; `resit/f07_concat_bytes.kso`
  is the same program on `list/concat` and passes.
- Still failing: `qualified_marker_in_pattern.kso`,
  `not_call_binding_before_return.kso`.

## Comfort

On the language and core library, a new start would be smoother than the
journal's. The engine-disagreement bugs that topped the journal's list are
gone for files that do not import the colliding module, `list/concat` takes
`text/bytes`, an interpolation accumulator is linear on every engine, `merge`
grows the threaded `sink` in one edit, and `text/uppercase` exists. The
original agent would have hit fewer surprises in its first hour. The
migration also showed new costs. Errs from a parse are now claimed at every
helper they pass through, every in-range slice wants a dead `none` arm, the
test that compared parse trees with `==` cannot be written, and two new
engine bugs turned up within a few lines of ordinary code (a `.>` into a
record pattern fails to build, and an own type named like an imported
function resolves differently on the interpreter). The checker's weaker
spots from the journal are unchanged: two maybe-nones still tie, a library
can check ok and fail when imported, a callback's arity is unchecked, and a
template still needs every brace escaped. Tooling (play files, binary
naming) is as it was. The port compiles with small, local edits, and most of
them were boilerplate rather than design changes.
