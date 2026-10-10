# FRICTION: porting GNU make to kanso

Journal kept while porting GNU make 4.3's core (reading makefiles, variables
and functions, explicit, pattern and static pattern rules, the mtime-driven
update walk, recipes through `std/os`, `-n`, `-k`). Entries are in the order
I hit them. Line references are to the files as they stand at the end.

## Entries

### F1: a file with definitions and statements is neither a program nor runnable
- kind: confusing-semantics
- severity: minor
- where: first scratch program, before the port had a layout
- wanted: the book's own shape (ch04, ch05, ch07): definitions, then
  `pub play = os/run ... .> show`, run with `kanso run a.kso`.
- wrote: `kanso play a.kso` for scratch work, and for the port a `main.kso`
  holding only imports and one statement, with every definition in a module
  directory beside it.
- why it matters: `kanso run a.kso` answered "`a.kso` is a library — nothing
  to run. it exports `play`", and after I dropped the `pub play =` it still
  said "library" because the file still held a `fn`. The book's samples are
  written as `pub play = ...` and shown under `kanso run`, so copying a book
  sample is the first thing that fails. The message does name `kanso play`,
  which got me moving.

### F2: text/split's empty-separator arm makes every variable-separator call fallible
- kind: confusing-semantics
- severity: major
- where: words/words.kso:16 (`split_on`), and every caller of it
- wanted:
  ```
  pub fn replace_all s "" to
    "{s}{to}"

  pub fn replace_all s from to
    text/join (text/split s from) to
  ```
- wrote:
  ```
  pub fn split_on s sep
    listed (text/split s sep)

  fn listed (err _)
    []

  fn listed pieces
    pieces
  ```
  and `split_on` everywhere instead of `text/split`.
- why it matters: std/text gives `split _ ""` an `err` arm, so the checker
  treats any call whose separator is a name as "can be an err" and refuses
  to hand it to `text/join`, to my own helpers, and to `==` in every test
  that touched the result: one `kanso test words` printed 21 errors, all from
  this. The `replace_all s ""` arm right above already took the empty string,
  but the checker does not carry that into the next arm. std/text's own
  `trim` dodges the same problem by calling `builtin_split` directly, which
  user code is not allowed to do ("`builtin_split` is internal to the
  standard library"). The err arm that can never fire has to be written
  anyway, and it is dead code that looks like error handling.

### F3: a helper type has to leave the function it helps
- kind: aesthetics
- severity: nit
- where: words/words.kso:8 (`type seen`)
- wanted: the two-field accumulator `seen` declared right above `uniq` and
  `remember`, the only code that uses it.
- wrote: the type at the top of the file, under the imports, with a comment
  pointing at `uniq`.
- why it matters: "canonical order places type declarations before
  functions". In a file of forty helpers, the types end up as a block at the
  top that a reader has to match back to their users by name. Aesthetic, and
  I would still rather keep it next to its function.

### F4: a loop bound proves an index only in one spelling
- kind: diagnostic
- severity: major
- where: glob/glob.kso:40 (`gather`), expand/expand.kso:43 (`each`), and
  every list walk in the port
- wanted:
  ```
  fn gather patterns i acc
    return effect acc if i > length patterns
    expand_one patterns[i] .> next
  ```
- wrote: `return effect acc if i < 1 or length patterns < i`.
- why it matters: the first spelling is refused with "this can be a none and
  `expand_one` has no arm for it". The diagnostic offers two fixes (resolve
  it here, or add a `none` arm) and neither is the one that works, which is
  to say the same bound a different way. I knew the accepted spelling only
  because the httpd port's journal had found it first. The guard also has to
  sit in the function that indexes: in `glob/glob.kso:in_class?` a guard in
  the caller, one call up, did not count.

### F5: a foreign record cannot be taken apart or built, and the import that only patterns use is "unused"
- kind: engine-bug
- severity: major
- where: expand/expand.kso:201 (`global`), db/db.kso:98 (`simple?`); repro
  in bugs/import_used_only_in_patterns/
