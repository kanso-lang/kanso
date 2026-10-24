# Re-sit: datalog

Compiler: main at 78f72a58 (2026-10-24). Scratch programs for the re-test are
in `resit/`. A single file is run with `kanso play <file>` (native) and
`kanso play <file> --interp`; a directory with `kanso run .` and
`kanso run . --interp`.

## Result

- check.sh: **pass**. 51 unit tests, 26 fixtures, three engines each.
- Engines agree: yes on every fixture, after a workaround. Before it, the
  interpreter ran out of stack on `tc_large` where both native tiers finished
  (see Bugs).
- Lines changed by the migration: 144 changed lines (`diff -r
  ports-orig/datalog ports/datalog -x resit -x bugs`, counting `<` and `>`
  lines) in 14 `.kso` files. `datalog/agrees_test.kso` became the new module
  `testkit/testkit.kso` (18 lines). No `expected` file changed.

## Migration

| change | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| `text/concat` is gone (gavel 2026-10-13, `list/concat` joins two lists) | ``error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation, `"{a}{b}"` `` | yes | 22 calls in 8 files |
| `std/text` left unused once `concat` moved | `error[unused]: unused import "std/text"` | yes | 4 files |
| `merge` is now an ambient builtin (gavel 2026-10-13, #1886), and the port had a private `merge` | ``error[name]: the name `merge` is already taken`` | partly: it does not say what took the name (N2) | 1 function, 3 lines; renamed `united` |
| a name bound to a call that can raise carries the err into the check (log 2026-10-12, #1884) | ``error[exhaustive]: this can be an err and `define` has no arm for it — dispatch on it here, or give `define` an `(err _)` arm`` | partly: it offers a dispatch or an `(err _)` arm, and the book's own fix, a `.>` step, is not mentioned (N1) | 17 functions: 13 in parser.kso, 2 in check.kso, 2 in run.kso, each `x = f ...` then `g x` rewritten as `f ... .> (x -> g x)` |
| `text/slice` past the end answers `none` (gavel 2026-10-13, #1890) | ``error[exhaustive]: this can be a none and `text/join` has no arm for it`` | yes | 2: lexer's `cut` wrapped in the module's `or_empty`; run_test's first-n-lines slice became `list/take` |
| a test file is an outside consumer of its module (gavel 2026-10-12) | ``error[name]: unknown name `agrees` `` | no: the helper is declared one file away, and nothing says test files no longer share declarations (N3) | `agrees`, `then` and `with_line` moved to a `testkit` module; 5 test files import it |
| the same ruling: a test cannot build the module's records | ``error[opacity]: `datalog/input` is foreign — only `datalog` builds a `input`; ask it for one through a pub function`` | yes, though I parsed an `.input` directive instead of adding a pub builder (N4) | 1 |
| a test's pipeline now raises at `parse` | ``error[exhaustive]: this can be an err and `testkit/agrees` has no arm for it`` | yes | `agrees` gained an `(err _)` arm; strata_test's pipeline written with `.>` |
| interpreter stack overflow (Bugs, B1) | `error[runtime]: the program ran out of stack: recursion went deeper than the stack holds` | no: it names `skip_block`, which never runs | eval.kso's `emit` renamed `emit_row`, 2 lines |

## Journal

| entry | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a file that defines functions cannot be run, and the book says it can | tooling | tooling | minor | still | `kanso run examples/guards.kso`: ``error: `guards.kso` is a library — nothing to run. give the module a main.kso entry, or run its definitions beside their statements with `kanso play` ``. `kanso play` runs it. A `pub play =` file such as `docs/book/samples/ch05/count.kso` is refused by both verbs, and ch05 still shows it run with `kanso run`. |
| F2 | a comment block followed by a blank line is "the file beginning with a blank line" | language | diagnostic | nit | fixed | Gavel 2026-10-13, "a file may open with a header comment and one blank line". `resit/f2/m.kso` (header, blank line, commented type) checks ok. |
| F3 | importing std/list in one file replaces a private type in another file | language | engine-bug | major | fixed | Gavel 2026-10-12, "arms travel with the value, and a file names only what it can see", and log 2026-10-18, "A module's own type beats an imported type's twin". `resit/f3/` (the bug unchanged) prints `m/cursor 1 3` on both engines. |
| F4 | a long `==` in a test cannot be wrapped, so it gets a throwaway name | language | aesthetics | nit | fixed | `resit/f4/m_test.kso`: `test_x = f4/spelled "..." ==` with the right side on an indented next line compiles and passes. The indented-whole-expression form is still refused (`a single-expression constant is written inline`). The 2026-10-13 spanning gavel says operator chains are "still broken by naming their parts", so the accepted trailing `==` is either undocumented or unintended. |
| F5 | a call that can fail must be dispatched on before a lambda or `==` sees it, even in a test | language | confusing-semantics | minor | ruled-no | Log 2026-10-12, "a name bound to a raising call carries its err into the check" (#1884, under part 3 of the 2026-09-15 gavel), closed the escape the entry used: binding the call to a name first. `resit/f5.kso` refuses `list/map (lex source) ...` and `resit/f5b.kso` refuses `bound = lex source` then `text/join bound "-"`. The book's answer is a `.>` step. This ruling cost 17 functions in this migration. |
| F6 | a new top-level name in one file breaks local bindings in another | language | refactoring-hazard | minor | still | `resit/f6/`: `fn tok` in parser.kso makes `tok = x + 1` in lexer.kso fail with ``error[name]: `tok` is already a declaration; rename the binding``, pointing at the lexer. |
| F7 | a pattern variable spelled like a marker type becomes a literal match, silently | language | confusing-semantics | major | still | `resit/f7.kso` (the entry's case) prints `no match` on both engines, with nothing at check. |
| F8 | std/expect is in lib/ but "not in the shipped library" | tooling | stdlib-gap | minor | still | `resit/f8/m_test.kso` importing `std/expect`: ``error: `std/expect` is not in the shipped library``. |
| F9 | no list patterns in a function head | language | missing-feature | nit | still | `resit/f9.kso`: `fn show_atom name []` gives `error[syntax]: expected a parameter pattern`. |
| F10 | a map used as a set trips the "comparing to `true`" lint, and there is no membership test | core-lib | diagnostic | minor | still | `resit/f10.kso`: ``comparing to `true` asks a question the value already answers``. No `has?` or set type in lib/. |
| F11 | `none` cannot seed a fold, so absence needs a private marker | language | confusing-semantics | minor | still | `resit/f11.kso`: ``this can be a none and `list/fold` has no arm for it`` at the literal `none` seed. |
| F12 | string literals and list literals cannot span lines | language | missing-feature | minor | fixed | Gavel 2026-10-13, "a bracketed list or map may span lines"; `"""` text blocks cover strings. `resit/f12.kso` writes the four-rule program as a text block and the long message as a spanning list, and prints `61 2` on both engines. |
| F13 | a parameter cannot be named after its type | language | aesthetics | nit | still | `resit/f13.kso`: ``error[name]: `plan` is already a declaration; rename the binding``; `resit/f13b.kso`: the same for a field destructured as `values`. |
| F14 | importing std/list replaces a private constructor with a std function, and nothing says so | language | engine-bug | blocker | still | `resit/f14/` (the bug unchanged): `kanso check` passes, and both engines stop with ``no overload of `m/describe` matches these arguments``. The 2026-10-12 fixes made a declared function beat an imported type, and a declared type beat an imported type; a declared type still loses to an imported function. |
| F15 | runtime errors carry no location | language | diagnostic | major | fixed | Gavels 2026-10-15 and 2026-10-17 on run-time failures naming their line. `resit/f15.kso` stops with `join takes a list of strings` / `failed in shown at f15.kso:7` on both engines. |
| F16 | `[if c a b]` is a four-element list on one engine and a refusal on the other | language | engine-bug | major | fixed | Log 2026-10-10, "four faults the ports found". `resit/f16b.kso`: ``error[syntax]: `if` takes 3 argument(s), and a list element is one atom ... Write `(if …)` `` at check, on both engines. |
| F17 | a bounds proof needs the list in a local name, not a field | language | confusing-semantics | nit | still | `resit/f17.kso`: the guard `return 0 if i < 1 or length pl.steps < i` followed by `exec pl.steps[i]` is still refused with ``this can be a none and `exec` has no arm for it``. |
| F18 | a map read after a write sorts the whole map, so read-modify-write loops are quadratic | core-lib | performance | blocker | partly | `list/group_by` is fixed: 32,000 scattered keys in 0.12 s natively (`resit/f18/g.kso`; the journal had 1.8 s). The entry's own `file_in` (read a bucket, push, put) is still quadratic: `resit/f18/f*.kso` takes 0.68 s at 4,000 tuples, 2.9 s at 8,000, 12.6 s at 16,000 and 72 s at 32,000. |
| F19 | a collection held in a record is copied on every update | core-lib | performance | major | partly | A map in a record is fixed: 40,000 puts through `rel (put r.seen n true)` take 0.23 s, and 80,000 take 0.25 s (`resit/f19/putm*.kso`; the journal had 54 s). A list in a record is not: 20,000 pushes take 1.3 s and 40,000 take 5.0 s (`push*.kso`). A map nested in a map is not: 10,000 puts take 3.3 s (`nested.kso`). |
| F20 | two-parameter dispatch with a `none` arm ties with literal arms | language | confusing-semantics | minor | fixed | Log 2026-10-10, "four faults the ports found" (disjoint patterns cannot tie). `resit/f20.kso`, the entry's `add_op` arms, prints `["1x"] []` on both engines. |
| F21 | `list/sort` sorts only scalars and takes no key | core-lib | stdlib-gap | major | partly | `list/sort_by` exists (gavel 2026-10-13, library gaps) and sorts pairs by their first element (`resit/f21.kso`). `list/sort` on lists still stops at run time with `comparison requires two values of one comparable type`, `failed in list/pick at std/list/list.kso:476`, which names the library's line and not the caller's. |
| F22 | `list/to_list (list/map ...)` everywhere, and no range | core-lib | aesthetics | nit | still | `list/map` is still lazy, and indexing its result stops at run time (`resit/f22.kso`: `a lazy sequence becomes a list with list/to_list`). No range function in std/list. |
| F23 | type declarations must open the file, away from their one use | language | aesthetics | nit | still | `resit/f23.kso`: ``canonical order places type declarations before functions; move `refused` up``. |
| F24 | no record update, so a seven-field state is rebuilt positionally | language | missing-feature | minor | fixed | `merge` (gavel 2026-10-13, #1886). `resit/f24.kso` updates two of seven fields with `merge acc { "bit":(acc.bit * 2) "ops":(push acc.ops "skip") }` and prints `2 ["skip"]` on both engines. |
| F25 | map entries have no field names to read | core-lib | diagnostic | nit | still | `resit/f25.kso`: ``error[name]: no record type has a field `key` ``. Appendix B still describes `entries` as "each an entry key value record". |
| F26 | the interpreter panics with a Rust backtrace when stdout closes | tooling | tooling | nit | fixed | Log 2026-10-12, "stdout reaches its reader, and a closed pipe stops every engine quietly". `resit/f26.kso` piped into `head -1` under `--interp`: prints one line, exits 141, and writes nothing to stderr. |
| F27 | no scratch file can import the module under work | tooling | tooling | minor | still | `resit/f27.kso`: ``error[import]: a play file imports the stdlib and nothing else``. The message now says what to do instead. |
| F28 | the interpreter is fifty times slower than a release build | tooling | performance | minor | still | `tc_large` today: 2.43 s interpreted, 0.18 s release (three runs, 0.18 to 0.20 s). The interpreter is about where the journal had it (2.66 s); the ratio is smaller because the release run is slower than the journal's 52 ms. Lexing alone takes 85 to 100 ms natively. The journal's number came from another machine and sitting, so this is not evidence of a regression until both compilers are timed on one box. |

Counts: fixed 9, partly 3, still 15, ruled-no 1, moot 0, worse 0.

## New friction

### N1: the err diagnostic offers two fixes and leaves out the one the book teaches
- scope: language
- kind: diagnostic
- severity: minor
- where: datalog/parser.kso (13 functions), check.kso, run.kso
- wanted: to be told to write `term_at ts p (tok ts p) .> (arg -> more_arguments ts arg acc)`.
- wrote: that, after reading chapter 04, which says "when a step should run only on a value, .> says so".
- why it matters: the message says ``dispatch on it here, or give `more_arguments` an `(err _)` arm``. Both work, but both are longer than the `.>` step, and the second adds an arm to a function that should never see an err. The journal listed the railway ("`run_tree` is five bindings and a call") under what worked well. Today it is three `.>` steps, and the compiler did not say so.

### N2: a new ambient name breaks a program that declared it, and the message hides which
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: datalog/strata.kso:171
- wanted: the private `fn merge a b` the port had, or a message naming the builtin that now owns the word.
- wrote: renamed it `united`.
- why it matters: ``error[name]: the name `merge` is already taken`` does not say what took it. Adding an ambient builtin (`merge` and `delete` both arrived on 2026-10-13) can break any program that used the word. The 2026-10-12 entry says a declared name wins over a builtin, which is not what happened here.

### N3: test files of one module no longer share helpers, and the error does not say why
- scope: language
- kind: diagnostic
- severity: minor
- where: datalog/check_test.kso and four others
- wanted: `agrees`, declared once in `agrees_test.kso`, used by every test file, as before.
- wrote: a new module `testkit/` with the helpers made pub, imported by five test files.
- why it matters: ``error[name]: unknown name `agrees` `` appeared 12 times for a name declared in a sibling file. Under the 2026-10-12 gavel each test file is its own consumer, which explains it, but the message does not mention that, and a module with several test files now needs a second module to share assertions.

### N4: a test cannot build the record a function takes
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: datalog/facts_test.kso:5-14
- wanted: `facts_from (input "f.facts" 3 "f") contents`, as before.
- wrote: parse `"\n\n.input f"`, take the first clause, and add a `none` arm for the index.
- why it matters: the function under test takes an `input` record, and only the module may build one. The choices are a pub builder that exists only for tests, or reaching the record through the parser, which makes the facts test depend on the parser.

### N5: a computed value in a map literal must be parenthesized, and the error does not say so
- scope: language
- kind: diagnostic
- severity: nit
- where: resit/f24.kso
- wanted: `merge acc { "bit":acc.bit * 2 "ops":push acc.ops "skip" }`.
- wrote: `{ "bit":(acc.bit * 2) "ops":(push acc.ops "skip") }`.
- why it matters: the first form gives `error[syntax]: expected an expression` at the `*`. With `merge`, a map literal is now how a record field is updated, so computed values in one are the common case. The list literal's error for the same mistake (`if` above) explains the one-atom rule; this one does not.

### N6: `length` and `push` are not covered by the err and none checks
- scope: language
- kind: confusing-semantics
- severity: minor
- where: resit/f5b.kso
- wanted: `bound = lex source` then `length bound` refused, the same as `text/join bound "-"`.
- wrote: nothing; this came up in the re-test.
- why it matters: the 2026-10-12 entry says a raised err is carried "to every operator, index and argument position that reads it". `text/join bound "-"` is refused, and `length bound` in the same file passes check. The diff re-sit found the same gap for a `none` reaching `push`.

## Bugs

### B1: the interpreter runs out of stack on a tail-call loop, depending on a same-named function in an earlier file
- reduction: `bugs/interp_overflow_split_arity/` (an entry and a two-file module, 40 lines)
- should: print `30000 1` on both engines.
- does: native prints `30000 1`. The interpreter stops with `the program ran out of stack` and names `m/skip` at `m/b.kso:15`, a function that never runs. It needs three things together: a second `emit` of another arity in a file that sorts before the loop's file, a `skip` arm whose body is a block `if`/`else` with a binding in the `else`, and the loop's step being named `emit`. Remove any one and it passes.
- in the port: datalog's lexer (`emit`/5 in lexer.kso) and evaluator (`emit`/4 in eval.kso). `tc_large` (3,000 lines) failed on the interpreter only. The workaround renames eval.kso's `emit` to `emit_row`.
- also: the 2026-10-17 gavel says a stack overflow "keeps its sentence with no line", and this message names a line, the wrong one.

## Comfort

On language and core-lib, the original agent would have an easier time with
the bugs and a harder time with the railway. Four of its top five are
partly or wholly answered. Imports no longer replace a private type in another
file (F3). Runtime errors name their line (F15), so the bisection the summary
complained about is unnecessary. The engines agree on `[if ...]` (F16).
`group_by` and a map held in a record no longer go quadratic. Two of the five
stand: a private type named like a std/list function still loses silently
(F14), and a pattern variable spelled like a marker still becomes a literal
(F7). The read-bucket-push-put loop at the heart of a hash index is still
quadratic (72 s for 32,000 tuples), and so is a list held in a record. Record
update with `merge`, spanning literals and text blocks would remove most of the
test-file glue and the seven-field rebuilds. The err railway the agent praised
now needs a `.>` at each stage. Seventeen functions changed for that, and the
diagnostic did not name `.>`. Test files can no longer share helpers or build
the module's records. A new interpreter-only stack overflow, which depends on
an unrelated function in another file, cost the most time in this migration,
and its message points at a function that never ran. On tooling, single-file
experiments still cannot import the module, std/expect is still unshipped, and
the interpreter is still slow enough to limit fixture size.
