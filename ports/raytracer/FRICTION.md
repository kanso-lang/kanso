# FRICTION: porting Ray Tracing in One Weekend to kanso

Compiler: `/tmp/claude-0/kanso-main/kanso` (main as of 2026-10-10).

## Entries

### F1: an entry file cannot hold a definition, and the book's `pub play =` samples run under neither verb
- kind: tooling
- severity: minor
- where: `probes/p1.kso` (the first file written)
- wanted: one file with a `type`, two `fn` arms and a `print`, run with
  `kanso run p1.kso`, the way most of chapters 2 to 4 present their samples.
- wrote: `kanso run p1.kso` answered "`p1.kso` is a library — nothing to run.
  give the module a main.kso entry, or run its definitions beside their
  statements with `kanso play`". Rewriting it the way chapters 4 and 5 do,
  with `pub play =` holding the statements, `kanso play` answered
  "`pub play` is a library's export — import this module from an entry file
  and name its `play`; `kanso play` takes bare statements". What runs is
  `kanso play` with bare statements and no `pub play`.
- why it matters: the first ten minutes went on finding which of three
  verbs-plus-spellings runs a twenty-line file. Both messages are clear on
  their own; the trouble is that the book teaches the spelling the tool
  refuses.

### F2: there is no type called `float`
- kind: diagnostic
- severity: nit
- where: `probes/p1.kso:8`
- wanted: `fn * (vec3 x y z) s:float`
- wrote: `s:float64`, after "no type is called `float`, so this arm can
  never match". The book says "float" in prose throughout chapter 2.
- why it matters: the message is good but could name `float64`; it is the
  obvious next question.

### F3: no exponent notation in float literals
- kind: missing-feature
- severity: minor
- where: `vec/vec.kso` (`near_zero?`), `rng/rng.kso` (`unit_vector`)
- wanted: `tiny = 1e-8`, `1e-160 < lensq`
- wrote: `tiny = 0.00000001`, and the rejection threshold raised to
  `0.0000000001` because writing 1e-160 out longhand is 161 characters, past
  the 80-column limit. `{1e10}` in a string is read as the name `e10`.
- why it matters: a numeric program counts zeros by eye. The original's
  1e-160 guard could not be spelled at all, so the port uses a different
  threshold than the book.

### F4: no unary minus on an expression
- kind: missing-feature
- severity: minor
- where: `vec/vec.kso` (`neg`, `refract`), `probes/p3.kso`
- wanted: `-x`, `-v`, `-dot uv n`
- wrote: `0.0 - x`; a `neg` function for vectors. Negative literals such as
  `-1.0` work; `{-x}` is "expected an expression".
- why it matters: the ray tracer negates constantly (the inward normal, the
  incoming direction in `refract`, `-w` for the camera frame). `0.0 - x`
  reads as arithmetic rather than as a sign, and an operator arm cannot
  supply unary minus for vec3 because there is no unary operator to extend.

### F5: std/math has `sqrt` and `round` and nothing else a geometry program needs
- kind: stdlib-gap
- severity: major
- where: `vec/vec.kso` (`abs`, `min_of`), `fmath/fmath.kso` (all of it)
- wanted: `math/abs`, `math/min`, `math/max`, `math/floor`, `math/tan`,
  `math/pi`, `math/infinity`, `math/pow` — C++'s `<cmath>` plus
  `std::numeric_limits<double>::infinity()`.
- wrote: `fmath/`, a module of my own: `abs`, `min_of`, `max_of`,
  `clamp`, `floor` (round, then step down if that overshot), sine and
  cosine as Taylor series, `tan` as their ratio, `radians`, `pi` typed out
  to 16 digits, and `infinity` as the literal 1 followed by 30 zeros,
  because `1.0 / 0.0` is a `math_failure` rather than inf.
