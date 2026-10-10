# FRICTION: porting the core of RE2 to kanso

A journal kept while writing a regular-expression engine in kanso: a parser
to an AST, simplification, compilation to a Pike VM program, and a grep-like
command line over it. Entries are in the order I hit them. Every code sample
was compiled with the kanso binary on main (`/tmp/claude-0/kanso-main/kanso`,
2026-10-10).

## Entries

### F1: a typeset must fit on one line, so a large one has to be split
- kind: aesthetics
- severity: minor
- where: re/ast.kso, the `node` typeset
- wanted: one name for the twenty node kinds, wrapped like any long line:
  ```
  pub type node alt any_char any_not_nl begin_line begin_text capture cat cls
    empty end_line end_text lit no_match not_word_boundary plus quest repeat star
    word_boundary
  ```
- wrote: five typesets, grouped only so that each fits in eighty columns:
  ```
  pub type anchor begin_line begin_text end_line end_text word_edge
  pub type atom any_char any_not_nl cls empty lit no_match
  pub type compound alt capture cat
  pub type loop plus quest repeat star
  pub type node anchor atom compound loop
  pub type word_edge not_word_boundary word_boundary
  ```
- why it matters: the compiler says "a subtype or typeset declaration is one
  line" and the width rule says eighty columns, so the two rules together cap
  the number of members. The grouping happened to be useful later (an arm on
  `_:anchor` covers six kinds), but `word_edge` exists only because of the
  column count. An AST is the first thing most language tools declare, and
  twenty kinds is a small one.

### F2: `list/sort` cannot sort records and takes no key
- kind: stdlib-gap
- severity: major
- where: re/classes.kso, `normalize`
- wanted: `list/sort_by spans (s -> s.first)`, or `list/sort spans` with
  records compared field by field, as Python compares tuples.
- wrote: pack each span into one integer, sort the integers, unpack:
  ```
  fn packed s
    s.first * shift + s.last

  sorted = list/sort (list/to_list (list/map spans packed))
  ```
- why it matters: normalizing a character class is a sort of intervals, and
  the only sort in the library orders numbers and strings. The packing trick
  works here only because code points fit in 21 bits. `list/sort` on records
  also passes `kanso check` and fails at run time with "comparison requires
  two values of one comparable type", so the gap shows up late.

### F3: an index read right after a bounds check still has to handle none
- kind: confusing-semantics
- severity: major
- where: re/classes.kso, `inside?`, `gaps`, `merge_from`
- wanted:
  ```
  fn inside? spans cp i
    return false if length spans < i
    s = spans[i]
    return false if cp < s.first
    ...
  ```
- wrote: every indexed walk as two functions, the second with a none arm
  that does the bounds check's job:
  ```
  fn inside? spans cp i
    within? spans cp i spans[i]

  fn within? _ _ _ none
    false

  fn within? spans cp i s
    return false if cp < s.first
    ...
  ```
- why it matters: the checker refuses `s.first` on a value that "can be a
  none" even when the line above proved the index is in range. Dispatching on
  none is a fine replacement for the bounds check once you see it, but it
  doubles the number of functions in every loop over a list, and the
  `xs[i]!` spelling that looks like the answer is a trap (F4).

### F4: `xs[i]!` in pure code passes the checker and fails at run time
- kind: diagnostic
- severity: major
- where: re/classes.kso (first draft of `inside?`); `bugs/strict_index_field_read.kso`
- wanted: either the value at `i`, or a compile-time refusal.
- wrote: `s = spans[i]!` then `s.first`. `kanso check re` said ok, and
  `kanso test re` failed with `error[runtime]: '.' reads a field of a record,
  not <io>` at `re:60:24`.
- why it matters: the book's own description of `xs[i]!` is "i know this
  element exists", which is exactly what a programmer means after a bounds
  check. It is an effect instead, and the checker that refuses `length
  os/args` does not refuse a field read on this box. The location names the
  module directory, not the file, so the line number has to be matched up by
  hand.