- wanted: the database module declares `var`, `rule` and the flavor markers,
  and the expander dispatches on them:
  ```
  fn global _ _ (db/var _ db/simple _ value)
    effect value
  ```
- wrote: field reads (`found.value`), and a predicate in the db module so the
  expander never names a marker:
  ```
  fn global sc name found
    return effect found.value if db/simple? found
  ```
- why it matters: the destructuring arm is refused with "`db/var` is foreign —
  its structure does not cross an import". So the natural layout, one module
  of types and several that work on them, does not work: only the declaring
  module can build or unpack its records, and db/db.kso grew a `with_*`
  function per field so the reader could change one. Then, dispatching on the
  markers alone (`fn flavored _ _ db/simple value`), the build failed with
  "unused import "../db"": a marker named in a pattern does not count as a
  use of the import, qualified or bare. With any other use of the import in
  the file, the same arms dispatch correctly on all three engines, so the
  check is wrong rather than the dispatch.

### F6: a binding may not share a name with a function in any file of the module
- kind: refactoring-hazard
- severity: minor
- where: expand/expand.kso:129 (`part`, was `piece`), expand/expand.kso:209
  (`loop`, was `looped`)
- wanted: `piece = text/slice s from ...` inside `cut`, in expand.kso.
- wrote: `part = ...`, because functions.kso, a different file of the same
  module, declares `fn piece` and `fn looped`.
- why it matters: "`piece` is already a declaration; rename the binding". The
  two names were written an hour apart in two files about different things.
  Adding a helper function to one file can break a local variable in any
  other file of the module, and the error points at the variable, not at the
  new function that caused it.

### F7: passing `none` to my own function needs an arm for it
- kind: confusing-semantics
- severity: minor
- where: reader/lines.kso:176 (`parted`)
- wanted: `parted targets none rest`, where `static` is the target pattern of
  a static pattern rule or none for an ordinary rule, stored into a record.
- wrote: `parted targets "" rest`, with "empty means none" in a comment.
- why it matters: "this can be a none and `parted` has no arm for it".
  `parted` only stores the value in a constructor. Absence is the case none is
  for, and the language refuses to let it flow into a field through a helper;
  the empty string I used instead is a sentinel the type system cannot see.

### F8: an arm after a `none` arm still answers "can be none"
- kind: diagnostic
- severity: major
- where: reader/lines.kso:117 (`classified`), and reader/lines_test.kso
- wanted: `assigned` answers an assignment or none, and `classify` dispatches:
  ```
  fn classified head none false _ _
    ...
  fn classified _ found false _ _
    found
  ```
- wrote: a marker `type unassigned` in place of none, and
  `fn classified head unassigned false _ _`.
- why it matters: every test calling `op_of (classify "x := 1")` was refused
  with "this can be a none and `op_of` has no arm for it". The none arms come
  first and the catch-all never receives none, but the checker still types the
  catch-all's `found` as possibly none and carries that out of `classify`. A
  `fn either none fallback` helper did not help either. The named marker is
  arguably the better design (chapter 04 recommends it), but I got there by
  being refused, and the error pointed at the test file, not at `classify`.

### F9: a list of twelve short words cannot be written
- kind: aesthetics
- severity: minor
- where: reader/lines.kso:200 (`directives`)
- wanted:
  ```
  directives = ["-include" "else" "endif" "export" "ifdef" "ifeq" "ifndef"
    "ifneq" "include" "override" "sinclude" "unexport"]
  ```
- wrote: two lists of six joined with `text/concat`. A string of the words
  split at run time did not fit in eighty columns either.
- why it matters: the line is 92 characters, collection literals may not
  span lines, and there is no string continuation. The workaround invents two
  names (`conditionals`, `others`) that exist only to fit the width.

