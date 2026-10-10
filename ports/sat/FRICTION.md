# FRICTION: porting minisat to kanso

A journal kept while porting a CDCL SAT solver (minisat's core) to kanso.
Every entry below was hit while writing the code in this directory, and every
code snippet was compiled with the compiler at
`/tmp/claude-0/kanso-main/kanso` (main, 2026-10-10). Timings are wall-clock
on a shared container that was running other builds at the same time, so
treat them as order-of-magnitude; the allocation counters are exact.

## Entries

### F1: a list has no indexed update
- kind: missing-feature
- severity: major
- where: solver/vec.kso (the whole module exists because of this)
- wanted: `xs = set xs i x`, or `put xs i x` the way maps take it. Every SAT
  solver is built on arrays you overwrite by position: the assignment, the
  decision levels, the reasons, the watch lists, the heap.
- wrote: `put` refuses a list at run time (`error[runtime]: put takes a map,
  a key, and a value`), so the first version used maps keyed by int. When
  those turned out to be unusable natively (F2, F3), I wrote a persistent
  vector: a tree of lists 32 wide, each write copying one list per level,
  built from this:
  ```
  fn with_slot xs i x
    return push xs x if length xs < i
    front = push (text/slice xs 1 (i - 1)) x
    text/concat front (text/slice xs (i + 1) (length xs))
  ```
  Every array in the solver goes through `slot v i` and `set_at v i x`
  (90 call sites), because indexing and `put` cannot be extended to a type
  of my own (F25).
- why it matters: the language's own advice is that values are immutable and
  the runtime reuses a uniquely-owned buffer in place. That promise is made
  for `push` and `put`, but the operation an array program performs most,
  overwriting slot i, has no builtin at all, so there is nothing for the
  runtime to make in place.

### F2: native map writes cost O(size) when the key already exists
- kind: performance
- severity: major
- where: bugs/map_replace_linear/, the solver's first version
- wanted: `churn (put m i (m[i] + 1)) ...` over a map of n int keys to cost
  the same per step whatever n is, as it does on the interpreter.
- wrote: nothing to write around it in source. 200,000 in-place updates
  (`put_mut_fast`, zero allocations) took, release build:

  | keys   | native | interpreter |
  |--------|--------|-------------|
  | 1,000  | 0.55 s | 0.32 s      |
  | 10,000 | 3.98 s | 0.69 s      |
  | 40,000 | 14.1 s | 0.34 s      |

  `k_map_replace` in src/runtime.c binary-searches the sorted view and then
  walks the pairs log backwards to find the same key, so a replace is linear
  in the map's size.
- why it matters: a map keyed by small ints is the only mutable-array
  substitute the language offers (F1), and natively it degrades with size
  while the interpreter does not. The interpreter is the faster engine here.

### F3: reading a map before writing it falls off the in-place path, by three orders of magnitude
- kind: performance
- severity: blocker
- where: bugs/bound_read_defeats_inplace_put/, solver (first version)
- wanted: a loop step that reads a map and then writes it:
  ```
  v = m[i]
  named (put m i (v + 1)) ...
  ```
- wrote: the same step with the read inline, `inline (put m i (m[i] + 1))`.
  20,000 steps over 100 keys, release build with `--counters`:

  | shape                                   | time  | alloc_bytes   |
  |-----------------------------------------|-------|---------------|
  | read inline in the put                  | 0.03 s| 8,304         |
  | read bound to a name first              | 51 s  | 8,083,537,392 |
  | read another key first (`j = m[1]`)     | 50 s  | 8,083,537,392 |
  | two maps, each read inside the other put| 53 s  | 8,082,737,696 |
  | the map inside a record, read inline    | >60 s (killed) |      |

  `held_peak_bytes` reached 6.46 GB for a 100-key map. The interpreter runs
  every shape in about 0.03 s.
  The SAT solver reads its arrays many times before each write, so every
  write took the slow path. On the first, map-based version:

  | formula             | native release | interpreter |
  |---------------------|----------------|-------------|
  | pigeonhole 5        | 9.7 s, 2.78 GB allocated | 0.60 s |
  | random 3-SAT 100/426| 117 s          | 1.9 s       |
  | pigeonhole 6        | killed by the kernel (exit 137) after 400 s | 5.7 s |

  I replaced every map with lists rebuilt around the written slot (F1),
  which the in-place machinery never touches, and native became faster than
  the interpreter again. The solver's access pattern (read two slots, write
  one), 20,000 steps over 500 slots:

  | representation            | native  | interpreter |
  |---------------------------|---------|-------------|
  | map                       | 61.9 s  | 0.55 s      |
  | list, copied on write     | 0.17 s  | 0.66 s      |
  | persistent binary tree    | 0.22 s  | 0.73 s      |
  | list of 32-slot chunks    | 0.17 s  | 0.49 s      |
