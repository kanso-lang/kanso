# FRICTION — porting a Datalog engine to kanso

Entries are numbered in the order I hit them.

## Entries

### F1: a file that defines functions cannot be run, and the book says it can
- kind: tooling
- severity: minor
- where: scratch file while learning (`a.kso`), before the port existed
- wanted: `kanso run a.kso` on a file with `fn` definitions and a `print`
  statement, as `examples/guards.kso` and many book panels (`pub play = ...`
  run with `kanso run`) show.
- wrote: `kanso play a.kso` with bare statements, and for the port a
  `main.kso` of statements only that imports a module directory.
- why it matters: `kanso run examples/guards.kso` in the repository's own
  examples answers "`guards.kso` is a library — nothing to run", and a file
  with `pub play =` is refused by both `run` and `play`. The first hour with
  the language is spent finding out which of three spellings the compiler
  currently accepts.

### F2: a comment block followed by a blank line is "the file beginning with a blank line"
- kind: diagnostic
- severity: nit
- where: datalog/syntax.kso:3
- wanted: a file header comment, a blank line, then the first declaration
  with its own comment.
- wrote: the header merged into the first declaration's comment with a bare
  `#` line between them.
- why it matters: the message says "the file may not begin with a blank
  line" and points at line 3, after two lines of comment. The rule is
  probably "a comment attaches to the next declaration", but the message
  names a different rule.

### F3: importing std/list in one file replaces a private type in another file
- kind: engine-bug
- severity: major
- where: datalog/lexer.kso:38 (the type now named `scanner` was `cursor`, and
  `scan_char` was `step`), triggered by
  `import "std/list"` in datalog/lexer_test.kso; reproduction in
  bugs/import_hides_local_type/
- wanted: a private `type cursor` and a private `fn step` in lexer.kso, which
  imports nothing from std/list.
- wrote: renamed them to `scanner` and `scan_char`.
- why it matters: adding an import to a *test* file broke the lexer, with an
  `error[opacity]` that says my own type is "foreign" and belongs to `list`.
  The module's one namespace means an import in any file reaches every file,
  and std/list exports generic names (`cursor`, `step`, `sorted`, `mapped`,
  `counting`, ...) that a program is likely to want. Either the local
  declaration should win or the collision should be reported as a collision.

### F4: a long `==` in a test cannot be wrapped, so it gets a throwaway name
- kind: aesthetics
- severity: nit
- where: datalog/lexer_test.kso:19
- wanted: `test_lexes_directives = spelled ".decl e(a)\n.output e" == "..."`
  wrapped after the `==`.
- wrote:
  ```
  test_lexes_directives =
    lexed = spelled ".decl e(a)\n.output e"
    lexed == "decl e ( a ) output e "
  ```
- why it matters: only `.`, `.>`, `.!` and `.?` may start a continuation
  line, and a one-expression constant must be written inline, so the only way
  under eighty columns is to invent a binding. Worse, the two rules disagree:
  in datalog/facts_test.kso, `test_reads_columns =` followed by an indented
  `agrees (read_facts ...) "..."` is refused with "a single-expression
  constant is written inline", and the inline form is then refused with "a
  line holds at most 80 characters — this one has 85". Aesthetic, but it is
  in every test file I wrote.