- why it matters: these are the first functions anyone reaches for in a
  numeric program, and each one hand-written is a place to get an edge
  case wrong. The obvious floor, `math/round (x - 0.5)`, answers -1 for
  0.0 because round takes halves away from zero; `fmath_test.kso` pins
  `floor 0.0 == 0` for that reason.

### F6: a pub record can be read by an importer but not built or taken apart positionally
- kind: confusing-semantics
- severity: major
- where: `rng/rng.kso`, `vec/vec.kso` (`new`), `probes/f6_foreign_record/`
- wanted: `vec/vec3 x y 0.0` in module rng, and
  `fn hit_sphere (vec/vec3 x y z) ...` arms elsewhere.
- wrote: a `pub fn new x y z` in vec for every other module to call, and
  `.x`/`.y`/`.z` reads at the use sites. The messages were
  "`vec/vec3` is foreign — only `vec` builds a `vec3`; ask it for one
  through a pub function" and "its structure does not cross an import; use
  its module's pub operations". Dot reads and keyed reads do cross.
- why it matters: a vector is plain data with no invariant to protect. `pub
  type` reads as "this type is public", and it turns out to mean "the name
  is public; the constructor and positional pattern are not". Every module
  with a record that others build needs a hand-written constructor
  function that repeats the field list.

### F7: no way for an importer to take every field of a two-field record in one statement
- kind: confusing-semantics
- severity: major
- where: `bugs/b02_full_keyed_read_passes_check/main.kso`, all callers of
  `rng`
- wanted: `{ gen value } = rng/double g` (or `rng/drawn g1 u = ...`)
- wrote: `d = rng/double g` then `d.gen` and `d.value` on separate lines.
- why it matters: the positional form is refused across an import (F6), and
  the keyed form must omit at least one field, so for a record whose fields
  are all wanted the importer has no destructuring form at all. The pair
  "value plus the next generator" is the shape of every random draw in this
  program, so this cost a line at every draw site. The keyed read also
  passes `kanso check` and fails at run time; see `bugs/b02`.

### F8: an operator arm in an imported module makes a same-named pub function report private
- kind: engine-bug
- severity: major
- where: `bugs/b01_operator_arm_hides_pub_name/`, `rng/rng.kso`
- wanted: `pub fn unit (gen state)` in rng beside vec's `pub fn unit v`,
  called as `rng/unit g`.
- wrote: renamed rng's function to `double` (after the original's
  `random_double`).
- why it matters: the error is `error[opacity]: unit is private to module
  rng` on a function declared `pub`, which sends you to check the wrong
  thing. It took a bisection over twenty declarations to learn that the `-`
  arm in vec was the trigger.

### F9: every top-level name in a module is a forbidden local name in every file of it, tests included
- kind: refactoring-hazard
- severity: major
- where: `tracer/camera.kso:38` (`tall`), `tracer/camera.kso:72`,
  `tracer/scenes.kso:39`, `tracer/sphere.kso:32` (`mat`), `tracer/ray.kso`
  (`bounds`), `tracer/tracer_test.kso`
- wanted: a local `rows = fmath/floor (...)` in camera.kso; a parameter
  `span` in sphere.kso; a test constant `target`; a binding `material` in
  `fn hit (sphere center material radius)`; a type `hit` beside a function
  `hit`.
- wrote: six renames, each forced by a declaration in a different file:
  `rows` became `tall` because render.kso has `fn rows`; the test constant
  `target` became `ball` because camera.kso binds `target`, and then
  `unit_ball` because scenes.kso binds `ball`; the constructor `span` became
  `bounds` because sphere.kso takes a parameter `span`; the pattern binding
  `material` became `mat` because material.kso declares the typeset
  `material`; the record `hit` became `hit_record` because of `fn hit`.
  The message each time is "`rows` is already a declaration; rename the
  binding", pointing at the local, not at the declaration that took it.
- why it matters: a module is one namespace across its files and nothing
  may shadow, so adding a helper in one file can break a function body in
  another that the author never opened. Test files count too, so a test
  constant can break production code. Twice the rename I chose collided
  again. The message would be much cheaper to act on if it named the file
  and line of the declaration that owns the name.

