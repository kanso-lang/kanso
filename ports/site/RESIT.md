# Re-sit: site

## Result

check.sh: pass (114 unit tests in five modules; 20 fixture cases and the
150-post scale case agree on interp, dev and release), engines agree: yes.
Lines changed by the migration: 607 (`diff -r -x resit -x bugs
ports-orig/site ports/site | grep -c '^[<>]'`), in 22 files. Most of it is
68 pass-through err arms. No expected-output file changed.

Scratch programs are in `resit/t/`; the check log is `resit/check.log`.

## Migration

| diagnostic (first line) | told me the fix? | sites |
|---|---|---|
| ``error[syntax]: `and` and `or` meet without parentheses — parenthesize the part that goes first, as in `(a and b) or c` `` (#1887) | yes | 4 |
| ``error[name]: unknown name `text/concat` — two lists join with `list/concat` `` (#1906), then ``error[unused]: unused import "std/text"`` where it was the last text call | yes | 14 calls in 8 files; 2 imports removed |
| ``error[exhaustive]: this can be a none and `upper` has no arm for it`` and the like in textkit, the first module checked (`text/slice` past the end answers `none`, #1890) | partly. It names the none. Some of these ranges really do run past the end (`text/slice s 2 (length s)` on `""`), and the journal's F20 shows the port already leaned on a clamp. I rewrote the port's own `textkit/clamped` to clamp and then turn a none into an empty value of the same kind, and sent all 120 `text/slice` calls through it | 120 calls in 16 files; 2 imports added; 7 lines re-split for 80 columns; one new local collided with a declaration in another file (``error[name]: `trimmed` is already a declaration``, F10) |
| ``error[exhaustive]: this can be an err and `all_read` has no arm for it — dispatch on it here, or give `all_read` an `(err _)` arm``: 80 refusals in tera, then more in toml and zola as each layer was fixed. A name bound to a raising call now carries its err (2026-10-12, #1884) | yes, both remedies it names work. The work is in the volume and in placing the arms (next row) | 68 `e@(err _)` arms; about 20 call sites rewritten as `.>` steps, mostly where the receiver was `list/map`, `list/fold`, `list/select` or a group with literal arms |
| ``error[formatting]: overloads of `X` appear most-specific first: literal, then concrete type, then generic`` and ``error[dispatch]: these `X` arms tie`` on the new err arms | no. Where an err arm goes in a group with literal or record arms took trial and error (see N1) | about 15 groups; 8 groups (`binary`, `check_test`, `emit`, `set_in`, `feed`, `not_found`, `appended`, `if_next`) gave up their err arm for a `.>` at the caller |
| ``error[name]: unknown name `surely` `` in tera_test.kso: a helper defined in lex_test.kso, another test file of the same module (#1883) | no | 1 helper copied |
| ``error[opacity]: `tera/engine` is foreign — only `tera` builds a `engine`; ask it for one through a pub function`` | yes; `engine_of` already existed | 1 |
| `error[runtime]: unknown name `~pieces`` at test time, for a private name inside a test file's helper function (bug below) | no; it passed the check | `pieces`, `tokens` and three token types made pub |
| ``error[name]: unknown name `date_prefix` `` and 8 more in zola tests; ``error[name]: `sorted_content` is not a type, so a binding cannot destructure with it`` | partly. The second says what is wrong but not that the type is private | 10 functions and 2 types made pub |
| ``error[runtime]: no overload of `meta_or` matches these arguments`` at test time: the test's arm `(fronted _ meta)` names a type it cannot see (markdown N1, raytracer F15) | no | 1 type made pub |
| ``error[exhaustive]: this can be an err and `list/map` has no arm for it`` in lex_test.kso helpers | yes | 2 helpers rewritten with `.>` |

## Journal

| F | title | kind | severity | scope | status | evidence |
|---|---|---|---|---|---|---|
| F1 | a long `if` cannot be wrapped | aesthetics | minor | language | ruled-no | #1882 gavel: "Operator chains, `if` and guards are not covered, and they are still broken by naming their parts" |
| F2 | parentheses that clarify `and` inside `or` are refused | aesthetics | nit | language | fixed | now required (#1887); the migration wrote the wanted `alpha = (64 < code and code < 91) or (96 < code and code < 123)` |
| F3 | ambient builtins take ordinary words from local names | confusing-semantics | minor | language | still | raytracer `resit/t/f27.kso`: `fn total values` → ``error[name]: `values` is already a declaration``, still without saying it is ambient |
| F4 | every brace in a string literal is an interpolation | aesthetics | major | language | still | a `"""` block interpolates too: `resit/t/f04.kso` holding `taxonomies = [{name = "tags"}]` → `error[syntax]: unexpected trailing tokens`. No raw string |
| F5 | a file with definitions and statements is not runnable by `run` | tooling | nit | tooling | still | sat `resit/t/f04.kso` under `kanso run` → "is a library — nothing to run" |
| F6 | `text/split` with a variable separator makes every wrapper "can be an err" | confusing-semantics | major | core-lib | still | `bugs/split_err_contagion.kso` → refused at the outer `replace` as before, and now also at `text/join parts to`, since the name `parts` carries the err (#1884) |
| F7 | no literal may pass 80 columns, and a wrapped list does not parse | missing-feature | major | language | fixed | `bugs/multiline_list_literal.kso` runs (#1882); text blocks hold long strings |
| F8 | dispatch takes `""` and `0` but not `[]` | missing-feature | minor | language | still | markdown `resit/t/f26.kso`: `fn code_text []` → `expected a parameter pattern` |
| F9 | an unused parameter in one arm must be `_` | refactoring-hazard | minor | language | still | `resit/t/f09.kso`: ``error[unused]: unused binding `line` `` |
| F10 | one namespace per module makes every helper name reserved in every file | refactoring-hazard | major | language | partly | test files no longer join the namespace (#1883). Module files still do: the migration's own `trimmed` collided with a local in another file, and `done` and `entry` are still reserved (`resit/t/f10.kso` → ``error[name]: `done` is already a declaration``) |
| F11 | a positional pattern reusing field names in another order binds the wrong fields | refactoring-hazard | major | language | still | `resit/t/f11.kso`: `fn linked (dest url title next)` over fields `next title url` prints `7 t /u`, `url` holding `next`'s value, with no complaint |
| F12 | runtime errors point at the module, not the file | diagnostic | major | language | fixed | runtime failures now print `failed in host at f21.kso:3` and the like (#1975, #1987) |
| F13 | a failing test says only "returned false"; std/expect is not shipped | tooling | minor | tooling | still | `import "std/expect"` → `` `std/expect` is not in the shipped library`` (`resit/t/f13.kso`); markdown `resit/t/f17` shows the bare "returned false" |
| F14 | below a guard, a keyed read binds nothing | engine-bug | major | language | still | `bugs/keyed_read_after_guard.kso` → ``error[name]: unknown name `acc` `` |
| F15 | the railway runs through names but not calls, and naming the call makes it eager | confusing-semantics | major | language | worse | names now carry the err (#1884), so the spelling loophole is closed, and every step the journal had routed through a name was refused: 80 refusals in tera alone and more in toml and zola, repaired with 68 pass-through arms and about 20 `.>` steps. The wanted `negated (not_expr ts (p + 1))` is still refused, and a binding above a guard still runs (sat F19) |
| F16 | the specificity ladder across several parameters is hard to predict | confusing-semantics | minor | language | ruled-no | #1888 gavel "overload arms stay in specificity order". The migration met it about fifteen more times (N1) |
| F17 | "a keyed read omits at least one field" is enforced at run time | engine-bug | minor | language | still | `bugs/keyed_read_all_fields.kso` under `kanso play` → `error[runtime]: a keyed read omits at least one field…` / `failed in shown at …:17` (now with a line) |
| F18 | "comparing to `true`" is refused when the value may not be a boolean | diagnostic | minor | language | still | `resit/t/f18.kso`: `meta["draft"] == true` → ``comparing to `true` asks a question the value already answers`` |
| F19 | a function-valued parameter is checked against the importer's function of the same name | engine-bug | major | language | still | `kanso check bugs/param_name_leak/kit` → ok; `kanso check bugs/param_name_leak/main.kso` → ``this can be a none and `keep?` has no arm for it`` at `kit/kit.kso:13` |
| F20 | `text/slice` past the end answers empty instead of clamping | confusing-semantics | major | core-lib | ruled-no | #1890 ruled that a range past the end answers `none`, not a clamp. The port keeps `textkit/clamped`, now for all 120 slices |
| F21 | `f == none` on a function value fails at run time with no location | engine-bug | major | language | still | `resit/t/f21.kso` checks clean and stops with `equality is not defined on a function or an effect…` / `failed in host at f21.kso:3`. The line is new; the check still lets it through |
| F22 | a missed `list/to_list` fails far away with the whole value printed | confusing-semantics | minor | core-lib | partly | refused at check when the lazy call is the argument (`resit/t/f22b.kso` → ``error[type]: `length` takes a list… make it a list with `list/to_list` ``; 2026-10-13 log). Through a name it still fails at run time, now with a line, and still prints the whole value (`f22.kso`) |
| F23 | `kanso check DIR` and `kanso check main.kso` disagree about a test file | tooling | minor | tooling | fixed | `run`, `check` and `build` no longer read test files (2026-10-12 gavel "a test file is an outside consumer of its module", #1883) |
| F24 | the qualified pattern `(os/file_not_found _)` is refused | confusing-semantics | nit | language | still | `resit/t/f24.kso` → ``error[opacity]: `os/file_not_found` is foreign — its structure does not cross an import`` |
| F25 | a long single-expression constant needs an invented binding | aesthetics | minor | language | still | raytracer `resit/t/f19m` → ``a single-expression constant is written inline`` |
| F26 | std/os cannot remove a file or directory | stdlib-gap | minor | domain-lib | still | std/os exports `env exit exists is_dir list_dir make_dir read_file read_file! read_bytes read_bytes! run start kill write_file modified`; no remove |
| F27 | native code made an accumulator quadratic and a folded chain of effects crash | performance | blocker | core-lib | partly | `bugs/native_accumulator`, dev build, `if` and `arm` forms at 4,000 / 8,000 / 32,000: 0.007 to 0.022 s (journal: 6.8 s, then no finish). `bugs/effect_fold_stack.kso` at n = 100: interp writes 100 files and prints `wrote 100`; native writes 42 and stops with `the program ran out of stack`. The engines still disagree |

Counts: fixed 4, partly 3, still 16, ruled-no 3, moot 0, worse 1.

## New friction

### N1: where a pass-through err arm goes in an overload group is found by trial
- kind: confusing-semantics
- severity: major
- scope: language
- where: tera/render.kso (`display`, `truthy?`, `index_into`, `binary`, `picked`, `and_then`, `or_else`), tera/parse.kso (`if_more`, `filter_block`, `macro_block`), toml/toml.kso (`doc_after`), tera/filters.kso (`check_test`)
- wanted: one rule for where `fn f e@(err _) …` sits among an overload's arms.
- wrote: each group by trial. In `display` the err arm goes after `none` and before `(missing path)`. In `index_into` it must come after `xs:[]some n:int`. In `truthy?` it goes after `true` and `false`. Two err arms in one group tie unless a third arm takes both errs (`fn index_into e@(err _) (err _) _`). Beside literal arms (`fn binary "==" l r _`, `fn check_test "odd" n:int _ _`) an err arm in another position ties with every literal, so those groups took a `.>` at the caller instead.
- why it matters: #1884 made these arms the standard repair for a railway,
  and the book's ch08 shows them, but the ordering message names only the
  arm that is out of place, not where it should go. In this port it cost
  about fifteen rounds of check.

### N2: a test helper in one test file is invisible to another
- kind: tooling
- severity: minor
- scope: tooling
- where: tera/lex_test.kso and tera/tera_test.kso (`surely`)
- wanted: one `surely` helper for the module's two test files, as the journal had it.
- wrote: the helper copied into tera_test.kso.
- why it matters: under #1883 each test file is its own importer, so test
  helpers are per file. Two copies of a helper is the duplication the
  project's hygiene rules ask to avoid.

### N3: a private type in a test's pattern silently never matches
- kind: confusing-semantics
- severity: major
- scope: language
- where: zola/front_test.kso (`meta_or (fronted _ meta)`)
- wanted: the check to refuse a pattern naming a type the file cannot see.
- wrote: `pub type fronted`.
- why it matters: the tests failed with ``no overload of `meta_or` matches
  these arguments``, a message about the call, not about the arm that can
  never match. Same family as markdown N1 and raytracer F15.

## Bugs

- `bugs/test_helper_private_name/` (new): a private name used inside a
  helper function of a test file passes the check, and the test fails at
  run time with ``error[runtime]: unknown name `~hidden` ``. Used directly in
  a test constant, the same name is refused at check.
- The journal's bugs, re-run: `effect_fold_stack.kso` still out of stack
  natively at n = 100, interp fine (F27); `native_accumulator` now linear
  (F27); `keyed_read_after_guard.kso` still fails (F14);
  `keyed_read_all_fields.kso` still a run-time error (F17);
  `multiline_list_literal.kso` fixed (F7); `param_name_leak` still refused
  from the importer (F19); `split_err_contagion.kso` still refused, now at
  two places (F6).

## Comfort

On language and core-lib, a programmer starting this port today would find
the formatting squeeze looser: lists span lines, `and`/`or` take the
parentheses they wanted, and runtime failures name their line, which would
have shortened the F11 and F21 hunts. The native accumulator cliff is gone.
The err rule is the larger change. It now reads names, which removes the
journal's complaint that it was about spelling, but a template engine whose
parser and renderer hand failures along a railway now needs an err arm or a
`.>` at nearly every step: the migration added 68 such arms, and placing
them among literal arms was guesswork. `text/slice` answering `none` sent
every slice in the port through a clamping helper. Positional patterns
still bind by alphabetical field order without complaint, keyed reads below
a guard still fail, a binding above a guard still runs, and a folded chain
of effects still overflows the native stack where the interpreter is fine.
On tooling, tests needed a dozen private names made pub, and a private name
inside a test helper failed only at run time. The journal said the checker
kept the author thinking about how it reads a line; today that is still
true, with more lines to think about.