### F5: a call that can fail must be dispatched on before a lambda or `==` sees it, even in a test
- kind: confusing-semantics
- severity: minor
- where: datalog/lexer_test.kso:5
- wanted: `text/join (list/to_list (list/map (lex source) (t -> t.text))) " "`
- wrote: a two-arm helper, one arm `(err e)` and one for the tokens.
- why it matters: chapter 04 sells the railway ("count the lines of error
  handling: zero") and then the checker refuses every place a visible call
  that may answer an err is handed to something. The rule is learnable, and it
  did catch a real gap (`escaped` had no `none` arm), but the book's opening
  promise and the compiler's behaviour point in opposite directions. Binding
  the call to a name first makes the refusal go away, which is a strange
  incentive, and I used it: `path = argv[2]` in a scratch harness, so that
  `os/read_file! path` would compile without a `none` arm that could never
  fire.

### F6: a new top-level name in one file breaks local bindings in another
- kind: refactoring-hazard
- severity: minor
- where: datalog/lexer.kso:74 and :213 (`tok = token ...`, now `made`),
  broken by adding `fn tok` to datalog/parser.kso; then `col`, `wider`,
  `ordered`, `fresh`, `negated`, `at`, `place`, `group`, `row`, `checked` and
  `line`, each a new function or type in one file colliding with a binding
  in another
- wanted: a helper `tok` in the parser, and `tok` as a local name in the lexer.
- wrote: renamed the lexer's locals to `made`, and the lexer's `col` helper to
  `column`.
- why it matters: no-shadowing is module-wide, and a module is every file in
  the directory, so adding a function in one file is an edit that can fail
  compilation in any other file. It happened about a dozen times. The
  error is clear ("`tok` is already a declaration; rename the binding"), but it
  points at the innocent file.

### F7: a pattern variable that happens to be spelled like a marker type becomes a literal match, silently
- kind: confusing-semantics
- severity: major
- where: datalog/parser.kso, every `(token _ word line name)` arm, before the
  token kinds were renamed to `lower_tok`, `upper_tok`, ...
- wanted: token kinds named `word` and `name`, and `name` as an ordinary
  binding in a pattern: `fn atom_at ts p (token _ word line name)`.
- wrote: renamed every kind to `*_tok` so no pattern variable can collide.
  Smallest case (play):
  ```
  type name
  type pair
    left
    right
  fn first (pair name _)
    "first is {name}"
  fn first _
    "no match"
  print (first (pair "ann" "bob"))     # prints "no match"
  ```
- why it matters: the same word is a binding or a constant pattern depending
  on whether a type of that name exists anywhere in the module, and nothing
  says which one you got. Every parser test failed with "expected a predicate
  name, found `edge`" and it took a while to see why. A binding in a pattern
  that collides with a marker could be an error the way a binding that
  collides with a function already is (F6).

### F8: std/expect is in lib/ but "not in the shipped library"
- kind: stdlib-gap
- severity: minor
- where: datalog/agrees_test.kso
- wanted: `test_x = expect (round_trip "p.") . to (equal "p.")`, as
  lib/expect/expect.kso documents, so a failing test shows both sides.
- wrote: my own `agrees got wanted` answering `mismatch got wanted`.
- why it matters: a bare `==` test reports `FAILED (returned false)` and
  nothing else. The fix exists in the repository and the toolchain refuses to
  import it, so every project writes its own four lines.

### F9: no list patterns in a function head
- kind: missing-feature
- severity: nit
- where: datalog/show.kso:6 and :21
- wanted: `fn show_atom (atom _ name [])` for the zero-arity case.
- wrote: `return name if length terms == 0` inside the general arm.
- why it matters: dispatch is the language's only switch, and the empty list
  is the commonest base case there is. Literals and markers can be patterns;
  `[]` cannot ("expected a parameter pattern").

### F10: a map used as a set trips the "comparing to `true`" lint, and there is no membership test
- kind: diagnostic
- severity: minor
- where: datalog/check.kso:57 (`bound_term?`), :193
- wanted: `firm[v] == true`, where `firm` maps variable names to `true`.
- wrote: a two-arm `present?` (`none` answers false, anything else true), and
  `present? firm[v]` everywhere a set is asked about. It ended up used in six
  files.
- why it matters: the lint says "write the value itself", but the value is
  `true` or `none`, and `if none ...` is a runtime error. The lint is right
  about booleans and wrong about lookups. A `has?` for maps, or a set type,
  would remove the need for the idiom.

### F11: `none` cannot seed a fold, so absence needs a private marker
- kind: confusing-semantics
- severity: minor
- where: datalog/check.kso:88 (`list/fold cs no_gap ...`)
- wanted: `list/fold cs none (gap c -> first_gap gap (read_gap shape c))`, to
  find the first undefined read.
- wrote: `type no_gap` and every arm rewritten to answer and match it.
- why it matters: "this can be a none and `list/fold` has no arm for it". The
  checker treats a literal `none` passed as an accumulator as an unhandled
  absence, though here it is the deliberate starting value. A private marker
  says the same thing in more words.

### F12: string literals and list literals cannot span lines
- kind: missing-feature
- severity: minor
- where: datalog/strata_test.kso:19-36, datalog/agrees_test.kso (`with_line`,
  `then`)
- wanted: a test program of four Datalog rules written as four lines of a
  string, or a long expected error message.
- wrote: two helper functions and pipe continuations to glue fragments:
  ```
  source = "e(1).\np(X) :- e(X), !q(X).\nq(X) :- e(X), r(X)."
    . with_line "r(X) :- p(X)."
  wanted = "2: the program cannot be stratified: `p` depends on the negation"
    . then " of `q` at line 2, and `q` depends on `p` (q -> r -> p)"
  ```
- why it matters: an interpreter's tests are mostly source text and messages.
  With an 80-column limit and no way to continue a literal, every test longer
  than one rule needs this. A list literal broken across lines gets "needless
  continuation" when it would fit and "expected an expression" when it
  would not, so it is not an alternative.

### F13: a parameter cannot be named after its type
- kind: aesthetics
- severity: nit
- where: datalog/eval.kso (every `pl`), datalog/plan.kso (`full_op`'s
  `(by_key t)`, first written `(key term)`)
- wanted: `fn emit ctx plan env acc`, `fn op_of (key term)`.
- wrote: `pl`, `tm`, `t`.
- why it matters: types and bindings share one namespace, so the obvious name
  for "the plan" is taken by `type plan`. The same goes for fields named after
  ambient functions: a field called `values` or `keys` cannot be destructured
  positionally (`values` is already a declaration), so the record field had to
  be renamed `listing` and `key_terms`.

### F14: importing std/list replaces a private constructor with a std function, and nothing says so
- kind: engine-bug
- severity: blocker
- where: datalog/plan.kso (`type repeat`, renamed `again`, now `by_repeat`);
  reproduction in bugs/import_hijacks_local_constructor/
- wanted: a private record `type repeat` with one field, built as
  `repeat (slot_of bound v)` and matched as `fn op_of (repeat t)`.
- wrote: renamed it `again`.
- why it matters: `kanso check` passes. At run time `repeat x` calls
  std/list's `repeat` (an infinite generator), so the value is not my record,
  and the failure is "no overload of `datalog/op_of` matches these arguments"
  far from the construction. F3 at least produced a compile error; this one
  compiles and computes the wrong thing. It cost the longest debugging session
  of the port.

### F15: runtime errors carry no location
- kind: diagnostic
- severity: major
- where: datalog/answer.kso:40 (the `[if ...]` of F16), datalog/plan.kso
  (F14)
- wanted: `error[runtime]: join takes a list of strings` with a file and a
  line, or the call chain an err carries.
- wrote: a scratch entry file importing the module by a long relative path
  (`import "../../../../../../../home/user/wt-ports/ports/datalog/datalog"`),
  and bisected by printing intermediate values.
- why it matters: an err reaching the endpoint names its birthplace and the
  functions it passed through. A runtime type error names neither, so the two
  bugs above were found by bisection. `kanso play` cannot import a local
  module ("stdlib imports only"), so there is no quick scratch file either.

### F16: `[if c a b]` is a four-element list on one engine and a refusal on the other
- kind: engine-bug
- severity: major
- where: datalog/answer.kso:40; reproduction in bugs/if_in_list_literal.kso
- wanted: `return [if (length rows > 0) "  yes" "  no"] if length vars == 0`
- wrote: `[(if (length rows > 0) "  yes" "  no")]`
- why it matters: a record constructor in the same place is refused with a
  good message that suggests the parentheses. `if` is not: `kanso check` says
  ok, the interpreter builds `[if true "  yes" "  no"]` and fails later in
  `text/join`, and the native engine refuses the whole program with "`if` as a
  bare value is not yet supported" and no location. The differential law says
  the engines agree or both refuse.

### F17: a bounds proof needs the list in a local name, not a field
- kind: confusing-semantics
- severity: nit
- where: datalog/eval.kso (`run_from`)
- wanted:
  ```
  return emit ctx pl env acc if i < 1 or length pl.steps < i
  exec ctx pl i env acc pl.steps[i]
  ```
- wrote: `steps = pl.steps` first, then the same guard and index on `steps`.
- why it matters: the guard proves `steps[i]` is in range, but only when the
  list is spelled the same way in both places as a bare name. The diagnostic
  ("this can be a none and `exec` has no arm for it") does not mention that a
  guard would satisfy it, so the fix had to be found in lib/list's source.

### F18: a map read after a write sorts the whole map, so read-modify-write loops are quadratic
- kind: performance
- severity: blocker
- where: datalog/relation.kso (`grow_index`, `insert_all`), datalog/eval.kso
  (`load_facts`)
- wanted: the ordinary way to build a hash index or group facts by relation:
  ```
  fn file_in base places buckets code
    k = key_of base places code
    put buckets k (push (or_empty buckets[k]) code)
  ```
  folded over every new tuple.
- wrote: sort the round's codes by key (each code tagged with its key as the
  high digits of one integer, because `list/sort` takes no key function),
  sweep them into groups, then merge the groups with the old buckets read once
  through `entries`, putting keys into a fresh map in ascending order. The
  same merge for the membership set. Facts are grouped with one
  `list/select` per relation instead of one map update per fact.
