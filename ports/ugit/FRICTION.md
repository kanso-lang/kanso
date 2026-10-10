# FRICTION: porting ugit to kanso

The journal for port #13. Entries were written as each problem came up, in
the order they came up. Every code sample was compiled (or refused, where
the refusal is the point) by `/tmp/claude-0/kanso-main/kanso` on 2026-10-10.

## Entries

### F1: a split that cannot be empty still owes a `none` arm on its last element
- kind: missing-feature
- severity: minor
- where: delta/lines.kso:14-22
- wanted:
  ```
  pieces = text/split content "\n"
  closing ended pieces[length pieces]
  ```
  with `closing` written for the two cases that occur: an empty tail and a
  non-empty one.
- wrote: a third arm, `fn closing ended none`, with a comment saying it is
  for the checker. The checker reported
  `error[exhaustive]: this can be a none and closing has no arm for it`.
- why it matters: `text/split` never answers an empty list, and
  `xs[length xs]` on a non-empty list is always in range, but the index
  type is `value | none` regardless. The same thing came back in
  delta/unified.kso (`moved? ps[i]` over `i` drawn from `1..length ps`), so
  the port carries dead arms a reader cannot tell from live ones.

### F2: a file's header comment may not be followed by a blank line
- kind: aesthetics
- severity: nit
- where: delta/myers.kso:1-5
- wanted:
  ```
  # Myers' O(ND) difference algorithm ...
  # the path can be walked back once the far corner is reached.

  # One line of the new file that the old one did not have.
  pub type added
  ```
- wrote: the two comments run together with a bare `#` line between them,
  after `error[formatting]: the file may not begin with a blank line`.
- why it matters: aesthetic. A comment about the whole file and a comment
  about the first declaration are different things, and the only way to
  separate them is a `#` line that looks like a typo. The diagnostic also
  names the wrong thing: the file begins with a comment, not a blank line.

### F3: a private function or type silently loses to a name std/list exports
- kind: engine-bug
- severity: major
- where: delta/myers.kso:65 (`step`), delta/merge3.kso:16 (`cursor`)
- wanted: a private helper `fn step a b prev_k k x y` in a module that also
  imports std/list, and later a private `type cursor` for the merge walk.
- wrote: renamed them `edit_at` and `place`. With `step`, the call compiled
  as std/list's record constructor `list/step` and the checker answered
  `error[arity]: step has 2 field(s), got 6 (construction is positional,
  fields alphabetical)`; in a one-file program it runs and prints
  `list/step 1 2` where the local function should have answered 3 (see
  bugs/local_fn_loses_to_imported_type.kso). With `cursor` it said
  `error[opacity]: list/cursor is foreign — only list builds a cursor`,
  about a type I had just declared in the same file.
- why it matters: std/list exports fourteen ordinary words as types
  (`step`, `cursor`, `sorted`, `mapped`, `paired`, `counting`...), and every
  module that imports it loses them without being told. The local
  declaration should win or the program should be refused at the
  declaration; instead the error lands at a use site and names a type the
  author never wrote.

### F4: `kanso run` refuses a file that mixes declarations and statements, and the book's samples do exactly that
- kind: tooling
- severity: minor
- where: scratch probes while learning the language
- wanted: `kanso run a.kso` on a file with a `fn` and a `print`, as in the
  book's `credit.kso`, `describe.kso` and most chapter samples.
- wrote: `kanso play a.kso`. `kanso run` says `error[syntax]: a file with
  declarations is a library, and a library has no statements to run`, and a
  file with `pub play = ...`, which chapter 04 and 05 show under
  `kanso run`, is refused by both verbs.
- why it matters: the first program a reader types from chapters 3 to 6
  does not run with the command printed above it. The error message does
  name `kanso play`, which is how I found it.

### F5: parentheses that only make precedence visible are a compile error
- kind: aesthetics
- severity: nit
- where: delta/myers.kso:44
- wanted: `k == 0 - d or (k != d and v[k - 1] < v[k + 1])`, which is how
  the condition is written in Myers' paper and in every other port of it.
- wrote: `k == 0 - d or k != d and v[k - 1] < v[k + 1]`, after
  `error[formatting]: these parentheses group nothing`.
- why it matters: aesthetic, but the reader of the second form has to know
  that `and` binds tighter than `or` to see that it is correct, and
  that is the question the parentheses answered.

