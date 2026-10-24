# Re-sit: rg

Re-sat on 2026-10-24 against main at 78f72a58. Scratch programs for every
status below are in `resit/`.

## Result

- check.sh: **pass**. 95 unit tests, 105 fixtures, 3 engines each, "all green".
- engines agree: **yes on every fixture**, **no on a directory of 100 or more
  files**. There dev and release crash with a stack overflow (release
  sometimes with SIGSEGV) and the interpreter answers. The fixtures never hold
  a directory that large. See Bugs.
- lines changed by the migration: 94 (`diff -r -x resit -x bugs
  ports-orig/rg ports/rg`, lines starting `<` or `>`), in 10 files. No
  expected-output file changed.

## Migration

| # | diagnostic (first line) | told me the fix? | sites |
|---|---|---|---|
| 1 | `error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c`` (ruling 2026-10-13, #1887) | yes | 1 |
| 2 | `error[name]: unknown name `text/concat` — two lists join with `list/concat`, and two strings by interpolation` (ruling 2026-10-13, #1906) | yes | 7 |
| 3 | `error[import]: `list` is not imported here — a module's files share their declarations, not their imports` | yes, though the caret sits on line 1 of the file, not on the call | 1 file |
| 4 | `error[exhaustive]: this can be a none and `text/join` has no arm for it — resolve it here, or give `text/join` a `none` arm (in cli.kso)` (ruling 2026-10-13, #1890: a slice past the end answers none) | partly. It says what is wrong. Its second suggestion, an arm on a library function, should be illegal under the ownership rule (N2). The idiom lib/path.kso uses, a two-arm helper, is not named. I added `some_list` and `some_text` to glob.kso. | 9 |
| 5 | `error[opacity]: `search/bad_usage` is foreign — only `search` builds a `bad_usage`; ask it for one through a pub function` (ruling 2026-10-12, a test file is an outside consumer) | yes. The tests now compare the refusal's sentence through a test-side `complaint` helper. | 5 |
| 6 | the same, for `search/target` | yes. Added `pub fn target_at` to lines.kso so the tests can get one (N3). | 6 |
| 7 | `error[name]: unknown name `ancestors`` (same ruling) | no. It does not say the name exists and is private. Made `ancestors` pub. | 3 |
| 8 | `error[opacity]: `concat` is private to module `list` — only pub names cross an import` in ignore_test.kso, after step 2 left the file importing std/text | no. `list/concat` is pub, and the fix is an import (N4). | 1 |
| 9 | `error[formatting]: a single-expression constant is written inline` on a rewritten test of 82 characters | no; same as F19 | 1 |

## Journal

| F | title | scope | kind | severity | status | evidence |
|---|---|---|---|---|---|---|
| F1 | an entry file cannot hold a single definition | tooling | tooling | minor | partly | `kanso run` still refuses such a file: "is a library — nothing to run … with `kanso play`". `kanso play` is now in `kanso help` and the book uses it (ch02, ch04). `resit/sliceguard.kso` |
| F2 | a regex counted repetition is read as interpolation | language | confusing-semantics | major | still | `resit/f2.kso`: `regexp/find "x{3}" "xxx x3"` prints `x3`; `resit/f2b.kso`: `"o{2,3}"` gives "kanso has no commas" |
| F3 | a boolean function must be renamed with `?` | language | refactoring-hazard | minor | still | `resit/f3.kso`: "error[naming]: `single` answers only true or false: name it `single?`" |
| F4 | a top-level name in one file breaks a local in another | language | refactoring-hazard | major | still | `resit/f4/`: "`escaped` is already a declaration; rename the binding" at m/a.kso:2, the innocent local, and the message still does not name b.kso |
| F5 | list, map and string literals must fit on one line | language | missing-feature | major | fixed | `resit/f5.kso` (a spanning list, both engines print `true`), `resit/f5b.kso` (help text as a `"""` block). Ruling 2026-10-13, #1882 |
| F6 | a map read is always maybe-none; `_` does not catch none | language | confusing-semantics | minor | still | `resit/f6e.kso`: `text/join m["globs"] ","` after a `put` gives "this can be a none"; `resit/f6b.kso`: `_` arm does not cover none; `resit/f6.kso`: `m["invert"] == true` still "comparing to `true` asks a question…". A literal map's known key reads clean (`resit/f6c.kso`) |
| F7 | the pipe skips a none, so `x . or_else d` never applies | language | confusing-semantics | major | fixed | `resit/f7.kso`: `m["nope"] . or_else "default"` prints `mode default` on interp and native |
| F8 | std/regexp has no compiled-pattern value | domain-lib | stdlib-gap | major | still | lib/regexp/regexp.kso exports only `match?`, `find`, `groups`, `find_all`, `match_at`, `matches`, `replace_all`, `split`, each taking the pattern string |
| F9 | an imported name collides with my type, unexplained | language | diagnostic | minor | still | `resit/f9.kso`: "no 3-argument arm of `tally` (arms take 1)". A local *function* `tally` of three parameters works with std/list imported (`resit/f9c.kso`), so this is now a bug: `bugs/type_constructor_hidden_by_import.kso` |
| F10 | `if` inside a list literal is four elements | language | engine-bug | major | fixed | `bugs/if_in_list_literal`: both engines refuse with "`if` takes 3 argument(s), and a list element is one atom … Write `(if …)`" |
| F11 | indexing a list needs a two-sided guard | language | confusing-semantics | minor | still | `resit/f11g.kso`: after `return acc if i > length names`, `shout names[i]` (a function of mine) is still "this can be a none"; the two-sided guard is still what the checker reads. `names[i] == "*"` and `text/uppercase names[i]` pass with no guard at all (`resit/f11f.kso`, `resit/f11c.kso`), because only arms the program wrote are checked; see N5 |
| F12 | no record update syntax | language | missing-feature | major | partly | `resit/f12.kso`: `merge c { "regex":r }` prints `1 2 b false` on both engines (ruling 2026-10-13, #1886). Construction is still positional (named construction declined 2026-07-29), so `config_of`'s 26 positional booleans remain |
| F13 | running an effect per element has no obvious spelling | core-lib | missing-feature | minor | worse | std/list and std/io still have no helper, and the book has no fold over effects. The fold idiom the journal settled on now crashes native at about 65 elements: `bugs/effect_fold_overflows_native.kso` |
| F14 | case folding is ASCII only; no text case functions | core-lib | stdlib-gap | minor | partly | `text/lowercase` and `text/uppercase` exist (ruling 2026-10-13, #1914), but `resit/f14.kso` prints `cafÉ CAFé`: ASCII only, as appendix B says. `(?i)CAFÉ` still misses `café` (domain-lib half, still) |
| F15 | stdout takes strings, so raw bytes cannot be written | domain-lib | stdlib-gap | minor | still | `resit/f15.kso`: "error[runtime]: write takes a string" |
| F16 | no lossy UTF-8 decode, no `\u` or `\0` escapes | core-lib | stdlib-gap | minor | still | `resit/f16.kso`: "unknown escape `\u`"; `resit/f16b.kso`: "unknown escape `\0`"; std/text has only `utf8` |
| F17 | a list of numbers and a byte string look the same | core-lib | confusing-semantics | minor | partly | `resit/f17.kso`: still a run-time refusal, but it now names the line: "find2 takes bytes / failed in the entry at f17.kso:3" (ruling 2026-10-15, #1975). Appendix B still types `find2` as `int[]` |
| F18 | overload ordering forces arms out of reading order | language | aesthetics | nit | ruled-no | 2026-10-13 "gavel: overload arms stay in specificity order", #1888 |
| F19 | a long constant must grow a binding | language | aesthetics | nit | still | `resit/f19m/`: "a single-expression constant is written inline"; hit again in this migration (cli_test.kso:41) |
| F20 | a test helper joins an overload group; verbs disagree | language | engine-bug | major | fixed | the repro in `resit/f20/`: check dir, test dir and check entry all answer ok. Ruling 2026-10-12, "a test file is an outside consumer of its module" |
| F21 | `\W`, `\D`, `\S` read as letters | domain-lib | engine-bug | major | partly | `bugs/regexp_negated_escapes` now prints `o: <none>` as expected. `resit/f21.kso`: `\q` still matches `q` |
| F22 | no is-terminal, file size, lstat or cwd | domain-lib | stdlib-gap | minor | still | std/os gained only `modified`; none of the four asked for |
| F23 | 25 to 100 times slower than rg; interpreter 100 times slower again | tooling | performance | minor | partly | `resit/perf/tree` (5.5 MB, 200 files in 5 directories), shared 4-CPU box: rg 0.010–0.012 s; release `-c TODO` 0.22–0.33 s; release `-n -w 'struct value'` 0.93–1.05 s; dev `-c` 0.54–0.62 s; interp `-c` 31.9 s (was 59 s). Native unchanged, interpreter about twice as fast. One flat directory of 200 files crashes native (Bugs) |
| F24 | `kanso build` writes into the cwd, named for the file | tooling | tooling | nit | still | `kanso help`: "write a native binary here, named for the program"; `resit/perf/bin/` holds `main` and `main.ll` |

Counts: fixed 4, partly 6, still 12, ruled-no 1, moot 0, worse 1.

## New friction

### N1: a slice the code just bounded still answers none
- scope: core-lib
- kind: confusing-semantics
- severity: major
- where: search/cli.kso:287, 326, 534; ignore.kso:43; lines.kso:230; decode.kso:98; walk.kso:166, 175, 177
- wanted: `text/join (text/slice parts 1 i) "/"` after `return acc if i >= length parts`
- wrote: `text/join (some_list (text/slice parts 1 i)) "/"`, with two two-arm helpers, one for lists and one for strings, because the empty value differs:
  ```
  fn some_list none
    []

  fn some_list xs
    xs
  ```
- why it matters: every one of the nine slices runs from a position inside the value to its end or to a bound the code just checked. The checker reads `return … if i > length xs` as a proof for an index (`resit/f11.kso`) but not for a slice (`resit/sliceguard.kso`). The library pays the same cost: lib/path.kso carries `whole none` with the comment "the arm is there because the checker cannot see that". `text/slice!` answers a box, and a span needs `text/to_span`, which answers none too.

### N2: the exhaustiveness message suggests an arm the ownership rule forbids, and the checker accepts it
- scope: language
- kind: diagnostic
- severity: minor
- where: resit/joinarm.kso
- wanted: the fix the message names: "give `text/join` a `none` arm (in cli.kso)"
- wrote: `fn text/join none _` in my own file. It compiles and runs on both engines (`resit/joinarm.kso` prints an empty line for an out-of-range slice).
- why it matters: the 2026-10-12 gavel says an arm is legal only when one of its parameter types belongs to the module that writes it, and the book says a module "cannot add an arm that goes wherever circles go". Neither `none` nor `join` is mine. The message steers a newcomer toward patching the standard library from application code, and the checker lets it happen.

### N3: a test that needs a record must get the module to export a maker
- scope: language
- kind: refactoring-hazard
- severity: minor
- where: search/lines.kso (`target_at`), search/lines_test.kso (6 sites)
- wanted: `searched c (target "f.txt" false) …` in a test, as before
- wrote: `pub fn target_at path with_name` / `target path with_name` in production code, and `target_at` in the tests
- why it matters: the 2026-10-12 ruling is clear and the message says what to do. The cost is a pub function whose only caller is the test file, which widens the module's surface to test it.

### N4: a test file's qualified call to an unimported module says the name is private
- scope: language
- kind: diagnostic
- severity: minor
- where: search/ignore_test.kso:29; reduced in resit/n_priv/
- wanted: to be told to import std/list
- wrote: `import "std/list"`
- why it matters: in a test file, `list/concat` without the import gives "error[opacity]: `concat` is private to module `list` — only pub names cross an import". `concat` is pub. The same call in an entry file says "unknown name `list/concat`" (`resit/n_priv2.kso`), which also does not suggest the import. Inside a module the message is right: "`list` is not imported here".

### N5: an out-of-range index reaches a library function unchecked, and the failure names the library's line
- scope: language
- kind: diagnostic
- severity: minor
- where: resit/f11c.kso
- wanted: either a check-time "this can be a none" for `text/uppercase names[i]`, or a run-time failure naming my line
- wrote: nothing; the program compiles and, with `i` past the end, stops with "error[runtime]: chars takes a string / failed in text/uppercase at std/text/text.kso:114"
- why it matters: the exhaustiveness check covers a none handed to a function the program wrote (F11 still stands there) and lets it through to a library function or to `==`. With no guard, the none fails inside std/text, and the message names std/text's line and not the call that passed it.

### N6: an over-long constant in a `play` file gets an unrelated message
- scope: tooling
- kind: diagnostic
- severity: nit
- where: resit/f19.kso
- wanted: the F19 message, "a single-expression constant is written inline"
- wrote: nothing; `kanso play` on the same shape says "only `if` opens an indented block; other calls take indented arguments on the line's own indent plus two"
- why it matters: the same layout gets two different explanations depending on whether the file is a test or a play file, and the play-file one does not mention the width rule or the binding fix.

### N7: a map literal is corrected one spacing rule at a time
- scope: language
- kind: diagnostic
- severity: nit
- where: resit/f6.kso, resit/f12.kso
- wanted: `{"globs": [], "invert": true}`, the spelling the 2026-10-13 `merge` ruling itself uses (`{"role": "chef"}`)
- wrote: `{ "globs":[] "invert":true }`, after three runs: "no commas", then "exactly one space here" at both braces, then "a map pair is tight: `key:value`"
- why it matters: each run reports one rule and hides the next. A message that printed the canonical form of the whole literal would take one run.

## Bugs

- `bugs/effect_fold_overflows_native.kso` (new): a chain of about 65 effects built with `list/fold` runs out of native stack (dev: "ran out of stack"; release: SIGSEGV). The interpreter runs 100,000. The port's `in_turn` walker uses exactly this, so `rg -c TODO dir` on one directory of 100 files crashes on dev and release (`resit/perf/`); 50 files pass.
- `bugs/type_constructor_hidden_by_import.kso` (new): `type tally` with three fields cannot be constructed when std/list is imported ("no 3-argument arm of `tally` (arms take 1)"), though a local function `tally` of three parameters is found.
- `bugs/if_in_list_literal`: fixed; both engines refuse with a clear message.
- `bugs/regexp_negated_escapes`: `\W` fixed; an unknown escape such as `\q` still matches the letter.
- `bugs/check_verbs_disagree`: fixed by the test-file ruling (`resit/f20/`).

## Comfort

Language and core library first. Starting this port today, the agent would
lose much less time to the language's surface. Long tables are written as
spanning list literals and help text as a `"""` block, which retires F5's
three workarounds. `merge` replaces the 28-line `with_regex`. `x . or_else d`
does what it reads as. An `if` in a list is refused with a message that names
the fix. A test helper can no longer leak into the module or make the verbs
disagree. The core hazards the journal ranked first are mostly unchanged. A
top-level name in one file still breaks a local in another, with the error on
the innocent local (F4). Map reads are still maybe-none, and `_` still does not
catch none (F6). `{n}` in a regex is still interpolation (F2). The `?` rename
is still forced (F3). The slice ruling adds a new tax: nine in-range slices
each needed a none-dispatching helper (N1), and the exhaustiveness message
points at an arm the ownership rule should forbid. The most serious new finding
is a crash: the fold-of-effects idiom from F13, which the walker is built on,
overflows the native stack at about 65 elements. So the port passes its
fixtures and crashes on a directory of 100 files. On the domain side, std/regexp
still has no compiled pattern and `\q` still matches `q`, and io still cannot
write raw bytes. The migration itself was quick: 94 lines over 10 files, and
most diagnostics named their fix. The interpreter is about twice as fast as it
was; native speed is unchanged.