- why it matters: a kanso map keeps its pairs in arrival order and builds a
  sorted view on the first read after a write (src/runtime.c,
  `k_map_sort_build`), so a loop that reads and writes the same map sorts it
  every iteration. Callgrind on the 2,500-node closure: 93% of 3.3 billion
  instructions in `k_msort_cmp` and `qsort`, reached through `m[k]` 3,080
  times, once per fact loaded into a map keyed by relation name. Release time
  went from 1,197 ms to 63 ms after the three rewrites, and a right-recursive
  closure from 37,135 ms to 82 ms. The standard library hits it too:
  `list/group_by` over n scattered keys takes 0.2 s for 8,000 and 1.8 s for
  32,000. Nothing in the book or appendix B says that interleaving reads and
  writes on one map is the expensive case, and the code that triggers it is
  the code the book teaches.

### F19: a collection held in a record is copied on every update
- kind: performance
- severity: major
- where: datalog/relation.kso (`relation`), datalog/eval.kso (the round's
  harvest, formerly a record of a set and a list)
- wanted: `type relation` holding the tuple list, the membership map and the
  indexes, updated one tuple at a time; a `harvest` record of `seen` and
  `tuples` threaded through the join loop.
- wrote: relations grow once per round through folds whose accumulator is a
  bare map or list, and the harvest is a bare list deduplicated at the end of
  the round.