- why it matters: the fast path is decided by the syntactic shape of the
  code, invisibly, and the slow path is quadratic, with memory that grows
  without bound. The exhaustiveness checker (F7) pushes you toward exactly
  the slow shape: binding `m[i]` to a name is how its "this can be a none"
  is satisfied, since the book says it reads calls and not names. Nothing
  in `check` or the book warns that the name costs a thousandfold.

### F4: the book's `pub play =` entry does not run
- kind: confusing-semantics
- severity: minor
- where: first scratch program
- wanted: what chapters 4 to 6 show, functions and a `pub play =` in one file
  run with `kanso run`.
- wrote: `kanso run` answers "`mb.kso` is a library — nothing to run", and
  `kanso play` answers "`pub play` is a library's export ... `kanso play`
  takes bare statements". What runs is bare statements at the bottom of the
  file, under `kanso play`, or an entry file importing a module directory.
- why it matters: the first program I wrote in the book's style did not
  run, and the two error messages point in opposite directions.

### F5: no float literal can say 1e100
- kind: missing-feature
- severity: minor
- where: solver/order.kso `big`, solver/db.kso `clause_big`
- wanted: `big = 1e100` and `clause_big = 1e20`, minisat's rescale limits.
- wrote: `1e100` lexes as `1` followed by a name and fails with "canonical
  form requires exactly one space here". The decimal spelling is 103
  characters, past the 80-column limit. So:
  ```
  big = ten_to 100 1.0

  fn ten_to 0 x
    x

  fn ten_to n x
    ten_to (n - 1) (x * 10.0)
  ```
  which is not even the same double as 1e100.
- why it matters: any numeric code with a large or small constant meets
  this. The diagnostic talks about spacing, not about exponents.

### F6: a list read in pure code needs a bounds guard spelled beside it
- kind: confusing-semantics
- severity: major
- where: every loop in the program, e.g. dimacs/dimacs.kso `words_from`
- wanted: `xs[i]` where `i` runs from 1 and stops at `length xs`.
- wrote: the read answers the element or none, and passing it to a function
  is refused unless a guard of exactly this shape sits above it:
  ```
  return acc if i < 1 or length xs < i
  ```
  `return acc if length xs < i` is not enough even when i starts at 1 and
  only grows; the `i < 1` half has to be spelled every time. The strict
  `xs[i]!` is no way out in pure code, because it answers an effect.
- why it matters: the guard is ceremony in every loop that reads a list,
  18 of them in this port. The lower bound check can never fire in any of
  them. I found the required shape by reading lib/list/list.kso, not the
  book.

### F7: the none check sees calls but not names or operators
- kind: confusing-semantics
- severity: major
- where: solver/watch.kso `propagate`, solver/analyze.kso `walk`
- wanted: `falsified = neg trail[qhead]`
- wrote: refused with "this can be a none and `neg` has no arm for it".
  This is accepted, and behaves the same:
  ```
  p = trail[qhead]
  falsified = neg p
  ```
  and `m[k] + 1` passes `check` (a missing key is then a runtime error,
  "`+` is not defined for these values"). The book says an operator is held
  to the same rule as a call. For maps, which have no bounds a guard can
  prove, the name is the only way through, so the program ends up with a
  binding per read. Combined with F3 that binding is also the expensive
  shape.
  It also misses a real err in an operator. The first `count_of` in
  cli/cli.kso was
  ```
  n = text/to_int word
  return n if 0 < n
  ```
  and `sat gen queens zero` printed `error[endpoint]: unhandled err reached
  the executor: ""zero" is not an integer" ... passed through list/find ←
  list/fold_go` to the user, because `0 < n` with `n` an err was accepted.
