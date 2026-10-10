# FRICTION: porting jlox to kanso

A journal kept while porting the tree-walking Lox interpreter from *Crafting
Interpreters* (Robert Nystrom) to kanso. Entries are in the order I hit them.
Every code sample was compiled with the kanso binary on main
(`/tmp/claude-0/kanso-main/kanso`, 2026-10-10).

## Entries

### F1: a single file of definitions and statements will not `run`, and the book says it will
- kind: confusing-semantics
- severity: minor
- where: scratch file `deep.kso` while learning; book ch05 `count.kso`, ch04 `version.kso`
- wanted: the book's own shape, run with `kanso run`:
  ```
  fn loop n acc
    if (n == 0) acc (loop (n - 1) (acc + 1))

  print "{loop 1000000 0}"
  ```
  or, as chapters 04 to 06 write it, `pub play = print ...` beside the `fn`s.
- wrote: `kanso play deep.kso` for scratch files, and for the real program a
  `main.kso` holding only imports and one statement, with every definition in
  a module directory, as kq does.
- why it matters: `kanso run` says the file "is a library — nothing to run",
  and the `pub play =` form the book uses in a dozen samples is now an error
  ("`pub play` is a library's export"). The first hour with the language is
  spent finding out which of the book's runnable examples still run.

### F2: there is no exponent float literal, and the error does not say so
- kind: diagnostic
- severity: nit
- where: scratch `floats.kso:2`
- wanted: `b = 1e21` (or `1.0e21`)
- wrote: `b = text/to_float "1e21"`, or the 22-digit literal
- why it matters: the compiler answers `canonical form requires exactly one
  space here` with a caret between `1` and `e21`, which reads as a spacing
  rule rather than "this literal form does not exist". The float renderer
  prints `1.0e+21`, so the language emits a spelling it cannot read back.

### F3: native `push` inside a helper function is quadratic
- kind: performance
- severity: major
- where: `bugs/push_in_helper_is_quadratic.kso`
- wanted:
  ```
  fn bump xs i
    push xs i

  fn fill xs n
    fill (bump xs n) (n - 1)
  ```
  to cost O(n), as it does on the interpreter and as `fill (push xs n) ...`
  does natively.
- wrote: `lox/chain.kso`, a newest-first chain of records that becomes a
  list once at the end, used for tokens, statements and printed lines. 10,000
  pushes through a helper take 2 s natively and 0.01 s on `--interp`; 40,000
  take more than 20 s.
- why it matters: extracting a function is the most ordinary refactoring
  there is, and here it changes the complexity class of the program on two of
  the three engines and not the third. `KANSO_COUNTERS=1` shows
  `evac_bytes=144143632` for 3,000 pushes: every helper return copies the
  whole list out of the helper's arena.

### F4: a native map is a log, so it cannot serve as a mutable store
- kind: performance
- severity: major
- where: `bugs/put_overwrite_in_helper_is_quadratic.kso`; design of `lox/cells.kso`
- wanted: an interpreter environment as `{ name:value }`, updated with `put`
  on every assignment, the way a jlox `HashMap` is.
- wrote: a persistent 4-ary trie of my own (`lox/cells.kso`), with names
  resolved to integer slots ahead of time, so no map is ever overwritten in a
  loop.
- why it matters: native `put` appends a pair even when the key exists
  (duplicates are "resolved on read"), and the first read after a write sorts
  the whole log. Overwriting one key 8,000 times from a helper takes 3 s;
  12,000 times reached 4.4 GB resident. On the interpreter the same program
  takes 15 ms. Nothing in the book or appendix B says `put` is for building a
  map once and reading it many times, and the interpreter gives no hint.

### F5: a persistent tree updated in a loop falls off a cliff natively
- kind: performance
- severity: major
- where: `bugs/persistent_trie_evac_cliff.kso`
- wanted: the trie from F4 to cost the same per update at any loop length.
- wrote: the trie, kept small by giving locals that no closure captures a
  stack region that is reused when their scope ends.
- why it matters: with 65,536 leaves the loop is linear up to about 70,000
  updates (1.0 s) and then takes 20 s at 80,000; `evac_bytes` goes from 213 MB
  to 7.7 GB. The workaround for F4 has its own cliff, and finding either one
  took the allocation counters and a reading of `src/runtime.c`.

