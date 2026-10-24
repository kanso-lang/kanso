# Re-sit: raytracer

## Result

check.sh: pass (58 unit tests, 23 fixtures, `file_three matches glass`),
engines agree: yes.
Lines changed by the migration: 204 (`diff -r -x resit -x build -x bugs
ports-orig/raytracer ports/raytracer | grep -c '^[<>]'`), in 12 files. No
expected-output file changed.

Scratch programs are in `resit/t/`. `resit/f25/` is the port with the F25
workaround taken out; `resit/f26/` also restores F26's wanted default. The
check log is `resit/check.log`.

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[exhaustive]: this can be an err and `fmath/max_of` has no arm for it — dispatch on it here…`` at `tracer/camera.kso:42`, plus ``…and `rows` has no arm for it`` at `tracer/render.kso:47`. A name bound to a raising call now carries its err (2026-10-12, #1884), so the journal's F11 workaround, binding `fmath/floor` to `tall` first, stopped working | partly. The first told me what; the second pointed at a file I had not touched and went away with the first. I wrote a two-arm `rows_of` helper whose err arm can never run | 1 |
| ``error[exhaustive]: this can be an err and `tracer/new_dielectric` has no arm for it — dispatch on it here, or give `tracer/new_dielectric` an `(err _)` arm`` and the same for `new_lambertian`, `new_metal`, `new_sphere` and an operator `/` (#1884) | partly. The suggested fix is an err arm on another module's constructor. What fits the book's railway is a bind step: `scalar rest n .> tracer/new_dielectric`, and for two fallible arguments `albedo .> (a -> fuzz .> (f -> tracer/new_metal a f))` | 5 |
| ``error[exhaustive]: this can be a none and `scalar` has no arm for it`` (`text/slice` past the end answers `none`, #1890) | partly. The ranges were guarded; I replaced each slice with a literal list of guarded indexes (`[rest[1] rest[2] rest[3]]`) or `list/to_list (list/drop rest 3)` | 5 |
| ``error[opacity]: `vec/vec3` is foreign — only `vec` builds a `vec3`; ask it for one through a pub function`` in vec_test.kso (a test file is an outside consumer, #1883) | yes; `vec/new` already existed | 20 calls, 1 file |
| ``error[name]: unknown name `camera_for` `` and 20 other names in tracer_test.kso and cli_test.kso (#1883) | no. "unknown name" does not say the name is private (see sat N3) | tracer: 7 functions, 2 constants and 2 types made pub, 4 new `new_` constructors, 2 destructuring helpers in the test; cli: 2 types and `defaults` made pub, a new `pub fn options_for`, 2 helpers in the test |
| ``error[formatting]: an inline constant has no indented block`` on making `default_view` pub (journal bug b07) | no | 1; the test calls `standard_view 0` instead |
| ``error[exhaustive]: this can be an err and `==` wants a value`` in a test of `pixel_line` | yes; the `same?` helper fmath_test already had | 1 |
| ``error[runtime]: no overload of `absorbed_gen` matches these arguments`` at test time | no. The test's helper named a private type in a pattern, which `check` accepts (F15) and which never matches | 1; made `absorbed` pub |
| ``error[runtime]: no overload of `scenefile/next_line` matches these arguments`` in tests and on every fixture, and a check refusal ``this can be a none and `tracer/scatter` has no arm for it`` reported at `tracer/render.kso:88` | no, and it misled. Making the marker `materials` pub in tracer turned the binder `materials` in scenefile's patterns into that type (bug b10) | reverted; the test uses `full_shading` |
| ``error[formatting]: a line holds at most 80 characters`` | yes | 3 |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | an entry file cannot hold a definition; `pub play =` runs under neither verb | tooling | minor | tooling | still | the book's `samples/ch04/missing.kso` under `kanso run` → "is a library — nothing to run. it exports `play`…"; a file with `fn` and `print` under `kanso run` → "is a library" (see sat `resit/t/f04*.kso`) |
| F2 | there is no type called `float` | diagnostic | nit | language | still | `resit/t/f02.kso` → ``no type is called `float`, so this arm can never match``, still without naming `float64` |
| F3 | no exponent notation in float literals | missing-feature | minor | language | fixed | sat `resit/t/f05.kso`: `1e100` and `1e-8` print `1.0e+100 1.0e-08` (#1889) |
| F4 | no unary minus on an expression | missing-feature | minor | language | still | `-x` → ``binary minus takes a space on each side, `a - b` `` (`resit/t/f04.kso`) |
| F5 | std/math has `sqrt` and `round` and nothing else | stdlib-gap | major | core-lib | still | `grep '^pub' lib/math/*.kso`: `random`, `round`, `round!`, `sqrt` |
| F6 | a pub record cannot be built or taken apart positionally by an importer | confusing-semantics | major | language | ruled-no | construction stays with the owner (2026-08-20 "only a type's owner constructs one", restated in the 2026-10-13 `merge` gavel, #1886). `resit/t/f06`: `v/vec3 n 0` → ``only `v` builds a `vec3` ``. A bare positional pattern `(vec3 x)` on the import works; the qualified one is still refused (sat F14) |
| F7 | an importer cannot take every field in one statement | confusing-semantics | major | language | still | `bugs/b02` passes check and fails at run: `a keyed read omits at least one field; reading every field is the positional form` |
| F8 | an operator arm makes a same-named pub function report private | engine-bug | major | language | still | `bugs/b01` → ``error[opacity]: `unit` is private to module `rng` `` |
| F9 | every top-level name is a forbidden local in every file, tests included | refactoring-hazard | major | language | partly | test files no longer join the module (#1883). A local still collides with a declaration in another file, and the message still does not name that declaration (sat `resit/t/f12`: ``error[name]: `moved` is already a declaration; rename the binding``) |
| F10 | `none` cannot seed a fold; the message offers a fix not mine to make | diagnostic | minor | language | still | `list/fold [3 1 2] none closer` → ``…give `list/fold` a `none` arm`` (`resit/t/f10.kso`) |
| F11 | `math/round` can answer none, and the checker is satisfied by naming the call | confusing-semantics | major | core-lib | worse | the name no longer satisfies it (#1884), which removes the inconsistency, but the wanted one-liner is still refused and the cheap workaround is gone: camera.kso now needs a two-arm `rows_of` helper, and tracer_test needs `same?` around `pixel_line`. `math/round!` answers a box, which pure code cannot open |
| F12 | a list literal wider than 80 columns cannot be written | engine-bug | major | language | fixed | `bugs/b05_wide_list_literal.kso` prints 2 (#1882) |
| F13 | no record update | missing-feature | major | language | fixed | `merge` (#1886); sat `resit/t/f11.kso` prints `f11/trail 2 5` |
| F14 | alphabetical fields fix the constructor's argument order | aesthetics | minor | language | ruled-no | the `merge` gavel (#1886): "Construction by field name stays declined (2026-07-29, named arguments)" |
| F15 | a pattern naming a type that does not exist is accepted | diagnostic | major | language | still | `bugs/b04` → `shapes: ok`. It bit the migration: a test helper `fn absorbed_gen (absorbed gen)` named a type the test file cannot see, checked clean and failed with ``no overload of `absorbed_gen` matches`` |
| F16 | `kanso check .` inside a module cannot see its siblings | tooling | minor | tooling | fixed | `cd bugs/b03_check_dot_loses_siblings/b && kanso check .` → `.: ok` |
| F17 | a failing test says only "returned false" | tooling | minor | tooling | still | `resit/t/f17`: `test_double ... FAILED (returned false)` |
| F18 | an operator arm does not count as using its import | confusing-semantics | minor | language | still | `resit/t/f18`: `import "../v"` in a file that adds two `v` values → ``error[unused]: unused import "../v"``; without it the program prints `v/vec3 3` |
| F19 | a long single expression must be inline | aesthetics | nit | language | still | `resit/t/f19m`: ``a single-expression constant is written inline``. The parentheses half is ruled-no (#1887) |
| F20 | random numbers are only an effect | stdlib-gap | major | core-lib | still | appendix B: `math/random int → io (yields int)`; nothing else |
| F21 | vec3 arithmetic on records costs about 60 times C | performance | major | core-lib | still | `bench/run.sh` twice today: kanso vecs 0.42 s and 0.34 s, C 0.010 s and 0.009 s, about 40 times (journal: 60 to 80) |
| F22 | a field read on a value that may be a marker passes the checker | diagnostic | minor | language | still | `bugs/b06`: check ok, run ``error[runtime]: `probe/miss` has no field `t` `` (now with its line) |
| F23 | unused private functions are not flagged | diagnostic | nit | tooling | still | `resit/t/f23.kso` with an unused `contains?` runs and prints 2 |
| F24 | `kanso check` will not check a `kanso play` file | tooling | nit | tooling | still | `kanso check resit/t/f23.kso` → "a file with declarations is a library, and a library has no statements to run" |
| F25 | the release build fails every bounce once an unused shading arm exists | engine-bug | blocker | language | fixed | `resit/f25` takes out `+ vec/zero`; the release binary matches the expected files for shade_hemisphere, diffuse, glass and cover_small |
| F26 | a none through record fields across modules is reported at the far end | diagnostic | major | language | worse | `resit/f26` restores the wanted `(tracer/shading_named name)` default; `kanso check` says ok, and `kanso run main.kso -- diffuse` stops with ``no overload of `tracer/surface` matches these arguments`` on native and interp. The hole the journal said the check found is no longer reported at all. A two-module reduction (`resit/t/g4`) is still caught, so the miss depends on the chain's length |
| F27 | ambient builtin names are reserved as locals | refactoring-hazard | minor | language | still | `fn total values` → ``error[name]: `values` is already a declaration`` (`resit/t/f27.kso`) |
| F28 | a parse error wants the err railway, and the checker wants an err arm everywhere | confusing-semantics | major | language | partly | binding first no longer silences the check (#1884), so the reader's ambiguity the journal named is gone. The wanted `tracer/new_lambertian (triple rest n)` is still refused; the accepted form is now a bind step, nested once per fallible argument |
| F29 | two arms that each win somewhere need a third arm | confusing-semantics | minor | language | still | `resit/t/f29.kso` → ``these `step` arms tie…`` |
| F30 | no `ends_with?` in std/text | stdlib-gap | minor | core-lib | fixed | `text/ends_with? "three.scene" ".scene"` prints true (#1914) |
| F31 | an advisory about a type crossing two modules on a fine design | diagnostic | nit | tooling | still | `kanso check scenefile` → ``advisory[door]: `parse` returns `tracer/scene` and the surface offers nothing that accepts it`` |

Counts: fixed 6, partly 2, still 19, ruled-no 2, moot 0, worse 2.

## New friction

### N1: a pattern binder silently becomes an imported type of the same name
- kind: confusing-semantics
- severity: blocker
- scope: language
- where: scenefile/scenefile.kso (every `(reading camera n materials objects)` arm) after `type materials` in tracer/render.kso became pub; bugs/b10_binder_becomes_imported_type/
- wanted: `materials` in a positional pattern to bind the field, whatever the imports declare.
- wrote: reverted `pub type materials` and had the test use `tracer/full_shading`.
- why it matters: making a marker pub in one module changed the meaning of
  patterns in another module that imports it. `check` stayed clean. The
  symptoms were a none refusal at `tracer/render.kso:88` naming `scatter`,
  and at run time ``no overload of `scenefile/next_line` matches``, neither
  of which points at the cause. It cost an hour of stubbing to find.

### N2: tests reach the module only through its pub surface, which this port's tests did not use
- kind: refactoring-hazard
- severity: major
- scope: tooling
- where: tracer/tracer_test.kso, cli/cli_test.kso, vec/vec_test.kso
- wanted: the journal's tests, which built rays, hit records and options by hand and compared whole records.
- wrote: `new_group`, `new_hit_record`, `new_quality` and `new_ray` in tracer/build.kso; `pub` on `camera_for`, `cover`, `get_ray`, `hit`, `pixel_line`, `scatter`, `sky`, `from_surface`, `grey`, `absorbed`, `miss`, cli's `options`, `bad_flag` and `defaults`; a new `options_for`; and destructuring helpers (`complaint`, `fields`, `absorbed_gen`, `unknown_name`) where a test compared against a record it can no longer build. The rule is ruled (#1883).
- why it matters: the journal listed "a test builds a hit record by hand and compares whole results" under what worked well. Under the ruling that test needs a constructor in the production module that only the test calls.

### N3: an err carried by a name needs a bind step per argument
- kind: aesthetics
- severity: minor
- scope: language
- where: scenefile/scenefile.kso `material_of "metal"`, `set_camera … "aspect"`, `sphere_line`
- wanted: `tracer/new_metal (triple first n) (scalar rest n)`.
- wrote: `albedo .> (a -> fuzz .> (f -> tracer/new_metal a f))`.
- why it matters: it is correct and readable for one argument
  (`scalar rest n .> tracer/new_dielectric`). With two it nests, and with
  three (a vector's x, y, z) it would nest again.

### N4: an err handed to std/list is reported inside std/list
- kind: diagnostic
- severity: major
- scope: language
- where: bugs/b09_err_reported_inside_std_list/
- wanted: one refusal at `t/use (list/first r.objects)`, the line the author wrote.
- wrote: nothing; found while reducing N1. `kanso run` prints four
  `error[exhaustive]` lines located at `std/list/list.kso:167`, `:174`,
  `:214` and `:220`, and none in the program.
- why it matters: the fix belongs in the caller, and the caller's file is
  not named.

### N5: `kanso test` names the test file for a failure inside std/list
- kind: tooling
- severity: minor
- scope: tooling
- where: scenefile/scenefile_test.kso, during N1
- wanted: the failure's place, `std/list/list.kso:214`, as `kanso run` printed it (`failed in list/fold_flat at std/list/list.kso:214`).
- wrote: nothing; `kanso test` printed `--> scenefile/scenefile_test.kso:214:19` for a 45-line test file.
- why it matters: the line number belongs to one file and the file name to another.

## Bugs

- `bugs/b10_binder_becomes_imported_type/` (new): a pattern binder named
  like an imported pub marker type is read as that type. Check is clean;
  every engine stops with "no overload of `m/size` matches".
- `bugs/b09_err_reported_inside_std_list/` (new): an err reaching
  `list/first` through a record field is reported at four lines inside
  std/list/list.kso and nowhere in the program.
- The journal's bugs, re-run: b01 still fails (F8); b02 still passes check
  and fails at run (F7); b03 fixed (F16); b04 still accepted (F15); b05
  fixed (F12); b06 still accepted (F22); b07 still refused (now met again
  when making `default_view` pub); b08, the release miscompile, no longer
  reproduces in the port (F25).

## Comfort

On language and core-lib, the original agent would find the formatting
walls lower: exponent literals, a wrapped scene list, `merge` for the flag
table and the camera, and `text/ends_with?` all work now, and the release
build no longer miscompiles the hemisphere arm. The numeric floor has not
moved. There is still no `floor`, `abs`, `min`, `tan`, `pi` or infinity,
no unary minus, and random numbers are still an effect, so `fmath/` and the
generator threaded through forty call sites stay. The err check is now
consistent: a name carries its err. That ends the journal's complaint that
the rule could be dodged by a rename, and it also takes the dodge away, so
a NaN-only floor now costs a helper with an arm that never runs, and the
scene-file railway is written with nested `.>` steps. The none check is less
reliable than the journal found it: the unknown `--shade` path it once
caught now passes check and stops at run time. Making a type pub can
silently change patterns in another module (N1), which is the worst new
trap. On tooling, tests must go through the pub surface, which cost this
port four constructors and a dozen pub names that only tests use. The
journal's summary said module boundaries cost more than they should; today
they cost about the same, in different places.