### F10: a literal `{` in a string needs an escape the book never shows
- kind: diagnostic
- severity: minor
- where: expand/expand.kso:63 (`fn dollar sc s j acc "\{"`)
- wanted: `"{"` to match make's `${name}`.
- wrote: `"\{"`, found by grepping lib/json for a brace.
- why it matters: `"{"` is "error[syntax]: unterminated interpolation". The
  message is accurate, but neither it nor appendix C says how to write the
  character. A makefile port spells `${` often.

### F11: the native engines crash on ordinary effect loops
- kind: engine-bug
- severity: blocker
- where: reader/reader.kso:233 (`each_target`), expand/expand.kso:51
  (`each`), update/update.kso:63 (`settled`), reader/reader.kso:300
  (`export_kind`), update/recipe.kso (`expanded_lines`, `each_line`); repro
  in bugs/native_crash_carrying_a_record/
- wanted: loops of the shape the book teaches for effects, a step bound to
  the next call:
  ```
  fn each_target d p i
    return effect d if i < 1 or length p.targets < i
    t = p.targets[i]
    target_rule (db/offer_goal d t) p t .> (next -> each_target next p (i + 1))
  ```
- wrote: the same loop with an empty effect bound in front of every step,
  which changes nothing a reader can see:
  ```
  fn each_target d p i
    effect done .> (_ -> target_at d p i)
  ```
  and, in three other places, a different order of the same work: marking an
  exported variable before assigning it rather than after, recording whether a
  goal had a recipe while planning it rather than planning it a second time.
- why it matters: the first full run of check.sh had six of 77 fixtures die
  natively (SIGSEGV, or "out of memory" with exit 1) while the interpreter
  was right. gdb put every one in memcpy or free under `k_repair_interior` <-
  `k_deep_copy` <- `k_beat_pop_slow` <- `k_exec`: the executor copying a
  bind chain's result out of a region it had already reused. The trigger was
  `c := $(PATH)` followed by any rule, `export A = 1` followed by any rule,
  or a goal named on the command line with no makefile at all. It depends on
  layout: adding one `io/write_err ""` anywhere in the reader made the first
  case pass, and inserting the same line one function over did not. I spent
  most of an afternoon reducing it to the 60-line repro, which has no I/O at
  all; it fails with SIGSEGV from `kanso build` and with "the program ran out
  of stack" from `kanso run`. The bc port's F15 looks like the same fault
  reached through a push loop instead of a map. I cannot say which loops in
  the port are safe rather than lucky. Every fixture passes on all three
  engines now, and the next edit could change that.

### F12: there is no record update
- kind: missing-feature
- severity: major
- where: update/update.kso:43-58 (`with_commands` ...), db/db.kso:68-81,
  cli/cli.kso:26-51, reader/reader.kso:56-66
- wanted: `rn with temporary: (push rn.temporary t)`, or any spelling of
  "this record, one field different".
- wrote: one function per field that takes the whole record apart and puts it
  back together:
  ```
  fn with_temporary (progress commands d failed opts recipes status _) ts
    progress commands d failed opts recipes status ts
  ```
- why it matters: the walk's state grew from five fields to seven over the
  port (`recipes` to fix F11, `temporary` for intermediate files). Each new
  field meant rewriting every `with_*` function in the file and the
  constructor call, eleven positional edits for one new fact, and a field
  added in the wrong alphabetical slot fails at every one of them. The
  options record in cli/cli.kso has nine fields and nine such functions,
  81 positional names in all. Because a record can only be built or taken
  apart in its own module (F5), these functions cannot be written once and
  shared either.

### F13: an imported name takes over a local one
- kind: refactoring-hazard
- severity: major
- where: cli/cli.kso:63 (`read_args`, was `parsed`), cli/cli.kso:77
  (`cli_assignment`, was `assignment`), update/update.kso:30 (`progress`,
  was `run`), db/db.kso:31 (`command`, was `step`)
