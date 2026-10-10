# FRICTION: porting GNU diffutils to kanso

Journal kept while porting `diff` (Myers' algorithm as GNU writes it, normal,
unified and context output, recursive directories) and a unified-diff
`patch`. Entries are in the order I hit them. Line references are to the
files as they stand at the end; where the code that hit the problem has since
been rewritten, the entry says so.

## Entries

### F1: a local name loses to a std/list export of the same name
- kind: engine-bug
- severity: major
- where: myers markers (first draft), report/changes.kso (first draft,
  `step`), report/unified.kso (`cursor`, now `place`), patch/apply.kso:69
  (`skipped`, now `passed_over`); repros in bugs/marker_named_like_import.kso
  and bugs/local_fn_loses_to_list_type_in_tests.kso
- wanted: ordinary local names that happen to match something std/list
  exports: markers `type keep`, `type drop`, `type add` for the edit script; a
  fold callback `fn step (walk ...) kept`; a record `type cursor`; a helper
  `fn skipped c outcome`.
- wrote: `kept`/`removed`/`inserted`, `advance`, `place`, `passed_over`, and
  in the end a grep of every declaration against std/list's pub names, which
  turned up `tally`, `max` and `min` as well (renamed to `count_runs`,
  `larger`, `smaller`).
- why it matters: this happened four times and looked different each time.
  The marker `drop` became list/drop: a list of markers printed as
  `[myers/keep <fn>]`, and only because a test file beside it had added
  `import "std/list"` (the file that declared `drop` imported nothing). The
  callback `step` became list/step's constructor, and the fold built
  `list/step` records ("`list/step` has no field `done`"). `cursor` was
  refused with error[opacity] ("only `list` builds a `cursor`"), which at
  least named the problem. `skipped` was the worst: the program behaved
  natively and on the interpreter, but under `kanso test patch` the call built
  std/list's `skipped` record. In the reduced repro, `kanso play` prints 6
  natively and fails with `--interp`, so the engines disagree, and my failing
  test was reporting the engine rather than my code. Chapter 11 says a local
  declaration outranks an import on a tie, and the compiler's own error
  elsewhere says "a module's files share their declarations, not their
  imports"; the `drop` case did neither.

### F2: a type named `fold` is refused as a call to list/fold
- kind: diagnostic
- severity: minor
- where: lines/lines.kso:13
- wanted: `pub type fold` with fields `case` and `space` (how lines fold
  together under -i/-b/-w), constructed as `fold false space_exact`.
- wrote: `pub type folding`.
- why it matters: the message was "error[arity]: no 2-argument arm of `fold`
  (arms take 3)". The file imports std/list and list/fold won over the local
  type's constructor (F1 again, caught this time). The message is about
  arity; nothing says two declarations named `fold` are in play.

### F3: there is no pure "this index is there" read
- kind: missing-feature
- severity: major
- where: lines/lines.kso:80 (`keys_of`), difftool/options.kso:170-178,
  patch/parse.kso (every `given lines[i] ""`)
- wanted: `list/fold (list/zip d.lines list/naturals) t (acc pair -> step
  acc pair[1] pair[2])`, where `pair` is a two-element list zip built; or
  `files[1]` right after checking `length files == 2`.
- wrote: dropped `zip` and took the line number from the accumulator; put a
  `given xs[i] fallback` around every index whose result goes to a function;
  split helpers that returned two-element lists (`[olds news]`) into two
  functions.
