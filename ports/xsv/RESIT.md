# Re-sit: xsv

Re-sat on 2026-10-24 against main at 78f72a58. Scratch programs for every
status below are in `resit/`.

## Result

- check.sh: **pass**. csv 24 and xsv 51 unit tests pass; "ok: 122 fixtures
  agree on interp, dev and release".
- engines agree: **yes** on every fixture and every bug reduction tried.
- lines changed by the migration: 236 (`diff -r -x resit -x bugs
  ports-orig/xsv ports/xsv`, lines starting `<` or `>`), in 19 files. No
  fixture or expected file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| 1 | `error[exhaustive]: this can be a none and `text/append` has no arm for it` at four slices in csv/read.kso (ruling 2026-10-13, #1890) | partly, as in rg. A `sliced` helper went into csv; one site also needed a binding to stay under 80 columns. | 4 |
| 2 | `error[name]: unknown name `text/concat` — two lists join with `list/concat`` (ruling #1906), then `error[import]: `list` is not imported here` (2 files) and `error[unused]: unused import "std/text"` (2 files) | yes; the import message's caret is on line 1, not on the call | 28 calls in 12 files, 4 import lines |
| 3 | the none message for 12 more slices in xsv (args, cli, floats, join, order, selection) | partly | 12, with `some_slice` and `some_text` added to xsv/strs.kso. The module already has a 2-argument `sliced` in slice.kso, and an arm of another arity in another file is not found (toml `bugs/arity-split-across-files`), so the helper needed a new name. |
| 4 | `error[exhaustive]: this can be an err and `frequencies` has no arm for it` inside `unless_failed bad (_ -> frequencies o sh picked limit)` (log 2026-10-12, #1884) | yes, in the sense that `.>` is the fix | 3 commands (frequency, table, search) rewritten from `bad = list/find [a b] failed?` / `unless_failed bad (_ -> …)` to `a .> (x -> b .> (y -> …))` |
| 5 | `error[exhaustive]: this can be an err and `after_selector` has no arm for it`, and the same for `ranged` and `stepped` | yes | 3 in selection.kso, each now a `.>` |
| 6 | `error[name]: unknown name `columns``, then 18 more private names across the test files (ruling 2026-10-12, a test file is an outside consumer) | no; it does not say the names are private | 20 functions made pub, 2 makers added (`columns_of`, `flag_of`) |
| 7 | `test_select_indexes ... FAILED` / `error[runtime]: unknown name `~resolve`` on 30 tests, after the checker had passed the test file | no. The checker let the private name through and the run-time message is a mangled name. | `resolve` and `parse_args` made pub (Bugs, N1) |
| 8 | `error[exhaustive]: this can be an err and `xsv/value_or` has no arm for it` in tests that bind `o = parsed_with …` | yes, but every spelling of the fix then hit a formatting rule: "needless continuation", "a single-expression constant is written inline", "a line holds at most 80 characters" | 8 tests, now `check = (o -> …)` / `parsed_with … .> check`, one through a test-side `cluster_read?` (N2) |

## Journal

| F | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a guard does not narrow an index | language | missing-feature | major | still | `resit/f1.kso`: after `return acc if i > length fields`, `field_onto acc fields[i]` gives "this can be a none and `field_onto` has no arm for it" |
| F2 | no way to continue a string literal | language | aesthetics | minor | still | no continuation; a one-line `"""` block is refused (ugit `resit/f19.kso`) |
| F3 | types must sit above every function | language | aesthetics | nit | still | ugit's first `resit/f3.kso`: "canonical order places type declarations before functions; move `cursor` up" |
| F4 | a runtime error names the module, not the file | language | diagnostic | minor | fixed | ruling 2026-10-15, #1975; `resit/f5.kso` reports "failed in the entry at f5.kso:3" |
| F5 | `[]` is not bytes, and nothing says so until run time | core-lib | confusing-semantics | minor | still | `resit/f5.kso`: `text/append [] 34` passes check and stops with "append takes bytes and a string, bytes, or byte" |
| F6 | `list/first` answers none unchecked | core-lib | engine-bug | minor | partly | `resit/f6.kso` and `bugs/first_none_unchecked.kso` still pass check and fail at run time, now with the line: "length takes a list, string, or map, not <none> / failed in width at first_none_unchecked.kso:14" |
| F7 | no list patterns in a parameter | language | missing-feature | nit | still | ugit `resit/f6.kso`: `fn joined [] s` gives "expected a parameter pattern" |
| F8 | a none arm makes disjoint record arms tie | language | engine-bug | major | fixed | `bugs/disjoint_arms_tie.kso` prints `x valued` on both engines |
| F9 | std/text has no prefix test, lowercase or padding | core-lib | stdlib-gap | major | partly | `resit/f9.kso` prints `true [   ab] [ab...] key` with `text/starts_with?`, `text/pad_start`, `text/pad_end`, `text/lowercase` (ruling 2026-10-13, #1914). `lowercase` is ASCII-only by its documentation, which was the journal's objection to its own helper |
| F10 | a local may not share a name with any function in the module | language | refactoring-hazard | major | partly | tests no longer join the module (ruling 2026-10-12), which removes three of the eleven cases. Cross-file and ambient collisions remain: rg `resit/f4/`, and `fn keys n` in `resit/perf/m` gave "the name `keys` is already taken" |
| F11 | `&` on a record constructor checks clean and fails differently per engine | language | engine-bug | minor | partly | `bugs/constructor_partial.kso`: both engines now say "no overload of `finish` matches these arguments", so they agree. Check still passes, and the message still blames `finish` |
| F12 | a parameter named `done` becomes a pattern | language | confusing-semantics | major | still | `bugs/param_named_done.kso`: both engines "no overload of `sel` matches these arguments", check clean |
| F13 | an err in a list literal is kept as an element | language | confusing-semantics | major | still | `bugs/err_in_list_literal.kso` prints `1 [err "too big"]` on both engines. The err is a raised one bound to a name, which the 2026-10-12 change otherwise checks |
| F14 | a call must meet an err arm, the same value through a name need not | language | confusing-semantics | minor | partly | errs: fixed (log 2026-10-12, #1884); migration steps 4, 5 and 8 are that rule. Nones through a name are still unchecked (ugit `resit/f13.kso` against `resit/f13b.kso`) |
| F15 | no multi-line string or list literal | language | missing-feature | minor | fixed | ruling 2026-10-13, #1882 (rg `resit/f5.kso`); a `"""` block holds help text (rg `resit/f5b.kso`) |
| F16 | `text/slice` empties when the end is past the end | core-lib | confusing-semantics | major | ruled-no | 2026-10-13, "gavel: `text/slice` past the end answers `none`" (#1890). Clamping was declined; past the end is now none |
| F17 | a map entry's parts cannot be read with a dot | core-lib | confusing-semantics | nit | still | ugit `resit/f16.kso`: "no record type has a field `key`" |
| F18 | an imported name joins the whole module's overloads | language | refactoring-hazard | major | still | `resit/f18/`: join.kso's private `matches`, in a module where only search.kso imports std/regexp, gets "this can be an err and `!=` wants a value". Without search.kso (`resit/f18b/`) it prints `true`. `bugs/import_in_one_file_joins_another/` |
| F19 | "this can be an err" never says where the err comes from | language | diagnostic | major | still | F18's message today still names neither regexp nor `matches`'s other arms; the migration's 25 err messages in toml and xsv name only the receiving function |
| F20 | an err pass-through arm beside literal arms has no legal order | language | diagnostic | major | partly | with the err arm last, `bugs/order_hides_tie.kso` still gets the formatting message. With it first (`resit/f20b.kso`) the tie message now says "write the arm that is most specific in every position", and `fn laid e@(err _) 0` added that way works (`resit/f20c.kso` prints `plain 3` on both engines) |
| F21 | counting by key is quadratic natively | core-lib | performance | major | fixed | `resit/perf/`, release: `list/tally` of 10,000 / 20,000 / 40,000 distinct keys in 0.015 / 0.032 / 0.061 s; `put m k (push (or_list m[k]) i)` the same. Interpreter 0.22 s at 40,000 |
| F22 | `concat` onto a fold's accumulator is quadratic | core-lib | performance | minor | fixed | `resit/perf/`: `list/fold lists [] (acc xs -> list/concat acc xs)` over 40,000 lists in 0.022 s release, 0.31 s interpreter |
| F23 | inline and 80 columns both apply | language | aesthetics | nit | still | met eight times in this migration (N2) |
| F24 | a list "cannot hold a none", except that it can | language | confusing-semantics | minor | still | `bugs/none_in_list.kso` prints `[1 <none> 3] 3 <none> [1 <none>] 2` on both engines |
| F25 | a non-UTF-8 file can only be handled as bytes | domain-lib | stdlib-gap | minor | still | `os/read_file` still answers the text or `file_not_found` |
| F26 | a type and a function share a name across files, unreported | language | diagnostic | minor | still | `bugs/type_fn_same_name`: "no 2-argument arm of `run` (arms take 1)" at the construction; nothing says the name is taken |

Counts: fixed 5, partly 6, still 14, ruled-no 1, moot 0, worse 0.

## New friction

### N1: a test helper may call a private name, and the test fails at run time with a mangled name
- scope: tooling
- kind: engine-bug
- severity: major
- where: xsv/selection_test.kso (`pick_from`), xsv/xsv_test.kso (`parsed_with`); `bugs/test_helper_reaches_private_name/`
- wanted: the checker to refuse `resolve` in a test-file function the way it refuses it in a test constant
- wrote: `resolve` and `parse_args` made pub
- why it matters: `kanso test` printed `test_select_indexes ... FAILED` / `error[runtime]: unknown name `~resolve`` for 30 tests after a clean check. A private name used in a test constant is refused at check ("unknown name"); the same name inside a helper the constant calls is not. The `~` spelling is internal, and nothing says the name is private.

### N2: a test of a raising parser costs a throwaway binding per test
- scope: language
- kind: aesthetics
- severity: minor
- where: xsv/xsv_test.kso (8 tests)
- wanted: the tests as they were: `o = parsed_with ["-s1-2"]` / `value_or o "--select" "" == "1-2"`
- wrote:
  ```
  test_args_short_attached =
    check = (o -> value_or o "--select" "" == "1-2")
    parsed_with ["-s1-2"] .> check
  ```
- why it matters: the binding style is refused now, because `o` carries `parse_args`'s err into `value_or` (#1884). `.>` is the fix. A two-line `.>` chain is a single expression, so it must be written inline, and inline it is over 80 columns. A one-line chain inside a block gets "needless continuation" or "a single-expression constant is written inline". The only spelling the formatter accepts gives the lambda a name that carries nothing.

### N3: twelve in-range slices need a none helper
- scope: core-lib
- kind: confusing-semantics
- severity: minor
- where: csv/read.kso (4), xsv/args.kso, cli.kso, floats.kso, join.kso, order.kso, selection.kso (12)
- wanted: `left = sorted_by (text/slice xs 1 half) cmp` in a merge sort that has just computed `half = n / 2`
- wrote: `left = sorted_by (some_slice (text/slice xs 1 half)) cmp`
- why it matters: this is the cost rg (N1), toml (N2) and ugit (N2) also record. Here it is sixteen sites, including all four slices of a merge sort whose bounds are arithmetic on `length xs`. The helper also could not be called `sliced`, because a 2-argument `sliced` already lives in another file of the module and an arm of another arity is not found across files.

## Bugs

- `bugs/test_helper_reaches_private_name/` (new): a test-file function calling a private module name passes check and fails at run time with "unknown name `~hidden`".
- `bugs/import_in_one_file_joins_another/` (new): std/regexp imported by one file of a module adds its `matches` arms to a private `matches` in another file that imports nothing.
- `bugs/disjoint_arms_tie.kso`: fixed.
- `bugs/constructor_partial.kso`: the engines now agree, but check still passes it and the message blames `finish`.
- `bugs/param_named_done.kso`, `bugs/err_in_list_literal.kso`, `bugs/none_in_list.kso`, `bugs/order_hides_tie.kso`, `bugs/type_fn_same_name`: unchanged.
- `bugs/first_none_unchecked.kso`: unchanged, except that the failure now names its line.
- `bugs/map_read_then_put_quadratic.kso`: not rerun as written; the same shapes in `resit/perf/` are linear (F21).

## Comfort

Language and core library first. The journal said xsv was "a pleasure at
the level of single functions and friction at the level of the module". The
function level got better. The text helpers the port wrote are now in
std/text. The usage screens could be text blocks or spanning lists instead of
chains of `push`. The two quadratic shapes, `list/tally` and concat onto an
accumulator, are linear. Disjoint arms no longer tie. The module level is
about the same. A local still collides with any function in any file (F10).
An import in one file still joins a function in another, now against a
ruling that says it should not (F18). A parameter named `done` is still a
pattern (F12). An err in a list literal is still kept (F13). Messages still
report where a failure surfaced rather than where it began (F19). The binding
rule (#1884) did what F14 asked. In practice it turned the port's "check
several failures at once" idiom (`list/find [a b] failed?`) into nested `.>`
chains, and it made the argument-parser tests fight the formatter (N2). The
re-sit also found a test-runner hole: a private name reached through a test
helper fails only at run time, under a mangled name (N1). The migration took
236 changed lines, mostly the slice helpers and test-surface exports. The
engines agreed throughout.