### F6: there is no pattern for an empty list
- kind: missing-feature
- severity: minor
- where: delta/unified.kso:30, delta/unified.kso:71, delta/merge3.kso:85
- wanted:
  ```
  fn joined [] s
    [s]

  fn joined acc (span from to)
    ...
  ```
- wrote: `return [(span from to)] if length acc == 0` as a guard inside
  the general arm, after `error[syntax]: expected a parameter pattern`.
- why it matters: dispatch is sold as the only switch, and literals are
  the top rung of the ladder; `0` and `""` are patterns but `[]` is not.
  Every fold that starts from an empty accumulator wants that arm.

### F7: `text/join` accepts a lazy `list/map` at check time and fails at run time
- kind: diagnostic
- severity: major
- where: delta/unified.kso:81
- wanted: `text/join (list/map inside (p -> marked p.edit)) ""`
- wrote: `text/join (list/to_list (list/map inside (p -> marked p.edit))) ""`.
  `kanso check` said ok; the binary stopped with `error[runtime]: join
  takes a list of strings and a separator` (bugs/join_accepts_lazy_map_at_check.kso).
- why it matters: map-then-join is the most common line in a program that
  prints anything, and it is the one line where forgetting `to_list` is
  not caught. The check verb exists to say whether a program will be
  refused; here it was wrong.

### F8: one module's private names are reserved in every file of the module, tests included
- kind: refactoring-hazard
- severity: major
- where: delta/myers.kso:64, delta/merge3_test.kso:7, delta/unified.kso:25
- wanted: local bindings `walked` and `stepped` in myers.kso; a test
  fixture constant `base`, and later `s`, in merge3_test.kso.
- wrote: renamed the locals `slid` and `edited`, and the test constant
  `abcde`. Adding `fn walked` and `fn stepped` to merge3.kso broke
  myers.kso with `error[name]: walked is already a declaration; rename the
  binding`. Adding the test constant `base` broke the parameter list of
  `merge3 base ours theirs`; renaming it `s` then broke a lambda
  `(s -> hunk ps s)` in unified.kso.
- why it matters: no-shadowing plus one namespace per module means a new
  private function in one file can break a parameter name in a file the
  author has not opened, and a constant in a test file can break the code
  it tests. The rule is meant to keep names unambiguous; at module scale it
  makes every short name a global.

### F9: two arms that can never match the same call still "tie"
- kind: confusing-semantics
- severity: minor
- where: delta/merge3.kso:86-96
- wanted:
  ```
  fn joined_to none acc chunk
    push acc chunk

  fn joined_to (clean before) acc (clean lines)
    push (text/slice acc 1 (length acc - 1)) (clean (text/concat before lines))
  ```
- wrote: the third parameter widened to a plain name in both arms, so that
  neither is more specific there. The checker said `error[dispatch]: these
  joined_to arms tie: each is the more specific one somewhere, and a call
  could match both`, but no call can: `none` and `clean` are disjoint in
  the first position.
- why it matters: the rule asks for a total order where the arms are
  already disjoint. The remedy the message names, an arm "most specific in
  every position", is the meet of the two (`none ... (clean lines)`),
  which for disjoint arms is code that can never run. I learned later
  (F21) that the meet arm is also how two none-able parameters are
  written, so the rule is consistent; what is missing is any recognition
  that `none` and `clean` cannot both describe one argument.

### F10: a constant whose expression is one line too long has no legal spelling but a throwaway binding
- kind: aesthetics
- severity: nit
- where: delta/myers_test.kso:39, delta/merge3_test.kso:28
- wanted:
  ```
  test_positions_are_one_based =
    edits ["a"] ["z" "a"] == [(added "z" 1) (kept "a" 2 1)]
  ```
- wrote: `want = [(added "z" 1) (kept "a" 2 1)]` on its own line, or a
  shorter test name. The block form was refused with `error[formatting]: a
  single-expression constant is written inline`, and the inline form with
  `a line holds at most 80 characters`.
- why it matters: aesthetic. Test names want to be sentences, the 80
  columns count the name, and the language forbids the one wrap that keeps
  the expression whole. I renamed tests to fit, which is the wrong thing to
  be optimising a test name for.

### F11: whether a `../` import resolves depends on how the entry directory was spelled
- kind: engine-bug
- severity: major
- where: every `kanso check .` run inside store/ and the scratch drivers;
  reproduced in bugs/import_above_invocation/