### F6: a collection literal cannot span lines
- kind: missing-feature
- severity: minor
- where: `lox/scanner.kso`, `word_kind`
- wanted:
  ```
  keywords = {
    "and":"and"
    "class":"class"
    ...
  }
  ```
- wrote: seventeen arms of `fn word_kind "and"` / `"and"`, one per keyword
  plus a catch-all.
- why it matters: the map literal fails with `expected )` on its second line,
  and appendix C only allows the four chain words as continuations. A table of
  sixteen keywords will not fit in 80 columns, so any table longer than one
  line has to become code. Dispatch arms are a fine way to say it here, but it
  should be a choice.

### F7: parentheses that only clarify are an error
- kind: aesthetics
- severity: nit
- where: `lox/scanner.kso`, `alpha?`
- wanted: `("a" <= ch and ch <= "z") or ("A" <= ch and ch <= "Z") or ch == "_"`
- wrote: `"a" <= ch and ch <= "z" or "A" <= ch and ch <= "Z" or ch == "_"`
- why it matters: `these parentheses group nothing` is true, but the reader
  now has to know that `and` binds tighter than `or` to read a range test.
  The same rule rejected `not (digit? c.cs[q + 1])`, and I would not have
  guessed that `not digit? x` parses as `not (digit? x)`. This is an aesthetic
  objection.

### F8: a catch-all `_` does not catch none
- kind: confusing-semantics
- severity: minor
- where: `lox/scanner.kso`, `slash`
- wanted:
  ```
  fn slash c "/"
    comment c "/"

  fn slash c _
    single c "/"
  ```
- wrote: the same, plus a third arm `fn slash c none` with the same body as
  the catch-all.
- why it matters: `c.cs[c.p + 1]` answers none at the end of input, and the
  checker refuses the call until an arm names none, even though the `_` arm's
  body would be right for it. Every "next character" dispatch in the scanner
  needs the duplicate arm.

### F9: an import in a test file renames a type in another file
- kind: refactoring-hazard
- severity: major
- where: `lox/scanner.kso` (type `cursor`), `lox/scanner_test.kso` (`import "std/list"`)
- wanted: a private `type cursor` in the scanner, and a test file in the same
  module that imports `std/list` to map over tokens.
- wrote: renamed the scanner's type to `lexer`.
- why it matters: adding `import "std/list"` to the *test* file turned every
  `cursor ...` construction in `scanner.kso` into
  `error[opacity]: 'list/cursor' is foreign — only 'list' builds a 'cursor'`.
  std/list exports a type named `cursor`, imported pub names join the bare
  namespace of the whole module, and the module's own type loses. The message
  never mentions that my file declares a `cursor` too, so it reads as if I had
  asked for list's type on purpose. It happened again with a private
  function: `fn grown` in `cells.kso` collided with list's exported
  `type grown`, and the call site reported
  `error[arity]: 'grown' has 2 field(s), got 3 (construction is positional...)`
  — an error about a record I never declared. std/list exports thirteen
  helper types (`bounded`, `capped`, `counting`, `cursor`, `cycled`, `grown`,
  `mapped`, `paired`, `repeated`, `sifted`, `skipped`, `sorted`, `step`), and
  every one of them is a name my module may not use once any file imports
  the list module. A name in my module should win over an import, or the clash
  should be reported as a clash, at the import.

### F10: no-shadowing reaches across files, so a new type breaks old code elsewhere
- kind: refactoring-hazard
- severity: major
- where: `lox/scanner.kso` `emit`; `lox/printer.kso` `show`
- wanted: a parameter named `literal` in the scanner's `emit`, and a pattern
  `(print_stmt expression)` in the AST printer.
- wrote: `emit c kind lexeme value width` and `(print_stmt inner)`.
- why it matters: adding `type literal` to `ast.kso` turned a parameter in
  `scanner.kso`, written an hour earlier, into
  `error[name]: 'literal' is already a declaration; rename the binding`. The
  same happened when the parser grew `fn expression`: no file in the module may
  bind a local called `expression` again. Because a module is one namespace,
  every top-level name in any file is reserved in every body of every other
  file. In a program with an AST, the natural names for node types (`literal`,
  `block`, `call`, `get`, `set`, `value`) are exactly the natural names for
  locals, so the AST file keeps renaming things in files nobody touched.