### F10: `none` cannot seed a fold; the message offers a fix that is not mine to make
- kind: diagnostic
- severity: minor
- where: `tracer/sphere.kso` (`fn hit (group objects)`)
- wanted: `list/fold objects none (&closer r span)`, the nearest hit so far
  starting as none.
- wrote: a marker `type miss` and `list/fold objects miss (...)`. The
  message was "this can be a none and `list/fold` has no arm for it —
  resolve it here, or give `list/fold` a `none` arm".
- why it matters: the marker reads better than none here, and chapter 04
  argues for exactly that, so the design is defensible. The message is not:
  it suggests adding an arm to a standard-library function.

### F11: `math/round` can answer none, so every floor is fallible, and the checker is satisfied by naming the call
- kind: confusing-semantics
- severity: major
- where: `fmath/fmath.kso` (`floor`, `settle`), `tracer/camera.kso:38`,
  `fmath/fmath_test.kso` (`floors_to?`, `same?`)
- wanted: `height = fmath/max_of 1 (fmath/floor (q.width / aspect))`, and
  `test_floor_whole = floor 3.0 == 3`.
- wrote: `floor` turns round's none into an err. Then the camera line was
  refused ("this can be an err and `fmath/max_of` has no arm for it"), and
  the accepted spelling is
  ```
  tall = fmath/floor (q.width / aspect)
  height = fmath/max_of 1 tall
  ```
  which passes because the checker reads calls and not names. The tests
  need a two-arm `same?` helper because `==` will not take a possible err.
- why it matters: NaN is the only reason a floor can fail, and the image
  height is never NaN. The check costs a helper in the tests and, in the
  program, a rewrite that silences it without handling anything. A rule
  that is satisfied by moving an expression onto its own line teaches
  people to move expressions onto their own lines. A `math/floor` that
  answered an int for finite input, or a `floor!` that insisted, would
  remove this whole family.

### F12: a list literal wider than 80 columns cannot be written
- kind: engine-bug
- severity: major
- where: `bugs/b05_wide_list_literal.kso`, `tracer/scenes.kso`
  (`three_materials`)
- wanted: the chapter 11 scene as one literal, one sphere per line:
  ```
  three_materials = [
    (sphere (vec/new 0.0 -100.5 -1.0) (lambertian yellow) 100.0)
    (sphere (vec/new 0.0 0.0 -1.2) (lambertian navy) 0.5)
    ...
  ]
  ```
- wrote: a block that binds each sphere to a name and ends in
  `[ground center left bubble right]`.
- why it matters: a scene is a list of objects, and a list longer than one
  line is the normal case. The one-per-line form is "expected an
  expression" when it is needed, and "needless continuation" when it is
  not, so the second message advertises a form the parser does not accept.
  Elements are juxtaposed, so each call must also be parenthesised, which
  the named-binding form happens to avoid.

### F13: no record update, so changing one field restates all of them
- kind: missing-feature
- severity: major
- where: `cli/cli.kso` (`set`, six arms), `tracer/scenes.kso` (four
  `view` constructions), `scenefile/scenefile.kso` (`set_camera`, seven
  arms)
- wanted: `opts with { width: n }`, or C++'s `cam.vfov = 20;` against a
  defaults object, which is how every chapter of the original configures
  its camera.
- wrote:
  ```
  fn set (options d gr sa sc _ sh w) "--seed" n
    options d gr sa sc n sh w
  ```
  once per flag, and every scene spells out all seven camera fields
  positionally even when it changes one of them from the default.
- why it matters: each new option is an edit to every `set` arm. Adding
  `--shade` meant a seventh field, so all five existing `set` arms were
  rewritten, plus `defaults`, the destructure in `launch`, and the
  expected record in `cli_test.kso`: eight edits for one flag. A
  transposed pair of same-typed positional arguments (`focus_dist` and
  `vfov` are both floats) compiles and renders the wrong picture.