- wanted: a local `fn parsed argv i o` in cli, which imports the reader; a
  local `type run` in update, which imports std/os; a local `type step` in
  db, whose module (through words) has std/list in reach.
- wrote: names nobody would choose first.
- why it matters: the reader exports `pub type parsed` and `pub type
  assignment`, and in cli my three-argument function `parsed` was answered as
  the reader's two-field constructor: "`parsed` has 2 field(s), got 3", at
  every call. My `type run` was refused with "no 5-argument arm of `run`
  (arms take 2)" because std/os exports `run`. `type step` became std/list's
  `step` ("`list/step` is foreign — only `list` builds a `step`"). Chapter 11
  says a local arm outranks an import on a tie; here the import won outright.
  The diff and datalog ports report the same thing from other directions.

### F14: a name may be a type, a field, a parameter or a test constant, but only one of them in a module
- kind: refactoring-hazard
- severity: minor
- where: reader/reader.kso:43 (`type open_rule`, was `pending`),
  reader/lines_test.kso:58 (`split_expected`, was `parts`), cli/cli.kso
  (`run_goals`, was `goals`), update/recipe.kso (`type recipe_line`, was
  `line`)
- wanted: a reader record with a field `pending` holding a value of `type
  pending`; a test constant `parts`; a function `goals`.
- wrote: renames, each found by a compile error in a different file.
- why it matters: with `type pending` declared, `fn with_db (reader conds _
  file lines missing pending) d` was refused, so a field cannot share a name
  with its own type even in the destructure that reads it. A constant `parts`
  in lines_test.kso made the parameter `parts` in reader.kso an error
  ("`parts` is already a declaration"), so test files are part of the
  namespace a refactor has to search. These are F6 again at larger scale:
  thirteen renames over the port.

### F15: std/os cannot read a file's modification time
- kind: stdlib-gap
- severity: blocker
- where: update/update.kso:191 (`stamps`), check.sh (`stage`)
- wanted: `os/mtime path`, answering a time or none, which is the one fact
  make is built on.
- wrote: one `stat` process per decision, parsing its output:
  ```
  pub fn stamps names
    os/run "stat" (text_concat ["-c" "%.9Y %n" "--"] names)
      .> (p -> effect (parsed_stamps (words/split_on p.stdout "\n")))
  ```
  and a harness that gives every fixture file a fixed time (`touch -d
  @1000000000`) plus the times a fixture's `times` file names, so a file a
  recipe writes is always newer than the inputs, as on a real disk.
- why it matters: without the harness, files copied in the same instant have
  undefined order and the fixtures would be nondeterministic. The workaround
  is honest, since the port reads real times from the real disk, but it
  needs GNU stat's `%.9Y` and costs a process per target. Asking stat for the
  target and every prerequisite at once keeps it to one.

### F16: std/os has no remove, no working directory and no environment for a child
- kind: stdlib-gap
- severity: major
- where: update/update.kso:209 (`remove_temporary`), update/recipe.kso:59
  (`environment`, `shell`), update/recipe.kso:92 (`finished`)
- wanted: `os/remove path` for make's deletion of intermediate files;
  `os/run` with an environment, so `export` works; a working directory, so
  `-C dir` and $(CURDIR) can exist; a child that writes to make's own stdout.
- wrote: `os/run "rm" ["-f" "--" ...]`; `os/run "env" ["NAME=value" ...
  "/bin/sh" "-c" line]`; no `-C` and no CURDIR; and a recipe's output
  written after the child exits, stdout first, stderr second.
- why it matters: a recipe that interleaves stdout and stderr shows them in
  the wrong order, and a long-running recipe shows nothing until it ends.
  The environment cannot be listed either, only read one name at a time
  (`os/env`), so make's environment variables are looked up lazily when a
  name is not otherwise defined, and `?=` asks the environment one name at a
  time.

