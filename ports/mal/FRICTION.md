# FRICTION: porting mal to kanso

A journal kept while writing the port. Entries are in the order I hit them.

## Entries

### F1: a single file with declarations cannot be run, and the book says it can
- kind: tooling
- severity: minor
- where: scratch experiment, first ten minutes
- wanted: the book's shape, `pub play = print (show xs)` beside `fn show ...`,
  run with `kanso run t1.kso` (ch05 and ch06 print every sample this way).
- wrote: a bare statement `print (show xs)` and `kanso play t1.kso`. The three
  attempts produced three different refusals: `run` says the file "is a
  library — nothing to run", `play` with `pub play =` says "`pub play` is a
  library's export ... `kanso play` takes bare statements", and `run` with a
  bare statement says "a file with declarations is a library".
- why it matters: the first program anyone writes after reading chapters 3 to
  6 is a few functions and a line that calls them. Every sample in the book
  that does this is spelled in a way the toolchain refuses, so the reader's
  first contact is a loop through three diagnostics to find the fourth form.

### F2: natively, a map `put` copies the whole map unless the map is provably unique
- kind: performance
- severity: major
- where: bugs/map_put_bound_is_quadratic.kso
- wanted:
  ```
  fn count_up n acc store
    store2 = put store 1 n
    count_up (n - 1) (acc + 1) store2
  ```
- wrote: the same loop with the `put` inlined into the tail call. With the
  binding, n = 8000 takes 5.2 s native and n = 16000 takes 33 to 50 s; the
  interpreter does 16000 in 0.7 s. Inlined, native does 16000 in 0.01 s. A
  map with one key is enough to show it.
- why it matters: an interpreter written in kanso threads its state (an
  environment store, here) through every step, and naming the new state
  before passing it on is the readable way to write that. The first
  measurement I took of the design this port uses spent two minutes on
  300,000 steps. It also inverts the usual advice: the readable form is the
  slow one, and nothing tells you.
- later: the binding is only one way in. A map that sits in a record field
  (`world (put w.atoms id v) ...`) or passes through a helper function
  (`count_up (n - 1) (bump store n)`) is copied by every `put` as well; the
  counters say `put_mut_fast=0` and about 250 KB copied per `swap!` once mal
  had 4,000 atoms. 4,000 swap!s took 3.4 s. I replaced the atoms map with a
  binary trie of records (lisp/env.kso, `cell_put`), which copies one path
  per update, and the same program took 0.08 s. The design this rules out
  is the one I drew first: a store of mutable environments in a map, which
  is how most mal ports handle `def!` and closures. I chose immutable
  environment frames instead (F9), partly for this reason.

### F3: inlining a variable can make a program illegal, and naming it again makes it legal
- kind: confusing-semantics
- severity: major
- where: a scratch file (the code is below), then every combinator in
  lisp/reader.kso
- wanted: a reader whose helpers return `err "EOF"` and let it ride:
  ```
  fn outer n
    wrap (inner n)
  ```
- wrote: either a pass-through arm on every function an err might reach,
  ```
  fn wrap e:err
    e
  ```
  or the same code with the call bound to a name first, which the checker
  accepts and which propagates at run time exactly as the book describes:
  ```
  fn outer2 n
    r = inner n
    wrap r
  ```