### F14: alphabetical fields fix the constructor's argument order, and it is often backwards
- kind: aesthetics
- severity: minor
- where: `tracer/ray.kso` (`interval`, `bounds`), `tracer/scenes.kso`
  (`default_view`)
- wanted: `interval 0.001 infinity` (min first, as everyone writes an
  interval), and a camera built with its fields labelled.
- wrote: `type interval` has fields `max` and `min`, so the constructor
  takes the upper bound first; a `fn bounds lo hi` exists only to swap
  them. For the camera I used the indented-argument form with a comment on
  each line naming the field, which the compiler accepted:
  ```
  default_view = view
    (16.0 / 9.0)              # aspect
    0.0                       # defocus_angle
    ...
  ```
- why it matters: the order of a constructor is part of its meaning for
  intervals, ranges and coordinates. This is aesthetic in that the program
  is correct either way, but `interval max min` is a bug waiting for the
  first reader who assumes the usual order.

### F15: a pattern naming a type that does not exist is accepted
- kind: diagnostic
- severity: major
- where: `bugs/b04_undeclared_type_in_pattern/`
- wanted: `pub fn area (circel r)` refused with "no type is called
  `circel`".
- wrote: nothing; I found it when `render` in `tracer/render.kso` named
  `(scene v world)` before `scene` existed and `kanso check` said ok.
- why it matters: dispatch is the only switch, so a misspelled type in an
  arm is a case that silently never matches. In an entry file the call
  site then says "arms take nothing_here", which is the only hint.

### F16: `kanso check .` inside a module cannot see its siblings
- kind: tooling
- severity: minor
- where: `bugs/b03_check_dot_loses_siblings/`
- wanted: `cd tracer && kanso check .`
- wrote: `kanso check tracer` from the parent.
- why it matters: checking the directory you are editing in is the first
  thing an editor integration does.

### F17: a failing test says only "returned false"
- kind: tooling
- severity: minor
- where: `tracer/tracer_test.kso` (three tests failed on first run)
- wanted: the two sides of the comparison, as any assertion library
  prints them.
- wrote: a throwaway `test_debug` whose value is a string; the runner
  prints a non-boolean result in full, which is how I found that the first
  Park-Miller draw from seed 1 is 0.0000225 and my test's expectation was
  wrong.
- why it matters: all three failures were wrong expectations, and each one
  needed a scratch test to see what the code had computed. The non-boolean
  trick works and is worth documenting; a failing `a == b` could print `a`
  and `b` by itself.

### F18: an operator arm does not count as using the import that declares it
- kind: confusing-semantics
- severity: minor
- where: `tracer/ray.kso` (`at`)
- wanted: `import "../vec"` at the top of ray.kso, because `at` adds and
  scales vec3 values with vec's `+` and `*` arms.
- wrote: removed the import after "unused import ../vec". The arms apply
  anyway.