### F17: an index guard on a field does not count
- kind: diagnostic
- severity: minor
- where: bugs/native_crash_carrying_a_record/rd/rd.kso (`walk`); the same
  shape is reader/reader.kso:96
- wanted: `return finished r if i < 1 or length r.lines < i` followed by
  `r.lines[i]`.
- wrote: `ls = r.lines` first, then the guard and the index on `ls`.
- why it matters: "this can be a none and `statement` has no arm for it".
  The accepted spelling from F4 works on a name and not on a field read of
  the same list. In the full reader I happened to pass `r.lines` to a helper
  and index there, so I only met this while reducing F11.

### F18: expansion is an effect, so none of it can be unit-tested
- kind: missing-feature
- severity: major
- where: expand/expand.kso:33 (`expand`), expand/expand_test.kso
- wanted: `test_patsubst = expand scope "$(patsubst %.c,%.o,a.c)" == "a.o"`.
- wrote: tests of the pure pieces (`split_args`, `closing`, `substituted`,
  `computed`) and a fixture per function family, checked through `$(info)`.
- why it matters: $(shell), $(wildcard), $(info), $(warning) and $(error)
  touch the world, so `expand` answers `<string>effect` for every text, and a
  `test_` constant cannot open a box. Chapter 05 describes a scripted
  executor that runs an effect against canned files and records a
  transcript; it exists in the interpreter's Rust tests and not in `kanso
  test`. The function at the centre of the program, the one most worth unit
  tests, is the one I could only test end to end.

### F19: no list patterns, and a boolean cannot continue onto the next line
- kind: missing-feature
- severity: minor
- where: cli/cli.kso (`begin`), update/recipe.kso (`shell`),
  reader/reader.kso (`conditional?`)
- wanted:
  ```
  fn begin o []
    ...
  fn shell [] command
    os/run "/bin/sh" ["-c" command]

  fn conditional? word
    word == "ifeq" or word == "ifneq" or word == "ifdef" or word == "ifndef"
      or word == "else" or word == "endif"
  ```
- wrote: `return ... if files == []` guards, and `testing? word or word ==
  "else" or word == "endif"` built on a second predicate.
- why it matters: "expected a parameter pattern" for `[]`, and "expected an
  expression" for the `or` on its own line. Literal dispatch is the language's
  answer to every other two-way choice, and the empty list is the one literal
  it does not take. With eighty columns and no way to continue an expression,
  a long condition has to be split into named predicates whether or not the
  names help.

### F20: tables of text fight the eighty-column rule
- kind: aesthetics
- severity: minor
- where: cli/cli.kso:64-96 (the usage message)
- wanted: the usage text as GNU make prints it, a block of option lines.
- wrote: a `help` record per option, produced by six literal-dispatch arms of
  `help_of`, and a `shown` function that pads the flags to column 31.
- why it matters: my first two attempts (a list of strings, then a list of
  records) were refused for width, three lines at a time. The final shape is
  reasonable code, but I wrote it to get past the formatter, not because the
  data asked for it. F9 is the same problem with a list of words.

### F21: reducing a bug fights the same rules as writing code
- kind: tooling
- severity: minor
- where: bugs/native_crash_carrying_a_record (while reducing it)
- wanted: delete a parameter's use, a field or a call, rebuild, and see
  whether the crash survives.
- wrote: `_` in place of every parameter that stopped being read, a rename
  whenever a deleted function's name freed up a binding, and wrapped trace
  lines that had to fit eighty columns.
- why it matters: each cut cost one or two extra compile rounds, refused with
  "unused binding", "needless continuation" or "reads none of what it is
  handed here, so this effect never happens". The program under reduction is
  throwaway, and the checker held it to the standard of code that ships. A
  `kanso build` of a single file of definitions and statements is also
  refused ("is a library"), so the repro needed a module directory and a
  `main.kso` before gdb could see it.

## What worked well

**Literal dispatch made make's tables read as tables.** The function table is
thirty arms of `fn arities "patsubst"` / `arity 3 3`, the pure functions are
arms of `fn computed "sort" vals`, and directives dispatch the same way
(`fn directed r i next "include" bare _`). Adding a function is adding an
arm. In C this would be a struct array and a lookup loop.

**Fatal errors are one line, anywhere.** `report/fatal at "missing
separator"` writes the message and answers `os/exit 2`, which is an err, so
it rides out of any depth of the reader, the expander or the update walk and
skips every step after it. Make's whole "*** ...  Stop." family needed no
plumbing: no error type threaded through the reader, no early returns.