### F5: only pipes and chain words may continue a line
- kind: aesthetics
- severity: minor
- where: re/classes_test.kso, re/match_test.kso, re/backtrack.kso, grep/scale.kso
- wanted: to break a long line after an operator, inside a lambda or inside
  a string, for example
  ```
  test_negate_fills_gaps = negate gapped == [(span 0 0) (span 4 6) (span 9 max_cp)]
  ```
  and
  ```
  again = bt body cps pos steps budget (p s -> more true body cps pos p s budget k)
  ```
- wrote: a named constant for the expected value (`top = max_cp`, eleven
  `*_wanted` constants in re/match_test.kso), a named lambda
  (`onward = (p s -> more true body cps pos p s budget k)`), and a table
  header assembled from two interpolated halves because a string literal
  cannot wrap either.
- why it matters: appendix c says width "never changes how many statements
  a body has", but in practice it does: every over-long expression that is
  not a pipe becomes an extra binding. Some of those names help. Others,
  like `top`, exist only to save a column. Renaming a helper to something
  longer re-breaks lines all over a test file (the `found` rename in F22
  produced two new width errors).

### F6: imports belong to a file, not to the module
- kind: confusing-semantics
- severity: nit
- where: re/classes.kso
- wanted: `import "std/text"` once for the module, since its files share
  every declaration.