- why it matters: the check cannot be satisfied by the obvious code, is
  defeated by a rename, and does not see the operator case where a real err
  got through. It reads as noise rather than safety.

### F8: a keyed read below a `return ... if` guard does not bind
- kind: engine-bug
- severity: major
- where: bugs/keyed_read_after_guard/
- wanted:
  ```
  return 0 if n == 1
  { a } = x
  a + 1
  ```
- wrote: `error[name]: unknown name `a``. The positional form
  `box a b = x` works in the same place, and so does the keyed read above
  the guard. I hoisted every keyed read above the first guard of its
  function with a script, and hit it again twice when writing new code.
- why it matters: the keyed read is the book's recommended way to take one
  field out of a record, and guards are the recommended early exit; together
  they fail with a message that says the name does not exist.

### F9: no list patterns in parameters
- kind: missing-feature
- severity: minor
- where: dimacs/dimacs.kso `header_of`, cli/cli.kso `generated`
- wanted: `fn header_of [p cnf v c] line` and
  `fn generated "random" [vars count seed] said`
- wrote: `error[syntax]: expected a parameter pattern`; a length check and
  three single-use names per arm.
- why it matters: argument vectors and split lines are lists of known
  length, and dispatch is the language's only switch, but it cannot see
  inside a list.

### F10: a long expression cannot be wrapped at an operator
- kind: aesthetics
- severity: minor
- where: dimacs/dimacs_test.kso, nearly every file
- wanted: a test assertion on two lines, breaking before `==`.
- wrote: only `.`, `.>`, `.!`, `.?` and indented call arguments may begin a
  continuation line, so each long assertion becomes a test plus a named
  constant for the expected string. The test file has six such constants.
- why it matters: with the 80-column limit, every long line is a rename or a
  new binding; most of mine were record reconstructions (F11). A rename
  that lengthens a name breaks the limit on lines nobody touched
  (`stats` to `stat_lines` broke two in cli.kso), and a binding introduced
  only to shorten a line changes what runs (F19).

### F11: no record update, so every state change restates every field
- kind: missing-feature
- severity: major
- where: solver/order.kso, solver/trail.kso, solver/search.kso
- wanted: `{ tr | qhead: qhead + 1 }` or OCaml's `{ tr with qhead = ... }`.
- wrote: one positional reconstruction per change:
  ```
  fn advance (assignment assigns level lim qhead reason size trail)
    assignment assigns level lim (qhead + 1) reason size trail
  ```
  The solver has six record types of 4 to 7 fields. Adding two counters to
  `stats` (learnt literals, literals removed by minimization) took 12 edits
  in 3 files, every one restating all six fields in alphabetical order.
- why it matters: a new field is an edit to every construction and every
  positional pattern of the record, and the positional order is
  alphabetical, so a rename that reorders fields silently swaps two fields
  of the same type. Performance pushes the other way too (F20): a record
  that carries a growing collection through a loop is quadratic natively,
  so the hot loops thread their state as 8 separate parameters instead.

### F12: a parameter, local or type in one file collides with a name in another
- kind: refactoring-hazard
- severity: major
- where: solver/order.kso `moved`, solver/watch.kso `falsified`, cli `stats`
- wanted: a local binding `moved` in order.kso, while watch.kso defines a
  function `moved`.
- wrote: `error[name]: `moved` is already a declaration; rename the
  binding`. It happened five times: adding `pub fn falsified` to model.kso
  broke a local in watch.kso, a file I was not editing; a parameter named
  `header` collided with the type `header`; and a function `stats` collided
  with the record `stats` I had renamed it to avoid F13. Later, `placed` and
  `at` (helpers in vec.kso) broke bindings of those names in analyze.kso.