- wanted: `cd store && kanso check .`
- wrote: `kanso check "$PWD"`. With `.` the compiler says `error: cannot
  resolve import "../steps" — a dot-prefixed path names a module beside
  the importing one, and there is no such directory`; with the absolute
  path the same tree is ok.
- why it matters: `resolve_import` walks `../` lexically over the path it
  was handed, so a relative path runs out of components early. The message
  sends you looking for a misspelled directory that is right there. Every
  multi-module project hits it the first time someone checks one module
  from inside its directory.

### F12: `os/exit` is an err, so the rescue that turns failures into "fatal:" also eats every exit
- kind: confusing-semantics
- severity: major
- where: ugit/cli.kso (run), first tried in a scratch probe
- wanted:
  ```
  pub fn run args
    command args .? fatal

  fn fatal (err (os/exit_status code))
    os/exit code

  fn fatal (err r)
    io/write_err "fatal: {r}\n" .> (_ -> os/exit 128)
  ```
- wrote: commands yield their exit code as a value (`effect 1`), the rescue
  yields 128, and one step after the rescue turns the code into
  `os/exit`. The first version printed `fatal: os/exit_status 3` for a
  deliberate `os/exit 3`, and the arm meant to let it through is refused
  with `error[opacity]: os/exit_status is foreign — its structure does not
  cross an import`.
- why it matters: rescue is licensed for foreign failures, and an exit
  born in std/os is foreign to every program, so the one top-level
  handler a CLI wants catches its own exits. Nothing outside std/os can
  tell an exit from a failure, so the only fix is to keep `os/exit` out of
  every chain that has a rescue on it.

### F13: binding an index to a name silences the `none` check that a direct use gets
- kind: confusing-semantics
- severity: minor
- where: store/commits.kso:32, 41, 54; store/objects.kso:47
- wanted: `header = text/split pieces[1] "\n"` and
  `with_field c words[1] rest`
- wrote:
  ```
  top = pieces[1]
  header = text/split top "\n"
  ```
  The direct forms were refused with `error[exhaustive]: this can be a
  none and text/split has no arm for it`; the bound forms compile. Same
  for `(text/split stored nul)[1]`, refused as "this can be an err" until
  the split was bound to a name.
- why it matters: chapter 04 documents that the checker reads calls and
  not names, so this is the rule as written. In practice it means the
  cheapest answer to a false positive (F1) is an extra line that moves the
  read out of the checker's sight, and a reader cannot tell which bindings
  exist for that reason.

### F14: std/os can make files and directories but cannot remove either
- kind: stdlib-gap
- severity: major
- where: store/work.kso:38-53
- wanted: `os/remove path` and `os/remove_dir path`, as Go's `os.Remove`.
- wrote:
  ```
  pub fn removed file
    os/run "rm" ["-f" file] .> (_ -> pruned (parent_of file))

  fn pruned dir
    os/run "rmdir" ["-p" "--ignore-fail-on-non-empty" dir] .> (_ -> effect done)
  ```
- why it matters: checkout and merge must delete files the target tree
  does not have, and ugit deletes refs (MERGE_HEAD). Shelling out ties the
  port to a POSIX `rm` and GNU `rmdir`, starts two processes per deleted
  file, and loses errors, since a non-zero status is an outcome nobody
  reads.

### F15: the interpreter refuses `text/concat` on `text/bytes`, the compiled engines accept it
- kind: engine-bug
- severity: major
- where: store/store_test.kso:22 (first draft); bugs/concat_bytes_interp_refuses.kso
- wanted: `text/concat (text/bytes "blob") (text/concat [0] (text/bytes "hi"))`
  to build the framed bytes of an object in a test.
- wrote: the expected digest pinned as a literal, computed with
  `printf 'blob\0hi' | sha256sum`. `kanso test` runs on the interpreter,
  which stops with `error[runtime]: concat takes two lists`; `kanso play`
  natively prints `[97 98 0]`.
- why it matters: the interpreter is documented as the oracle, and here
  the oracle is the engine that is wrong. Tests run on the interpreter, so
  a test can fail on code the shipped binary runs correctly.