### F11: a record constructor merges the same err with itself
- kind: confusing-semantics
- severity: major
- where: scratch `rail2.kso` while choosing the parser's style
- wanted: write the parser in direct style and let a failure ride the
  railway, since the checker only demands err arms for calls it can see:
  ```
  fn pair pos a b
    first = consume pos a
    second = consume first.pos b
    parsed "{first.node} and {second.node}" second.pos
  ```
- wrote: continuation style, one function per step, each with an
  `e@(err _)` arm (see F17).
- why it matters: when the second `consume` fails, both arguments of
  `parsed` are that one err, and a constructor *merges* failures, so the
  caller's `(err f)` arm gets `f = [fault fault]`, a list holding the same
  reason twice, and `f.message` dies with `error[runtime]: '.' reads a field of
  a record, not [...]`. The compiler page documents that constructors merge
  where calls take the first, but chapter 04 says "a call keeps the first
  failure", and a constructor is written exactly like a call. Merging an err
  with itself should at least not duplicate it.

### F12: a comment cannot stand on its own
- kind: aesthetics
- severity: nit
- where: `lox/ast.kso` header; `lox/parser.kso` section dividers
- wanted:
  ```
  # The syntax tree the parser builds. ...

  type assign
  ```
  and `# --- statements ---` lines with a blank line on each side.
- wrote: every comment glued to the declaration below it.
- why it matters: `the file may not begin with a blank line` and `exactly one
  blank line separates top-level declarations` both fire, so a comment about
  the whole file reads as a comment about its first type, and a long file
  cannot be divided into sections. This is an aesthetic objection.

### F13: 80 columns and no string continuation make expected-output tests awkward
- kind: aesthetics
- severity: minor
- where: `lox/parser_test.kso`, `test_parse_for`
- wanted: `test_parse_for = parsed_as "for (...) print i;" == "(block (var i 0.0) ...)"`
- wrote:
  ```
  test_parse_for =
    got = parsed_as "for (var i = 0; i < 3; i = i + 1) print i;"
    body = "(block (print i) (; (= i (+ i 1.0))))"
    want = "(block (var i 0.0) (while (< i 3.0) {body}))"
    got == want
  ```
- why it matters: a golden string longer than about 60 characters has to be
  cut into pieces at arbitrary points and glued back with interpolation, and
  a list of expected strings cannot be broken across lines either (F6). The
  test then reads worse than the code it tests.

### F14: two private helpers with one name in two files silently become one function
- kind: refactoring-hazard
- severity: major
- where: `lox/parser.kso` and `lox/resolver.kso`: `if_then`, `if_else`,
  `while_body`, `call_args`, `call_arg`, `assign_to`, `class_super`,
  `class_methods`, `class_method`
- wanted: each phase free to name its own continuation helpers
  `if_then`, `while_body` and so on.
- wrote: renamed nine resolver helpers (`if_then_resolved`, `resolve_args`,
  `assignment_of`, ...).