- why it matters: a module is one namespace, so a new top-level name
  anywhere can break a binding anywhere, and the error lands in the file you
  did not touch.

### F13: imported names collide with your own types and functions
- kind: diagnostic
- severity: major
- where: solver/search.kso `tally`, gen/gen.kso `step`
- wanted: a record type `tally` in a file that imports std/list, and a
  function `step` in another.
- wrote: `error[arity]: no 4-argument arm of `tally` (arms take 1)` and
  ``error[arity]: `step` has 2 field(s), got 1``. Both are std/list names
  (`list/tally`, `list/step`) that join the short-name space on import. The
  messages never say the other definition comes from std/list. Renamed to
  `stats` and `lcg_next`.
- why it matters: every pub name in every imported module is a reserved word
  you cannot see, and the diagnostic describes a shape mismatch rather than
  a collision.

### F14: a pattern on a foreign record works bare and fails qualified
- kind: diagnostic
- severity: minor
- where: cli/cli.kso `answer`, `report`, `outcome`
- wanted: `fn report (dimacs/formula clauses _ vars) opts`, which says where
  the type lives.
- wrote: ``error[opacity]: `dimacs/formula` is foreign — its structure does
  not cross an import``. The unqualified `(formula clauses _ vars)` is
  accepted and works, so the structure does cross; only the spelling the
  book recommends for imported names is refused.
- why it matters: the diagnostic states a rule that is not true, and sent
  me looking for accessor functions I did not need.

### F15: no comparator sort
- kind: stdlib-gap
- severity: minor
- where: solver/reduce.kso `sort_by`
- wanted: minisat's reduceDB sorts learnt clauses by
  `(size > 2, activity)`. `list/sort` orders only mutually comparable
  elements, and lists and records are not comparable
  (`error[runtime]: comparison requires two values of one comparable type`
  for `list/sort [[2.5 1] [1.0 7]]`).
- wrote: an 18-line merge sort taking a `before?` function.
- why it matters: sorting by a key is common enough that every other
  standard library has it.

### F16: no multi-line string or list literal
- kind: missing-feature
- severity: nit
- where: cli/cli.kso `usage`, `stat_lines`
- wanted: a four-line usage message as one literal, and a list of four
  strings across four lines.
- wrote: a string literal stops at the newline ("unterminated string"), and
  `[` on its own line is "expected an expression". So four named constants,
  joined.
- why it matters: small, but a usage string is in every command-line
  program.

### F17: list slicing and concatenation live in std/text
- kind: aesthetics
- severity: nit
- where: solver/watch.kso, solver/trail.kso, solver/vec.kso
- wanted: `list/slice`, `list/concat`.
- wrote: `import "std/text"` in six solver files that never touch text,
  for `text/slice` and `text/concat` on lists.
- why it matters: aesthetic. A reader of watch.kso sees a text import in a
  file about clauses.

### F18: a passed literal `none` is refused as a sentinel
- kind: confusing-semantics
- severity: nit
- where: dimacs/dimacs.kso, the header before `p cnf` is read
- wanted: `begin [] [] none`, meaning "no header yet".
- wrote: "this can be a none and `begin` has no arm for it", so a marker
  `type headless`. The marker reads better; noting it because the message
  suggests adding a `none` arm, which is the opposite of what the book
  recommends.
- why it matters: minor.

### F19: a binding above a guard runs even when the guard returns
- kind: confusing-semantics
- severity: major
- where: solver/watch.kso `stuck`, solver/search.kso `add_clause`
- wanted: to shorten an over-long `return` line by naming its value first:
  ```
  forced = enqueue tr first cid
  open = value_of assigns first == 0
  return visit forced cls wat ws (i + 1) kept fl props if open
  ```
- wrote: the name is evaluated whether or not the guard fires, so on the
  conflict path the solver enqueued a literal that was already false. With
  maps nothing showed; with fixed-size vectors it wrote past the end of the
  trail and failed on `queens 8` with `error[runtime]: slice takes a list
  or string`. The fix is a helper function per guarded call:
  ```
  return forcing tr cls wat ws i kept fl props cid first if open
  ```
  A scratch program confirms the interpreter evaluates it: a binding of a
  50-million-step loop above `return 1 if n < 10` takes 36 s for a call that
  returns 1.