- why it matters: `xs[i]` may be none, and the exhaustiveness check refuses
  passing it to a function with no none arm ("this can be a none and
  `lower` has no arm for it"). `xs[i]!` is the "I promise" spelling, but it
  answers an effect, which pure code cannot open, and a length check two lines
  earlier does not count. In a diff, where nearly everything is "line i of
  file a", this shaped much of the program. The check is also uneven:
  `put acc xs[i] v` and `p.xv[x + 1] == p.yv[y + 1]` pass, while
  `stop_at xs xs[i] (i + 1) n` does not.

### F4: `kanso check <module>` says ok to code the entry then refuses
- kind: tooling
- severity: minor
- where: difftool/diff.kso:21 and cli/cli.kso:10 (when both lived in cli/)
- wanted: `kanso check cli` to report what `kanso check .` reports about the
  files in cli/.
- wrote: ran `kanso check .` from the project root every time.
- why it matters: `kanso check cli` printed `cli: ok` while `kanso check .`
  refused the same files with three error[exhaustive] and one error[opacity].
  Checking a library on its own skips whatever is decided from a caller, so
  "ok" from the module check meant less than it said.

### F5: a parameter named `done` is a pattern, not a binding
- kind: confusing-semantics
- severity: major
- where: report/changes.kso (first draft), repro in bugs/field_named_done.kso
- wanted: `fn advance (walk done i j open) kept`, binding the record's `done`
  field (the finished change blocks) to `done`.
- wrote: renamed the field and the binding to `closed`.
- why it matters: `done` is the nullary a finished write yields, so in a
  pattern it is a literal, and the arm only matches when the field holds
  `done`. Nothing at check time; at run time "no overload of
  `report/advance` matches these arguments". The same word is accepted as a
  field name in the type declaration. `none`, `true`, `false` and `done` all
  read as ordinary words and all turn a binding into a literal test. (In a
  body, `done = ...` is refused with error[name], which is the better
  behaviour.)

### F6: native code crashes where the interpreter is right
- kind: engine-bug
- severity: blocker
- where: difftool/diff.kso:99-100, repro in bugs/native_crash_after_reads.kso
- wanted: `announce opts pa pb nested .> (_ -> io/write (shown ...)) .> (_ ->
  effect 1)`: print the `diff -r a/f b/f` header, then the hunks, then answer
  the status.
- wrote: the header and the body built into one string, ending with a single
  `io/write text .> (_ -> effect 1)`.
- why it matters: the interpreter printed the right diff; the native binary
  died with "ran out of stack", "free(): invalid pointer" or
  "munmap_chunk(): invalid pointer" depending on how I sliced it. The repro
  needs two `os/read_file!` results, the real Myers and change-block code,
  and a three-step bind chain; remove any of them and it runs. It took about
  an hour of bisection to find a shape that survives, and I do not know why
  the workaround works, so I cannot tell whether other parts of the port are
  one edit away from the same crash. check.sh runs every fixture on both
  native tiers for that reason.

### F7: names collide across the files of a module
- kind: refactoring-hazard
- severity: major
- where: cli/cli.kso vs difftool/options.kso (`finish`), myers/myers.kso vs
  myers/discard.kso (`more`), myers/myers_test.kso vs myers/discard.kso
  (`run`), myers/discard.kso vs myers/shift.kso (`settle_run`),
  patchtool/patch.kso vs patchtool/options.kso (`named`), patch/patch_test.kso
  vs patch/parse.kso (`body`)
- wanted: private helpers and local bindings whose names matter only inside
  their own file.
- wrote: renamed one side each time, and moved diff's and patch's option
  parsers into separate modules (difftool/, patchtool/) so their helpers
  stopped meeting.
- why it matters: one namespace per module is the documented design, but it
  has three sharp edges in practice. (1) When an edit deleted cli.kso's
  `finish` arms, `run_diff rest .> finish` bound silently to options.kso's
  `finish acc` and failed at run time with "indexing takes a list or
  string". (2) Adding a top-level `fn more` in myers.kso broke the binding
  `more = consec + 1` in discard.kso with error[name], and adding a test
  helper `fn run` in myers_test.kso broke `run = ...` in discard.kso the same
  way: adding a function to one file can stop another file compiling, and a
  test can break the code it tests. (3) Two private `fn given` in two files
  of one module are "overlapping overloads". Short verbs (`finish`, `step`,
  `walk`, `run`, `body`, `more`) get used up quickly, and with F1 std/list's
  vocabulary is used up too.

### F8: reading a map and then growing it copies it, every time
- kind: performance
- severity: major
- where: myers/discard.kso:35 (`count_runs`), myers/compare.kso:82 (`diag`),
  lines/lines.kso:56 (`number`); repro in
  bugs/map_read_then_put_is_quadratic.kso
- wanted: the read-modify-write loops GNU writes over arrays:
  `put seen k (first_seen seen[k] next)` to number lines, `list/tally` to
  count them, and `fd[d - 1]`, `fd[d + 1]`, then `put fd d x` for Myers' V
  vector.
- wrote: code that never reads a map it is about to `put` into. Lines are
  compared by their key strings instead of being numbered through a map;
  counts come from sorting the keys and putting each run's length once; each
  round of the search builds a fresh list with `push` and reads only the
  previous round's list; the discard codes are a list rebuilt left to right.
- why it matters: measured natively. A fold that only `put`s 20,000 int keys
  takes 12 ms; the same fold reading `m[k - 1]` before each put takes 1.0 s,
  whether the read is in a binding, in the put's own argument, or in a caller
  that passes the value down. `list/tally` on 20,000 ints takes 0.7 s for the
  same reason, so the standard library has the problem too. Numbering 4,000
  lines took 1.0 s (now 7 ms). A 20,000-line diff with 100 edits was killed
  for memory after 58 s (now under half a second). A 3,000-line diff with
  about 900 edits had not finished after 120 s with map-based V vectors (now
  0.9 s). callgrind put 92% of the time in `k_map_sort_build` and `msort`, so
  a read after a put seems to re-sort the map. None of this shows in the
  source: the slow and fast versions differ only in where a lookup sits, and
  the book promises `put` is in place "when the map is uniquely owned".

### F9: no vector you can write at an index
- kind: missing-feature
- severity: major
- where: myers/shift.kso (all of it), myers/discard.kso:74-150
- wanted: GNU's shift_boundaries and discard_confusing_lines are loops over
  `char changed[]` with `changed[--start] = 1; changed[--i] = 0;`, cursors
  moving both ways. I wanted a list I could set at an index, or a
  `build`-style block that allowed it for the length of one function.
- wrote: each `while` became a function, the four cursors travel in a
  five-field `walker` record, and the flag arrays became maps from line to
  `true`. The settling pass, which backs up over a run and rewrites it,
  copies each run into a map of its own and copies it back into a list.
- why it matters: the kanso is about as long as GNU's C (256 lines against
  roughly 200), so length is not the cost. The cost is that a faithful port
  is a translation of every loop into recursion with explicit state, and
  each translation is a chance for an off-by-one: I made one in `thin`, where
  the C backs `j` up by the already-incremented `consec`, and only the
  fuzzer against GNU found it. With F8, a map is not a usable stand-in for an
  array either.

