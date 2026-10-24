# Re-sit: make

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the re-test are
in `resit/`. A single file is run with `kanso play <file>` (native) and
`kanso play <file> --interp`; a directory with `kanso run .` and
`kanso run . --interp`.

## Result

- check.sh: **pass**. 73 unit tests in four modules, 82 fixtures, three
  engines each.
- Engines agree: yes on every fixture, after a workaround. Before it, 57 of 82
  fixtures failed on the interpreter only, because std/io's new `read_line`
  took over the port's own function of that name (see Bugs).
- Lines changed by the migration: 160 changed lines (`diff -r
  ports-orig/make ports/make -x resit -x bugs`, counting `<` and `>` lines)
  in 10 `.kso` files. 36 of them are the 18 arms of `computed` gaining `pub`.
  No expected file changed.

## Migration

| change | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| `text/slice` past the end answers `none` (gavel 2026-10-13, #1890) | ``error[exhaustive]: this can be a none and `blank?` has no arm for it — resolve it here, or give `blank?` a `none` arm`` | yes, site by site; it took several check rounds, because each fix moved the `none` to the next function (N1) | about 28: a `none` arm on `words/blank?`; `whole` in words (5 calls); a `pub fn or_blank` in words used from update (2); `sub_text` in the reader (13); the module's `or_empty` in expand (3); `no_words` and `frames` for two list slices; the lone-`$` arm `dollar _ _ _ acc ""` became `acc none` |
| `text/concat` is gone (gavel 2026-10-13, `list/concat` joins two lists) | ``error[name]: unknown name `text/concat` — two lists join with `list/concat` ...`` | yes | 4 calls in 3 files |
| wrapped slices made two lines too long | `error[formatting]: a line holds at most 80 characters` | yes | 2, each split with a binding |
| a test file is an outside consumer (gavel 2026-10-12): private names | ``error[name]: unknown name `function_name` `` | yes | 4 functions made pub: `function_name`, `substituted`, `shell_text`, and every arm of `computed` |
| the same: a test cannot build the reader's records | ``error[opacity]: `reader/bare_line` is foreign — only `reader` builds a `bare_line` ...`` | yes | 2 tests compare fields instead |
| std/io's `read_line` (gavel 2026-10-13, library gaps) replaced the port's own `read_line` on the interpreter (Bugs, B1) | ``error[runtime]: `<io>` is not callable`` at update/recipe.kso:38 | no: nothing names std/io, and check passed | the function renamed `recipe_line_of`, 2 lines |

## Journal

| entry | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a file with definitions and statements is neither a program nor runnable | tooling | confusing-semantics | minor | still | `resit/f1.kso` (`pub play = os/run ...`): `kanso run` says ``is a library — nothing to run. it exports `play` ``; `kanso play` says ``error[syntax]: `pub play` is a library's export``. |
| F2 | text/split's empty-separator arm makes every variable-separator call fallible | core-lib | confusing-semantics | major | still | `resit/f2.kso`, the entry's `replace_all`: ``this can be an err and `text/join` has no arm for it``, although the arm above takes `""`. |
| F3 | a helper type has to leave the function it helps | language | aesthetics | nit | still | `resit/f3.kso`: ``canonical order places type declarations before functions; move `seen` up``. |
| F4 | a loop bound proves an index only in one spelling | language | diagnostic | major | still | `resit/f4.kso`, the entry's `gather` with `return effect acc if i > length patterns`: ``this can be a none and `expand_one` has no arm for it``. |
| F5 | a foreign record cannot be taken apart or built, and the import that only patterns use is "unused" | language | engine-bug | major | still | `resit/f5/` (the bug): ``error[unused]: unused import "../a"`` on both engines. With the import removed it becomes ``error[unused]: unused binding `a/hot` `` (N4). `resit/f5b/`: `fn value_of (db/var _ value)` gives ``error[opacity]: `db/var` is foreign``. |
| F6 | a binding may not share a name with a function in any file of the module | language | refactoring-hazard | minor | still | `resit/f6/`: `fn piece` in functions.kso makes `piece = s + 1` in expand.kso fail with ``error[name]: `piece` is already a declaration``. |
| F7 | passing `none` to my own function needs an arm for it | language | confusing-semantics | minor | still | `resit/f7.kso`: `parted ["a"] none`, where `parted` only builds a record, gives ``this can be a none and `parted` has no arm for it``. |
| F8 | an arm after a `none` arm still answers "can be none" | language | diagnostic | major | partly | A two-parameter group now narrows: `resit/f8.kso` (`classified head none` then `classified _ found`) prints `:=` on both engines. The entry's five-parameter shape does not: `resit/f8m/`, `classified head none false _ _` then `classified _ found false _ _`, still makes `classify` answer "can be none" at the test's `op_of` call. |
| F9 | a list of twelve short words cannot be written | language | aesthetics | minor | fixed | Gavel 2026-10-13, "a bracketed list or map may span lines". `resit/f9.kso` writes the twelve directives as one spanning list and prints `12` on both engines. |
| F10 | a literal `{` in a string needs an escape the book never shows | language | diagnostic | minor | still | `resit/f10.kso`: `print "{"` gives `error[syntax]: unterminated interpolation`, with no hint. `\{` appears in ch08's sample code but is not explained in ch02, ch08 or appendix C. |
| F11 | the native engines crash on ordinary effect loops | language | engine-bug | blocker | fixed | Log 2026-10-10, "two native crashes the ports found in the arena". `resit/f11/` (the bug) prints `1` from `kanso run`, a dev binary, a release binary and the interpreter. With the four `effect done .>` workarounds removed from a copy of the port (expand.kso `each`, recipe.kso `expanded_lines` and `each_line`, reader.kso `each_target`), check.sh passes all 82 fixtures on all three engines. |
| F12 | there is no record update | language | missing-feature | major | fixed | `merge` (gavel 2026-10-13, #1886). `resit/f12.kso` writes `with_temporary` for a seven-field `progress` as `merge rn { "temporary":(push rn.temporary t) }` and prints `["a.o"] ok` on both engines. It is still only allowed in the declaring module (F5). |
| F13 | an imported name takes over a local one | language | refactoring-hazard | major | partly | A local function named like an imported type now wins (log 2026-10-12, "a name the author declared wins over a builtin and an import"): `resit/f13b/`, a local `fn parsed argv i o` beside an imported `pub type parsed`, prints `x 1 0` on both engines. A local type named like an imported function still loses: `resit/f13/`, `type run` in a file importing std/os, gives ``error[arity]: no 5-argument arm of `run` (arms take 2)``. A new case of the same family is B1. |
| F14 | a name may be a type, a field, a parameter or a test constant, but only one of them in a module | language | refactoring-hazard | minor | partly | A test constant no longer collides with the module (gavel 2026-10-12): `resit/f14t/` has a constant `parts` beside a parameter `parts` and passes. A field named after its type still cannot be destructured: `resit/f14.kso`, ``error[name]: `pending` is already a declaration``. |
| F15 | std/os cannot read a file's modification time | domain-lib | stdlib-gap | blocker | fixed | `os/modified` (gavel 2026-10-13, library gaps). `resit/f15.kso` prints `1000000000000 os/file_not_found "gone.c"` on both engines. Milliseconds, where the port's `stat` reads nanoseconds. The port still calls `stat`; moving it is not part of this migration. |
| F16 | std/os has no remove, no working directory and no environment for a child | domain-lib | stdlib-gap | major | still | std/os exports `exit`, `exists`, `is_dir`, `list_dir`, `make_dir`, `read_file`, `read_bytes`, `run`, `start`, `kill`, `write_file`, `modified`, `env`. No remove, no working directory, no child environment. |
| F17 | an index guard on a field does not count | language | diagnostic | minor | still | `resit/f17.kso`: `return "done" if i < 1 or length r.lines < i` then `statement r.lines[i]` gives ``this can be a none and `statement` has no arm for it``. |
| F18 | expansion is an effect, so none of it can be unit-tested | tooling | missing-feature | major | still | `resit/f18/`: `test_expand = f18/expand "a" == "xa"` gives ``error[effect]: this is an effect ... and `==` takes a value``. Written as `.> (v -> effect (v == "xa"))`, the runner prints `FAILED (returned <io>)`. |
| F19 | no list patterns, and a boolean cannot continue onto the next line | language | missing-feature | minor | partly | A line ending in `or` now continues onto the next: `resit/f19.kso` prints `true false` on both engines. A line starting with `or`, the entry's spelling, is still `expected an expression` (`f19b.kso`). `fn begin o []` is still `expected a parameter pattern` (`f19c.kso`). Appendix C lists only `.`, `.>`, `.!` and `.?` as continuation forms (N3). |
| F20 | tables of text fight the eighty-column rule | language | aesthetics | minor | fixed | Spanning list literals (gavel 2026-10-13) and `"""` text blocks. `resit/f9.kso` prints a usage block written as a text block. |
| F21 | reducing a bug fights the same rules as writing code | tooling | tooling | minor | still | Reducing B1 and the datalog re-sit's interpreter overflow met the same refusals: `error[unused]`, the 80-column rule, and ``an entry file holds statements only`` for a one-file reproduction. |

Counts: fixed 5, partly 4, still 12, ruled-no 0, moot 0, worse 0.

## New friction

### N1: every in-range slice needs a `none` sink, one function at a time
- scope: core-lib
- kind: refactoring-hazard
- severity: major
- where: words/words.kso, expand/expand.kso, expand/functions.kso, reader/lines.kso, reader/conditional.kso, update/*.kso
- wanted: the slices as they were. Each range is computed from a position the code just found in the same string, so none of them can run past the end.
- wrote: about 28 wraps in five helpers (`whole`, `or_blank`, `sub_text`, `or_empty`, `no_words`), plus a `none` arm on `blank?`. The check reports one module's slices at a time, so it took several rounds: fixing `words` exposed `expand`, which exposed the reader.
- why it matters: make is a text processor, and most of its slices are "from here to the end" or "up to the separator I found". `text/slice!` answers a box, which would make pure code effectful, and `text/to_span` answers `none` too, so the only spelling for a slice the code knows is in range is a `none` arm that never runs. std/regexp and std/path carry the same helper (`or_empty`, `whole`). Each module needs its own copy, because a private helper does not cross a module.

### N2: a `none` slice interpolated into a string prints `<none>`, and check passes
- scope: language
- kind: confusing-semantics
- severity: minor
- where: words/words.kso:`fill`, which builds `"{before}{stem}{text/slice pattern (at + 1) (length pattern)}"`
- wanted: the check that refuses a `none` passed to a function to refuse it in an interpolation.
- wrote: nothing; this range is in bounds.
- why it matters: `print "[{text/slice "ab" 3 4}]"` checks ok and prints `[<none>]` on both engines. Before the slice ruling the same text printed `[]`. A program that built a string from a slice that has gone past the end now writes `<none>` into its output with no refusal and no runtime error.

### N3: a line that ends in an operator continues, though appendix C says it cannot
- scope: language
- kind: confusing-semantics
- severity: nit
- where: resit/f19.kso
- wanted: to know which line breaks are legal.
- wrote: `word == "ifndef" or` at the end of one line and `word == "else" or word == "endif"` on the next, indented. It compiles and runs.
- why it matters: appendix C lists `.`, `.>`, `.!` and `.?` as the only continuation forms, and the 2026-10-13 spanning gavel says operator chains "are still broken by naming their parts". The compiler also accepts a trailing `==` (see the datalog and httpd re-sits, F4 and F9). Either the book is out of date or the compiler accepts a form the language does not have.

### N4: a qualified marker in a pattern is reported as an unused binding
- scope: language
- kind: diagnostic
- severity: minor
- where: resit/f5/b/b.kso:12
- wanted: with the "unused" import of `../a` removed, an error saying `a/hot` names a module the file does not import.
- wrote: nothing; this came up while re-testing F5.
- why it matters: the message is ``error[unused]: unused binding `a/hot` ``. It reads `a/hot` as a parameter name. With the import kept, the error is that the import is unused. Neither message says how to write a dispatch on another module's marker.

### N5: tests can no longer reach the pure pieces they were written for
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: expand/expand.kso, expand/functions.kso, reader/lines_test.kso
- wanted: the expand tests as they were, on `function_name`, `substituted`, `shell_text` and `computed`.
- wrote: those four made pub, including all 18 arms of `computed`.
- why it matters: F18 says expansion can only be unit-tested through its pure pieces. Since the 2026-10-12 gavel those pieces must be pub to be tested at all, so the expander's public surface now includes every function its tests touch.

## Bugs

### B1: a file's own function loses to std/io's `read_line`, on the interpreter only
- reduction: `bugs/local_loses_to_io_read_line/read_line.kso` (24 lines)
- should: print `1:x` on both engines. The file declares `fn read_line at s` and calls `read_line 1 "x"`. The 2026-10-12 log entry says a name the author declared wins over a builtin and an import.
- does: native prints `1:x`. The interpreter stops with ``error[runtime]: `<io>` is not callable`` at the call. Nothing refuses it at check.
- in the port: update/recipe.kso has had a private `read_line` since the port was written; std/io gained `read_line` on 2026-10-13. 57 of 82 fixtures failed on the interpreter and passed natively. The workaround renames the port's function `recipe_line_of`.

## Comfort

On language and core-lib, the original agent's top item is gone, and its
third is half gone. The native crash in effect loops (F11) no longer happens:
the reduction passes, and the whole port passes on three engines with the four
`effect done` trampolines removed. `merge` replaces the eleven positional
`with_*` rebuilds, and spanning list literals and text blocks remove the
width workarounds in F9 and F20. A local function named like an imported type
now wins, and test constants no longer collide with the module. What stands:
records still cannot be built or taken apart outside their module, a local
`type run` still loses to `os/run`, index guards still work in one spelling on
a plain name only, `text/split` with a variable separator is still fallible,
and there are still no list patterns. The migration itself was the most
expensive of the four ports. make is a text processor, and the slice ruling
needed about 28 `none` sinks for ranges the code had already bounded, found
one module per check round. A library addition, `io/read_line`, quietly
replaced one of the port's own functions on the interpreter alone. On
domain-lib, `os/modified` now gives make its file times, but std/os still has
no remove, working directory or child environment. On tooling, effects still
cannot be unit-tested, and single-file experiments and reductions still fight
the ship-quality rules.
