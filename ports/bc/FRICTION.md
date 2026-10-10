# FRICTION: porting GNU bc to kanso

A journal kept while porting GNU bc 1.07 to kanso. Entries are in the order I
hit them. Every code sample was compiled with the compiler on main
(`/tmp/claude-0/kanso-main/kanso`, 2026-10-10). Line references are to the
port as it stands at the end; where an entry's code is gone because I worked
around it, the entry says so.

## Entries

### F1: a strict index inside a call passes `check` and fails at run time
- kind: diagnostic
- severity: minor
- where: scratch program while measuring maps (not in the port)
- wanted: `print "{mp/spin m (text/to_int a[1]!)}"`, or a compile error
  telling me `a[1]!` is a box.
- wrote: `os/args .> (a -> a[1]!) .> (s -> ...)`
- why it matters: `kanso check` said `ok`; the run said
  `error[runtime]: to_int takes a string, bytes, or int, not <io>`. Chapter
  05 promises that handing a box to something that wants a value is refused
  at compile time, and for `length os/args` it is. For a stdlib function
  whose parameter is unannotated it is not, so the promise depends on which
  function you happen to call.

### F2: there is no multi-line string, and `{` in a string must be escaped
- kind: missing-feature
- severity: major
- where: `bc/mathlib.kso`, `embed_mathlib.sh`
- wanted: the `-l` library, which is 150 lines of bc, as one literal:
  ```
  library_source = """
  define e(x) {
    ...
  """
  ```
- wrote: the library lives in `lib/math.bc`, and a shell script generates a
  kanso file holding one `. push "..."` line per bc line, with every `{`
  written `\{`, every `"` and `\` escaped, and comments dropped so each line
  fits in 80 columns. check.sh fails if the generated file is stale.
- why it matters: embedding a script, a template or a test input is common,
  and here it needs a code generator. bc source is full of braces, and an
  unescaped one is read as the start of an interpolation (`error[syntax]:
  unterminated interpolation`), so even a single line of bc needs editing to
  become a kanso string. The `. push` chain also trips `needless
  continuation` when the library is short enough to fit on one line.

### F3: no name may repeat anywhere in a module, so locals collide with functions in other files
- kind: refactoring-hazard
- severity: major
- where: every file in `bc/`; the renames are listed below
- wanted: a local `one` in the parser while `num.kso` exports `one`; a local
  `after` in `machine.kso` while `exec.kso` has a function `after`.
- wrote: eleven renames while writing the port, each found by a fresh
  `error[name]: \`x\` is already a declaration; rename the binding`:
  `one` -> `item`, `power` -> `strength`, `padded` -> `pad_to`,
  `units` -> `run_units`, `after` -> `after_statement`,
  `gathered` -> `collected`, `word` -> `arg`, `formals` -> `every`,
  `values` -> `given` (the ambient `values`), `written` -> `backed`,
  `source` -> `reading`, and `step` (F4). A constant in a test file counts
  too (F20).
- why it matters: a module is one namespace, so naming a helper in one file
  can break a function body in another that the author never opened. Adding
  `fn units` to `run.kso` broke `dec places units = n` in `digits.kso`. In a
  larger module every new top-level name is a search of every file for
  locals that happen to share it.

### F4: importing std/list makes `step` a type in my module
- kind: diagnostic
- severity: minor
- where: `bc/run.kso`, `take_unit` (it was called `step`)
- wanted: a private `fn step u label m k`
- wrote: renamed to `take_unit`
- why it matters: the error was `error[arity]: \`step\` has 2 field(s), got
  4 (construction is positional, fields alphabetical)` on a call to my own
  four-argument function. std/list has a `pub type step`, and an imported pub
  joins the short-name space, so my function call was read as a construction
  of std/list's record. Nothing in the message mentions std/list.