### F10: no record update, so options parse into a map first
- kind: missing-feature
- severity: minor
- where: difftool/options.kso:33-198, patchtool/options.kso
- wanted: a default `options` record and one field updated per flag, as
  `{ opts with recursive = true }` in OCaml or `Options { recursive: true,
  ..opts }` in Rust.
- wrote: flags go into a map keyed by strings (`put acc "recursive" true`),
  and `built` reads each back out with `given acc["recursive"] false`, then
  calls the nine-field constructor once with one argument per line.
- why it matters: a map of mixed values loses the checking: a typo in a key
  string is a silently missing option, and every read needs a default. The
  alternative, rebuilding a nine-field record positionally for each flag,
  would put a nine-argument constructor call in twenty places.

### F11: I wrote `given` (a default for none) in five modules
- kind: stdlib-gap
- severity: minor
- where: difftool/options.kso:200, lines/lines.kso:120,
  myers/compare.kso:149, patch/parse.kso:181, patchtool/options.kso:124
- wanted: `m[k] ?? 0`, or a std function `or_else m[k] 0`.
- wrote: `fn given none fallback` / `fn given v _` in each module that
  indexes something whose miss is not interesting.
- why it matters: F3 makes this two-arm helper necessary nearly everywhere,
  and each module needs its own copy or has to export one.

