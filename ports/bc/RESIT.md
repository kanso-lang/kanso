# Re-sit: bc

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the journal
re-test are in `resit/`, named after the entry they test. `resit/f15/`
holds the three F15 snapshots from `bugs/`, migrated with the same edits as
the port so they compile today. Timings are wall time on a 4-CPU container
shared with three other agents.

## Result

- check.sh: pass. 47 unit tests, the embedded-library check, 33 fixtures.
- Engines agree: yes, on all 33 fixtures. The differential fuzzer against
  GNU bc 1.07.1 (`fuzz/run.sh`) found 0 differing batches of 30 for
  `arith` and 0 of 30 for `bases`; `mathlib` differed in 7 of 10 batches,
  all in the last digit or in GNU's negative zero, the divergences
  README.md lists.
- Lines changed by the migration: 181 lines of `diff -r -x bugs -x resit`
  against ports-orig/bc, 50 of them in test files.
- No expected-output file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| M1 | `` error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c` `` | yes | 4 |
| M2 | `` error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes. Then `` error[import]: `list` is not imported here — a module's files share their declarations, not their imports `` and two `unused import "std/text"`, both of which said what to do. | 10 in 3 files, 3 import lines |
| M3 | `` error[exhaustive]: this can be an err and `enter` has no arm for it — dispatch on it here, or give `enter` an `(err _)` arm (in calls.kso) `` (a name bound to a raising call carries its err, #1884) | yes, one function at a time. The twenty-odd bindings F6 added to dodge this check are now checked too. Each arm added (`fn expect e@(err _) _` / `e`) makes that function's answer an err, so its callers then need one: three rounds, 34 errors in the first. I wrote a script that reads the caret and adds the arm at that position. Two of the diagnostics in the last round (`bc/loop_for`, `bc/print_items`) came with the module prefix and no source line. | 22 arms in 22 functions, 3 files |
| M4 | `` error[exhaustive]: this can be a none and `text/utf8` has no arm for it — resolve it here, or give `text/utf8` a `none` arm (in lexer.kso) `` (text/slice past the end answers none, #1890) | no; an arm on a std function is impossible. A helper `utf8_in` in the lexer whose `none` arm answers `""`. For the two-character operator lookahead at the last byte of the input this arm is real: there is no pair there. | 4 lexer sites, 1 in digits.kso |
| M5 | none at check. Every fixture stopped at run time on every engine with `error[runtime]: length takes a list, string, or map, not <none>` / `failed in bc/in_base at bc/digits.kso:22` | no. `text/slice bs (point + 1) (length bs)` with no decimal point starts two past the end, and that now answers `none` (one past the end still answers empty). Wrapped in `or_empty`. The checker passed it because the `none` goes to `length`, a builtin (N2). | 1 |
| M6 | `` error[opacity]: `bc/dec` is foreign — only `bc` builds a `dec`; ask it for one through a pub function `` (a test file is an outside consumer, 2026-10-12 gavel) | it says what to add. `pub fn dec_of scale units` in num.kso, used by the tests in place of the constructor. Four tests then passed 80 columns and needed a `got =` binding (F22 of kv's journal). | 34 constructions in 2 test files |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a strict index inside a call passes check | diagnostic | minor | language | fixed | `resit/f01`: `text/to_int a[1]!` is refused with `this is an effect — a box the words open — and `text/to_int` takes a value; open it with `.>`` (log 2026-10-14). |
| F2 | no multi-line string, and `{` must be escaped | missing-feature | major | language | partly | `resit/f02b.kso`: the library fits in one `"""` block, so the `. push` chain is no longer needed. `resit/f02_text_block.kso`: an unescaped `{` in the block still gets `unterminated interpolation`, so bc source still needs every brace escaped and `embed_mathlib.sh` would still be a generator. |
| F3 | no name may repeat anywhere in a module | refactoring-hazard | major | language | still | `resit/f03`: a parameter named `values` gets `` `values` is already a declaration; rename the binding ``. kv's re-sit, F8, reproduces the cross-file case. |
| F4 | importing std/list makes `step` a type in my module | diagnostic | minor | language | fixed | `resit/f04`: `fn step u label m k` in a module importing std/list prints `7 1` on both engines. In a play file the native build still resolves such a name to the std type (inflate's re-sit, N5). |
| F5 | re-putting a key into a native map is quadratic | performance | major | core-lib | still | `resit/f05`: a map held in a record, read and then re-put each step: 10,000 steps in 3.6 s and 40,000 in 68.7 s, release. A bare `put` loop is fast (160,000 in 0.02 s). Inflate's re-sit (F7) measures the same read-then-put shape. |
| F6 | the checker wants an err arm for a call but not for the same call named | refactoring-hazard | major | language | partly | #1884 (log 2026-10-12, "a name bound to a raising call carries its err into the check") made calls and names agree for errs. The agreement went the strict way, so the port now carries 22 `(err _)` arms (M3). For a none the old asymmetry stands (inflate's re-sit, F16). Diagnostics with a module prefix and no source line still occur (M3). |
| F7 | one err read through two fields becomes a list of two errs | confusing-semantics | major | language | still | `bugs/err_read_twice_merges.kso`: `caught: ["boom" "boom"]` then `caught: boom`, on both engines. |
| F8 | the wrappers F7 needs are held to rules the constructors are not | confusing-semantics | minor | language | still | `resit/f08`: `got (make_if 1 none 2) s` gets `` this can be a none and `make_if` has no arm for it ``, where the constructor `if_stmt 1 none 2` takes the none. |
| F9 | a comment cannot stand on its own | aesthetics | nit | language | fixed | `resit/f09_section.kso`: a header comment, and a `# --- statements ---` comment followed by a blank line in the middle of the file, both accepted (gavel 2026-10-13, header comment). |
| F10 | a type must sit at the top of its file | aesthetics | nit | language | still | `resit/f10_type_late.kso`: `canonical order places type declarations before functions; move `cursor` up`. |
| F11 | a record with a dozen fields and no way to change one | missing-feature | major | language | fixed | `resit/f11_merge.kso`: `merge m { "scale":5 }` prints the updated machine on both engines (#1886). The port's twelve setters are unchanged; moving them is a refactor, not a migration. |
| F12 | `list/zip` pairs cannot be taken apart in a parameter | missing-feature | nit | language | still | `resit/f12_pair_pattern.kso`: `fn back_of [p a]` gets `error[syntax]: expected a parameter pattern`. |
| F13 | a keyed read of every field passes check and fails at run | diagnostic | minor | language | still | `resit/f13`: `kanso check` ok; the run stops with `a keyed read omits at least one field; reading every field is the positional form`, now naming `f13/m/m.kso:6`. |
| F14 | two arms on different record types called a tie | diagnostic | minor | language | still | `resit/f14_tie.kso`: `` these `misplaced_in` arms tie ``. mustache's disjoint none-vs-string case (its F5) is fixed; two different record types are not seen as disjoint. |
| F15 | native builds corrupt memory in loops that carry the parser state | engine-bug | blocker | language | fixed | The three snapshots, migrated (`resit/f15/`): `loop_list_clobbered` prints `0` natively and on the interpreter; `beat_rewind_crash` prints both expected stderr lines and exits 0 natively; `beat_carry_parser_state` exits 0 natively and its release output matches `bc -l` on all 60 lines. The migration changed code around the loops, and the journal says these depended on layout, so this is evidence, not proof. |
| F16 | a keyed read of an err stops the program | engine-bug | major | language | partly | `bugs/keyed_read_of_err.kso`: no more native stack overflow; both engines print `cannot read fields of err "boom"; keyed reads take a record` with the line. The keyed read still stops where a dot read passes the err on. |
| F17 | a native loop carrying a record never gives memory back | performance | major | core-lib | fixed | release build, `for (i = 0; i < 300000; i++) s += i`: 0.15 s and 10 MB peak (journal: 535 MB); GNU bc 0.06 s. |
| F18 | the engines run out of stack at different depths | engine-bug | minor | language | partly | `define f(n)` recursion: the interpreter answers f(2000) and stops by f(3000) naming `bc/invoke at calls.kso:26`; native answers f(5000) and stops by f(8000), now with `the program ran out of stack` instead of a silent segfault, but naming `bc/misplaced_in at parser.kso:785` (N3). Depths still differ. |
| F19 | standard input arrives all at once | missing-feature | major | domain-lib | fixed | `resit/f19`: an `io/read_line` loop answers line by line on both engines (#1914). The port still reads `io/stdin` whole. |
| F20 | test constants are one line, inside the module's namespace | tooling | minor | tooling | partly | `resit/f20`: a test constant `autos` beside a module parameter `autos` no longer collides (2026-10-12 test-file gavel), and a test program fits in a `"""` block. Its braces still need escaping. |
| F21 | a pub type cannot be taken apart outside its module | confusing-semantics | nit | language | still | the same refusal as kv's re-sit, F12: `` `m/merged` is foreign — its structure does not cross an import ``. |

Counts: fixed 7 (F1, F4, F9, F11, F15, F17, F19), partly 5 (F2, F6, F16,
F18, F20), still 9 (F3, F5, F7, F8, F10, F12, F13, F14, F21), ruled-no 0,
moot 0, worse 0.

## New friction

### N1: an `(err _)` arm on one function makes its callers need one too
- kind: refactoring-hazard
- severity: major
- scope: language
- where: bc/parser.kso (14 arms), bc/exec.kso (5), bc/calls.kso (3)
- wanted: the 2026-10-10 parser, where `closed = expect opened ")"` and
  `got check.node closed` passed a syntax error up untouched.
- wrote: 22 arms of the form

      fn expect e@(err _) _
        e

  added in three rounds by a script, because each arm makes the function's
  answer an err in the checker's eyes, and its callers are then refused.
- why it matters: a recursive-descent parser passes its state through every
  function, and a syntax error can come out of any of them. ch04 says "the
  signature says nothing", but the checker now asks every function on the
  path to say it, one diagnostic at a time. The alternative, `.>` at each
  call that binds a parse result, rewrites every such call and can run into
  the native build failure mustache's re-sit found (its N2).

### N2: a `none` from a slice that starts past the end reaches `length` unchecked
- kind: confusing-semantics
- severity: major
- scope: core-lib
- where: bc/digits.kso:19; resit/n_slice_bounds.kso
- wanted: `part = text/slice bs (point + 1) (length bs)`, the fraction
  digits, empty when there is no point.
- wrote: `part = or_empty (text/slice bs (point + 1) (length bs))`.
- why it matters: every fixture failed at run time with `length takes a
  list, string, or map, not <none>`, and `kanso check` said ok. The edge is
  hard to predict: `resit/n_slice_bounds.kso` prints `[]` for
  `slice "a" 2 1` (one past the end), `none` for `slice "" 2 0` (two past),
  and `[]` for `slice "ab" 0 1` (before the start). The #1890 ruling says a
  range that starts past the end answers `none`; one past the end does not,
  and position 0 does not either.

### N3: native names the wrong function when bc recursion overflows
- kind: diagnostic
- severity: minor
- scope: language
- where: `define f(n) { ... f(n - 1) }` with n = 8000
- wanted: the overflow named where the recursion runs, as the interpreter
  does (`failed in bc/invoke at bc/calls.kso:26`).
- wrote: nothing; the release binary says
  `failed in bc/misplaced_in at bc/parser.kso:785`, a placement check that
  runs while the unit is parsed. A ten-line reduction with a parse-like pass
  followed by deep recursion ran to ten million on both engines, so there is
  no postcard.
- why it matters: the line sends the reader to the parser. The 2026-10-17
  gavel says an overflow keeps its sentence with no line; the 2026-10-19
  entry names "the outermost cycle still running", and the native build's
  choice of cycle is not one the interpreter agrees with.

### N4: a test builds a number through a function written only for it
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: bc/num.kso (`dec_of`), bc/num_test.kso, bc/digits_test.kso
- wanted: `add (dec 1 15) (dec 2 225) == dec 2 375`, as on 2026-10-10.
- wrote: `pub fn dec_of scale units` and `dec_of` in 34 places; four tests
  grew a `got =` binding because the longer name crossed 80 columns.
- why it matters: the same rule kv's and inflate's re-sits hit (their N5
  and N3). For a numeric type the expected value is the clearest test, and
  writing it now needs a public constructor the program does not use.

## Bugs

- No new reductions. The three F15 snapshots in `bugs/` no longer
  reproduce once migrated (`resit/f15/`). `bugs/err_read_twice_merges.kso`
  still shows F7. `bugs/keyed_read_of_err.kso` no longer crashes natively;
  both engines stop with the same runtime error.
- N3 is an engine disagreement in stderr, observed in the port and not
  reduced.

## Comfort

The language and core library are more trustworthy than the journal's last
paragraph allowed. The native memory corruption that topped the journal does
not reproduce in the three snapshots, a loop that carries the interpreter
state now runs in constant memory, a box handed to `to_int` is caught at
check, `merge` would replace the twelve hand-written setters, a header
comment and section comments are accepted, and `read_line` makes an
interactive bc writable. The original agent would spend fewer hours on
native crashes and would not need a differential harness just to trust the
binary, though it would still want one for the arithmetic. The failure model
is still where the time goes. #1884 removed the trick of naming a value to
pass the check, so the parser now carries 22 err arms that only hand the err
back. One err read twice still becomes a list of two, a keyed read still
stops on an err, and a `none` from a slice reached `length` in every fixture
without a word from the checker. Map read-then-put is still quadratic, so
variables would still live in slot lists. Names still collide across files.
A new start would be safer in the runtime and noisier in the checker.