### F5: a native map keeps every write, so re-putting a key you already have is quadratic
- kind: performance
- severity: major
- where: `bc/parser.kso`, `with_scalar`, `with_array`, `with_func`
- wanted: the obvious symbol table, written on every sight of a name:
  ```
  fn with_scalar (ps arrays funcs pos scalars toks voids) name
    found = scalars[name]
    slot = if (found == none) (length scalars + 1) found
    got slot (ps arrays funcs pos (put scalars name slot) toks voids)
  ```
- wrote: the same, returning early when the name is known, so `put` only
  runs on a new name.
- why it matters: the version above re-puts the same key with the same value.
  A script of 4,000 lines of `x=1` took 2.1 s natively and allocated 403 MB
  (1,000 lines: 28 MB; 2,000: 100 MB); after the change, 0.08 s and 29 MB.
  The interpreter was superlinear too, though less. Measured separately,
  overwriting one key 10,000 times in a loop takes 6.4 s natively and
  40,000 times 109 s, while the interpreter does 160,000 in 0.34 s. Because
  of this I kept bc's variables in lists indexed by slot numbers resolved at
  parse time, and never in a map.

### F6: the checker wants an err arm for a call, but not for the same call bound to a name
- kind: refactoring-hazard
- severity: major
- where: `bc/parser.kso` (for loops, definitions, `closed = expect ...` in a
  dozen places), `bc/exec.kso` (`r = exec body c.machine`)
- wanted:
  ```
  check = optional_expr (expect first.ps ";") ";"
  while_next (exec body c.machine) body check
  ```
- wrote:
  ```
  after_first = expect first.ps ";"
  check = optional_expr after_first ";"
  r = exec body c.machine
  while_next r body check
  ```
- why it matters: `error[exhaustive]: this can be an err and \`optional_expr\`
  has no arm for it`. The book says the checker "reads calls, not the names
  they are bound to", and that is exactly what happens: introducing a name
  makes the error go away and changes nothing at run time. So inlining a
  binding, the most ordinary refactoring there is, can turn a compiling
  program into one that does not, and the fix the message suggests (an err
  arm on every parser function) is not what anyone does. I added about
  twenty such bindings. The same holds for none: `text/to_int a[2]` is
  refused and `given = a[2]` then `text/to_int given` is not. Some of these
  diagnostics also come without the source line and with the module prefix
  on the function name (`bc/loop_for`), unlike every other error.

### F7: one err read through two fields becomes a list of two errs
- kind: confusing-semantics
- severity: major
- where: `bc/eval.kso` `ev`, `bc/parser.kso` `got` and `make_*`,
  `bugs/err_read_twice_merges.kso`
- wanted: the railway the book describes. An evaluation step answers
  `evaluated machine value` or an err; the next step reads both fields:
  ```
  r = eval operand m
  evaluated r.machine (negate r.value)
  ```
- wrote: every two-field result is built through a function instead of the
  constructor, because a function call keeps the first failure and a
  constructor keeps all of them:
  ```
  pub fn ev m v
    evaluated m v
  ```
  and the same for `parsed` (`got`), `unit`, `for_stmt`, `if_stmt`,
  `while_stmt`, `definition`, `call` and `element`.
- why it matters: with the constructor, a bc runtime error that crossed one
  `evaluated r.machine (...)` arrived at the top as `[fault fault]`, so my
  `(err (fault m message))` arm no longer matched it and the program died
  with `unhandled err reached the executor` and a page of machine state. Each
  level doubles the list, so an error under a recursion 30 calls deep would
  be a list of 2^30 copies. The book's rule is "a call keeps the first
  failure; an operator keeps both"; a constructor looks like a call and acts
  like an operator.

### F8: the wrappers F7 needs are then held to rules the constructors were not
- kind: confusing-semantics
- severity: minor
- where: `bc/ast.kso` `absent`, `bc/machine.kso` `undefined`
- wanted: `got (return_stmt none) s`, `make_if check none yes`,
  `slot_get m.funcs slot none`