- why it matters: the 80-column rule (F10) makes "name it on the line above"
  the standard way to wrap a long return, and that rewrite silently changes
  what the program computes on the other path, including its failures.

### F20: a collection grown inside a record is quadratic natively
- kind: performance
- severity: blocker
- where: dimacs/dimacs.kso (the first version), bugs/accumulator_in_record_quadratic/
- wanted: the reader's state as one record, `reading clauses current
  header`, with each clause pushed onto `clauses`.
- wrote: three separate loop parameters. A scratch benchmark, release
  build, pushing two-element rows:

  | rows  | bare parameter | inside a record |
  |-------|----------------|-----------------|
  | 2,000 | 0.02 s         | 0.28 s          |
  | 4,000 | 0.004 s        | 1.14 s          |
  | 8,000 | 0.007 s        | 4.37 s          |

  The interpreter does the record version of 8,000 in 0.014 s. In the port,
  reading the 1,480 clauses of 10-queens took 6.56 s natively before the
  change and 0.03 s after; the interpreter took 0.50 s either way. Solving
  12-queens went from 204 s to 1.1 s. A profile put 84% of the parse in
  `k_ten_holds_outside`, `k_deep_copy` and `k_copy_size`: the per-iteration
  evacuation copies the whole accumulator every time round the loop.
- why it matters: grouping loop state into a record is what the book's style
  suggests and what any reader wants, and natively it turns a linear loop
  quadratic with no sign in the source. The fix is the opposite of good
  structure: functions with 8 positional parameters.

### F21: no assertion, and a debugging err spreads through the program
- kind: missing-feature
- severity: major
- where: solver/vec.kso, while chasing the F19 bug
- wanted: `assert i <= size` inside `set_at`, failing with a message.
- wrote: `return err "set_at {i} past {slots v}" if slots v < i` made
  `kanso check` refuse 19 call sites across 7 files ("this can be an err
  and `scan` has no arm for it"). What I used instead was a deliberate
  runtime type error that shows a value of my choosing:
  ```
  return length (past_end i (slots v)) if slots v < i or i < 1
  ```
  which printed `length takes a list, string, or map, not solver/past_end
  65 64` and found the bug.
- why it matters: a temporary check in a leaf function should be a one-line
  edit. The railway is meant to carry a failure out untouched, but the
  checker turns a new err in a leaf into an edit at every caller.

### F22: runtime errors carry no location
- kind: diagnostic
- severity: major
- where: the F19 bug, `queens 8`
- wanted: the function and line that called `text/slice` with a non-list.
- wrote: `error[runtime]: slice takes a list or string` and nothing else,
  on the interpreter and the native build alike. Under `kanso test` the
  location is `solver:41:10`, a module, line and column with no file name;
  the module has a dozen files, so I printed line 41 of each.
- why it matters: err values carry a birthplace and a trace, which is
  excellent; the runtime errors that come from bugs, the ones you most need
  to find, carry nothing.

### F23: `kanso test` resolves some names differently from `kanso run`
- kind: engine-bug
- severity: major
- where: bugs/function_named_like_imported_type/
- wanted: functions named `grown` (in vec.kso) and `done` (in analyze.kso).
- wrote: `kanso run` called them. Under `kanso test`, `grown [] 5` built a
  `list/grown` record (the type behind `list/iterate`) and `done x` failed
  with "`<done>` is not callable": the test runner prefers a library type
  or the ambient `done` marker to the module's own function. Renamed both
  (`extended`, `learned_from`).
- why it matters: the test runner and the program disagree about what a
  name means, so tests can fail on correct code or, worse, pass on code the
  program does not run. `check` accepts both declarations.

### F24: arm order across several parameters is hard to satisfy
- kind: diagnostic
- severity: minor
- where: dimacs/dimacs.kso `literal`, `header_or`; solver/search.kso
  `add_clause`
- wanted: arms that differ in two positions, such as
  `fn literal ... (header declared variables) ws k n` beside
  `fn literal ... h ws k 0` and `fn literal ... (malformed line reason)`.
- wrote: "overloads of `literal` appear most-specific first" and, for other
  groups, "these arms tie: each is the more specific one somewhere". Each
  time the fix was to make every arm generic in all positions but one and
  destructure inside with a keyed read, or to add a `malformed?` predicate
  and a `return ... if`.
- why it matters: the tie message is accurate, but the ladder is explained
  in the book for one parameter, and with several it took trial and error
  each time.

### F25: indexing and `put` cannot be extended to your own type
- kind: missing-feature
- severity: major
- where: solver/vec.kso and its 90 call sites
- wanted: once the arrays became my own vector type, keep writing `v[i]`
  and `put v i x`, the way `+` can take an arm for a type you own
  (examples/operator_arms.kso).
- wrote: `slot v i` and `set_at v i x` at every site. The change from maps
  to vectors touched every array access in the solver, and `kanso check`
  passed while half of them were still map syntax: `put` on a vector is a
  runtime error, not a check error, so the compiler could not list the
  sites left to convert.
- why it matters: swapping a data structure's representation is a routine
  performance change, and here it is a manual edit of every use with no
  help from the checker.

### F26: the native runtime spends most of a SAT solve copying
- kind: performance
- severity: major
- where: the whole solver
- wanted: native code close to minisat, which keeps everything in place.
- wrote: the solver as it stands is 6 to 17 times faster natively than on
  the interpreter (release build: pigeonhole 6 in 0.42 s against 7.1 s;
  random 3-SAT 120/511 in 0.87 s against 5.6 s; 10-queens in 0.08 s
  against 0.80 s). A callgrind profile of pigeonhole 6, taken after the
  parser fix and before deep minimization, shows where it goes:
  `k_b_slice`, `k_deep_copy`, memmove, `k_b_concat`, `k_b_push` and
  `k_copy_size` together take 46% of instructions; `k_div` and `k_mod`
  take 14% (integer division is a call, which hurts the vector's index
  arithmetic); `k_keyed_field` and `strcmp` take 6.5%, which is keyed record
  reads looking fields up by name at run time. Today pigeonhole 6
  allocates 896 MB in 8.0 million allocations to make 18,092
  propagations. Making a watch list a cons chain, so adding a watcher is
  one cell instead of a list copy, made it slower: the per-iteration
  evacuation then copies the chain cell by cell (pigeonhole 6 went from
  0.8 s to 4.1 s, though on a different search path, 2,199 conflicts
  against 1,264, and 51 KB allocated per propagation against 43 KB).
- why it matters: the language promises in-place reuse of uniquely owned
  values, but a program whose state is several large structures read and
  written together never has a uniquely owned value at the write, so it
  pays for copies minisat never makes. The interpreter, by contrast, ran
  the map-based version fastest of all.

### F27: readability parentheses are an error
- kind: aesthetics
- severity: nit
- where: scratch benchmarks, solver/vec.kso
- wanted: `j = (k / 7) % n + 1`, to say that the division happens first.
- wrote: `error[formatting]: these parentheses group nothing`, so
  `k / 7 % n + 1`. The opposite case is real too: `bits/shr idx (5 * d - 5)
  & 31` is correct because application binds tighter than `&`, but nothing
  on the page says so.
- why it matters: aesthetic, and a small tax on numeric code, where the
  precedence of `%`, `&` and application is exactly what a reader checks.

### F28: a persistent tree loop falls off a cliff past some step count
- kind: performance
- severity: major
- where: bugs/persistent_tree_cliff/
- wanted: one of the vector candidates for F1, a binary tree of records
  rewritten along one path per step, to cost the same per step for as long
  as it runs.
- wrote: nothing; I measured it and chose lists of lists instead. Release
  build, 14 new nodes per step over a 16,384-leaf tree:

  | steps   | time   | evac_allocs   | ten_blocks |
  |---------|--------|---------------|------------|
  | 60,000  | 0.45 s | 3,577,751     | 9          |
  | 80,000  | 11 s   | 227,591,487   | 8,222      |
  | 100,000 | 39 s   | 1,036,152,293 | 28,222     |

  Allocation grows linearly throughout (55 MB, 73 MB, 91 MB); the
  evacuation work does not.
- why it matters: a program that is fast in testing becomes 50 times slower
  per step, on average, on a longer input, with nothing in the program to
  explain it. A SAT solver runs exactly this kind of long loop over
  long-lived state.

## What worked well

**Persistence makes backtracking free.** Deep minimization (minisat's
`litRedundant`) marks variables while it explores, and on failure must undo
the marks it made. In C++ that is a `toclear` stack. Here a failed walk simply
returns `stays` and the caller keeps the marks it had:
```
verdict = implied tr cls seen [q] abstract
return minimize tr cls verdict lits (i + 1) kept abstract if implied? verdict
minimize tr cls seen lits (i + 1) (push kept q) abstract
```
The same holds for the search state at a restart or a failed experiment: an
old value is still there, unchanged, for as long as a name points at it.

**Dispatch reads like a specification.** Restart policies are two markers and
two `budget` arms; the reader's "before the header" state is a marker the
`read_words` arm can name; CLI verbs and options are string-literal arms:
```
fn budget geometric 0
  100