### F16: the `entry` record's fields have no names a dot can read
- kind: confusing-semantics
- severity: nit
- where: store/trees.kso:73-77
- wanted: `list/fold (entries from) into (m e -> put m e.key e.value)`
- wrote:
  ```
  list/fold (entries from) into with_entry

  fn with_entry m (entry k v)
    put m k v
  ```
  after `error[name]: no record type has a field key`.
- why it matters: appendix B describes `entries` as "an entry key value
  record" and `_.title` is sold as the way to read one field, but `entry`
  has no field names to read, so every walk over a map needs a named arm.

### F17: `kanso check` passes a keyed read of every field; it fails at run time
- kind: diagnostic
- severity: minor
- where: ugit/date.kso:12 (first draft); bugs/keyed_read_of_every_field_is_a_runtime_error.kso
- wanted: `{ day month year } = civil days`, or failing that a compile
  error.
- wrote: `civil_date day month year = civil days`. `kanso check` said ok,
  and `kanso test` failed every date test with `error[runtime]: a keyed
  read omits at least one field; reading every field is the positional
  form`.
- why it matters: this is a canonical-form rule, the kind every other
  part of the language reports before running anything, and it shows up
  only when the line executes. A branch that rarely runs would ship with
  it.

### F18: no do-notation, so every value an effect yields costs a nested lambda or a named function
- kind: missing-feature
- severity: major
- where: ugit/view.kso:22-34, ugit/commit.kso:46-52, ugit/diff.kso:48-50,
  ugit/merge.kso:62-71, store/names.kso
- wanted: the shape a command has in any imperative language, or in
  Haskell's `do`:
  ```
  looked =
    b <- store/current_branch
    at <- store/get_ref "HEAD"
    hf <- store/files_of_commit at.value.value
    idx <- store/read_index
    work <- store/work_ids
    m <- store/get_ref "MERGE_HEAD"
    effect (view b at.value.value hf idx m.value.value work)
  ```
- wrote:
  ```
  looked = store/current_branch
    .> (b -> store/get_ref "HEAD" .> (at -> with_head b at.value.value))

  fn with_head b head
    store/files_of_commit head
      .> (hf -> store/read_index .> (idx -> with_index b head hf idx))

  fn with_index b head hf idx
    in_merge = store/get_ref "MERGE_HEAD"
    store/work_ids .> (work -> in_merge .> (m -> viewed b head hf idx m work))

  fn viewed b head hf idx m work
    effect (view b head hf idx m.value.value work)
  ```
- why it matters: `a .> (x -> b) .> (y -> c)` reads well, but `c` cannot
  see `x`, so any step that needs two earlier results must nest, and
  nesting runs into the 80-column limit after two levels. The way out is
  a chain of functions whose only job is to receive one more value and
  pass the rest along (`with_head`, `with_index`, `viewed`, `based`,
  `branching`, `started`...), each with a growing parameter list. About
  a third of the functions in ugit/ exist for this reason. ugit is
  mostly reads followed by writes, which is the case this hurts most.

### F19: the 80-column limit, with no way to continue a string, was the most frequent compile error in the port
- kind: aesthetics
- severity: major
- where: every file in ugit/ and store/; for example ugit/merge.kso:57-59,
  ugit/merge.kso:117-120, ugit/checkout.kso:69-77
- wanted: `pending_merge = fatal_says "You have not concluded your merge (MERGE_HEAD exists)."`
- wrote:
  ```
  pending_merge = fatal_says "{unconcluded} (MERGE_HEAD exists)."

  unconcluded = "You have not concluded your merge"
  ```
  A text block does not help: `error[formatting]: a text block holds two
  or more lines; one line is written "..."`.