- wrote: a marker `absent` for a missing statement part, and `undefined` for
  a function slot whose `define` has not run
- why it matters: the constructors took `none` and a raised err in any field.
  The moment they were wrapped in a one-line function, the checker demanded
  a `none` arm and an `(err _)` arm on the wrapper (`this can be a none and
  \`got\` has no arm for it`). So the workaround for F7 needed its own
  workaround: new marker types, and another round of F6 bindings.

### F9: a comment cannot stand on its own
- kind: aesthetics
- severity: nit
- where: every file; the headers became part of the first declaration's
  comment
- wanted:
  ```
  # --- expressions ---

  pub type arith
  ```
- wrote: no section headers. A file-level comment is joined to the first
  declaration's comment with a `#` line between.
- why it matters: `error[formatting]: the file may not begin with a blank
  line`, reported at the blank line after a comment. A 600-line parser has
  natural sections (cursor, units, statements, definitions, expressions) and
  nowhere to name them.

### F10: a type must sit at the top of its file, away from the functions that use it
- kind: aesthetics
- severity: nit
- where: `bc/parser.kso` `misplaced`, `bc/machine.kso` `laid_out`
- wanted: a small private record declared next to the three functions that
  build and read it, at the bottom of the file.
- wrote: moved to the top. `canonical order places type declarations before
  functions; move \`misplaced\` up`
- why it matters: a helper type and its helpers now live 600 lines apart.

### F11: a record with a dozen fields, and no way to change one
- kind: missing-feature
- severity: major
- where: `bc/machine.kso`, all the setters
- wanted: `m with { scale: 5 }`, or anything like it
- wrote: a setter per field, each a positional pattern over all twelve
  fields, with the field names shortened so the line fits in 80 columns:
  ```
  pub fn set_scale (machine ar col fnm fs ib ls ns ob out _ vs wd) sc
    machine ar col fnm fs ib ls ns ob out sc vs wd
  ```
  `_` is not allowed in a binding pattern, only in a parameter pattern, so
  every setter is written as a function head rather than a destructuring
  line.
- why it matters: an interpreter's state is one record that every step
  changes a little. Twelve names chosen for width (`ls` for `last`, `ns` for
  `notes`) are the price of the 80-column rule meeting positional
  construction. Adding a field to the parser's state (`voids`, for bc's void-call check) meant
  editing eleven patterns and constructors; the compiler found them, but it
  was eleven edits for one field.

### F12: `list/zip` hands back two-element lists, and a parameter cannot take one apart
- kind: missing-feature
- severity: nit
- where: `bc/calls.kso`, `write_backs`
- wanted: `fn back_of [p a]`
- wrote: zip dropped; the indices are walked instead, `p = params[i]`,
  `a = args[i]`
- why it matters: `error[syntax]: expected a parameter pattern`. The standard
  library produces pairs as lists, and the language has no pattern for them.

### F13: a keyed read of every field passes `check` and fails at run time
- kind: diagnostic
- severity: minor
- where: `bc/run.kso` and `bc/calls.kso` (now `other.machine`)
- wanted: `{ machine:m } = other`, where `other` is `ran`, `broke`, ...,
  each a record with the single field `machine`
- wrote: `m = other.machine`
- why it matters: `kanso check` said ok, and the program stopped on its first
  statement with `error[runtime]: a keyed read omits at least one field;
  reading every field is the positional form`. That is a formatting rule; it
  should be found where the other formatting rules are.

### F14: the checker calls two arms on different record types a tie
- kind: diagnostic
- severity: minor
- where: `bc/parser.kso`, `misplaced_in`
- wanted:
  ```
  fn misplaced_in (break_stmt line) false _
    misplaced line "Break outside a for/while"

  fn misplaced_in (return_stmt line _) _ false
    misplaced line "Return outside of a function."
  ```
- wrote: one arm per statement type with an `if` on the flag inside it
- why it matters: `error[dispatch]: these \`misplaced_in\` arms tie: each is
  the more specific one somewhere, and a call could match both`. No call can
  match both, because the first argument is a `break_stmt` in one and a
  `return_stmt` in the other.