### F12: `text/slice` partly past the end answers nothing at all
- kind: confusing-semantics
- severity: major
- where: patch/apply.kso:57 and :62
- wanted: `text/slice rows (c.pos + 1) (where - 1)`, the lines between the
  last hunk and this one, where a zero-length hunk's `where` can lie past the
  end of the file.
- wrote: `text/slice rows (c.pos + 1) (lesser (where - 1) (length rows))`.
- why it matters: appendix B says out-of-range bounds "yield an empty result
  rather than a failure", and that is what happened: a range that started
  inside the file and ended past it answered no lines, and patch silently
  dropped the last three lines of the file it was patching. Python clamps;
  Go and Rust fail loudly. Answering empty for a range that is only partly
  out of bounds is the one choice that loses data without a sound. The fuzzer
  against GNU found it.

### F13: eighty columns with no way to wrap a string or a list
- kind: aesthetics
- severity: minor
- where: patchtool/patch.kso:183-196 (`cant_find`), cli/cli.kso (usage),
  every `*_test.kso`
- wanted: a long message written as one literal, as adjacent literals, or
  as a list literal spread over several lines.
- wrote: messages cut into bindings (`first`, `second`, `third`, `tail`)
  and glued with interpolation; the usage text as a chain of
  `io/write_err`; tests that compare against a long list split into two lists
  and `text/concat`.