- why it matters: `r = inner n; wrap r` and `wrap (inner n)` are the same
  program, and only one compiles (`error[exhaustive]: this can be an err and
  wrap has no arm for it`). The book says so plainly ("the checker reads calls,
  not the names they are bound to"), but the effect in a recursive-descent
  reader is that the cheapest way past the error is to introduce names, which
  is the opposite of what an exhaustiveness check is for. A refactoring tool
  that inlines a variable would break the build.

### F4: no way to write a long string or a long list across lines
- kind: missing-feature
- severity: minor
- where: lisp/repl.kso:32-48 (the mal prelude), lisp/core.kso:8-29 (builtin
  names)
- wanted: the prelude as mal source, one definition per line, or at least a
  list literal that wraps:
  ```
  prelude = [
    "(def! not (fn* (a) (if a false true)))"
    "(defmacro! cond (fn* (& xs) ...))"
  ]
  ```
- wrote: one constant per fragment, glued with interpolation, because a string
  literal cannot contain a newline (`error[syntax]: unterminated string`), a
  list literal cannot continue onto a second line (`expected an expression`),
  and every line stops at 80 columns:
  ```
  cond_src = "{cond_head} {cond_then} {cond_odd} {cond_rest}"

  cond_head = "(defmacro! cond (fn* (& xs) (if (> (count xs) 0)"
  ```
  The builtin-name table became six constants joined with `text/concat`.
  My first attempt glued the strings with `. text/concat "..."`, which
  appendix B documents for strings and the runtime refuses
  (bugs/text_concat_refuses_strings.kso).
- why it matters: an interpreter, a template engine or a test with an
  expected document all want text longer than 80 columns in the source. With
  an 80-column rule and no continuation for literals, the only spelling left
  splits a definition at arbitrary points and names each piece, and the
  constants must then be read in alphabetical order rather than source order.

### F5: one namespace for a module, its imports and the ambient builtins
- kind: refactoring-hazard
- severity: major
- where: lisp/core.kso (field `entries`), lisp/eval.kso (`then_form`),
  lisp/repl.kso (`tally`, lambda parameter `more`), lisp/env.kso (`hit`),
  lisp/lisp_test.kso (`shown`)
- wanted: names chosen for the reader of one file: a record field called
  `entries`, a helper called `hit`, a counter type called `tally`, a lambda
  parameter called `more`.
- wrote: `table`, `found_in`, `score_card`, `ls`, `outcome_text`. Each
  collided with something I could not see from the file I was editing:
  - `entries` is ambient: ``error[name]: `entries` is already a declaration;
    rename the binding`` on a field pattern.
  - `then_form` was a function in reader.kso; a field of the same name in a
    pattern in eval.kso is refused.
  - `more` was a function in reader.kso; a lambda parameter in repl.kso was
    refused.
  - adding a helper `shown` to the test file broke two local bindings called
    `shown` in printer.kso, a file the change did not touch.
  - `tally` comes from `import "std/list"` and `hit` from
    `import "std/regexp"`, because imported pub names join the short-name
    space. The diagnostics did not say so: ``no 2-argument arm of `tally`
    (arms take 1)`` and `` `hit` has 3 field(s), got 4 ``.
- why it matters: adding an import, or a helper in any file of the module,
  can break a lambda parameter in another file. The two import cases are the
  worst, because the message talks about arity and the real cause is a name
  that appeared from a module I imported for something else.

### F6: a parameter named after a marker is silently a literal pattern
- kind: confusing-semantics
- severity: major
- where: lisp/eval.kso, `fn ret (k_args done env i items k) v w` (before the
  rename); reduced in bugs/marker_name_as_pattern_binder.kso
- wanted: a continuation record with a field `done` holding the values
  evaluated so far, destructured in the arm head.
- wrote: the field renamed to `acc`. With `done`, the program checked clean
  and every function call failed at run time with ``error[runtime]: no
  overload of `mal/ret` matches these arguments``. `done` is the nullary the
  standard library declares for "an effect finished", so in a pattern it is
  the literal `done`, and the arm only matches when the field holds `done`.
- why it matters: the same token is a binder or a literal depending on
  whether some module somewhere declared a marker by that name, and the
  checker says nothing. It cost me the first debugging session of the port;
  the runtime error names the function but not the arm or the argument that
  failed to match. A warning when a pattern position names a marker that the
  arm's own type would never hold, or a sigil for literals, would have saved
  it.

### F7: every list index is a none the checker wants handled
- kind: aesthetics
- severity: minor
- where: 40 sites across lisp/eval.kso, lisp/core.kso, lisp/reader.kso,
  lisp/repl.kso
- wanted: `ev items[i] env (k_args all env (i + 1) items k) w`, inside a
  function whose guard has just checked `i <= length items`.
- wrote: `ev (arg items i) env ...` with
  ```
  fn arg xs i
    or_nil xs[i]

  fn or_nil none
    nil

  fn or_nil v
    v
  ```
  and the same pair again as `char_at`/`or_empty` for the reader and
  `line_at` for the test runner.
- why it matters: the check is right in general and wrong at most sites in an
  interpreter, where the bound was checked one line up. The `!` form answers
  an effect, so it is not the "I know it is there" spelling that pure code
  wants. What I wanted is a pure strict index that fails as an err; what I
  have is three copies of a none-to-default helper.

### F8: native memory grows with every step of the interpreter loop
- kind: performance
- severity: major
- where: lisp/eval.kso (the machine), measured with `--counters`
- wanted: a mal loop that runs in constant space,
  `(def! sum2 (fn* (n acc) (if (= n 0) acc (sum2 (- n 1) (+ n acc)))))`, to
  run in constant native memory, since the machine's continuation does not
  grow and each step's frame is dead after the step.
- wrote: nothing that fixed it. The counters for `(sum2 10000 0)` and
  `(sum2 40000 0)`:
  ```
  alloc_bytes=37853040   arena_peak_bytes=37748736   beat_iters=3
  alloc_bytes=151133040  arena_peak_bytes=148897792  beat_iters=3
  ```
  Every byte allocated stays until exit: about 3.7 KB per mal iteration, so
  a million iterations would hold 3.7 GB. I rewrote the machine on a scratch
  copy as a trampoline, so that one function `run (step state)` is the only
  loop, which is the shape that gets a beat in a toy (`beat_iters=200001`,
  arena flat at 1 MB, with a closure, a map and a growing record in the
  state). The mal machine still got `beat_iters=3`. In the emitted IR the
  trampoline is a clean self-recursive function, `d_lisp/~run_1`, ending in
  `musttail call ... @"d_lisp/~run_1"`, with no `k_beat_iter` in it; the
  only beats in the whole binary are in small self-recursive helpers
  (`cases_of`, `expect_end`, and std/regexp's parser). Nothing says why the
  beat analysis declined the machine, so I cannot tell which of my choices
  to undo. `(fib 25)` on the release build: `alloc_bytes=796424928`,
  `arena_peak_bytes=796917760`, `held_peak_bytes=2192`. The live data never
  passes 2 KB; the process holds 800 MB.
- why it matters: any long-running program whose loop is an interpreter, a
  state machine or an event loop is in this position, and the failure mode is
  an out-of-memory kill rather than slowness. mal-in-mal running the step 5
  suite was killed this way before I fixed an unrelated bug, and the kill gave
  no hint of which. A `kanso check` note naming the loops that did not get a
  beat, and the value that stopped them, would make this a refactoring task
  instead of a guess. The interpreter engine does not have the problem.

### F9: no reference cells, so environments and atoms are rebuilt by hand
- kind: missing-feature
- severity: major
- where: lisp/env.kso, lisp/eval.kso (`knot_fn`, `found_in`, `do_def`)
- wanted: what every other mal port writes, an environment object that
  `def!` mutates and closures share:
  ```
  class Env: def set(self, k, v): self.data[k] = v
  ```
- wrote: three separate mechanisms, because kanso values never change and
  there is no cell type:
  1. globals live in a map inside a `world` record that every machine step
     takes and returns, so a top-level `def!` is seen by closures made earlier;
  2. local frames are immutable `frame outer vars` records, and a closure bound
     by `let*` (or by a `def!` inside a `do`) stores the marker `knot` as its
     environment. Looking the name up puts the frame back in:
     ```
     fn found_in (closure body knot macro meta params rest) _ here _
       closure body here macro meta params rest
     ```
     That is what makes `(let* (f (fn* ...) g (fn* ... (f n))) ...)` mutual
     recursion work without a cycle in the data;
  3. atoms are ids into a trie in the world (F2).
- why it matters: the knot trick is correct for the mal test suite but it is
  a semantic approximation I had to invent and then prove to myself: a
  `def!` inside a nested `if` in a function body binds nothing, where every
  other mal binds it in the call's frame. A `build` block (ch03) ties knots
  at construction time, but only for a shape known statically, and an
  interpreter's environments are built at run time. Some sanctioned way to
  hold a mutable cell inside an effect, or a documented idiom for "a store
  threaded through a loop" that the compiler keeps in place, would remove
  most of this file.

### F10: adding a field to a record means editing every construction
- kind: refactoring-hazard
- severity: minor
- where: lisp/env.kso (`define`, `emit`, `without_output`, `new_atom`,
  `set_atom`, `next_id`), lisp/machine_types.kso (`k_hmap`)
- wanted: a record update, `{ w | out = [] }` or `w with out = []`.
- wrote: one helper per field, each spelling the whole record out in
  alphabetical field order:
  ```
  fn without_output w
    world w.atoms w.globals w.next_id []
  ```
  When `k_hmap` needed the source map as well as the keys, its fields went
  from `done env i keys k meta` to `acc entries env i k meta names`
  (alphabetical, and two renamed, F5 and F6), and every `k_hmap ...` call had
  to be rewritten positionally to match. The checker catches a wrong count,
  but two fields of the same type swapped are silent.
- why it matters: construction is positional and the positions are decided by
  the alphabet, so the order of arguments at a call site is unrelated to what
  the code means, and an insertion in the middle shifts every argument after
  it. The world record is the thing every step of the interpreter rebuilds.

### F11: arm order across several parameters is hard to predict
- kind: diagnostic
- severity: minor
- where: lisp/printer.kso `pr_str`, lisp/repl.kso `check_form`
- wanted: the printer as one overload group,
  ```
  fn pr_str v:int _ _
  fn pr_str s:string true _
  fn pr_str s:string false _
  fn pr_str nil _ _
  fn pr_str true _ _
  ```
- wrote: four reorderings, each answered by `overloads of pr_str appear
  most-specific first`, and then `these pr_str arms tie: each is the more
  specific one somewhere` for the string arms, which I resolved by moving the
  `readably` flag into a helper (`string_form s readably`). Along the way I
  learned that `true` and `false` count as literals but my own marker `nil`
  counts as a concrete type, so `true _ _` must precede `s:string true _`,
  which must precede `nil _ _`. In repl.kso the tie was real: `check_form
  e:err _` and `check_form form read_back` both match an err read in
  read-back mode, and the checker was right to refuse it.
- why it matters: the ladder is simple for one parameter and opaque for
  three. The tie diagnostic is the useful one; the ordering one names the
  rule but not which earlier arm the flagged arm must move above.

### F12: io/stdin is all of stdin, so the REPL is not interactive
- kind: stdlib-gap
- severity: major
- where: lisp/repl.kso `main`, `settle (wants_line ...)`
- wanted: read a line, evaluate it, print, read the next line, as mal's
  `readline` and every REPL do.
- wrote: `io/stdin .> (input -> repl (input_lines input) 1 (boot []))`, which
  waits for end of input before evaluating anything. Measured:
  `(echo "(+ 1 2)"; sleep 2; echo "(+ 3 4)") | ./mal` prints both answers
  together after two seconds. mal's `readline` builtin reads from the same
  pre-split list, so a mal program that prompts a user works only on piped
  input.
- why it matters: the project brief says "a REPL over stdin", and the result
  passes every transcript fixture while being unusable at a terminal. std/io
  has `stdin`, `write` and `write_err` and nothing that reads less than
  everything.

### F13: the runtime's "no overload matches" names no argument
- kind: diagnostic
- severity: minor
- where: lisp/eval.kso `ret`, first run of the REPL (see F6)
- wanted: which arm group, and the shape of the value that matched no arm.
- wrote: bisected by hand with `echo '(list 1)' | kanso run . --interp` and
  friends until `(list)` worked and `(list 1)` did not.
- why it matters: the message was
  ``error[runtime]: no overload of `mal/ret` matches these arguments``. `ret`
  has seventeen arms and three parameters; printing the record tag of each
  argument would have pointed at `k_args` in one step.

### F14: "this can be an err" with no path back to the err
- kind: diagnostic
- severity: minor
- where: lisp/repl.kso `settle`, `settled_world`
- wanted: to know which function in the machine can answer an err, so I
  could handle it where it is born.
- wrote: pass-through arms, because the checker reported the machine's entry
  point (``this can be an err and `settle` has no arm for it``) and nothing
  beneath it:
  ```
  fn settle e:err _ _ _
    e
  ```
  I added probe functions that wrap `ev`, `ret`, `builtin`, `apply_fn` and
  `throw` in a one-arm function, and none of them was flagged, so I still do
  not know where the err the checker sees comes from. A related surprise in
  lisp/lisp_test.kso: a helper written `fn reason_of e:err` left
  `read_failure "(1 2" == "EOF"` refused with `this can be an err and ==
  wants a value`, and the same helper written `fn reason_of (err r)` was
  accepted. In a ten-line scratch file the two spellings behaved the same, so
  I could not reduce it.
- why it matters: the exhaustiveness rule asks me to dispatch on the err, and
  the honest response is to find its source. Without a trail the cheapest
  compliant edit is an arm that hands the err on unread, which is what the
  rule exists to prevent.

### F15: a long test constant has to grow a throwaway binding
- kind: aesthetics
- severity: nit
- where: lisp/lisp_test.kso (`test_closures_capture`, `test_errors_are_caught`
  and three more)
- wanted:
  ```
  test_errors_are_caught =
    shown_of "(try* (throw \"no\") (catch* e (str e \"!\")))" == "\"no!\""
  ```
- wrote: the source string bound to a name first, because the form above is
  refused (`a single-expression constant is written inline`) and the inline
  form is 87 columns (`a line holds at most 80 characters`):
  ```
  test_errors_are_caught =
    src = "(try* (throw \"no\") (catch* e (str e \"!\")))"
    shown_of src == "\"no!\""
  ```
- why it matters: two layout rules that are each reasonable leave exactly one
  legal spelling, and it adds a name nobody wanted. `==` is not a
  continuation token, so there is no way to wrap the comparison itself.

### F16: braces in a mal string are interpolation in a kanso string
- kind: diagnostic
- severity: nit
- where: lisp/lisp_test.kso `test_reader_keeps_vectors_and_maps`
- wanted: `round_trip "[1 (2 \"three\" :four) {\"k\" [nil true false]}]"`
- wrote: `\{` for every mal map literal inside kanso source. The error for
  the unescaped form was ``error[syntax]: unexpected character `\` `` with the
  caret on the escaped quote after the brace, because the brace had opened an
  interpolation and `\"` is not legal inside one.
- why it matters: an interpreter's tests are full of source text, and mal's
  map literal is a brace. The rule is fine; the message describes the
  symptom two characters later rather than the brace that caused it.

### F17: the binary is named after the directory, with no way to rename it
- kind: tooling
- severity: nit
- where: the project root
- wanted: the module directory called `mal`, since the project is mal, and
  `kanso build .` writing a binary called `mal` beside it.
- wrote: the module directory renamed to `lisp`, after `error: this build is
  named mal, and a directory of that name is here`. There is no `-o`; check.sh
  moves `./mal` to `build/mal-dev` and `build/mal-release` after each build.
- why it matters: the obvious name for a project's main module is the
  project's name, which is also the binary's name.

### F18: `== false` is refused, and mal's false? needs exactly that
- kind: aesthetics
- severity: nit
- where: lisp/core.kso `false?`, `true?`; lisp/eval.kso `falsy?`
- wanted: `ret k (arg args 1 == false) w`
- wrote: ``error[name]: comparing to `false` asks a question the value already
  answers — write `not` the value``, so
  ```
  fn false? false
    true

  fn false? _
    false
  ```
  `not` is not what mal means: `(false? nil)` is false, and `not` on my
  `nil` marker stops the program with ``error[runtime]: an if condition is
  true or false, got t19/nil`` (checked in a scratch file; the message talks
  about an `if` the program never wrote).
- why it matters: the lint assumes the value is a boolean. In an interpreter
  the value is anything, and comparing it with `false` is identity, not a
  redundant test. The dispatch version is fine; the message's advice would
  have been wrong.

## What worked well

**Dispatch on literals reads like a table.** The special forms and the core
namespace are overload groups keyed by a literal inside a record pattern, and
there is no `if` chain anywhere in the evaluator:
```
fn ev_list (sym "def!") items env k w
  ev (arg items 3) env (k_def env k (name_of (arg items 2))) w

fn builtin "cons" args k w
  ret k (mlist (text/concat [(arg args 1)] (seq_items (arg args 2))) nil) w
```
Adding `quasiquoteexpand` was one arm, written next to its siblings. In
Python this would be a dict of handlers or an `elif` ladder.

**Guaranteed tail calls made the evaluator simple.** Every step of the CEK
machine (`ev`, `ret`, `throw`, `apply_fn`) calls the next in tail position,
and all three engines run that in constant stack. mal's own tail-call
requirement then costs nothing: a call in tail position reuses the caller's
continuation, and `(sum2 10000 0)` and ten-thousand-deep mutual recursion
pass on the interpreter as well as natively. Most mal ports in languages
without this write an explicit `while True` loop with reassignment.

**A field reader that works across record types.** Unwinding to the nearest
`try*` is three arms, the last of which reads the `k` field of whatever
continuation frame it was given, across seventeen record types:
```
fn throw halt e w
  threw e w

fn throw (k_try env handler k name) e w
  ev handler (extend (new_frame env) name e) k w

fn throw frame_k e w
  throw frame_k.k e w
```

**Effects as values fit an interpreter well.** The machine is pure; when it
needs a file, a line of input or the time it stops with a record
(`wants_file k path w`) and the driver performs the effect and resumes it.
That one 30-line `settle` serves the REPL, the file runner, the runtest
runner and the mal-in-mal runner, and the test runner needs no mocking.

**The three engines agreed on everything.** All 33 fixtures, including the
mal-in-mal runs, produced byte-identical output on the interpreter, the dev
build and the release build on the first run of check.sh. I did not meet a
single engine divergence in the port.

**Immutable data removed a class of mal bugs.** mal's tests check that `vec`,
`cons`, `concat`, `assoc` and `with-meta` leave their arguments untouched,
and that `defmacro!` does not mark an existing function as a macro. Those are
classic bugs in mutable ports; here they pass without any care taken.

**Several diagnostics told me the fix.** ``pr_str takes 3 argument(s), and a
list element is one atom ... Write `(pr_str …)` to call it``; ``a lambda is
parenthesised: `f = (x -> …)` ``; `` `m/world` is foreign — only `m` builds a
`world`; ask it for one through a pub function``; and the dispatch tie in
`check_form` (F11) found a real ambiguity in my test runner.

**Integers without a ceiling.** mal's numbers are kanso ints, so
`(* 99999999999 99999999999)` and `(+ 9223372036854775807 1)` are exact on
every engine with no code in the port (fixtures/repl/numbers.in). A C or Go
port overflows there.

**Native speed.** `(fib 20)` in mal takes 0.08 s on the release build against
3.5 s on the interpreter, and step 5's suite runs in a quarter of a second.
`kanso check .` on the whole port takes 0.13 s.

## Summary

The five I would fix first, in order:

1. **F8, native memory is never reclaimed in the interpreter loop.** The live
   data stays under 2 KB while `(fib 25)` holds 800 MB, and nothing tells the
   author which construct stopped the beat analysis. This makes any
   long-running interpreter, server or simulation unsafe natively.
2. **F2, `put` on a map that is not provably unique copies the whole map.**
   It turned a store-of-environments design (the usual one for mal) from
   linear to quadratic, and drove the choice of immutable frames and a
   hand-written trie for atoms.
3. **F6, a parameter named after a marker is a literal pattern.** It checks
   clean and fails at run time with a message that names neither the arm nor
   the marker; one stray `done` cost the first debugging session.
4. **F12, no line-at-a-time stdin.** The REPL passes every transcript test and
   cannot be used at a terminal.
5. **F5, one namespace across files, imports and builtins.** A helper added
   in one file, or an import added for one function, breaks a binding in
   another file, sometimes with an arity message about the wrong thing.

Writing mal in kanso was mostly pleasant once the evaluator had the right
shape. The language pushed me, correctly, toward an explicit-continuation
machine, and that shape made tail calls, exceptions and suspended I/O fall out
of the same few records; the dispatch tables for special forms and builtins
are the clearest code in the port. The friction came from three directions.
The first was the absence of any mutable cell, which meant inventing a way to
tie recursive `let*` closures to their frame and threading a world record
through every step. The second was the formatting and naming rules, which
are individually reasonable and together cost a dozen compile cycles per
file: arm order, 80 columns with no way to continue a literal, a namespace
shared with every import. The third was the native runtime, which ran the
finished interpreter fast but held every byte it ever allocated, for reasons
I could measure and could not diagnose.