### F15: native builds corrupt memory in loops that carry the parser state
- kind: engine-bug
- severity: blocker
- where: `bc/parser.kso` (`formals`, `skip_newlines`, `skip_separators`,
  `recover`, `statement_list`, `block_body`, `print_items`, `arguments`),
  `bc/calls.kso` `gather`; `bugs/loop_list_clobbered/`,
  `bugs/beat_carry_parser_state/`, `bugs/beat_rewind_crash/`
- wanted: the ordinary loop shape, a function that calls itself with the
  parser state and an accumulator:
  ```
  fn formals acc s closer
    ...
    item = formal_of s
    more = push acc item.node
    k2 = kind_at item.ps
    return formals more (advance item.ps) closer if k2 == ","
    return got more (advance item.ps) if k2 == closer

  fn skip_separators s
    k = kind_at s
    return skip_separators (advance s) if k == "newline" or k == ";"
    s
  ```
- wrote: list builders that recurse and build on the way back out
  (`got (text/concat [item.node] rest.node) rest.ps`), and skips that scan
  integer positions in the token list and move the cursor once:
  ```
  fn skip_separators s
    moved_to s (past_kinds s.toks s.pos "newline" ";")
  ```
- why it matters: three separate failures, all native-only, all gone when
  the loop stopped carrying the parser state (a record holding the token
  list and three maps). A function with one parameter and four autos failed
  with `no overload of \`bc/current_of\` matches these arguments`, because
  the autos list read back corrupted after the body was parsed. Other inputs
  segfaulted in `k_beat_rewind_slow` freeing a wild pointer. The worst came
  from a random test: a math-library session that hung computing an
  enormous power, then, after an unrelated edit, segfaulted with valgrind
  showing a parsed constant whose `digits` string had become the integer
  0x51. The emitted `.ll` shows the skip loops as beat loops
  (`k_beat_iter_carry`); rewriting them took the crash away with no other
  change. Each failure depends on allocation layout: deleting any one line
  of the input, feeding it as a file instead of on standard input, or
  copying the snapshot to another directory can hide it, and a small
  standalone program carrying a map through a beat loop did not reproduce
  it. The interpreter was right every time, and `check.sh` was green on all
  three engines while the native builds still had this in them; only
  differential testing against GNU bc found the last two. I spent several
  hours on it, and I cannot say for certain that no loop left in the
  port has the shape.

### F16: a keyed read of an err stops the program, where a dot read passes it on
- kind: engine-bug
- severity: major
- where: `bc/eval.kso` (`scale(e)`), `bc/num.kso` `scale_of`;
  `bugs/keyed_read_of_err.kso`
- wanted: `{ scale:places } = r.value`, where `r` may be the err of a
  failed evaluation that the railway should carry to the top of the unit
- wrote: `scale_of r.value`, a one-line function whose parameter pattern
  lets an err pass:
  ```
  pub fn scale_of (dec places _)
    places
  ```
- why it matters: `r.value.scale` and a function call both carry an err
  along; the keyed read instead stops the program with
  `cannot read fields of err ...; keyed reads take a record` on the
  interpreter, and in a native build of the port it was a segmentation
  fault (the standalone reproduction reports a stack overflow instead).
  `scale(1/0)` was enough to kill bc. Of the three ways to read a field,
  one silently opts out of the failure model.

### F17: a native loop that carries a record never gives memory back
- kind: performance
- severity: major
- where: `bc/exec.kso` `loop_while`, `loop_for`; the whole interpreter
- wanted: `for (i = 0; i < 1000000; i++) s += i` in constant memory, as GNU
  bc runs it (0.6 s, a few hundred kilobytes)