- why it matters: a module is one namespace and overloading is by pattern,
  so the parser's `fn if_else cond branch (parsed ...)` and the resolver's
  `fn if_else condition then_branch (resolved ...)` merged into a single
  dispatch group with no message at all. I found out only because one pair
  (`class_method`) happened to tie, and because the parser's
  `e@(err _)` arms leaked into the resolver's helpers and made the checker
  demand err arms in a phase that never fails ("this can be an err and
  `block_done` has no arm for it"). Two files that compile alone can merge
  into a program that dispatches across them. Splitting the phases into
  separate modules is not a way out: a type declared in another module can
  only be built there (`error[opacity]: ... only list builds a cursor`), so
  the parser could not construct the AST if the AST lived in its own module.

### F15: indexing a list in a loop makes the checker demand none arms
- kind: confusing-semantics
- severity: minor
- where: `lox/resolver.kso`, `resolve_list` (first version)
- wanted:
  ```
  fn resolve_list stmts i acc rs
    return resolved acc rs if length stmts < i
    resolve_next stmts i acc (resolve_stmt stmts[i] rs)
  ```
- wrote (later with a chain instead of `[]`, F30):
  ```
  fn resolve_list stmts rs
    list/fold stmts (resolved [] rs) resolve_step
  ```
- why it matters: `this can be a none and 'resolve_stmt' has no arm for it`.
  The guard on the line above proves `i` is in range, but the checker does
  not read guards, so every bounded index loop needs a `none` arm on whatever
  receives `xs[i]`, or `xs[i]!` (which answers an effect, so cannot be used
  in pure code). The fold is better code, so this pushed me somewhere good.
  Where an index loop is needed (the interpreter's statement loop has to stop
  at a `return`), the way through is to bind `stmt = stmts[i]` first: the
  checker reads calls, not names, so the none arm is no longer demanded. The
  parser gets away with `toks[p]` because it reads a field
  (`(peek ps).kind`) before it dispatches. Both work for reasons that have
  nothing to do with whether the index can miss.

### F16: there is no record update, so one changed field rewrites them all
- kind: missing-feature
- severity: major
- where: `lox/resolver.kso` (`resolver` has five fields, rebuilt in 9
  places), `lox/interpreter.kso` (`machine`)
- wanted: `{ rs with errors = push rs.errors message }`, or
  `rs.errors = ...` outside a build block.
- wrote:
  ```
  fn complain rs tok message
    noted = push rs.errors (located tok message)
    resolver rs.class_kind noted rs.fn_kind rs.level rs.scopes
  ```
  and helpers `with_scopes`, `with_class` so that most sites name one field.
- why it matters: every state-threading program is made of "the same record
  with one field changed". Writing all fields positionally means adding a
  field edits every construction site, getting two same-typed fields swapped
  is silent, and the 80-column limit is soon hit
  (`w = work` exists only to make a line fit). Later, giving closures an
  identity (F22) meant one new field and nine edits across two files, and the
  call-depth limit (F21) went into `frame` rather than `machine` because ten
  `machine ...` lines were already within a few columns of 80. The `.field =`
  syntax already exists inside `build` blocks; outside them nothing does this
  job.

### F17: without exceptions or early return, a parser is a chain of continuations with a pass-through arm on each
- kind: missing-feature
- severity: major
- where: `lox/parser.kso` (42 arms), `lox/interpreter.kso` (24 arms)
- wanted: Java's version of one rule is one method:
  ```
  Stmt varDeclaration() {
    Token name = consume(IDENTIFIER, "Expect variable name.");
    Expr initializer = match(EQUAL) ? expression() : null;
    consume(SEMICOLON, "Expect ';' after variable declaration.");
    return new Stmt.Var(name, initializer);
  }
  ```
- wrote: four functions, because every step that can fail must hand its
  result to the next function as an argument, and each receiver needs an arm
  that passes the failure on:
  ```
  fn var_named (parsed name ps)
    return var_init name (expression (advance ps)) if check? ps "="
    var_end name (literal nil) ps

  fn var_named e@(err _)
    e

  fn var_init name (parsed init ps)
    var_end name init ps

  fn var_init _ e@(err _)
    e

  fn var_end name init ps
    ended = consume ps ";" "Expect ';' after variable declaration."
    then_node ended (var_stmt init name)
  ```
- why it matters: the book promises that an err "flows through functions"
  and costs "zero lines of error handling", but the checker refuses every
  call whose argument is visibly a fallible call unless the receiver has an
  `(err _)` arm. Deleting the 66 arms produced 40-odd
  `error[exhaustive]: this can be an err and 'var_named' has no arm for it`.
  So the railway is free only through a name, and through a name a
  constructor merges the failure into a list (F11). The pass-through arms are
  about 200 lines of a 2,600-line program, and each one says nothing. A
  `?`-style operator, or letting an arm-less receiver pass an err on without
  complaint (which is what the runtime does anyway), would remove them.

### F18: a long-running loop never gives memory back on the native engines
- kind: performance
- severity: blocker
- where: `lox/interpreter.kso`, `spin` and the whole machine-threading design
- wanted: a Lox loop of 200,000 iterations that allocates and drops two small
  objects per pass to run in constant memory, as it does under jlox.
  ```
  var p = Point(0, 0);
  for (var i = 0; i < 200000; i = i + 1) { p = p.add(Point(1, 2)); }
  ```
- wrote: nothing that fixes it. I rewrote the while loop as one
  self-recursive function in case the beat analysis needed that shape; the
  counters did not move.
- why it matters: `KANSO_COUNTERS=1` shows `arena_peak_bytes` equal to
  `alloc_bytes` for the whole statement (13,196,328,960 bytes at 200,000
  passes), and the release binary peaked at 12 GB resident (the box has 15).
  The interpreter keeps memory flat on the same kind of program: a
  million-pass Lox `for` loop sat at about 11 MB resident under `--interp`,
  where the native counters for it come to 3.2 KB per pass, never freed. Per
  `src/beat.rs`, an arena is rewound only inside a self-recursive "beat
  loop" whose carried arguments are scalars or values threaded unchanged from
  entry. An interpreter carries a new state record every step, so it can never
  be one, and nothing else frees memory. In a small test outside the port,
  even a loop the analysis accepted (`beat_iters` equal to the pass count)
  had an arena peak that grew with the pass count: 10 MB at 100,000 passes,
  38 MB at 400,000, for a record of five small fields. kanso has no
  garbage collector, and the arena design assumes short-lived work such as a
  request or a decode. A program that is one long computation over evolving
  state (an interpreter, a simulation, a game loop) runs out of memory. The
  only thing the port does about it is flush output after each top-level
  statement.

### F19: a dispatch tie goes unreported once a third arm is added, and the engines then disagree
- kind: engine-bug
- severity: blocker
- where: `bugs/dispatch_tie_unreported_engines_disagree.kso`;
  `lox/interpreter.kso` `binary_on` (first version)
- wanted:
  ```
  fn binary_on "==" _ left right m
    evaluated m (equal? left right)
  ...
  fn binary_on kind _ left:float64 right:float64 m
    evaluated m (arithmetic kind left right)
  ```
- wrote: two levels of dispatch, operator first (`binary_on`), operand types
  second (`plus`, `numeric`).
- why it matters: the `"=="` arm and the typed arm tie (each is more specific
  in one position). With only those two arms, `kanso check` says so. With the
  other arms of the real group present it said nothing, and `1 == 2`
  evaluated as `1 <= 2` and printed `true`. In the reduced repro the
  interpreter picks one arm and the native build the other, which breaks the
  differential promise. Only running the Crafting Interpreters test suite
  caught it; my own tests compared strings, not numbers.

### F20: a type constructor passed as a function value checks and interprets, but will not build
- kind: engine-bug
- severity: major
- where: `bugs/constructor_as_value_native.kso`; `lox/parser.kso` `logic_or`
  and the other binary levels
- wanted: `left_assoc ps logic_and ["or"] logical`
- wrote: `left_assoc ps logic_and ["or"] (l o r -> logical l o r)`
- why it matters: `kanso check` says ok, `--interp` runs it, and `kanso build`
  fails with `native backend: 'lox/logical' as a bare value is not yet
  supported`. A feature that only some engines speak should be refused by
  `check`, not discovered at build time.

### F21: running out of stack is a clean error on one engine and a segfault on the others, at different depths
- kind: engine-bug
- severity: major
- where: `lox/interpreter.kso` `max_calls`;
  `bugs/native_stack_overflow_segfault.txt`,
  `bugs/test_runner_aborts_on_deep_recursion.kso`
- wanted: deep Lox recursion to fail the same way everywhere.
- wrote: a Lox call-depth limit of 1,000 that raises clox's "Stack
  overflow." runtime error, carried in each frame (`depth`) because the
  machine record was too wide to take another field (F16).
- why it matters: `fun f(n) { if (n == 0) return 0; return f(n - 1) + 1; }`
  survives depth 2,000 and dies at 2,500 on `--interp` with
  `error[runtime]: the program ran out of stack`, survives 3,000 and dies
  silently by 4,000 on the dev build, and survives 4,000 and dies by 6,000 on
  the release build, each native one with `Segmentation fault` and exit
  139. The three engines give three answers for one program, and two of them
  give no message. gdb puts the native fault inside the trie write
  (`d_lox/store_4`), a few frames below the one that ran out. Every pure-kanso
  deep recursion I wrote to reduce it, including one that writes a trie at
  each level, got the clean "ran out of stack" message natively, so the
  guard has a gap I could not isolate. `kanso test` is a fourth: a test that runs the same Lox
  function to depth 700 aborts the whole test run with Rust's
  `thread 'main' has overflowed its stack`, losing every other test's result,
  where depth 400 passes and `kanso run --interp` reaches 2,000. So the
  port's own stack-overflow case cannot be a unit test; it is a fixture only.

### F22: identity has to be built by hand
- kind: missing-feature
- severity: minor
- where: `lox/values.kso` (`closure` gained an `id`), `lox/interpreter.kso`
  (`numbered`, `bind_method`)
- wanted: Lox `==` on functions, bound methods, classes and instances is
  identity, as in Java.
- wrote: every closure, class and instance carries a number taken from the
  machine's counter, so making a bound method now threads the machine:
  ```
  fn bind_method method object m
    evaluated (numbered m) (bound_to method object m.next)
  ```
- why it matters: `foo.method == foo.method` must be false and was true,
  because two bindings of one method are structurally equal. Structural
  equality is the right default, but a program that models objects needs
  fresh identity, and getting it means threading a counter through code that
  was otherwise pure. Found by the test suite, not by me.

### F23: a pure evaluator cannot read the clock, so Lox's `clock()` is frozen
- kind: confusing-semantics
- severity: minor
- where: `lox/run.kso` `start`; `lox/interpreter.kso` `apply native_clock`
- wanted: `clock()` returning the current time each time it is called, as
  every Lox benchmark uses it.
- wrote: `time/now` is read once before the program starts, and every
  `clock()` answers that number.
- why it matters: effects are descriptions performed after evaluation, and
  the interpreter is one evaluation, so it has no way to ask the world a
  question in the middle. The fix would be to turn the whole interpreter into
  a chain of effects, which would make every Lox expression an io value and
  make the --plan / scripted-executor story moot. Benchmarks that print
  `clock() - start` print 0.

### F24: there is no way to read standard input a line at a time, so there is no REPL
- kind: stdlib-gap
- severity: minor
- where: `lox/run.kso` `launch`
- wanted: jlox's prompt loop: read a line, run it, print `> `, repeat.
- wrote: with no script argument the port reads all of standard input with
  `io/stdin` and runs it as one script.
- why it matters: `std/io` has `stdin` (all of it, at once) and nothing that
  reads a line. Any interactive program is out of reach.

### F25: native stdout is buffered past a stderr write, so the two streams interleave differently by engine
- kind: engine-bug
- severity: minor
- where: `lox/run.kso` `top_next` (the runtime-error arm);
  `bugs/stdout_after_stderr_native.kso`
- wanted: the program's output, then the error, in that order, when both
  streams go to one pipe (`lox script.lox 2>&1`).
- wrote: `check.sh` captures stdout and stderr separately.
- why it matters: the chain is `flush m .> (_ -> io/write_err report) .> (_ ->
  os/exit 70)`. Under `--interp` the output comes first; under both native
  builds the error comes first and the output at exit. The bind is supposed
  to be the order, and on the native engines it is not the order the reader
  sees.

### F26: a constant must be one line, and a line must be 80 columns
- kind: aesthetics
- severity: nit
- where: `lox/values_test.kso`, `test_number_large`
- wanted:
  ```
  test_number_large =
    lox_num "1234567.5" == "1234567.5" and lox_num "12345678.9" == "1.23456789E7"
  ```
- wrote:
  ```
  test_number_large =
    below = lox_num "1234567.5"
    below == "1234567.5" and lox_num "12345678.9" == "1.23456789E7"
  ```
- why it matters: `a single-expression constant is written inline`, but the
  inline form is 93 columns, and `and` is not a continuation token. The only
  way through is to invent a binding nobody needs. It happens with every test
  that checks two things.

### F27: builtin and ambient names are reserved, and you learn which by colliding
- kind: diagnostic
- severity: nit
- where: `lox/interpreter.kso` (`bind` became `bound_to`, `values` became
  `collected`); `lox/resolver.kso` (`done` became `readied`)
- wanted: a helper `fn bind (closure f code) object` (jlox's
  `LoxFunction.bind`), a local `done`, a parameter `values`.
- wrote: other names.
- why it matters: `the name 'bind' is already taken` and
  `'done' is already a declaration; rename the binding` do not say what took
  them: `bind` is the long name of `.>`, `done` is what a write yields, and
  `values` is an ambient map function. Appendix B lists eight ambient names,
  but `fn rescue`, `fn annotate`, `fn effect` and `fn bind` are all refused
  the same way, and `done` is refused as a local yet accepted as a function
  name (`fn done x` compiles).

### F28: a comparison chain gets "unexpected trailing tokens"
- kind: diagnostic
- severity: nit
- where: `lox/values.kso`, `divide`
- wanted: `negative = a < 0.0 != negative_zero? b`
- wrote: `negative = (a < 0.0) != negative_zero? b`
- why it matters: refusing to chain comparisons is reasonable, but the
  message is `error[syntax]: unexpected trailing tokens` with the caret on
  `!=`, which does not say that comparisons do not associate or that
  parentheses fix it. Elsewhere the same compiler rejects parentheses as
  redundant (F7), so a reader cannot guess which way it wants it.

### F29: a printed record does not show its own structure
- kind: tooling
- severity: minor
- where: debugging the resolver, a throwaway `lox/debug.kso`
- wanted: to look at a resolved tree while a test was failing.
- wrote: a `pub fn debug` returning `"{first}"`, an entry `main.kso` in a
  scratch directory with a symlink to the module (`kanso play` only takes
  stdlib imports), and then reading this:
  ```
  lox/scoped { 2:true } [lox/define lox/literal 1.0 lox/token "identifier"
  "a" 1 "a" lox/local lox/define lox/literal 2.0 ...
  ```
- why it matters: nested records render as a flat run of words with no
  parentheses, so where `define`'s three fields end and the next node starts
  has to be worked out by counting fields against the type declarations.
  There is no debugger and no trace print in pure code, so this rendering is
  the main way to see a value, and for any tree it is close to unreadable.
  (The failure turned out to be in my test, not the resolver.)

### F30: pushing onto a list held in a record is quadratic on every engine
- kind: performance
- severity: major
- where: `bugs/record_field_push_quadratic.kso`; `lox/scanner.kso` (first
  version), `lox/resolver.kso` `resolve_list` (first version)
- wanted: a scanner whose state record carries the token list:
  ```
  fn emit c kind lexeme value width
    tok = token kind lexeme c.line value
    lexer c.cs c.errors c.line (c.p + width) c.source (push c.tokens tok)
  ```
- wrote: `chain c.tokens tok`, and `chain_list` at the end (see F3).
- why it matters: this is the most ordinary accumulator shape, and the
  reduced repro takes 1.2 s for 10,000 pushes on the interpreter and 3.3 s
  natively, 4.0 s and 33 s for 20,000, and at 40,000 the native build is
  killed for memory. On the interpreter the list is copied because the old
  record still holds it; natively it is the evacuation from F3. I found it
  through a 2,351-line file in the book's test suite that the interpreter
  took 83 s to scan and parse, against 0.3 s natively. After moving tokens,
  block statements and resolved lists to chains (and the slicing change in
  F31), the same file takes 5 s of CPU on the interpreter. Both engines' quadratic shapes are different, so a
  program tuned on one can still be quadratic on the other.

### F31: on the interpreter, slicing a string costs the length of the whole string
- kind: performance
- severity: minor
- where: `lox/scanner.kso` `source_text`
- wanted: each lexeme cut from the source, `text/slice c.source c.p (stop - 1)`.
- wrote:
  ```
  fn source_text c from to
    text/join (text/slice c.cs from to) ""
  ```
  which slices the list of characters the scanner already holds.
- why it matters: on `--interp`, 20,000 three-character slices of a
  150,000-character string took 6.2 s and 40,000 took 12.4 s, against 0.09 s
  for building the string alone: each slice walks the string from the start,
  presumably to count characters. The native builds do not show it. Scanning
  1,600 lines of Lox took 4.6 s of CPU with string slices and 1.1 s with list
  slices. Appendix B describes `text/slice` without any cost, and nothing says
  that positions in a string are character counts paid for on each call.

## What worked well

**Dispatch reads like the grammar.** The scanner is one arm per character
and the parser one arm per leading token, and both read like the tables in
the book:
```
fn statement_at ps "for"
  for_open (consume (advance ps) "(" "Expect '(' after 'for'.")

fn statement_at ps "if"
  if_open (consume (advance ps) "(" "Expect '(' after 'if'.")
```
jlox needs `GenerateAst.java`, an `accept` method on every node and a
`Visitor` interface with one method per node type. Here the node types are
23 short `type` declarations in `ast.kso`, and the evaluator is
`fn eval (binary left op right) m`, one arm per node. Adding a node is one
type and one arm in each phase.

**The pure core is testable without any setup.** All of jlox's I/O ended up
in the 62 lines of `run.kso`. Everything else is a function from values to
values, so `interpreter_test.kso` runs whole Lox programs (closures, classes,
`super`, runtime errors) inside `kanso test` with no files, no captured
stdout and no mocks:
```
test_run_super =
  base = "class A \{ m() \{ return \"A\"; } }"
  derived = "class B < A \{ m() \{ return super.m() + \"B\"; } }"
  run_lox "{base} {derived} print B().m();" == "AB"
```

**Structural equality makes assertions easy.** A resolver test compares a
rewritten node against a node built in the test
(`inner.expression == local_get 1 (scan "a").tokens[1] 1`), with no
`equals` methods to write.

**Immutable frames fit Lox's static scoping.** Because the resolver fixes
every reference before the program runs, a closure only needs the scope
chain as it was when the closure was made, which in kanso is the frame value
at that moment. Chapter 11 of the book exists because jlox's closures share a
mutable `Environment` that later declarations can change underneath them. A
frame value cannot change after a closure takes it, so only variables a
closure captures need a mutable cell.

**`list/fold` with named step functions.** Folding a resolver state over a
statement list (`list/fold stmts (resolved chain_start rs) resolve_step`) is
shorter than the index loop, and the step function is an ordinary
dispatching function, so the per-element logic stays testable.

**Early-return guards keep functions flat.** `return x if cond` covers most
of what would otherwise be nested `if`s:
```
fn synchronize ps
  return ps if at_end? ps or (previous ps).kind == ";"
  if (starts_statement? (peek ps).kind) ps (synchronize (advance ps))
```

**Tail calls hold.** The interpreter is written in continuation style
throughout, and a Lox `while` of a million passes runs through a chain of
tail calls without growing the stack on any engine.

**Shortest float digits for free.** kanso renders a float with the shortest
digits that round-trip, which is what modern Java does, so matching jlox's
number output meant rearranging kanso's digits, not generating any.
Agreement with jlox on every number in the book's test suite came from 60
lines.

**The engines agree, and the counters are deterministic.** Apart from the
tie in F19, all 27 fixtures and the book's 246 test programs produce the same
bytes on the interpreter, the dev build and the release build. When memory
went wrong (F3, F18), `KANSO_COUNTERS=1` gave exact allocation and arena
figures that did not change from run to run. On a machine with a load
average of 11, wall-clock timings were useless and the counters were the
only reliable measure I had.

**Absence as data where it is expected.** `os/read_file` answering
`file_not_found` made the "no such script" path one extra arm in
`run_file`, with no error handling around the read.

## Summary

The five I would fix first:

1. **F18, memory is never reclaimed in a long-running loop on the native
   engines.** An interpreter, or any program that is one long computation
   over a changing state record, grows until it dies: 12 GB for 200,000
   passes of a small Lox loop. Nothing in the language lets the program
   avoid it.
2. **F19, a dispatch tie that the checker misses and the engines resolve
   differently.** It turned `1 == 2` into `true` with no diagnostic. Overload
   dispatch replaces every conditional in kanso, so it has to be either
   unambiguous or refused.
3. **F14 (with F9 and F10), one namespace per module, with overloading,
   means files interfere with each other silently.** Same-named private
   helpers in two files merge into one dispatch group; a new type in one file
   breaks a parameter name in another; an import in a test file renames a
   type in a third. I renamed things more than thirty times to keep sixteen
   files apart.
4. **F17, every fallible step needs a pass-through err arm.** 66 arms,
   about 200 lines, were boilerplate, and the one way to avoid them (binding
   through a name) runs into constructor merging (F11).
5. **F3, F4 and F30, ordinary list and map updates go quadratic.**
   Extracting `push` into a helper, or overwriting a map key in a loop, is
   fine on the interpreter and pathological on both native builds; pushing
   onto a list kept in a record is quadratic everywhere. The only ways to find
   out are `KANSO_COUNTERS` and reading `runtime.c`, and the fix in every case
   was to stop using the built-in list or map.

Close behind: no record update syntax (F16), which taxed every state-threading
function in the program.

Writing jlox in kanso split into two halves. The front end (scanner, parser,
resolver, printer) suited the language: dispatch on tokens reads like the
grammar, the AST needs no visitor machinery, and the resolver as a tree
rewrite was cleaner than jlox's side table. The cost there was ceremony:
continuation functions with pass-through arms, records rebuilt field by
field, names renamed to dodge the module-wide namespace, and lines broken to
stay under 80 columns. The back end fought the language. Lox is a language of
mutable shared state, and kanso has no mutable cell, so I built one, a
persistent trie with a stack region and a heap region, and threaded a
eight-field machine record through every evaluation step. That works and
agrees byte for byte with jlox on the book's own test suite, on all three
engines. But the native engines cannot run it for long, because the memory
model assumes short-lived work, and the interpreter is about 15 to 35 times
slower than native. The language was most helpful where the problem was
already a pure function from input to output, and it got in the way as soon
as the program had to model a world that changes.