**Markers for states.** The update walk's `broken`, `building`, `fresh` and
`remade`, the reader's `no_rule` and `no_inline`, and `unassigned` for a line
that is not an assignment are each one line to declare, compare with `==`,
and dispatch on (`fn visit rn t parent building` is the cycle check).

**`return x if cond`.** Most functions in the port start with two or three
guards and then do the work, which matches how make's own rules read
("nothing to do if it is fresh; fail if a prerequisite failed; ...").

**Exact integers.** Modification times arrive from stat as
`1791597647.952198018`; dropping the dot and calling `text/to_int` gives
nanoseconds with no thought about overflow.

**Determinism.** `os/list_dir` answers sorted names, which is what
$(wildcard) promises, and maps iterate in key order, so the port's output
never depended on a hash seed. Every fixture produced the same bytes on every
run of every engine once F11 was worked around.

**The interpreter as the oracle.** check.sh compares three engines on every
fixture. That is how F11 was found at all: the interpreter was right, and the
disagreement pointed straight at the native runtime instead of at my code.

**Effects as descriptions made `-n` cheap to think about.** A recipe line is
echoed, then run, as two steps of one chain, and `-n` simply stops after the
echo for lines without `+`. Ordering is always explicit in the `.>` chain, so
I never had to wonder whether a message would appear before a recipe's
output.

## Summary

The five I would fix first:

1. **F11, native crashes in effect loops.** A blocker: six of 77 fixtures
   crashed natively on ordinary code, the trigger moves with unrelated edits,
   and the workarounds (an empty effect before each loop step, two reordered
   operations) carry no meaning a reader could recover. Together with bc's
   F15 it says the beat and carry runtime has a hole that a medium-sized
   program finds by itself.
2. **F15 and F16, std/os.** A make without file times is not a make. Also
   missing: remove, a working directory, a child's environment, and a child
   that writes to the parent's stdout.
3. **F12 with F5, records.** No record update, and no way to build or take
   apart a record outside its module, together turn every new field into a
   dozen edits and push all state changes into the declaring module.
4. **F13 with F6 and F14, one namespace for everything.** Imported types win
   over local functions, test constants collide with parameters in other
   files, and a field cannot share its type's name. Thirteen renames, each
   found as an error somewhere other than its cause.
5. **F4, F8 and F17, how the checker proves none and err away.** The guard
   that works is one spelling on a plain name; a `none` arm earlier in a
   group does not narrow the arms after it; and text/split's unreachable err
   (F2) spreads to every caller.

Writing make in kanso was mostly pleasant at the level of single functions.
Make is a reader, an expander and a graph walk, and each of those came out as
small arms that read close to the GNU manual. The friction was at the joins.
Because $(shell) and $(wildcard) are effects, expansion is an effect, so
reading is an effect and almost the whole program lives inside `.>` chains
that carry records. That is exactly the shape that crashed the native
runtime, it is the shape the record rules make expensive to change, and it is
the shape `kanso test` cannot test. Against GNU make 4.3 the port agrees on 79
of 82 fixtures; the other three are the usage message and two features I
chose to refuse (`define`, target-specific variables), not ones the language
kept me from writing. It made me say it in more places than
I wanted, and the native engines could not be trusted to run it until I had
found and dodged a runtime bug I do not understand.