- wrote: nothing; I measured and left it
- why it matters: each iteration builds a new machine record, and the
  native arena keeps all of them until the statement ends: 180 MB at
  100,000 iterations, 535 MB at 300,000, about 1.8 KB an iteration
  (`KANSO_COUNTERS`: `arena_peak_bytes` equal to `alloc_bytes`,
  `beat_iters=4`). The compiler only rewinds the arena in loops whose
  carried values are scalars or untouched arguments (src/beat.rs), and an
  interpreter's state is neither. Rewriting the loops as direct
  self-recursion changed nothing. A toy loop carrying a record rebuilt each
  iteration stayed flat at 10 MB when it called itself directly, and grew
  when two functions called each other, so the rule is subtle enough that I
  could not predict which of my loops it applied to.

### F18: the engines run out of stack at different depths, and native says nothing
- kind: engine-bug
- severity: minor
- where: any recursive bc function; `README.md` divergences
- wanted: `define f(n) { if (n < 2) return (1); return (n * f(n - 1)) }`
  to reach the same depth on every engine, or fail the same way
- wrote: nothing; documented
- why it matters: GNU bc ran f(20000). The interpreter stops between 2,000
  and 2,500 bc calls with `error[runtime]: the program ran out of stack`; a
  native build runs f(5000) and segfaults at f(8000) with no message. The
  book says the interpreter's limit is deliberate, but a program that passes
  on the native engines and fails on the oracle, or the reverse, is the
  disagreement check.sh exists to catch.

### F19: standard input arrives all at once, so bc cannot be interactive
- kind: missing-feature
- severity: major
- where: `bc/run.kso` `run_stdin`; `read()` in `bc/eval.kso`
- wanted: read a line, run it, print, read the next line: what `bc` does at
  a terminal, and what `read()` needs
- wrote: `io/stdin .> (c -> run_text "(standard_in)" c tables m finish)`.
  `read()` raises a runtime error.
- why it matters: `io/stdin` yields the whole of standard input as one
  string, and std/io has no line reader. A calculator that prints nothing
  until you press Ctrl-D is not a calculator. The port is fine for scripts
  and pipes; the interactive half of bc cannot be written.

### F20: test constants are one line, inside the module's namespace
- kind: tooling
- severity: minor
- where: `bc/*_test.kso`
- wanted:
  ```
  test_dynamic_scope = prints "define g() { return (y) }
  define f(y) { return (g()) }
  f(4)
  " == "4\n"
  ```
- wrote: the program in named constants, braces escaped, glued by
  interpolation:
  ```
  scoped = "define g() \{ return (y) }\n{scoped_f}f(4)\n"

  scoped_f = "define f(y) \{ return (g()) }\n"

  test_dynamic_scope = prints scoped == "4\n"
  ```
- why it matters: a test is a constant, a constant is one line, and a line
  is 80 columns, so a descriptive test name plus a small program and its
  expected output does not fit. Test names got shorter than I wanted. And
  the helper constants live in the module namespace (F3): a test constant I
  called `autos` broke two function heads in `calls.kso` whose parameter had
  the same name.

### F21: a pub type cannot be taken apart outside its module
- kind: confusing-semantics
- severity: nit
- where: a scratch harness to time the parser alone
- wanted: `fn next (bc/finished _) n` in a program that imports `bc`
- wrote: the harness inside the module, deleted afterwards
- why it matters: `error[opacity]: \`bc/finished\` is foreign — its
  structure does not cross an import`. `pub type` exports the name and not
  the shape, so a program can receive a value of an exported type and not
  dispatch on which one it got. For an outcome type like `finished | unit |
  bad_unit` the shape is the API.

## What worked well

**Big integers made the decimal arithmetic small.** A bc number is a kanso
int and a scale (`bc/num.kso`). Every scale rule GNU applies (add, multiply,
divide, remainder, power, sqrt) is a line or two, and kanso's `/` already
truncates toward zero the way bc does, so `-7 / 2` needed no correction. The
whole of bc's arithmetic is about 130 lines. Differential testing against
GNU bc agreed on 38,000 random arithmetic statements, and the native build
computes pi to 500 places with the `-l` library in 0.04 s where GNU bc takes
0.22 s.