- why it matters: a list literal on several lines is refused ("indentation
  must be 0 or 2 spaces, found 4", then "needless continuation"), and there
  is no string continuation. Tests hit two rules at once: a single-expression
  constant "is written inline", and inline it is 83 columns, so the only
  legal form adds a binding such as `out = ...` whose only job is to satisfy
  the width rule. The width limit was the most frequent compile error in this
  port, and most of the fixes were renames to shorter words.

### F14: positional construction with many fields
- kind: refactoring-hazard
- severity: minor
- where: patchtool/patch.kso:32 (`job`), patch/apply.kso (`reversed`),
  patch/parse.kso:86 (`hunk_at`)
- wanted: `job births: b, born: c, deaths: d, dies: e, ...`, or any way to
  name arguments at a constructor.
- wrote: `job births born deaths dies fp hunks opts`, four booleans in a row
  whose order is fixed by the alphabet; `reversed` lists all eleven of
  `hunk`'s fields in order with two pairs swapped.
- why it matters: swapping two of those booleans compiles and runs. I wanted
  to add a `flipped` field to `hunk` for reject files and chose a workaround
  instead (swapping the marks in the raw lines), because adding the field
  meant editing every constructor call, and a new field shifts the position
  of every field after it.

### F15: what crosses a module boundary is uneven
- kind: diagnostic
- severity: minor
- where: difftool/diff.kso (`loaded`), patchtool/patch.kso:220-244,
  lines/lines.kso:30 (`fold_with`)
- wanted: `fn loaded _ (os/file_not_found p)` and `fn moved?
  (patch/found _ offset)`, qualified the way calls are.
- wrote: the bare forms `(file_not_found p)` and `(found _ offset)`, which
  are accepted; and `pub fn fold_with case space` in lines/ so difftool could
  build a `lines/folding`.
- why it matters: the qualified pattern is refused with "`os/file_not_found`
  is foreign — its structure does not cross an import", while the
  unqualified spelling of the same pattern destructures it without complaint.
  A pub type cannot be constructed outside its module, which is a defensible
  rule, but the message for the pattern case reads as though destructuring
  were the problem.

### F16: std/os cannot tell a file's time, or remove a file
- kind: stdlib-gap
- severity: major
- where: difftool/diff.kso:78-86 (`stamp`), patchtool/patch.kso:307
- wanted: `os/stat path` (or `os/mtime`) for the header line GNU writes as
  `--- a.txt\t2026-10-10 01:07:08.338739821 +0000`, and `os/remove path`
  for a patch that deletes a file.
- wrote: headers with no time at all, except GNU's fixed epoch stamp for a
  file `-N` stands in for; `os/run "rm" ["-f" name]` to delete.
- why it matters: diff's unified and context headers are the most visible
  part of its output, and every one in this port differs from GNU's.
  Deleting through a subprocess works only where `rm` exists. `os/read_file`
  also fails at the executor on bytes that are not UTF-8, so diff reads every
  file with `os/read_bytes` and decodes it with `text/utf8` to decide whether
  it is binary.

### F17: two dispatch corners
- kind: confusing-semantics
- severity: minor
- where: difftool/options.kso:212 (`spacing none`), report/normal.kso:16
  (`command`)
- wanted: `fn spacing _` to catch everything else, a missing map entry
  included; and `fn verb 0 _ c` / `fn verb _ 0 c` / `fn verb _ _ c` for the
  three normal-format commands.
- wrote: an extra `fn spacing none` arm, and `command` as three `return ...
  if` guards.
- why it matters: `_` does not match none, which I learned from an
  error[exhaustive] rather than from the book. The `verb` arms were refused
  as a tie because a call `verb 0 0` matches both; a change is never empty on
  both sides, so the fix is an arm that can never run, or giving up dispatch
  for guards.

### F18: declaration placement rules
- kind: aesthetics
- severity: nit
- where: myers/compare.kso:1-10, patch/apply.kso:24-38
- wanted: a header comment, a blank line, then the first declaration; and a
  small record type (`trimmed_run`, `ordering`, `job`) next to the functions
  that use it.
- wrote: the header comment runs straight into the first type ("the file may
  not begin with a blank line"), and every type sits at the top of its file
  ("canonical order places type declarations before functions").
- why it matters: chapter 07 says declaration order "carries narrative, and
  it stays yours", but types are excepted, so a helper record lives a hundred
  lines from its only user. This is aesthetic.

### F19: the same program resolves imports or not, by how its path is spelled
- kind: tooling
- severity: minor
- where: benchmark programs in a scratch directory (since deleted)
- wanted: `cd pf && kanso check .` for a program whose library imports
  `"../../../lines"`.
- wrote: always passed the program's absolute path or its plain name.
- why it matters: `kanso check pf` and `kanso check /abs/path/pf` say ok;
  `cd pf && kanso check .` and `kanso check ../pf` say "cannot resolve import
  `../../../lines`". The relative path seems to be normalized lexically from
  the root as typed. The compiler's own advice when a build's name clashes
  with a directory ("build it from inside (`cd pf && kanso build .`)") leads
  straight into it.

### F20: bisecting under canonical form
- kind: tooling
- severity: minor
- where: difftool/diff.kso while bisecting F6
- wanted: to replace a call with `print "hello"` and run.
- wrote: each bisection step also had to rejoin a continuation the rule now
  called "needless" and use a binding the rule now called "unused"
  (`print "hello {nested}"`).
- why it matters: the rules are right for finished code. While hunting a
  crash, three of my bisection steps were refused for formatting before they
  could tell me anything.

### F21: the book's single-file programs do not run as written
- kind: tooling
- severity: nit
- where: first experiments, before any port code
- wanted: a file of definitions with `pub play = print ...`, run with
  `kanso run file.kso`, as chapters 04 and 05 show.
- wrote: `kanso play file.kso` for one-file experiments, and a `main.kso` of
  statements plus library directories for the port.
- why it matters: `kanso run` answers "`b.kso` is a library — nothing to run.
  it exports `play`", and an entry file may hold no definitions. The message
  names `kanso play`, so this cost minutes, not hours.

## What worked well

- **Dispatch on literals makes a command-line parser a table.** Every option
  is one arm: `fn long argv i acc "--brief"`, `fn letter argv i acc "u"
  rest`, `fn named argv i acc "--strip" value`. Bundled flags (`-ruN`) fall
  out of a `letter` arm that recurses on the rest of the word. In C this is a
  `getopt_long` table plus a switch; here the table is the code.
- **Markers instead of enums.** `unified_format`, `context_format` and
  `normal_format` are one-line types, and `fn formatted unified_format a b
  ...` picks the printer. Adding a format is adding an arm.
- **The none check found real bugs.** `-U` with no number after it,
  `--label` as the last argument, `-p` without a count: each was an
  error[exhaustive] before it was a crash, and each got an arm
  (`fn counted _ _ _ _ none`) that prints GNU's message.
- **Failures as values at the edges.** `text/utf8` answers an err for bytes
  that are not text, and `fn decoded bytes (err _)` turns that into "Binary
  files differ" in two lines. `os/read_file` answering `file_not_found` made
  "No such file or directory" and `-N` plain dispatch.
- **Structural equality.** Two binary files are compared with `a == b` on
  the records holding their bytes, and the unit tests compare whole outputs
  as lists of strings with `==`.
- **Effects as values made patch's dry run cheap.** The driver builds the
  messages and the writes as values; `--dry-run` returns before binding the
  writes. Backup, result and reject file are one `.>` chain.
- **Two engines and an oracle.** The interpreter is what let me show F6 was
  the compiler's fault and not mine, and check.sh's three-engine comparison
  is one loop.
- **Fast builds.** A dev build of the whole port takes about 0.3 s, so the
  differential fuzzers against GNU could rebuild and rerun after every
  change.
- **Determinism.** `os/list_dir` answers sorted names and map iteration is
  sorted, so recursive diff output is stable without a sort of my own.
- **The predicate rule.** "`same_run` answers only true or false: name it
  `same_run?`" is the review comment I would have made myself.

## Summary

The five I would fix first:

1. **F6**, native code crashing where the interpreter is right. A program is
   not shippable if a three-step bind chain can corrupt the heap, and I
   could not tell why my workaround worked.
2. **F1**, a local name losing to a std/list export, with the two engines
   disagreeing about it. It surfaced four ways (wrong value, wrong
   constructor, opacity error, test-only failure), and the only reliable
   defence was grepping my declarations against std/list's exports.
3. **F8**, a map read before a `put` turning every put into a copy. Each GNU
   algorithm that uses an array became quadratic until rewritten around the
   rule, and the rule is invisible in the source.
4. **F7**, names colliding across the files of a module: adding a helper in
   one file, or a test, can break or silently rebind code in another.
5. **F3**, no pure way to say an index is present. It, and the `given`
   helper it forces (F11), are on most lines that touch a list.

Writing this program in kanso was two different experiences. The parts that
are about shapes (option parsing, the output formats, the patch grammar, the
"missing, binary or text" decisions) were pleasant, shorter than the C
equivalents, and hard to get wrong, because dispatch and the none and err
checks pushed every case into the open. The parts that are about arrays,
which is the core of diff (Myers' V vectors, the discard codes,
shift_boundaries' flag arrays), fought the language at every step: there is
no array to write, maps are slow when read and written together, an index
always might be none, and each `while` became a function with a long
parameter list that had to fit in eighty columns. The result matches GNU byte
for byte on everything I compared, but most of the effort went into finding
shapes the compiler would accept, compile correctly and run quickly, rather
than into diff itself. The naming problems (F1, F5, F7) cost the most
surprise per incident: three different mechanisms made a name mean something
other than what the file in front of me said.