- why it matters: a reader looking for where `+` on a vec3 comes from finds
  no import; the arms are ambient. That is the documented design ("an
  operator is an ambient dispatch group"), but it means a file's imports
  no longer list what the file depends on.

### F19: a long single expression must be inline, and inline it is too long
- kind: aesthetics
- severity: nit
- where: `tracer/tracer_test.kso` (`test_sky_is_pale_at_the_horizon`)
- wanted:
  ```
  test_sky_is_white_at_the_horizon =
    sky (ray (vec/new 1.0 0.0 0.0) vec/zero) == vec/new 0.75 0.85 1.0
  ```
- wrote: "a single-expression constant is written inline", so I invented a
  local, `level = ray (...)`, to make it two statements.
- why it matters: the formatting rules leave exactly one way to write it,
  and that way needs a name nobody wanted. A related rule refused
  `(2.0 * k) * (2.0 * k + 1.0)` as "parentheses group nothing", which is
  true and which I would still have written for the reader.

### F20: the only random numbers are an effect, so a pure renderer threads its own generator through everything
- kind: stdlib-gap
- severity: major
- where: `rng/rng.kso`, and 40 `.gen` / `rng/paired` sites across
  `tracer/camera.kso`, `material.kso`, `render.kso` and `scenes.kso`
- wanted: C++'s `random_double()` deep inside `scatter`, or a pure
  generator value in std/math to draw from.
- wrote: a Park-Miller generator in its own module, where every draw
  answers a `drawn` record of the value and the next generator:
  ```
  dx = rng/between g -0.5 0.5
  dy = rng/between dx.gen -0.5 0.5
  ...
  lens = lens_point cam dy.gen
  rng/paired lens.gen (ray (target - lens.value) lens.value)
  ```
  Every function on the path from the pixel loop to `scatter` takes a
  generator and answers a pair, including `ray_color`, `follow`, `shade`,
  `sample`, `row`, `get_ray` and the scene builders.
- why it matters: `math/random` is an effect (chapter 6), so using it in
  `scatter` makes `scatter`, `ray_color` and everything above them answer an
  io. I compiled that version of the random unit vector to compare,
  in `probes/fxvec.kso`:
  ```
  coord 0 .> (x -> coord 0 .> (y -> coord 0 .> (z -> on_sphere x y z)))
  ```
  It runs, but the geometry would then be effects, and a test can no
  longer compare a scattered ray with `==`. The hand-threaded version keeps
  everything pure and testable, at the price of one line per draw and the
  constant chance of passing the old generator where the new one belonged,
  which nothing catches: two draws from the same `g` give the same number.
  `math/random` also yields only ints, so a float in [0, 1) is a division
  either way.

### F21: vector arithmetic on records costs about 60 times what C pays
- kind: performance
- severity: major
- where: `bench/run.sh` (the loops below), `build/release/main`
- wanted: the book's final render, 1200x675 at 500 samples, in the hours
  the C++ takes rather than days.
- wrote: nothing different; I measured. In a release build, ten million
  iterations of `v + vec/new 0.5 0.25 0.125` take 1.01 s, and the same loop
  in C (`clang -O2`, a struct of three doubles) takes 0.016 s. Ten million
  iterations of three scalar float additions take 0.23 s. The cover scene
  (485 spheres, depth 50) renders 22,400 samples in 12.1 s, 0.54 ms per
  sample, which puts the book's final image at roughly 61 hours. The
  machine was shared with other jobs, so the absolute times move between
  runs (a later run of `bench/run.sh` under load gave 2.32 s against
  0.030 s); the ratio stayed between 60 and 80.
- why it matters: a ray tracer is mostly three-float records, and here each
  one is an allocation. The fixtures are sized for the interpreter's sake
  anyway, but the native binary is not yet a substitute for the C++ on the
  real image.

### F22: a field read on a value that may be a marker passes the checker
- kind: diagnostic
- severity: minor
- where: `bugs/b06_field_read_on_marker/`, `tracer/tracer_test.kso`
  (`test_group_picks_the_nearest` reads `found.t` from `hit`, which may be
  `miss`)
- wanted: the same refusal the checker gives for a possible none or err.
- wrote: dispatch on `miss` in the renderer (`fn shade miss ...`); in tests
  I read `.t` directly and rely on the test failing if it ever sees a miss.
- why it matters: chapter 04 recommends a marker over none for an expected
  "no answer", and then the checker protects the none and not the marker.
  The failure is "error[runtime]: `probe/miss` has no field `t`".

### F23: unused private functions are not flagged
- kind: diagnostic
- severity: nit
- where: `tracer/ray.kso` (a `contains?` I ported from the original's
  `interval` and never called), `probes/p16/`
- wanted: "unused function `contains?`", in the same spirit as the unused
  binding and unused import errors.
- wrote: deleted it after noticing by reading.
- why it matters: chapter 01 sells "clutter is an error"; an unused
  binding is refused, an unused import is refused, and an unused private
  function of any length is silent.

### F24: `kanso check` will not check a `kanso play` file
- kind: tooling
- severity: nit
- where: `probes/p12b.kso`
- wanted: `kanso check p12b.kso` on a file of declarations and statements.
- wrote: `kanso play` it and read the run-time error. `check` says "a
  file with declarations is a library, and a library has no statements to
  run — `kanso play` runs declarations beside statements in one file".
- why it matters: the verb that exists to say "is this legal?" refuses the
  smallest legal program shape, the one used for every experiment.

### F25: the release build fails every scene with a bounce once an unused shading arm exists
- kind: engine-bug
- severity: blocker
- where: `bugs/b08_release_bounce/`, `tracer/render.kso` (`surface
  hemisphere`)
- wanted: chapter 9's uniform-hemisphere model as one more arm of
  `surface`:
  ```
  fn surface hemisphere rec _ depth world g
    d = rng/on_hemisphere g rec.normal
    c = ray_color hemisphere (ray d.value rec.p) (depth - 1) world d.gen
    rng/paired c.gen (c.value * 0.5)
  ```
- wrote: `outgoing = ray (d.value + vec/zero) rec.p`. With the arm as
  wanted, `kanso build --release` produced a binary that stopped every
  scene with bounces (not only hemisphere ones; the default material
  model too) after a few rows with "error[runtime]: no overload of `-`
  matches these arguments". The interpreter and the dev-tier binary were
  right. Taking `d.value` straight from the generator's record into the
  ray is the trigger; `d.value * 1.0` still fails and `d.value +
  vec/zero` does not.
  A greedy reduction by whole declarations got the program down to 520
  lines (bugs/b08) and stopped there; a 90-line model of the same shape
  does not fail.
- why it matters: this was the one engine disagreement in the port, it hit
  code that never ran, and it only showed because check.sh runs all three
  engines. A program tested on the interpreter and shipped with
  `--release` would have failed in the field.

### F26: the checker tracks a none through record fields across modules and reports it at the far end
- kind: diagnostic
- severity: major
- where: `cli/cli.kso` (`defaults`), `tracer/render.kso:68`
- wanted: `options 50 11 100 name 1 (tracer/shading_named "materials") 400`
  as the default options.
- wrote: a `pub full_shading = materials` constant in tracer for the
  default. The original line was refused with "this can be a none and
  `tracer/ray_color` has no arm for it ... --> ./tracer/render.kso:68:20",
  a file I had not touched. `shading_named` can answer none for an unknown
  name; the none rode through `options.shade`, `tracer/run`, `quality.mode`
  and `camera.mode` to the call in `sample`.
- why it matters: the inference is impressive, and it found a real hole.
  The report names the last link of a five-link chain, in another module,
  and suggests giving `ray_color` a none arm, which is the wrong fix. The
  first link was the one to name.

### F27: ambient builtin names are reserved as locals too
- kind: refactoring-hazard
- severity: minor
- where: `scenefile/scenefile.kso` (the parameter `rest`)
- wanted: a parameter `values` for the numbers on a directive line.
- wrote: `rest`. All fourteen uses were refused with "`values` is already a
  declaration; rename the binding" — `values` is the ambient function that
  lists a map's values.
- why it matters: the same rule as F9, applied to the eight ambient names
  (`values`, `keys`, `entries`, `length`, `push`, `put`, `print`, `if`).
  `values`, `keys` and `length` are some of the most natural local names
  there are.

### F28: a parse error wants the err railway, and the checker wants an err arm on every function it passes through
- kind: confusing-semantics
- severity: major
- where: `scenefile/scenefile.kso` (whole file), first draft in this
  session refused with 24 errors
- wanted: helpers that fail with `err "line {n}: {reason}"` and callers
  that state only the happy path, as chapter 04 promises ("functions state
  their happy-path logic and nothing else").
- wrote: the same code, but with every fallible call bound to a name before
  it is passed on:
  ```
  albedo = triple rest n
  tracer/new_lambertian albedo
  ```
  rather than `tracer/new_lambertian (triple rest n)`, which is refused
  because `triple` can answer an err and `new_lambertian` has no `(err _)`
  arm. Index reads needed a `return ... if length xs < 3` guard in the same
  function before `xs[1]`, `xs[2]` and `xs[3]` were accepted; the guard in
  the function that built the list did not count.
- why it matters: the railway works, and the error messages it produced are
  good ("line 4: big is not a number"). But the checker only looks at
  direct calls, so the way to write a railway that compiles is to name
  every intermediate result, and a reader cannot tell whether a binding
  exists for clarity or to step around the checker. The alternative, a
  marker type for the failure, needs a bad-input arm on every function in
  the chain, which is the boilerplate the railway exists to remove.

### F29: two arms that each win somewhere need a third arm for the overlap
- kind: confusing-semantics
- severity: minor
- where: `cli/cli.kso` (`step`)
- wanted:
  ```
  fn step opts "--shade" raw     # --shade takes a word, not a number
  fn step _ flag none            # any flag with no value after it
  fn step opts flag raw
  ```
- wrote: an extra `fn step _ "--shade" none` above them. Without it: "these
  `step` arms tie: each is the more specific one somewhere, and a call
  could match both".
- why it matters: the rule is sound, and the message says what to do. It
  is the price of having no `match` with ordered cases: an overlap two arms
  agree on still needs its own arm, and with three parameters the
  overlaps multiply.

### F30: no `ends_with?` in std/text, and 1-based inclusive slices invite the off-by-one
- kind: stdlib-gap
- severity: minor
- where: `cli/cli.kso` (`file?`)
- wanted: `text/ends_with? name ".scene"`
- wrote: `text/slice name (n - 5) n == ".scene"`, after a first version
  compared the last six characters to `"scene"` and every scene file was
  reported as "no scene called scenes/three.scene".
- why it matters: prefix and suffix tests are among the first string
  functions any CLI needs.

### F31: an advisory about a type crossing two modules, on a design that is fine
- kind: diagnostic
- severity: nit
- where: `scenefile/scenefile.kso` (`parse`)
- wanted: silence; `scenefile/parse` answers a `tracer/scene`, and the
  caller passes it to `tracer/run_scene`.
- wrote: nothing. Every check prints "advisory[door]: `parse` returns
  `tracer/scene` and the surface offers nothing that accepts it —
  re-export what callers need, or wrap it".
- why it matters: the caller imports tracer anyway. An advisory that fires
  on correct code on every check gets ignored, and then so does the next
  one.

## What worked well

**Operator arms made the vector code read like the C++.** Five short arms
in `vec/vec.kso` give vec3 `+`, `-`, `/` and `*` both ways round with a
scalar, and from then on the geometry is written the way the book writes
it:
```
corner = lookfrom - w * focus - u * viewport_w / 2.0 + v * viewport_h / 2.0
```
`pub fn * s (vec3 x y z)` beside `pub fn * (vec3 x y z) s` was accepted
as it stands; the checker worked out that neither arm overlaps the other.

**Dispatch is the right shape for materials.** The original's abstract
`material` class with a virtual `scatter` became three arms of one function,
each unpacking its own record (`fn scatter (metal albedo fuzz) r_in rec g`),
and the two possible outcomes became two records, `scattered` and
`absorbed`, that `follow` dispatches on. When I added the book's earlier
shading models, each was one more marker and one more arm of `surface`, and
nothing else changed. Scene lookup (`fn scene_named "cover" g grid`) and
the flag table (`fn set (...) "--depth" n`) are the same idea and read as
tables.

**The three engines agreed to the byte.** Integer-exact Park-Miller,
IEEE floats and the same order of operations everywhere meant 23 fixtures,
images and error transcripts alike, matched on the interpreter, the dev
binary and the release binary on the first run of check.sh, until F25. The
interpreter was fast enough to make the oracle practical: a million
recursive calls in 0.37 s, and every fixture image in under 8 s.

**Pure code with structural `==` made the renderer easy to test.** A test
builds a hit record by hand and compares whole results:
```
out = scatter (metal grey 0.0) skim rec g0
out == absorbed (rng/unit_vector g0).gen
```
There is no mock and no tolerance helper for records; a ray, a hit and a
scattered result are values. Threading the generator (F20) is what kept
this possible, so that cost bought something.

**Moving code between files is free.** Splitting `interval` out of
`ray.kso` into its own file needed no edit anywhere else, because a module
is one namespace. (F9 is the other side of the same coin.)

**The checker found real things.** Adding a `mode` field to `quality`
turned every stale positional construction, including two in a test file,
into "`quality` has 4 field(s), got 3". The none-tracking of F26, for all
its bad aim, found a real path from an unknown `--shade` word to a crash. A
`return ... if length xs < 3` guard is understood, so `xs[1]` after it needs
no ceremony.

**Small things that helped.** `&closer r span` as a fold callback, with no
lambda. Indented arguments with a comment on each line as a field label.
`os/read_file` answering `file_not_found` as data, so a missing scene file
is one arm and a one-line message. Error messages that state the fix:
"comparing to `false` asks a question the value already answers — write
`not` the value", "these parentheses group nothing". A test whose value is
not a boolean prints that value in full, which works as a debugger.

## Summary

The five I would fix first, in order:

1. **F25, the release miscompile.** A correct program, right on the
   interpreter and the dev tier, failed in every bouncing scene once built
   with `--release`, triggered by an arm that never ran. Nothing else on
   this list can lose a user's trust as fast.
2. **F9 and F27, one namespace with no shadowing.** Eight forced renames,
   each caused by a declaration in a different file (a test file twice, an
   ambient builtin once). At minimum the message should name the
   declaration that owns the name; better, a local could shadow a name
   declared in another file.
3. **F6 and F7, records that cross an import.** A `pub type` cannot be
   built or taken apart positionally outside its module, and the keyed
   read cannot name every field, so an importer has no destructuring form
   for a record it needs all of, and every exporting module grows `new_`
   functions (`vec/new`, `rng/paired`, the whole of `tracer/build.kso`).
   The keyed read also passes `check` and fails at run time (bugs/b02).
4. **F11 and F28, a checker that is satisfied by a name.** The none and
   err checks read calls and not names, so the accepted way to pass a
   possibly-failing value along is to bind it first. That makes every
   such binding ambiguous to a reader, and it made a NaN-only failure in
   `floor` cost a helper in every test that touches it.
5. **F5, F3 and F20, the numeric floor.** No `floor`, `tan`, `pi`, `abs`,
   `min`, infinity or exponent literals, and random numbers only as an
   effect. Each is small; together they are a module of hand-written
   numerics and a generator threaded through forty call sites before the
   first pixel.

Next after those: F13 (no record update, which made the flag table restate
seven fields per flag) and F21 (vector records at about 60 times C's cost).

Writing a ray tracer in kanso was pleasant where the book's design already
leans functional and awkward where it leans on mutation. The geometry and
the materials were the best part: operator arms let the vector maths read
like the original, materials became dispatch arms that I could test by
comparing whole records, and adding the book's earlier shading models was
one marker and one arm each. The hard parts were all about plumbing. The
random generator had to go through every function between the pixel loop
and `scatter`, because the language's own random numbers are effects. The
camera and command-line options, which the C++ sets a field at a time,
became positional records rebuilt in full. And module boundaries cost more
than they should: names collide across files, imported records can only be
read, and the checker's none and err rules are satisfied by moving an
expression onto its own line. Determinism was the language's strongest
showing. With a pure generator and IEEE floats, 23 fixtures matched across
three engines byte for byte, and the one time they did not, the fault was
the compiler's, which check.sh caught on its first run.
