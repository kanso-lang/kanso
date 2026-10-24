# Re-sit: lox

## Result

check.sh: fail. 27 of 28 fixtures pass on all three engines, and the unit
tests pass (44 of 44). `binary_trees` fails on dev and release: both native
builds run for more than twelve minutes on a fixture the interpreter
finishes in about a second. The check was stopped there by killing the two
runs after three minutes each (`resit/check.log`). This is a regression:
the fixture passed on 2026-10-10. Reduced in `bugs/nested_beat_carry_copies_store.kso` (B1).

Engines agree: yes on the 27 that finish; on `binary_trees` the native
builds never finish.

Lines changed by the migration: 30 (`diff -r` against ports-orig/lox,
counting `<` and `>` lines, `resit/`, `bugs/` and build output left out).

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c` `` (#1887) | yes, with the spelling | 1 (scanner `alpha?`) |
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906) | yes. The file also needed `import "std/list"`, which the message does not mention | 1 |
| ``error[exhaustive]: this can be a none and `text/join` has no arm for it`` (#1890) | partly. It points at the call; the cause is a slice that cannot miss. One `sliced none fallback` helper | 3 |
| ``error[name]: unknown name `vacant` `` in cells_test.kso (a test is an outside consumer, #1883) | no. The names came one per run | 4: markers `vacant`, `nil` and type `normal` made pub; `local_get 1 tok 1` in resolver_test rewritten as three field reads, because a test may not build the module's record |
| none: `binary_trees` hangs natively | no diagnostic | not fixable in the port (B1) |

No expected-output file changed.

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a single file of definitions and statements will not `run`, and the book says it will | confusing-semantics | minor | tooling | still | the book's `samples/appb/input.kso` under `kanso run`: ``is a library — nothing to run. it exports `play` ``. `kanso check` and `kanso build` both refuse a play-shaped file too (`check` with `error[syntax]: a file with declarations is a library`), so a reduction can only be timed natively through `kanso play` |
| F2 | there is no exponent float literal, and the error does not say so | diagnostic | nit | language | fixed | `resit/f2_literals.kso`: `1e21`, `1.5e-3`, `0xff` print `1.0e+21 0.0015 255 true` (#1889). The renderer's own `1.0e+21` reads back (`resit/f2_readback.kso`) |
| F3 | native `push` inside a helper function is quadratic | performance | major | core-lib | fixed | `resit/f3_push.kso`, 100,000 pushes through `bump`: 0.06 s native (journal: 40,000 took more than 20 s) |
| F4 | a native map is a log, so it cannot serve as a mutable store | performance | major | core-lib | fixed | `resit/f4_put.kso`, 100,000 overwrites of key 7 from a helper: 0.07 s; string key (`f4_put_str.kso`): 0.08 s |
| F5 | a persistent tree updated in a loop falls off a cliff natively | performance | major | language | fixed | `resit/f5_trie.kso` (the journal's bug): 70,000 updates 0.39 s, 80,000 0.42 s, 160,000 0.80 s. A re-entrant loop over the same kind of tree is the new cliff (B1) |
| F6 | a collection literal cannot span lines | missing-feature | minor | language | fixed | `resit/f6_map.kso`: the keyword map in spanning form prints `fun 11` on both engines (#1882) |
| F7 | parentheses that only clarify are an error | aesthetics | nit | language | partly | the wanted `("a" <= ch and ch <= "z") or (...)` is now the required spelling (#1887), and the migration wrote it. `not (digit? "1")` is still refused (`resit/f7_not.kso`), as the same gavel says |
| F8 | a catch-all `_` does not catch none | confusing-semantics | minor | language | ruled-no | `resit/f8_none.kso` still refused. The July exhaustiveness gavel holds that a catch-all binding is not a stated disposition for none (archive, 2026-07-24, "the first exhaustiveness arm, and a zero that was a lie") |
| F9 | an import in a test file renames a type in another file | refactoring-hazard | major | language | fixed | `resit/f9`: a private `type cursor` and `fn grown` beside `import "std/list"` in both the module and its test; the test passes and the program prints `1 6` (#1883; log 2026-10-18, "A module's own type beats an imported type's twin") |
| F10 | no-shadowing reaches across files, so a new type breaks old code elsewhere | refactoring-hazard | major | language | still | `resit/f10`: `type literal` in ast.kso and a parameter `literal` in scan.kso: ``error[name]: `literal` is already a declaration; rename the binding`` |
| F11 | a record constructor merges the same err with itself | confusing-semantics | major | language | still | `resit/f11`: `failed: [m/fault "expected x" m/fault "expected x"]` on both engines |
| F12 | a comment cannot stand on its own | aesthetics | nit | language | partly | `resit/f12`: a header comment and one blank line is accepted (#1887, second part). A `# --- statements ---` divider between declarations is still refused with `exactly one blank line separates top-level declarations`; the gavel says nothing else about comment placement changes |
| F13 | 80 columns and no string continuation make expected-output tests awkward | aesthetics | minor | language | partly | a long list now spans lines (#1882), and a `"""` block holds long text. A long `==` comparison still has to be broken by naming parts, which #1882 rules out of scope |
| F14 | two private helpers with one name in two files silently become one function | refactoring-hazard | major | language | still | `resit/f14`: `fn if_else cond (parsed n)` in parser.kso and `fn if_else cond (resolved s)` in resolver.kso check clean and dispatch as one group (`parser 1 p`, `resolver 2 r`), with no message |
| F15 | indexing a list in a loop makes the checker demand none arms | confusing-semantics | minor | language | still | the guard is still not read (mal's `resit/f7_index.kso`). Binding the read to a name still silences the check (`resit/f15_bound.kso` prints `int 2`), though #1884 made the same binding carry an err (N3) |
| F16 | there is no record update, so one changed field rewrites them all | missing-feature | major | language | fixed | `merge w { "out":[] }` (#1886), shown in mal's `resit/f10_merge.kso` |
| F17 | without exceptions or early return, a parser is a chain of continuations with a pass-through arm on each | missing-feature | major | language | still | mal's `resit/f3_inline.kso` and `f3_named.kso`: an err reaching an arm-less receiver is refused whether inlined or named. The arms are required by the 2026-09-15 ruling "a bare err halts where it lands", and #1884 closed the name route the journal mentions. No `?` operator exists |
| F18 | a long-running loop never gives memory back on the native engines | performance | blocker | language | partly | memory is fixed: the journal's Point loop, release build with counters, 2,000 passes: `arena_peak_bytes=2097152`. Time is now the blocker: 500 passes 0.56 s, 1,000 2.7 s, 2,000 14.5 s, 4,000 more than 60 s, and 20,000 more than 120 s, where the interpreter takes 1.5 s and 16 s. Same mechanism as B1 |
| F19 | a dispatch tie goes unreported once a third arm is added, and the engines then disagree | engine-bug | blocker | language | fixed | `bugs/dispatch_tie_unreported_engines_disagree.kso`: ``error[dispatch]: these `pick` arms tie`` |
| F20 | a type constructor passed as a function value checks and interprets, but will not build | engine-bug | major | language | fixed | `bugs/constructor_as_value_native.kso` and the module form `resit/f20` print the record on both engines |
| F21 | running out of stack is a clean error on one engine and a segfault on the others | engine-bug | major | language | partly | `max_calls` raised to 100,000 in a copy (`resit/lb`): f(20000) gives `error[runtime]: the program ran out of stack` on interp, dev and release, no segfault. The test runner reports the case and carries on (`resit/f21`: `test_deep ... FAILED`, `1 passed, 1 failed`, exit 1). The depth still differs: f(3000) prints 3000 on release and runs out on dev and the interpreter |
| F22 | identity has to be built by hand | missing-feature | minor | language | still | no identity primitive in appendix B; equality is structural (ch02). No ruling found in the log |
| F23 | a pure evaluator cannot read the clock, so Lox's `clock()` is frozen | confusing-semantics | minor | language | still | unchanged design: effects are descriptions run after evaluation. No ruling found |
| F24 | there is no way to read standard input a line at a time | stdlib-gap | minor | core-lib (basic io) | fixed | `io/read_line` (#1914); mal's `resit/f12` answers two lines two seconds apart |
| F25 | native stdout is buffered past a stderr write | engine-bug | minor | language | fixed | `bugs/stdout_after_stderr_native.kso` with `2>&1`: stdout line first on both engines (log 2026-10-12, "stdout reaches its reader") |
| F26 | a constant must be one line, and a line must be 80 columns | aesthetics | nit | language | ruled-no | #1882 keeps operator chains out of the spanning form; mal's `resit/f15` still gets `a single-expression constant is written inline` |
| F27 | builtin and ambient names are reserved, and you learn which by colliding | diagnostic | nit | language | still | `resit/f27_*.kso`: ``the name `bind` is already taken``, ``the name `rescue` is already taken``, ``` `values` is already a declaration``` and ``` `done` is already a declaration```, none saying what took the name |
| F28 | a comparison chain gets "unexpected trailing tokens" | diagnostic | nit | language | still | `resit/f28_chain.kso`: `error[syntax]: unexpected trailing tokens` at `!=` |
| F29 | a printed record does not show its own structure | tooling | minor | core-lib | still | `resit/f29_render.kso`: `[f29_render/define f29_render/literal 1.0 f29_render/token "identifier" "a" 1 1 ...]`, flat, with no parentheses |
| F30 | pushing onto a list held in a record is quadratic on every engine | performance | major | core-lib | fixed | `bugs/record_field_push_quadratic.kso` at 40,000: 0.07 s native, 0.06 s interp |
| F31 | on the interpreter, slicing a string costs the length of the whole string | performance | minor | core-lib | still | `resit/f31_where.kso`, 20,000 three-character slices at position 1: 0.04 s on a 700-character string, 3.0 s on a 147,000-character one, with `--interp`; native 0.06 s |

Counts: fixed 12, partly 5, still 12, ruled-no 2, moot 0, worse 0. F18
is counted as partly. Its memory half is fixed, but the program it
describes still cannot run 200,000 passes natively, now for time.

## New friction

### N1: a loop re-entered from its own lap copies the carried store every lap
- kind: performance
- severity: blocker
- scope: language
- where: lox/interpreter.kso `run_from`; tests/binary_trees.lox;
  `bugs/nested_beat_carry_copies_store.kso`
- wanted: the interpreter as it ran on 2026-10-10, where binary_trees took
  under a second natively
- wrote: nothing; the port cannot avoid it
- why it matters: the change that fixed F18's memory makes a statement loop
  that calls a Lox function (which runs the statement loop again) pay for
  the whole store on every lap. Native time grows about fourfold per
  doubling of the heap. binary_trees passed before and now runs for more
  than twelve minutes natively. The Point loop from F18 takes 14.5 s for
  2,000 passes against 1.5 s on the interpreter. gdb samples are all in
  `k_beat_iter_carry` → `k_deep_copy` / `k_copy_size` →
  `k_interior_survives`. A flat loop carrying the same tree stays linear.

### N2: a test sees only pub names, so private markers became public
- kind: refactoring-hazard
- severity: minor
- scope: language
- where: lox/cells.kso `vacant`, lox/ast.kso `nil`, lox/interpreter.kso
  `normal`, lox/resolver_test.kso
- wanted: the tests as written
- wrote: three `pub` markings, and `inner.expression == local_get 1 tok 1`
  replaced by `got.depth == 1 and got.name == a and got.slot == 1`, because a
  test may read a record's fields but not build one
- why it matters: the ruling (#1883) is deliberate. Its cost is that a
  marker the module uses internally (`vacant` for an empty trie slot)
  becomes part of the module's public surface so that a test can compare
  against it. Each missing name was reported in a separate run.

### N3: a name bound to an index still hides its none, though it no longer hides an err
- kind: confusing-semantics
- severity: minor
- scope: language
- where: `resit/f15_bound.kso`
- wanted: one rule for both failure values
- wrote:
  ```
  item = items[i]
  shown item
  ```
  which compiles, where `shown items[i]` is refused for its none
- why it matters: #1884 made a local or constant bound to a raising call
  carry its err to every later use. The same binding still drops a none.
  F15's workaround therefore still works, and it is the spelling the
  checker cannot see through.

### N4: a stack overflow names a line, though the ruling says it does not
- kind: diagnostic
- severity: nit
- scope: language
- where: `resit/lb` with `max_calls = 100000`, `deep.lox` at depth 20,000
- wanted: what the 2026-10-17 gavel on #1987 describes: the overflow
  "keeps its sentence with no line"
- wrote: nothing; every engine prints
  `failed in lox/run_from at lox/interpreter.kso:190` under the sentence
- why it matters: the gavel dropped the overflow's place because engines run
  out at different depths and could name different places. Here all three
  name the same line, so nothing diverges, but the behaviour and the ruling
  disagree.

## Bugs

- **B1** `bugs/nested_beat_carry_copies_store.kso`: a self-recursive loop
  whose lap re-enters the same loop, carrying a persistent binary trie that
  each lap writes one leaf into. `kanso play` natively: 1,022 writes 0.07 s,
  2,046 writes 1.5 s, 4,094 writes 7.0 s, 8,190 writes more than 30 s. The
  interpreter does 16,382 writes in 0.4 s. This is what makes binary_trees
  hang and the F18 loop superlinear.

The nine bugs in the original `bugs/` are all fixed today: constructor as
value (F20), dispatch tie (F19), native stack overflow segfault and the test
runner abort (F21), the trie cliff (F5), push and put in a helper (F3, F4),
push onto a record field (F30), and stdout after stderr (F25).

## Comfort

On the language and core library, most of the journal's top five are
answered. The dispatch tie that turned `1 == 2` into `true` is refused
(F19). An import in a test can no longer rename a type (F9). `push`, `put`
and a record's list no longer go quadratic (F3, F4, F30). `merge` replaces
the field-by-field rebuilds (F16). A spanning literal holds the keyword
table (F6), and the hex and exponent literals the scanner wanted exist (F2).
What a new start would still meet is the module namespace: a parameter
cannot share a name with a type in another file (F10), and two files' helpers
merge into one group without a word (F14). The parser would still need its
66 pass-through err arms, now without the name loophole (F17). The
experience would be worse in one respect that matters most to this port: the
interpreter's memory no longer grows natively, but a loop that calls a Lox
function is now superlinear in time natively (N1, B1), so binary_trees
hangs and a 20,000-pass loop does not finish in two minutes. The journal
ended saying the native engines "cannot run it for long" because of memory;
today they cannot run it for long because of time. On the two programs
measured here, binary_trees and the Point loop, the interpreter is now the
faster engine.