fn budget geometric k
  budget geometric (k - 1) * 3 / 2

fn budget luby k
  100 * luby_of k
```

**Determinism across engines came for free.** VSIDS activities are floats,
bumped, decayed by 1/0.95 and rescaled, and the decision order depends on
their comparisons. All three engines printed byte-identical answers and
statistics on every fixture from the first run, with no effort on my part.

**Tail calls everywhere.** The solver is a set of mutually tail-recursive
functions (search, propagate, visit, moved, stuck), 7,000 conflicts deep in
the larger runs, and no engine ever ran out of stack.

**Tests as constants.** 54 unit tests, each one line; the private helpers of
a module are testable from its test file with no ceremony.

**Absence and failure as data at the edge.** `os/read_file` answering
`file_not_found` as a value made the "no such file" message a one-arm
affair, and the DIMACS reader answers a `malformed` record that the CLI
dispatches on, so every input error carries a line number to the user
without exceptions or result wrappers.

**Several diagnostics taught me something.** "`answer` answers only true or
false: name it `answer?`"; "`neg` takes 1 argument(s), and a list element is
one atom ... Write `(neg …)`"; "these `add_clause` arms tie: each is the
more specific one somewhere". Each named the problem and the fix.

**`&` partial application.** `list_of (&budget geometric) 4` needed no
lambda.

## Summary

The five I would fix first:

1. **F3: a read before a write falls off the in-place path.** A thousandfold
   cliff decided by whether a value has a name, with gigabytes of memory, in
   the one data structure that stands in for arrays. Together with F2 it
   made the obvious solver unusable natively.
2. **F1 and F25: no indexed update, and no way to give one to your own
   type.** An array program has to build its own persistent vector and
   route every access through named functions.
3. **F20: collections in records are quadratic natively.** It punishes
   the program structure the language otherwise encourages.
4. **F19: bindings above a guard run anyway.** Combined with the 80-column
   rule it turns a formatting fix into a behaviour change, and it produced
   the one real correctness bug of the port.
5. **F23 and F22: the test runner resolves names differently, and runtime
   errors have no location.** Both made debugging slower than the bugs
   deserved.

Writing a CDCL solver in kanso was two different experiences. The logic went
in well: two watched literals, first-UIP analysis, deep minimization and
VSIDS each came out as a handful of small functions whose arms read like the
textbook, the solver was correct on the first run, and the immutability that
should have been the obstacle made backtracking and minimization simpler than
in C. The cost model was the hard part. The first, idiomatic version ran up
to 60 times slower natively than on the interpreter, and getting native ahead again
took a persistent vector written by hand, flattening records into 8-parameter
functions, and a profiler, none of which the source code could warn me
about. On the interpreter the map-based version was the fastest thing I
wrote. A language that refuses mutation needs either an array type it can
update in place reliably, or a cost model visible enough that a programmer
can see the slow path before the profiler does.