- why it matters: `put` and `push` reuse the buffer only when nothing else
  holds it, and a map read out of a record is held by the record. Measured with
  `kanso play`: 40,000 `put`s on a map parameter take 0.2 s; the same puts on
  a map destructured from a record and rebuilt take 54 s; 40,000 `push`es on a
  list in a record take 21 s; 10,000 puts on a map nested in a map take 8 s.
  The obvious data model for a relation (a record of its structures) made the
  first working engine quadratic, at 80 microseconds per tuple. The cost is
  invisible in the source: the same expression is O(1) or O(n) depending on
  who else can see the value.

### F20: two-parameter dispatch with a `none` arm ties with literal arms
- kind: confusing-semantics
- severity: minor
- where: datalog/plan.kso (`add_op`, `full_op`, `keyed_op`, `kind_at`),
  datalog/eval.kso (`probe_found?`, `partly_found?`)
- wanted:
  ```
  fn add_op acc _ none _
    acc
  fn add_op acc _ (by_key _) true
    acc
  fn add_op acc place (by_key t) false
    push acc (check_col place t)
  ```
- wrote: one function per boolean (`add_op` dispatching to `full_op` or
  `keyed_op`), and a guarded `kind_at` so that no `none` reaches the group.
- why it matters: "these `add_op` arms tie: each is the more specific one
  somewhere". A `none` arm with wildcards elsewhere ties with any arm that
  has a literal in another position, even though `none` and a `by_key` record
  can never both match. Each tie cost a new helper function. The checker
  separately demanded the `none` arm in the first place (the index could
  miss), so the two rules together push toward guards and away from dispatch.

### F21: `list/sort` sorts only scalars and takes no key
- kind: stdlib-gap
- severity: major
- where: datalog/relation.kso (`grouped`), datalog/intern.kso, the whole
  tuple-code design
- wanted: `list/sort [[3 1] [1 2] [1 1]]`, or a sort by key, to put query
  answers and index groups in order.