- why it matters: I hit `a line holds at most 80 characters` about forty
  times, often at 81 to 84, and usually on one of three things: a message
  string (git's messages are long), a nested `.>` lambda (F18), or a test
  whose name is a sentence (F10). The fixes add bindings whose names carry
  nothing (`said`, `left`, `unconcluded`) and split messages a reader
  would grep for. Text blocks, which would solve the multi-line case, are
  documented on the compiler page but not in the book.

### F20: a foreign record can be read with a dot but not built or matched, so the store exports a constructor per record
- kind: confusing-semantics
- severity: minor
- where: store/repo.kso:12-22, 41-42 (`direct`, `symbolic_to`,
  `new_commit`, `logged_as`)
- wanted: in ugit/, `store/ref_value false oid`, `store/commit ...`, and
  arms like `fn branch_of (store/ref_value true target)`.
- wrote: four one-line pub functions in store that call the constructor
  for the caller. The patterns are refused with `error[opacity]:
  lib/thing is foreign — its structure does not cross an import`, while
  `t.size` and `{ name } = t` on the same value are accepted, and so is
  the unqualified `(file_not_found _)` pattern for std/os's record,
  which the qualified `(os/file_not_found _)` is not.
- why it matters: `pub type` publishes the fields for reading and
  nothing else. That may be the intended opacity, but then dot access is
  the odd one out, and the rule for which spelling of std/os's type is
  legal in a pattern took three tries to find.

### F21: `none` cannot reach a generic parameter, and two none-able parameters need four arms
- kind: missing-feature
- severity: major
- where: ugit/view.kso:42-52 (`compared`), ugit/merge.kso:7-10, 85-98 (`absent`, `known`)
- wanted: a function over a map read on each side, written with guards:
  ```
  fn compared p a b
    return change "new file" p if a == none
    return change "deleted" p if b == none
    if (a == b) none (change "modified" p)
  ```
- wrote: the full lattice, `compared _ none none`, `compared p none _`,
  `compared p _ none`, `compared p a b`; and for the merge decision,
  which reads three maps, a marker type `absent` and a function `known`
  that turns none into it, because three none-able parameters would need
  eight arms. Guards on a generic parameter are refused with
  `error[exhaustive]: this can be a none and compared has no arm for it`,
  and a typeset `type slot none string` on the parameter does not change
  that.
- why it matters: comparing two maps key by key is the core of status,
  diff, checkout and merge, and a missing key is the normal case. The
  language's own value for "absent" is the one value a plain parameter
  will not accept, so the port invented a second one.

### F22: maps have `put` and no way to remove a key
- kind: stdlib-gap
- severity: minor
- where: ugit/add.kso:54-60
- wanted: `remove idx path`, for staging a deleted file.
- wrote:
  ```
  fn without m gone
    list/fold (entries m) {} (acc e -> kept_unless acc e gone)

  fn kept_unless acc (entry k v) gone
    if (listed? gone k) acc (put acc k v)
  ```
- why it matters: rebuilding the map is O(n) per removal, and an index is
  exactly the kind of map that loses keys.

### F23: std/text has no prefix test or substring search
- kind: stdlib-gap
- severity: minor
- where: store/strings.kso (all of it)
- wanted: `text/starts_with? s p`, `text/contains? s p`, `text/index_of`.
- wrote: `starts_with?` as `text/slice s 1 (length prefix) == prefix`,
  `contains?` as `length (text/split s piece) > 1`, and `after` for the
  rest of a string past a known prefix.
- why it matters: ref files ("ref: refs/heads/x"), short names
  ("refs/heads/"), pathspecs and object-id prefixes are all prefix
  questions. Every program that parses a line format will write these
  three again.

### F24: no `list/reverse`
- kind: stdlib-gap
- severity: nit
- where: delta/myers.kso:83-90
- wanted: `list/reverse edits` after backtracking.
- wrote: `reversed` and `turned`, seven lines that index from the end.
- why it matters: a nit, but a backtracking algorithm produces its answer
  backwards, and there is no cons to build it the other way.

### F25: a multi-line list literal does not parse
- kind: aesthetics
- severity: nit
- where: ugit/status.kso:10-14
- wanted:
  ```
  sections = [
    (listed "Changes to be committed:" (list/map staged shown))
    (listed "Changes not staged for commit:" (list/map unstaged shown))
    (listed "Untracked files:" (list/map loose (p -> "\t{p}\n")))
  ]
  ```
- wrote: three bindings, then `[to_commit not_staged new_files]`.
  The literal was refused with `error[syntax]: expected an expression`
  at the end of its first line.
- why it matters: aesthetic. The indented-argument form exists for calls
  but not for list literals, so a list of three long items has to be
  named piece by piece.

### F26: nothing that performs an effect can be a test, so the commands are tested only through fixtures
- kind: tooling
- severity: minor
- where: steps/ (no test file), ugit/ugit_test.kso covers only pure helpers
- wanted: a test that runs `status_cmd` against an in-memory tree and
  compares what it would print, which chapter 05 describes as the
  scripted executor.
- wrote: unit tests for the pure parts (arguments, changes, merge
  decisions, labels, dates, diff and merge algorithms) and shell
  fixtures for everything that touches the store.
- why it matters: chapter 05 says effects are values so that they can be
  tested without mocks, but the scripted executor lives in the
  interpreter's Rust tests and a `test_` constant cannot reach it. For a
  program that is mostly effects, "pure core, imperative shell" leaves
  most of the program in the shell.

### F27: `x == true` is flagged even when x can be none
- kind: diagnostic
- severity: nit
- where: store/commits.kso:77, 93
- wanted: `return walking rest seen acc if seen[oid] == true`
- wrote: `seen[oid] != none`, after `error[name]: comparing to true asks
  a question the value already answers — write the value itself`.
- why it matters: the value is `true | none`, so writing it bare would
  hand `if` a none, which the book says is a runtime error. The advice is
  wrong for exactly the case where comparing to true is meaningful.

### F28: interleaving `put` and reads on a map is quadratic in compiled code and linear in the interpreter
- kind: performance
- severity: major
- where: delta/myers.kso:35-45; bugs/map_put_then_read_resorts.kso
- wanted: the textbook Myers loop, which updates `v[k]` and reads
  `v[k - 1]` and `v[k + 1]` in turn.
- wrote: two maps per round, one read-only and one write-only, with a
  comment explaining why. Before the change, `ugit diff` of a 3,000-line
  file with 60 changed lines took 3.3 s as a dev binary, 4.0 s as a
  release binary and 0.4 s on the interpreter. callgrind put 80% of the
  instructions in `k_map_sort_build`, `k_msort_cmp` and `k_key_cmp`. After
  it, 0.009 s. The repro loop with 20,000 keys takes 7.7 s natively and
  0.06 s on the interpreter.
- why it matters: the release binary was ten times slower than the
  interpreter, and nothing in the language or the docs says that the
  order of map operations has a cost. I found it only because a fixture
  felt slow.

### F29: a function name reserves that word in every file, so commands became `*_cmd`
- kind: refactoring-hazard
- severity: minor
- where: ugit/cli.kso:49-98, and every command function
- wanted: commands named `branch`, `status`, `add`, `tag`, `show`, as in
  the original.
- wrote: `branch_cmd`, `status_cmd` and so on, eleven renames in one
  pass, after `error[name]: branch is already a declaration; rename the
  binding` at three parameters in log.kso that held a branch name. An
  imported module's pub types join the same space: a log helper called
  `logged` was read as store's pub type `logged` (`error[arity]: logged
  has 2 field(s), got 3`), the F3 problem with my own module this time.
- why it matters: this is F8 seen from the other side. The natural name
  for a command is the noun its arguments are also called, and the
  module can have only one of them.

### F30: std/sha256 on the interpreter hashes about 30 KB a second
- kind: performance
- severity: minor
- where: store/objects.kso:15 (`object_id`), fixtures/cases/diff_large.sh
- wanted: `kanso test` and `--interp` runs that hash a 29 KB file in
  well under a second.
- wrote: nothing different. `sha256/hex (text/bytes t)` on a 29 KB file
  takes 1.07 s wall (0.48 s user) on the interpreter and 0.09 s user
  natively. ugit hashes every working-tree file for `status`, `diff`,
  `checkout` and `merge`, so the 3,000-line fixture spends about 25 of
  its 30 seconds on the interpreter, mostly here.
- why it matters: minor for ugit, whose fixtures are small, but the
  interpreter is where tests run, and a content-addressed store hashes
  everything it touches. The library comment says a builtin "would only
  buy speed on a path that runs once per built file"; in a version
  control tool it is the path that runs most.

## What worked well

**Dispatch on literals and small record types made the merge readable.**
The decision a three-way merge makes for one path became a table of four
record types (`ours_stand`, `theirs_win`, `both_changed`, `modify_delete`)
and one function of guards, and applying it is one arm per outcome
(ugit/merge.kso:85-175). Adding the modify/delete case was a new type and
a new arm, with no change elsewhere. The same shape served commands
(`fn dispatch "add" args`), status summaries (`fn summary 0 0 0 none`) and
the old/new/same classification of a path.

**Structural equality on lists and maps.** The heart of diff3 is "did this
side change this stretch?", which is `from_ours == from_base` on two lists
of lines (delta/merge3.kso:76-80). Comparing a tree to a tree is comparing
two maps of path to id. No `equals` methods, no deep-compare helper, and
the tests compare whole edit scripts and chunk lists with `==`.

**Effects as values let checkout and merge share code.** `moved v oid head
note` makes the working tree match a commit and then runs `head`, which is
an effect the caller built and passed in: a symbolic HEAD update for a
branch switch, a detached one for a commit, a branch move for a
fast-forward merge (ugit/checkout.kso:63, ugit/merge.kso:73-75). In Python the
same sharing needs a callback or a flag; here the step is just a value.

**The rescue gave one place for every fatal message.** Store functions
raise `err "not a valid object name {oid}"` and similar, and
`command ... .? fatal` in ugit/cli.kso turns any of them into
`fatal: ...` and exit 128. No command checks for failure on the way; a
bad revision in the middle of `merge` aborts it cleanly, before any file
is written, because the failure rides the chain past every later step.

**The engines agree, and pinning time was one variable.** All 15 fixtures
produce byte-identical output on the interpreter, the dev binary and the
release binary, including the SHA-256 ids, which depend on the commit
timestamps. `KANSO_NOW` pins `time/now` for every engine, so making the
fixtures deterministic took one exported variable rather than an injected
clock.

**The standard library had the hard parts.** std/sha256, std/json for
the index, `os/list_dir` returning sorted names (so the working-tree walk
and therefore every status and tree are deterministic), and `entries`
returning sorted keys (so trees come out in the order git wants without a
sort).

**Unused-binding errors cleaned up after refactors.** The unused check
caught two `creating` parameters the duplicate-branch arm had stopped
reading, and imports left behind when code moved between files
(`../steps` in add.kso, `../delta` and `std/text` in merge.kso). In my
first CLI probe the exhaustiveness check refused `pick args[1]` until there
was an arm for no arguments at all, which is the `ugit` with nothing after
it case, and every command in the port has that arm as a result.

**Myers and diff3 ported cleanly.** Recursion with the specificity ladder
reads close to the paper: `fn walked_back oid 0 _` is the base case, the
general arm is the step. The Myers diff passed 2,000 fuzzed cases on its
first run (each patch applied with `patch` reproduced the new file, and no
script was longer than GNU diff's), and the diff3 merge matched `diff3 -m`
byte for byte on every clean merge where the two tools chose the same
alignment.

## Summary

The five I would fix first:

1. **F3, a local name silently losing to a std/list export.** It produced
   a program that compiled and did the wrong thing (`list/step 1 2` where
   a function call was meant). Everything else on this list costs time;
   this one costs correctness, and the fourteen exported names are
   ordinary words.
2. **F28, put-then-read on a map re-sorts it in compiled code.** The
   release binary was ten times slower than the interpreter on the core
   algorithm of the program, and the cause was an operation order that
   every textbook algorithm uses.
3. **F21, none and generic parameters.** Comparing values read from two
   or three maps is most of what a version control tool does, and the
   language's own absent value cannot reach an ordinary parameter. Either
   guards should count as handling none, or a parameter should be able to
   admit it.
4. **F18, no do-notation for effects.** A command reads five things and
   then writes three. Without a way to bind several effect results in
   one scope, a third of the functions in ugit/ exist to carry values
   from one lambda to the next, and they are what pushes lines past 80
   columns (F19).
5. **F8 and F29, every private name reserved across the module.** A
   helper added in one file broke a parameter in another, and a test
   constant broke the code under test. No-shadowing is a good rule inside
   a function; at module scope it needs to stop at the file, or at least
   at parameters.

The engine bugs (F11, F15, F17) are simply bugs, with reproductions in
bugs/.

Writing ugit in kanso split cleanly into two experiences. The pure core,
the diff, the merge, commit encoding and the merge decision table, was
pleasant: short functions, dispatch where Python would have an if-chain,
and equality that does what it says. Those parts were correct early and
stayed correct. The shell, which is most of a version control tool, was
slower going. Every command is a sequence of reads, a decision, and a
sequence of writes, and kanso asks for each read to be threaded through a
lambda or a helper function. Map lookups that miss, which is how a file
that exists on only one side shows up, cannot reach ordinary parameters.
The 80-column rule then turns each of those workarounds into another
binding or another function. Most of the time I spent on the shell went
into finding a spelling the compiler would accept for code that was
already correct.