- wrote: the import at the top of each file that uses `text/`.
- why it matters: the error is clear ("a module's files share their
  declarations, not their imports"), but chapter 07 spends a page on "one
  namespace across its files", and the exception is easy to miss.

### F7: a scratch file with a function and a statement will not `run`
- kind: tooling
- severity: minor
- where: scratch files while learning
- wanted: `kanso run scratch.kso` on a file with `fn` declarations and a
  `print` below them, as several book chapters show.
- wrote: `kanso play scratch.kso`. `kanso check scratch.kso` on the same file
  then refuses it ("a library has no statements to run"), so a play file has
  no checker of its own; only running it reports formatting errors.
- why it matters: in the first hour every experiment is this shape, and the
  three verbs disagree about whether it is a program.

### F8: a list literal cannot continue onto a second line
- kind: missing-feature
- severity: minor
- where: re/fold.kso (the case-folding table), grep/cli.kso (`flags`)
- wanted:
  ```
  rules = [(rule 32 65 90) (rule -32 97 122) (rule 32 192 214) (rule 32 216 222)
    (rule -32 224 246) (rule -32 248 254) (rule 121 255 255) ...]
  ```
- wrote: the table as six named rows joined with a fold, which is also what
  lib/sha256 does with its round constants:
  ```
  latin = [(rule 32 65 90) (rule -32 97 122) (rule 32 192 214) (rule 32 216 222)]

  latin_low = [(rule -32 224 246) (rule -32 248 254) (rule 121 255 255)]
  ...
  rules = [latin latin_low greek greek_low cyrillic cyrillic_low]
    . list/fold [] text/concat
  ```
- why it matters: the error for the wrapped form is `expected ')'` at the
  end of the second line, which does not say that a literal may not wrap.
  Any table-driven code (Unicode data, opcode tables, test vectors) meets
  this, and the names given to the rows carry no meaning.

### F9: where a `none` arm goes on the ladder is not what the message says
- kind: diagnostic
- severity: minor
- where: re/compile.kso `anchored?`, re/parse.kso `brace_char`, re/group.kso `flag_char`
- wanted: the ladder as the diagnostic states it, "literal, then concrete
  type, then generic", with none treated as a concrete type.
- wrote: by trial. In `brace_char` the none arm had to come after the
  string-literal arms; in `anchored?` it had to come before the marker arm
  `begin_text`, ahead of every other concrete arm. In `follow` (re/pike.kso)
  a none arm whose earlier parameters were generic was refused, and had to
  repeat the record pattern of its siblings, `(pending _ _ rest) none`, to
  be accepted. Each refusal printed the same sentence.
- why it matters: a none arm is the most common arm in this program (39 of
  them), because every index read can be none (F3). Separately, a generic
  `_` parameter does not accept none, which is consistent with the book but
  surprising the first time a catch-all arm is reported as missing a case.

### F10: no list patterns in a parameter
- kind: missing-feature
- severity: minor
- where: re/parse.kso `branched`, `braced`
- wanted:
  ```
  fn branched [one]
    one

  fn branched branches
    alt branches
  ```
- wrote:
  ```
  fn branched branches
    if (length branches == 1) branches[1] (alt branches)
  ```
  which then needs `branches[1]` to be accepted as possibly none.
- why it matters: dispatch is the language's only switch, and the empty list
  and the singleton are the two cases every list-walking function has. The
  error is "expected a parameter pattern" at the bracket.

### F11: one namespace means every name in the module, its tests and its imports collides with every local
- kind: refactoring-hazard
- severity: major
- where: throughout; twenty-nine renames by the end
- wanted: a local binding or parameter that can reuse a common word.
- wrote: these renames, each forced by an `error[name]: 'x' is already a
  declaration` or by a stranger error:
  - `merged`, `top`: test constants in re/classes_test.kso blocked locals of
    the same name in re/bracket.kso and re/pike.kso.
  - `close`, `opened`, `counted`, `plain`, `codes`, `digits`, `frag`,
    `done`, `joined`, `shown`, `label`, `slot`, `posix`, `longest`: a
    private function or constant in one file blocked a local in another
    file of the module. Adding `fn joined` to re/parse.kso broke a local in
    re/simplify.kso; adding `fn slot` to re/pike.kso hit a pattern variable
    in re/compile.kso; the local `posix` in the matcher collided with the
    POSIX class table in re/classes.kso.
  - `node`, `leaf`/`atom`, `subject`, `got`: a type or typeset name blocked
    parameters named `node`, `subject` and `got` in other files, and the
    typeset `atom` blocked the parser's function `atom`.
  - `steps`: adding the accessor `pub fn steps` to re/api.kso broke six
    parameters named `steps` in re/api.kso and re/pike.kso at once.
  - `repeat`, `repeated`, `tally`, `step`: names exported by std/list. My AST
    type `repeat` became "no 4-argument arm of `repeat` (arms take 1)", my
    marker `repeated` became "`list/repeated` is foreign", and my function
    `step` in re/pike.kso, a file that does not import std/list, became
    "`step` has 2 field(s), got 8", because another file of the module does.
  - `search`, `compile`: a type and a function I added clashed with
    functions of the same name in another file.
  - three test names (`test_counted`, `test_negated_class`,
    `test_fold_kelvin`) used in two test files: "overlapping overloads of
    `test_counted` are illegal".
- why it matters: no-shadowing is sold as "a name means one thing in the
  place you read it", but the place that decides is the whole module, its
  tests, and every module any of its files imports. Adding a test constant,
  a helper or an import anywhere can break a function nobody touched, and
  the messages for the std/list cases name the wrong problem. Late in the
  port I was choosing names for being unusual rather than for being right
  (`wants_longest`, `cap_at`, `both_chars`). F22 is the case the checker
  misses entirely.

### F12: a pub record is opaque outside its module, fields and all
- kind: confusing-semantics
- severity: major
- where: re/api.kso, grep/cli.kso
- wanted: in the CLI,
  ```
  fn show (err (re/syntax_error at code text))
    print "error parsing regexp: {code}: `{text}` at {at}"
  ```
  or `e.code`, or a match record whose `start` and `end` the caller reads.
- wrote: an accessor function in re/api.kso for every field a caller needs
  (`message`, `column`, `hits`, `steps_taken`, `hit_start`, `hit_end`,
  `group_start`, `group_end`, `group_names`, `group_count`), and matches
  carried as plain lists of offsets. The errors were
  "`re/syntax_error` is foreign — its structure does not cross an import" and,
  for `e.code`, "no record type has a field `text`".
- why it matters: `pub type` reads as "this type is part of my API", but the
  type is only a name outside the module; it cannot be destructured, read or
  even constructed there. A library that returns structured results has to
  write a getter per field, which is the Java ceremony the rest of the
  language avoids. Neither chapter 07 nor appendix a says that pub stops at
  the type's name.

### F13: an argument cannot be `none` on purpose
- kind: confusing-semantics
- severity: minor
- where: re/pike.kso `execute`
- wanted: `scan ... none`, with `none` meaning "no match yet", and arms
  `fn outcome none steps` / `fn outcome caps steps`.
- wrote: a private marker, `type unmatched`, passed instead of none.
- why it matters: the checker reports the literal `none` I wrote as "this can
  be a none and `scan` has no arm for it". Once explained it makes sense, as
  none is reserved for "a lookup missed", but the book introduces none as the
  value for absence and the first instinct is to use it for that.

### F14: every function that receives a parse result needs an err arm
- kind: aesthetics
- severity: minor
- where: re/parse.kso, re/group.kso, re/bracket.kso, re/api.kso, grep/*
- wanted: the railway as chapter 04 first describes it, where an err rides
  past functions that never mention it.
- wrote: eighteen arms of this form, fifteen in the program and three in
  test helpers:
  ```
  fn appended _ _ e@(err _)
    e
  ```
  one for every function whose argument is a call to something that can
  answer an err. Two more test helpers (`counted_steps`, `backtracked`)
  turn an err into -1, because `*` and `<` refuse an argument that "can be
  an err" even in a test. The book's loophole, binding the call to a name first so
  the checker cannot see it, would remove them, but it reads as hiding.
- why it matters: a recursive-descent parser is exactly the program the
  railway should make pleasant, and the checker turns each layer into a
  dispatch on err anyway. The arms are short and mechanical, which is the
  sign of ceremony rather than meaning.

### F15: a release build drops `musttail` from a tail call with seven parameters
- kind: engine-bug
- severity: major
- where: re/pike.kso `scan` (before the workaround); `bugs/release_tail_call/`
- wanted: the Pike VM's loop over the subject,
  ```
  fn scan prog cps n start pos q matched
    ...
    scan prog cps n start (pos + 1) ran.next ran.matched
  ```
  running in constant stack on every engine, as the book promises.
- wrote: the four things that do not change during a search bundled into a
  record so that the loop takes four parameters:
  ```
  type subject
    cps
    n
    prog
    start

  fn scan s pos q matched
  ```
  and the same for find_all's loop in re/api.kso.
- why it matters: `regex --steps '(a*)*b'` on a 40,000-character line
  segfaulted in the release build only (32,452 frames of `d_re/scan_7` in
  gdb) and ran on the dev build and the interpreter. The release IR emits
  `call preserve_nonecc` without `musttail` for such calls; whether a given
  program survives depends on whether LLVM turns the recursion into a loop
  by itself. The demonstration this port exists for, linear time on long
  inputs, crashed on the optimized build.

### F16: no way to build a record from another with one field changed
- kind: missing-feature
- severity: major
- where: re/group.kso `set_flag`, re/pike.kso (`queue` rebuilt in five places)
- wanted: `{ f with fold: v }` or `f.fold = v` as an expression.
- wrote: one arm per field, each restating the record:
  ```
  fn set_flag f "i" v
    flags f.dot_nl v f.multi f.ungreedy

  fn set_flag f "m" v
    flags f.dot_nl f.fold v f.ungreedy
  ...
  ```
- why it matters: state threaded through a recursive loop is always a record
  where one field changes per step. Positional construction in alphabetical
  field order means each rebuild names every field in an order that is not
  the order you think in, and adding a field means editing every rebuild.

### F17: a helper type cannot sit beside the function it serves
- kind: aesthetics
- severity: nit
- where: re/api.kso `sweep`
- wanted: the small state record for find_all's loop written directly above
  find_all.
- wrote: it at the top of the file with the other types; the compiler says
  "canonical order places type declarations before functions".
- why it matters: chapter 07 says declaration order "carries narrative, and
  it stays yours", but a type is always exiled from its narrative.

### F18: comparisons do not chain, so an exclusive-or needs a name
- kind: aesthetics
- severity: nit
- where: grep/cli.kso `scanned`
- wanted: `picked = hits != [] != has? options "-v"`
- wrote: `matched = hits != []` and then `picked = matched != has? options "-v"`.
- why it matters: the error is "unexpected trailing tokens", which reads as
  a parser problem rather than "comparison is not associative here".

### F19: every `{` in a pattern string is an interpolation
- kind: diagnostic
- severity: minor
- where: re/parse.kso `piece "\{"`, re/dump.kso, re/parse_test.kso, re/match_test.kso
- wanted: `fn piece "{" cs at st acc prev`, and test patterns written as
  they are typed: `tree_of "a{2,5}"`, `tree_of "\\x{10FFFF}"`.
- wrote: `fn piece "\{" cs at st acc prev`, `"lit\{{cp_text cp}}"` in the
  dump, and `"a\{2,5}"`, `"\\x\{10FFFF}"` in the tests.
- why it matters: the escape is reasonable, but the errors for the
  unescaped forms point elsewhere: `"{"` is "unterminated interpolation",
  `"a{2,5}"` is "kanso has no commas", and `"\\x{10FFFF}"` is "identifiers
  are snake_case, all lowercase, always". A program that carries regular
  expressions, templates or JSON in string literals meets this on every
  counted repetition, and the messages send you looking in the wrong place.

### F20: `text/slice` past the end answers nothing rather than what is there
- kind: confusing-semantics
- severity: minor
- where: re/parse.kso `chunk`
- wanted: `text/slice cs 1 4` on a three-element list to give the three
  elements, as a slice does in Python or Go's `s[a:min(b, len(s))]`.
- wrote:
  ```
  fn chunk cs from to
    text/join (text/slice cs from (smaller to (length cs))) ""
  ```
- why it matters: appendix b documents it ("out-of-range or inverted bounds
  yield an empty result"), but a range that merely overhangs is usually a
  request for "up to the end". The bug showed up as an error message quoting
  an empty fragment for `\xZ`, found only by the Go oracle.

### F21: std/expect is in lib/ but not in the compiler
- kind: tooling
- severity: minor
- where: re/match_test.kso
- wanted: `test_x = expect (finds "a|ab" "ab") . to (equal "[[0 1]]")`, so
  that a failing test prints what it got. lib/expect/expect.kso exists and
  its header explains exactly this use.
- wrote: `error: 'std/expect' is not in the shipped library`, so a local
  record and helper that do the same:
  ```
  type differs
    actual
    expected

  fn same actual expected
    if (actual == expected) true (differs actual expected)
  ```
- why it matters: a bare `==` test failure says only "FAILED (returned
  false)". Twenty-one tests failed at once (F22) and the first useful
  information came from writing this helper.

### F22: a function in a test file named like a type in the module is silently a constructor
- kind: engine-bug
- severity: major
- where: re/match_test.kso (`found`); `bugs/fn_named_like_type/`
- wanted: either my helper `fn found pattern subject` to run, or "the name
  `found` is already taken", which is what the checker says when the type
  and the function share a file.
- wrote: the helper renamed to `finds`. Before that, `kanso check re` said
  ok and every test calling `found "a|ab" "ab"` built the record
  `re/found "a|ab" "ab"` declared in re/pike.kso (`found caps steps`), so
  twenty-one tests failed with values that made no sense.
- why it matters: this is F11's collision without the error. A module
  growing a type can silently change what an existing call in a test file
  means.

### F23: an arm for a parameter that can never hold that type is accepted
- kind: diagnostic
- severity: minor
- where: grep/cli.kso `grep`
- wanted: a compile error for
  ```
  fn grep _ (file_not_found path) _
  ```
  where the second parameter is always a compiled regexp and the file
  result arrives third.
- wrote: the arm moved to the right position. The mistake showed up only at
  run time, as `error[runtime]: split takes two strings` with no file or
  line, when a missing file's `file_not_found` reached `text/split`.
- why it matters: the checker can see that the second argument at every
  call is what `re/compile` answers, so the arm can never be chosen. A dead
  arm is almost always a typo, and the runtime message for its consequence
  names neither the function nor the line.

### F24: no escape for a code point in a string literal
- kind: missing-feature
- severity: nit
- where: re/match_test.kso `test_match_fold_kelvin`
- wanted: `"kK\u{212a}"`
- wrote: the Kelvin sign pasted into the source as itself, which looks
  exactly like a K. The error for `\u` is "unknown escape `\u`".
- why it matters: tests of Unicode handling are about characters that look
  alike or are invisible, which is when a source file most needs to spell a
  code point by number.

## What worked well

**Dispatch on literal characters made the parser a grammar table.** Each
operator and escape is one arm, and the file reads like the syntax
reference it implements:
```
fn piece "*" cs at st acc prev
  quantify cs at (at + 1) st acc prev 0 -1

fn piece "+" cs at st acc prev
  quantify cs at (at + 1) st acc prev 1 -1

fn single _ at "n"
  esc (at + 2) 10
```
The same move turned the POSIX class table into fourteen arms of
`fn posix "alpha"`, with `fn posix _` answering none for an unknown name.
Nothing had to be looked up in a map at run time, and adding a class is
adding an arm.

**Immutable maps made capture bookkeeping free.** The hard part of a Pike
VM in C is that each thread owns a capture array that must be copied when a
split forks it and freed when the thread dies. Here a thread's captures are
a map, a split hands the same map to both sides, and a save is
`put caps slot pos`. Sharing is safe because nothing mutates, so the code
that RE2 spends a reference-counted allocator on is one expression:
```
fn follow s pos q (pending caps pc rest) (i_save slot)
  visit s pos q (pending (put caps slot pos) (pc + 1) rest)
```

**Three engines that agree to the byte made the linear-time claim
checkable.** The step counter is part of the program's output, and the
interpreter, the dev build and the release build print the same counts
for every fixture, including `(a*)*b` against 4,000 a's (44,010 steps) and
the backtracker's 655,372 steps at n = 16. A performance claim usually
needs a benchmark harness; here it is an expected-output file.

**An err with a record reason carried syntax errors from deep in the
parser to the CLI with no plumbing.** The parser raises
`err (syntax_error at code text)` wherever it is, and the one function that
renders it dispatches on the shape:
```
pub fn message (err (syntax_error _ code text))
  "error parsing regexp: {code}: `{text}`"
```
F14 is the cost; the benefit is that no layer of the parser has a status
code to check.

**Typesets let one arm cover a family.** `fn emit edge:anchor _` compiles
all six assertions to `[(i_assert edge)]`, and `fn nullable? _:anchor`
answers for all of them. The grouping forced by F1 turned out to be the
grouping the compiler wanted.

**Closures made the backtracker's continuation-passing style direct.** Each
node takes the rest of the match as a lambda, and the lambdas close over
whatever they need:
```
fn try_node (plus greedy body) cps pos steps budget k
  bt body cps pos steps budget (p s -> looping greedy body cps p s budget k)
```

**Absence as data at the file boundary.** `os/read_file` answering
`file_not_found` meant the CLI's missing-file message is one arm, and the
grep exit codes (0, 1, 2) fell out of `os/exit` on the three paths.

**Canonical form removed every layout decision.** I never thought about
where a brace or a blank line goes, and every file in the port looks the
same.

## Summary

The five I would fix first:

1. **F11 and F22, the module-wide namespace.** Twenty-nine renames, a
   silent miscompile when a test helper shared a type's name, and error
   messages about arity and opacity when the real problem was a name
   imported by a different file. This was the most frequent interruption
   by a wide margin, and it got worse as the program grew.
2. **F15, the release build dropping `musttail`.** The program this port
   exists to demonstrate, linear time on long inputs, segfaulted on the
   optimized build only. The workaround (fewer parameters) is easy once
   known and hard to guess from the symptom.
3. **F12, opaque pub records.** A library that returns structured results
   needs a getter per field, so the regex API is a dozen accessor
   functions and matches travel as untyped lists of ints.
4. **F3 and F4, index reads after a bounds check.** Thirty-nine none arms
   in about 2,500 lines. Some do real work (the end of the pattern is a
   none), but many stand in for a bounds check the code had already made,
   and the `xs[i]!` spelling that looks like the fix fails at run time.
5. **F16, no record update.** Every loop in the matcher threads a record
   through itself, and each step rebuilds it field by field in alphabetical
   order.

Writing a regex engine in kanso was mostly pleasant at the level of single
functions and mostly frustrating at the level of the module. Dispatch on
literals is the right tool for a parser, immutable values are the right
tool for a Pike VM, and the three-engine agreement turned "linear time"
from a claim into a fixture. The friction came from the edges: names that
collide across files, records that cannot cross a module boundary, lines
that cannot wrap, and none arms around index reads. None of it blocked the
port; together it meant that many of my edits were renames, reorders and
helper constants that the program did not need, and two problems
(F15 and F22) took real debugging because the toolchain reported them as
something else or not at all.