- wrote: tuples encoded as integers in base (number of constants + 1), with
  constant ids assigned in sorted order, so that `list/sort` on plain
  integers does the work. To group codes by index key, each code is tagged
  with its key in the high digits (`key * span + code`), sorted, and untagged.
- why it matters: `list/sort` on lists fails at run time with "comparison
  requires two values of one comparable type", and there is no `sort_by`.
  The integer encoding turned out to be the fastest representation, so this
  one ended well, but I chose it because sorting left no other way.

### F22: `list/to_list (list/map ...)` everywhere, and no range
- kind: aesthetics
- severity: nit
- where: 32 occurrences of `list/to_list (list/map`, and `upto` in
  datalog/relation.kso
- wanted: `list/map xs f` answering a list, and `1..n` or `list/range 1 n`.
- wrote: `list/to_list (list/map xs f)`, and
  `fn upto n = list/to_list (list/take list/naturals n)`.
- why it matters: adapters are lazy, so nearly every map is followed by
  `to_list`, which also costs eleven characters of an eighty-column line.
  Aesthetic.

### F23: type declarations must open the file, away from their one use
- kind: aesthetics
- severity: nit
- where: datalog/run.kso:9 (`type refused`), datalog/facts.kso (`type row`),
  datalog/strata.kso (`frontier_step`)
- wanted: the small record a group of functions passes around declared next
  to those functions.
- wrote: moved to the top of the file ("canonical order places type
  declarations before functions; move `refused` up").
- why it matters: appendix C says declaration order "carries narrative, and
  it stays yours", and then types are pinned to the top. In a file whose
  helpers each use a private record, the records sit a hundred lines from
  where they mean anything.

### F24: no record update, so a seven-field state is rebuilt positionally
- kind: missing-feature
- severity: minor
- where: datalog/plan.kso (the `columns` record of an early draft, replaced
  by `laying` and per-column kinds)
- wanted: `acc with bit: acc.bit * 2, ops: push acc.ops skip_col`
- wrote: first, `columns acc.before (acc.bit * 2) acc.bound (push
  acc.keyed_ops skip_col) acc.keys acc.mask (push acc.ops skip_col)` four
  times over, every line past eighty columns; then a redesign so that no
  record had more than two fields.
- why it matters: positional construction means every change to one field
  restates the other six, in alphabetical order, and adding a field breaks
  every constructor. The redesign was better code, but the language forced it
  rather than suggested it.

### F25: map entries have no field names to read
- kind: diagnostic
- severity: nit
- where: datalog/eval.kso (`absorb`, `put_distinct`)
- wanted: `list/fold (entries found) db (acc e -> absorb base acc e.key e.value)`
- wrote: `fn absorb base db (entry name rows)` and a destructuring helper for
  each fold over `entries`.
- why it matters: "no record type has a field `key`". Appendix B describes
  `entries` as "each an `entry key value` record", which reads as fields
  named `key` and `value`.

### F26: the interpreter panics with a Rust backtrace when stdout closes
- kind: tooling
- severity: nit
- where: `kanso run main.kso --interp -- tests/inputs.dl | head -3`
- wanted: a quiet exit, as the native binaries do.
- wrote: nothing; avoided piping the interpreter into `head`.
- why it matters: "thread '<unnamed>' panicked at ... failed printing to
  stdout: Broken pipe", followed by two stack traces. Harmless, but it looks
  like a crash in the user's program.

### F27: no scratch file can import the module under work
- kind: tooling
- severity: minor
- where: debugging F14 and F16, and timing the pipeline's stages
- wanted: a scratch file beside the module with a few definitions and a
  `print`, run with one command.
- wrote: a `main.kso` in the scratchpad that imports the module through
  `../../../../../../../home/user/wt-ports/ports/datalog/datalog`, plus a
  temporary `debug.kso` inside the module for the definitions, since an entry
  file may not define functions and `kanso play` takes standard library
  imports only.
- why it matters: with runtime errors unlocated (F15), printing intermediate
  values is the debugger, and each probe needed two files and a rebuild.

### F28: the interpreter is fifty times slower than a release build
- kind: performance
- severity: minor
- where: bench.sh
- wanted: an oracle fast enough to run the largest fixture in the edit loop.
- wrote: fixtures sized so the interpreter finishes in under three seconds:
  `tc_large` takes 2,664 ms interpreted, 89 ms in a dev build and 52 ms in
  release; a 12,000-edge closure takes 24 s interpreted and 0.5 s in release.
- why it matters: check.sh spends almost all its time in the interpreter, and
  the largest program I could put in the fixture set is smaller than the one
  I wanted to compare across engines. Recorded as a measurement rather than a complaint, since the
  interpreter is the oracle and not the product.

## What worked well

- **Dispatch as a grammar table.** The lexer is one arm per character
  (`fn scan_char c "(" acc`, `fn scan_char c ":" acc`) and the parser one arm
  per token shape (`fn clause_at ts p (token _ directive_tok _ "input")`).
  Reading the parser is reading the grammar, and adding `.input` and
  `.printsize` was four arms each.
- **`none` as end of input.** `cs[p]` past the end answers `none`, so every
  "unexpected end of input" is an arm on the same function rather than a
  bounds check before it. The exhaustiveness checker found the one place I
  forgot (`escaped` with no `none` arm, an unterminated escape at the end of
  a file).
- **The railway for refusals.** `run_tree` is five bindings and a call,
  with no error handling: a syntax error, an arity clash or an
  unstratifiable cycle is an err that passes every later stage and is
  reported once, by one `(err e)` arm in the command line.
- **Exact integers.** A tuple's code is its ids written in base (constant
  count + 1). With three thousand constants a six-column code passes 64 bits,
  and nothing in the engine has to care.