**Dispatch on literals reads like a table.** Operator semantics, precedence
and the registers are each one overload group:
```
fn arithmetic "/" a b m
  return fail_with m "Divide by zero" if zero? b
  ev m (divide a b m.scale)

fn power_of "||"
  1

fn fetch (register "scale") m
  from_int m.scale
```
The lexer dispatches on byte values (`fn lex_at 34 bs i line acc` is the
string case), and the interpreter is one `exec` group with an arm per
statement type. Adding `define void` meant one arm in `quiet?`.

**err gave bc's error recovery almost for free.** bc abandons the rest of
the line on a runtime error but keeps what the line already did. `fail_with`
raises `err (fault machine message)` from anywhere in the evaluator, the
err passes every function on the way up without a line of plumbing, and a
single `(err (fault fm message))` arm where the unit started reports it and
carries on with the machine as it stood. Function calls catch it on the way
through to restore their autos (`leave` in `bc/calls.kso`). Within one
module this is a clean exception mechanism, once F7 is worked around.

**Values that never change made dynamic scope simple.** Saving a caller's
variables before binding parameters is keeping the old values; restoring
them is putting them back. Passing an array by value costs nothing until
someone changes it. There is no aliasing to think about, except where the
native runtime gets it wrong (F15).

**Pure evaluation made whole-program unit tests trivial.** `program_test.kso`
runs bc programs through the real parser and interpreter with no I/O at all
and compares the text they would print:
`test_quit_is_read_not_run = prints "1\nif (0) quit\n2\n" == "1\n"`.
Nothing is mocked, because nothing in the interpreter touches the world.

**The interpreter is a real oracle.** Every native bug in F15, F16 and F18
showed up as the native output disagreeing with `--interp`, and the
interpreter was right each time. Having a second engine that is slow but
trustworthy is what made those bugs findable at all.

**Tail calls are loops.** `while` and `for` are tail-recursive functions and
run 300,000 iterations without growing the stack.

## Summary

The five I would fix first:

1. **F15**, native memory corruption in loops that carry a record. It
   produced wrong answers, hangs and crashes that depend on layout, passed
   every fixture on all three engines, and was found only by random testing
   against another implementation.
2. **F7** with **F6**, **F8** and **F16**: the failure model's edges. An err
   duplicates itself through a constructor, the checker's rule depends on
   whether a value has a name, the workaround for one is refused by the
   other, and one of the three ways to read a field does not carry errs at
   all. Each is small; together they decide whether the railway can be
   trusted in a real program.
3. **F3**, one namespace for every name in every file of a module, test
   files included. It cost a dozen renames in a 2,600-line program and will
   cost more in bigger ones.
4. **F17** and **F5**: native memory that grows with every iteration of any
   loop carrying state, and maps that grow with every write. An interpreter,
   a simulation or a server is exactly this shape.
5. **F19**, no way to read standard input a line at a time, which rules out
   the interactive half of bc and anything else that holds a conversation.

Writing bc in kanso was two different experiences. The language itself fit
the problem well: bc's numbers fell out of big integers, its operators and
statements fell into overload groups, and its unusual error recovery fell
out of err. The parser and interpreter came together quickly and read
cleanly. Then a long tail of friction came from the edges. Canonical form,
the 80-column rule and positional records with no update syntax made the
interpreter's state awkward to carry around. The err rules forced bindings
and wrapper functions that exist only to satisfy the checker. And the native
runtime corrupted memory in ways that only random testing against GNU bc
exposed, after check.sh had been green on all three engines for hours. The
port now matches GNU bc everywhere I could compare, apart from the
divergences README.md lists, most of them places where GNU 1.07 is wrong or
crashes. Even so, I would not trust
a native build of a kanso program of this shape without a differential
harness behind it.