- **Determinism for free.** `keys`, `values` and `entries` come back in key
  order, so every iteration over a map is reproducible. Every fixture agreed
  byte for byte across the interpreter, the dev build and the release build
  the first time check.sh ran, evaluation counters included, and all 26
  still do.
- **Immutable snapshots for semi-naive evaluation.** The full relations and
  the previous round's delta are plain values passed to every plan in a
  round. There is no way for a round to see its own new tuples, which is the
  classic bug in a hand-rolled semi-naive loop.
- **Absence of a file as data.** `os/read_file` answers `file_not_found`, so
  a missing `.input` file is an arm (`fn took ... (file_not_found _)`) that
  produces a proper message, inside an effect chain that reads any number of
  files in order.
- **Tests are constants.** Fifty-one unit tests, most of them two lines, with
  no framework to learn.

## Summary

The five I would fix first:

1. **F14 / F3: imports leak into every file and replace local names.** A
   private `type repeat` silently became std/list's `repeat`, compiled, and
   computed the wrong thing. Either the nearest declaration should win or the
   collision should be an error.
2. **F18: reading a map after writing it sorts the whole map.** The
   read-modify-write loop the book teaches is quadratic, including
   std/list's own `group_by`. It made my first engine twenty times slower
   than the final one, and I found it only with callgrind and the runtime's
   C source.
3. **F19: collections inside records are copied on every update.** Whether
   `put` is O(1) or O(n) depends on who else holds the value, and the natural
   data model, a record of a relation's structures, is the slow case.
4. **F7: a pattern variable spelled like a marker type is a literal match.**
   Silent, and it broke every parser test at once. Treat the collision as an
   error, as F6 already does for functions.
5. **F15 / F16: runtime errors have no location, and the engines disagree
   on `[if ...]`.** Every runtime failure in this port was found by
   bisection.

Writing a Datalog engine in kanso was mostly pleasant at the level of single
functions and hard at the level of the program. Dispatch, `none` and the err
railway fit a lexer, a parser and a checker closely, and those files read
like the grammar and the rules they implement. The trouble started when
the program needed a data structure that changes: a relation that grows each
round, an index that gains buckets. The language presents every value as
immutable and promises in-place updates when nobody else is looking, and in
practice both promises depend on details the source does not show: whether a
map was read since it was written, whether a list sits inside a record. The
first correct version of the engine was quadratic in three places, none of
them visible in the code, and the fixes (sort-and-merge instead of lookup,
bare accumulators instead of records, integers instead of tuples) are
techniques from a lower-level language. Around those, the one-namespace module
made every new helper a possible compile error in another file, and twice a
silent change of meaning. The formatter's eighty columns, with no way to
continue a string or a list, shaped the tests more than anything else did.
