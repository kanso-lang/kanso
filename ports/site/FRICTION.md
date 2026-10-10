# FRICTION: porting Zola to kanso

The journal for the static site generator port. Entries were written as the
problems came up, in the order they came up. Every code sample was compiled
with the toolchain at `/tmp/claude-0/kanso-main/kanso`.

## Entries

### F1: a long `if` cannot be wrapped, so it becomes a dispatch helper
- kind: aesthetics
- severity: minor
- where: toml/toml.kso (`value_at cs "\"" p`, `quoted`)
- wanted:
  ```
  fn value_at cs "\"" p
    if (text/slice cs p (p + 2) == ["\"" "\"" "\""])
      (long_string cs (skip_first_newline cs (p + 3)) [])
      (basic_string cs (p + 1) [])
  ```
- wrote:
  ```
  fn value_at cs "\"" p
    quoted cs p (text/slice cs p (p + 2) == ["\"" "\"" "\""])

  fn quoted cs p true
    long_string cs (skip_first_newline cs (p + 3)) []

  fn quoted cs p false
    basic_string cs (p + 1) []
  ```
- why it matters: the only continuation tokens are `.`, `.>`, `.!` and `.?`,
  and the block form of `if` is refused when no branch binds a name. So an
  expression-form `if` that runs past 80 columns has no legal rendering at
  all. Binding the two branches to names first is not an option either when
  one branch recurses, since both would be evaluated. Every time it happened
  the fix was a two-arm helper on `true`/`false` (`quoted`, `merge_step`,
  `meld_pick` and others), each a new name for one line of logic.

### F2: parentheses that clarify `and` inside `or` are refused
- kind: aesthetics
- severity: nit
- where: toml/toml.kso (`bare?`)
- wanted: `alpha = (64 < code and code < 91) or (96 < code and code < 123)`
- wrote: `alpha = 64 < code and code < 91 or 96 < code and code < 123`
- why it matters: `error[formatting]: these parentheses group nothing`. True,
  but mixing `and` and `or` without parentheses is the case most linters ask
  you to parenthesise (gcc's -Wparentheses, clippy's precedence lints), so
  the one legal spelling is the one those tools would flag.

### F3: ambient builtins take ordinary words away from local names
- kind: confusing-semantics
- severity: minor
- where: toml/toml.kso (first draft used `keys` for a dotted key path)
- wanted: `fn set_in m keys v` and `keys = key_path cs p []`
- wrote: `fn set_in m names v`
- why it matters: `error[name]: \`keys\` is already a declaration; rename the
  binding`, eight times in one file. `keys`, `values`, `entries`, `length`,
  `put` and `push` are ambient, so they are reserved in every scope of every
  file even when the file never calls them. `keys` is the natural name for a
  list of TOML keys. The message says "already a declaration" without saying
  it is an ambient builtin you did not write.

### F4: every brace in a string literal is an interpolation, which taxes a template engine
- kind: aesthetics
- severity: major
- where: toml/toml_test.kso, and every test of the template engine later
- wanted: `parsed "taxonomies = [{name = \"tags\"}]"`
- wrote: `parsed "taxonomies = [\{name = \"tags\"}]"`
- why it matters: there is one string syntax and `{` always opens an
  interpolation, so there is no raw string to hold text that is mostly
  braces. A template language is `{{ ... }}` and `{% ... %}` throughout, and
  TOML inline tables and JSON are braces too. Combined with escaped double
  quotes, a test line reads `"\{\{ page.title | upper }}"`.

### F5: a file with both definitions and statements is not runnable by `run`
- kind: tooling
- severity: nit
- where: the first probe program
- wanted: `kanso run probe.kso` on a file with an `fn` and a `print`
- wrote: `kanso play probe.kso`, or a module directory with `main.kso`
- why it matters: the brief's verb list (run, check, test, build) does not
  include `play`, and `run` answers "is a library — nothing to run". The
  message names the fix, so this cost one try. The reverse also bit: the
  book's samples write `pub play = ...`, and `kanso play` refuses that with
  "`pub play` is a library's export — ... `kanso play` takes bare
  statements", so a sample copied from the book does not run under the verb
  the error message for the first case recommends.

### F6: `text/split` with a variable separator makes every wrapper "can be an err"
- kind: confusing-semantics
- severity: major
- where: textkit/textkit.kso (`replace`)
- wanted:
  ```
  fn replace s from to
    text/join (text/split s from) to

  print (replace (replace "a-b" "-" "+") "+" "*")
  ```
- wrote: a hand-written scan over `text/chars` (`replace` and `swap` in
  textkit.kso), so that no `split` with a non-literal separator remains.
- why it matters: `text/split` has an `_ ""` arm that answers an err, and the
  checker cannot tell that `from` is never empty, so `replace` "can be an
  err". From then on every call whose argument is a call to `replace` is
  refused (`error[exhaustive]: this can be an err and \`replace\` has no arm
  for it`), and so is every function that wraps it, outward through the
  program. Binding the split to a name first did not help. Guarding with an
  arm `fn replace s "" _` did not help either, because the general arm still
  calls split with a string the checker cannot see into (the guarded
  variant is bugs/split_err_contagion.kso). Go and JavaScript split on an empty
  separator into characters. Python raises, but an exception there does not
  oblige every caller of every wrapper to write a handler.

### F7: no literal may pass 80 columns, and a wrapped list literal does not parse
- kind: missing-feature
- severity: major
- where: markdown/markdown_test.kso; bugs/multiline_list_literal.kso
- wanted:
  ```
  test_table =
    want = text/join [
      "<table>\n<thead>\n<tr>\n<th style=\"text-align: left\">a</th>"
      "<th style=\"text-align: right\">b</th>\n</tr>\n</thead>"
    ] "\n"
    html "| a | b |\n|:--|--:|" == want
  ```
- wrote: short unit tests only, and every expected-html case longer than a
  line moved out of `kanso test` into fixture files (`fixtures/markdown/`)
  checked by `check.sh`.
- why it matters: the 80-column cap applies to string literals, there is no
  string continuation and no multi-line string, and the wrapped list form is
  refused both ways. With short elements it is "needless continuation: this
  statement fits on one line", which reads as if wrapping were legal for a
  statement that does not fit. With one long element it is `error[syntax]:
  expected an expression`. The standard library works around the same thing
  in lib/sha256, where one table of constants is split into `k0` through
  `k10` and stitched back with `text/concat`. For a program whose tests are
  "this markdown gives this html", the consequence is that the expected
  output cannot be written where the test is.

### F8: dispatch takes `""` and `0` as patterns but not `[]`
- kind: missing-feature
- severity: minor
- where: markdown/blocks.kso (`closed`, `interrupts?`)
- wanted:
  ```
  fn closed []
    ""

  fn closed lines
    "{text/join lines "\n"}\n"
  ```
- wrote: `if (length lines == 0) "" "{text/join lines "\n"}\n"`
- why it matters: `error[syntax]: expected a parameter pattern`. The book
  teaches literal arms as the base case of a recursion (`fn fact 0`), and the
  empty list is the base case of most recursions over lists. Strings get it,
  numbers get it, lists do not, and neither does the empty map `{}`
  (zola/pages.kso, where an absent neighbour page wanted `fn with_shallow m _
  {}` and became a `none` arm).

### F9: an unused parameter in one arm must be renamed `_`, so adding a parameter touches every arm twice
- kind: refactoring-hazard
- severity: minor
- where: markdown/blocks.kso (`read_block`, ten arms)
- wanted: add a `line` parameter to `read_block` and use it in the three arms
  that need it.
- wrote: the parameter added to all ten arms, then seven of them changed
  again from `line` to `_` after `error[unused]: unused binding \`line\``.
- why it matters: the unused rule applies to each arm's parameters
  separately, so a group whose arms share a signature cannot share parameter
  names. Adding or reordering a parameter is an edit per arm, and the edit
  differs by whether that arm reads it. Rust and Haskell warn on an unused
  pattern variable; neither refuses to compile.

### F10: one namespace per module turns every helper name into a reserved word in every other file
- kind: refactoring-hazard
- severity: major
- where: markdown/blocks.kso, markdown/inline.kso, markdown/render.kso
- wanted: a local `closer = cs[p + digits]` in blocks.kso, a local
  `target = destination cs (close + 1)` in inline.kso, a parameter `cells` in
  render.kso, a local `done = list/fold ...` and `entry = { ... }`.
- wrote: `closer` became `emphasis_close`, `target` the type became `dest`,
  `cells` became `row`, `done` became `out`, `entry` became `head`, `cell`
  (a parameter in blocks.kso) became `spec`.
- why it matters: adding `fn closer` to inline.kso broke a local variable in
  blocks.kso, a file I had not opened. The no-shadowing rule plus one
  namespace across the directory means a new top-level name anywhere in a
  module can fail compilation anywhere else in it. Test files share the
  namespace too: a helper `fn html source` in markdown_test.kso broke six
  locals named `html` in render.kso. Builtin type and marker
  names join in: `done` (the value a write yields) and `entry` (the record
  `entries` answers) cannot be local names in any program. The diagnostic is
  clear about which name; it does not say where the clashing declaration
  lives, which matters when it is in another file or is a builtin.

### F11: a positional pattern that reuses the field names in another order binds the wrong fields, silently
- kind: refactoring-hazard
- severity: major
- where: markdown/inline.kso (`linked`, `pictured`)
- wanted: the compiler to notice that
  ```
  type dest
    next
    title
    url

  fn linked cs p close (dest url title next) links st
  ```
  names `url` the field called `next`.
- wrote: `(dest next title url)`, after a debugging session.
- why it matters: positional patterns bind by declaration order, and the
  declaration order is forced to be alphabetical, which is rarely the order
  a reader thinks of the fields in (`url title next` for a link target). I
  wrote the field names in reading order, every name matched a real field,
  and `url` got the integer. The program compiled and failed at run time
  with `error[runtime]: slice takes a list or string` and no usable location
  (F12). A pattern variable spelled exactly like a different field of the
  same record is almost always this mistake and could be refused.
  The same mistake happened twice more on the construction side, where no
  names are written at all: `looped key name source head.line` for fields
  `key line name source` (tera/parse.kso), and `sort_pair (attr_path x key)
  x` for fields `item key` (tera/filters.kso). The first failed at run time
  as "no overload of `eval` matches these arguments", the second sorted by
  the wrong thing and printed `tera/missing "n"` twice. Three of the bugs in
  this port were argument order against an alphabetical field list.

### F12: runtime errors point at the module, not the file, or at nothing
- kind: diagnostic
- severity: major
- where: the F11 bug, reported three ways
- wanted: `error[runtime]: slice takes a list or string` with the file and
  line of the `text/slice` call that received the integer.
- wrote: bisected by hand, rendering one input at a time from a scratch
  entry program.
- why it matters: under `kanso test` the report said `--> markdown:29:3`,
  which is a module name and a line; four files in the module have a line
  29, and in the right file (inline.kso) line 29 is the declaration of the
  function two calls above the failing slice. Under `kanso run --interp` the
  same error printed no location at all. A second runtime error said
  `chars takes a string` at `markdown:121:38`, which was again the caller's
  line in an unnamed file.

### F13: a failing test says only "returned false"
- kind: tooling
- severity: minor
- where: markdown/markdown_test.kso
- wanted: `test_link ... FAILED: left was "<p><a href=...", right was ...`, or
  a way to print from a test.
- wrote: a scratch `main.kso` that imports the module and prints
  `markdown/render` on one input at a time. Only `pub` names cross that
  import, so private helpers like `destination` could not be called from it.
- why it matters: a test is a boolean constant, so when `md x == want` is
  false the runner cannot show `md x`. For a renderer whose tests compare
  strings, every failure started with writing a throwaway program to see the
  actual output. After this entry I found `std/expect` in lib/, whose
  `expect x . to (equal y)` answers a `mismatch` record the runner prints,
  which is exactly the fix. Importing it answers `error: \`std/expect\` is
  not in the shipped library`. Chapter 07 never mentions it and appendix B
  does not list it, so the one tool for this is documented only in a
  source file the compiler will not load.

### F14: below a `return ... if` guard, a keyed read binds nothing and `_` patterns slip through
- kind: engine-bug
- severity: major
- where: tera/lex.kso (`verbatim`); bugs/keyed_read_after_guard.kso
- wanted:
  ```
  fn verbatim name cs st
    ...
    return fail name st.line "`raw` without `endraw`" if end == none
    ...
    { acc line } = closed
    cut name cs (scan (push acc (t_tag "endraw" line)) line ...)
  ```
- wrote: `added = add_text st words` and then `added.acc`, `added.line`.
- why it matters: the keyed read is the form the compiler recommends for
  taking part of a record, and below a guard it fails with `error[name]:
  unknown name \`acc\``, which sent me looking for a typo. Deleting the guard
  makes the same lines compile. In the same position the positional form with
  `_` holes, which the compiler refuses everywhere else, is accepted and
  runs. Guards are the book's recommended way to write early exits (they are
  in examples/guards.kso and kq), so the two features meet often.

### F15: the railway runs through names but not through calls, and naming the call makes it eager
- kind: confusing-semantics
- severity: major
- where: tera/expr.kso (`not_expr`, `unary`, `primary_at`, `name_at`)
- wanted: a recursive-descent parser whose steps hand a failure along,
  written the way the railway chapter describes:
  ```
  fn not_expr ts p
    return negated (not_expr ts (p + 1)) if keyword? ts p "not"
    compare ts p

  fn negated (parsed p v)
    parsed p (e_not v)
  ```
- wrote: two arms on a boolean per step, with the call bound to a name inside
  the arm that needs it:
  ```
  fn not_expr ts p
    not_or_compare ts p (keyword? ts p "not")

  fn not_or_compare ts p true
    inner = not_expr ts (p + 1)
    negated inner

  fn not_or_compare ts p false
    compare ts p
  ```
- why it matters: `negated (not_expr ...)` is refused with "this can be an
  err and `negated` has no arm for it", while `left = and_expr ts p` followed
  by `or_more ts left` is accepted and does exactly the same thing at run
  time: the err skips `or_more` and rides out. So the rule is about spelling,
  not about what can happen. The obvious way to satisfy it, binding the call
  first, changes the program, because bindings are evaluated eagerly: the
  bound `not_expr ts (p + 1)` above the guard runs whether or not the guard
  fires (checked with a probe whose bound call never returns: it hangs). In
  a parser that is a call at every token position on every level, which is
  exponential. Writing `fn negated (err e)` arms that hand the err back is
  the other way out and is the kind of boilerplate the railway was meant to
  remove. Every step of the parser came out as a pair of arms.

### F16: the specificity ladder across several parameters is hard to predict
- kind: confusing-semantics
- severity: minor
- where: tera/render.kso (`binary`, `integer`, `index_into`, `display`,
  `truthy?`, `as_text`), tera/parse.kso (`loop_end`, `tag_body`)
- wanted: to write an overload group in the order that reads as a decision
  (the `missing` guard arms first, then the operators), as the book says
  "the arms may be written wherever they read best".
- wrote: six rounds of `error[formatting]: overloads of \`binary\` appear
  most-specific first: literal, then concrete type, then generic`, each
  answered by moving one arm. `op l:int r:int` must precede
  `_ (missing path) _`; `"/" _ 0` must precede `"+" l r`; `none` must precede
  a record pattern; and for `loop_end ... head "else"` against
  `... (looped ...) _` neither order was accepted, so one arm had to drop its
  pattern and destructure in the body.
- why it matters: the rule is stated for one parameter (literal, concrete,
  generic). With two or three dispatching parameters the order is some count
  of specific positions, which I never managed to predict before the
  compiler told me. The diagnostic names the arm but not the arm it must go
  above.

### F17: "a keyed read omits at least one field" is enforced at run time
- kind: engine-bug
- severity: minor
- where: zola/front.kso (`nested`); bugs/keyed_read_all_fields.kso
- wanted: `kanso check` to refuse `{ map next } = inner` for a two-field
  record, like the other canonical-form rules.
- wrote: `ylevel map next = inner`, after the test run failed.
- why it matters: the rule is about spelling, so it belongs with the other
  formatting errors at check time. `kanso check zola` said ok; `kanso test`
  then failed with `error[runtime]: a keyed read omits at least one field;
  reading every field is the positional form` at `zola:88:18`. Under `kanso
  play` the same error has no location at all. A canonical-form rule that
  only fires on the path that executes it can sit in an untested branch.

### F18: "comparing to `true`" is refused even when the value may not be a boolean
- kind: diagnostic
- severity: minor
- where: zola/content.kso (`draft?`, `owner_of`)
- wanted: `p.meta["draft"] == true`, where the front matter value may be
  missing (none), `true`, `false`, or a string someone typed.
- wrote: a three-arm helper `flag?` on `true`, `none` and `_`.
- why it matters: `error[name]: comparing to \`true\` asks a question the
  value already answers — write the value itself`. The advice is right for
  a boolean and wrong for a map lookup over parsed user data: writing the
  value itself hands `if` a none or a string, which is a runtime error. The
  rule fires on the spelling without looking at what the left side can be.

### F19: a function-valued parameter is checked against the importer's function of the same name
- kind: engine-bug
- severity: major
- where: textkit/textkit.kso (`sort_with`), checked from tera;
  bugs/param_name_leak/
- wanted: `meld_pick ... (before? right[j] left[i])` inside textkit, where
  `before?` is the comparator parameter.
- wrote: the parameter renamed `goes_first?` and the two elements bound to
  names before the call.
- why it matters: textkit checked clean on its own. The moment tera (which
  has its own private `fn before? a:int b:int`) imported it, `kanso check
  tera` reported `this can be a none and \`before?\` has no arm for it` at a
  line in textkit.kso. The checker resolved textkit's parameter against
  tera's top-level function. The engines run the right function; only the
  check is wrong. It means a library that checks clean can stop checking
  when someone imports it, for a reason that depends on the importer's
  private names, and the error points into the library.

### F20: `text/slice` past the end answers an empty result instead of clamping
- kind: confusing-semantics
- severity: major
- where: zola/outputs.kso (`pager_file`), found as a blank second page of
  the blog
- wanted: `text/slice pages 3 4` on a three-element list to answer `[x3]`,
  as `pages[2:4]` does in Python, `pages[2..4.min(len)]` reads in Rust, and
  `slice(2, 4)` does in JavaScript.
- wrote: `textkit/clamped`, which clamps `to` before slicing, and used it
  for pagination, the feed limit and the template `slice` filter.
- why it matters: appendix B says out-of-range bounds "yield an empty result
  rather than a failure, so slicing never surprises you with a none". The
  empty result was the surprise: the last pager of a paginated section came
  out empty with no error anywhere, because its range ran one past the end.
  Clamping is what the last chunk of every pagination, batching and
  truncation loop needs; empty is right only when `from` is past the end.

### F21: `f == none` on a function value fails at run time, with no location
- kind: engine-bug
- severity: major
- where: tera/filters.kso (`host_filter`), tera/render.kso (`host_function`)
- wanted:
  ```
  f = e.engine.filters[name]
  return err "line {line}: unknown filter `{name}`" if f == none
  answered (f v args) line
  ```
- wrote: a two-arm dispatch, `fn host_filter none ...` and
  `fn host_filter f ...`.
- why it matters: the host's filters and functions are lambdas kept in a
  map, so a lookup answers a function or none. Comparing that with `none`
  checked clean and then the whole build died with `error[runtime]: equality
  is not defined on a function or an effect — write an arm for the case you
  mean`, with no file and no line. The advice in the message is the fix, but
  finding which `==` in four thousand lines meant reading every lookup into
  a map that could hold a function. The checker knows the map's values are
  functions (it type-checks the call `f v args`), so it could refuse the
  comparison.

### F22: adapters are lazy, and forgetting `list/to_list` fails far away with the whole value printed
- kind: confusing-semantics
- severity: minor
- where: zola/outputs.kso (`sitemap`), and 40-odd `list/to_list (list/map
  ...)` across the port
- wanted: `urls = textkit/sort_with (list/map listed (f -> f.listing)) ...`
- wrote: `found = list/to_list (list/map listed (f -> f.listing))` first.
- why it matters: `list/map`, `list/select` and `list/reject` answer lazy
  sequences, and `length`, indexing and `text/slice` refuse them. The
  failure came from `length` inside textkit's sort, as `error[runtime]:
  length takes a list, string, or map, not list/mapped <fn> list/cursor 1
  [zola/out_file "<!doctype html> ...` followed by every rendered page of
  the site, about 20 KB of output, before the useful last line: "a lazy
  sequence becomes a list with list/to_list". The laziness is a good
  default for chains; the cost is that nearly every `list/map` in a program
  that indexes its results is wrapped in `list/to_list`, and a missed one is
  a runtime error rather than a check error.

### F23: `kanso check DIR` and `kanso check main.kso` disagree about a test file
- kind: tooling
- severity: minor
- where: zola/front_test.kso
- wanted: one answer from the checker.
- wrote: an `(err _)` arm in the test helper.
- why it matters: `kanso check zola` and `kanso test zola` both passed with
  `test_toml_body = body_of_front "..." == "body"`. `kanso check main.kso`,
  which imports zola, then refused that same test line with "this can be an
  err and `==` wants a value". A module that checks clean on its own can
  fail to check from its importer, so "the module is ok" is not a property
  you can test for in the module.

### F24: the qualified pattern `(os/file_not_found _)` is refused; the bare one works
- kind: confusing-semantics
- severity: nit
- where: zola/disk.kso (`with_config_text`)
- wanted: `fn with_config_text root (os/file_not_found _)`, qualified like
  every other name from std/os in the file.
- wrote: `fn with_config_text root (file_not_found _)`.
- why it matters: the qualified form answers `error[opacity]:
  \`os/file_not_found\` is foreign — its structure does not cross an
  import`. The unqualified form, which is what chapter 05 and kq write,
  matches the same record. The book prefers qualified spellings "because it
  reads as documentation", and here the qualified spelling is the one that
  does not compile.

### F25: a single-expression constant must be inline, so a long one needs an invented binding
- kind: aesthetics
- severity: minor
- where: zola/content_test.kso, tera/tera_test.kso, markdown/markdown_test.kso
- wanted:
  ```
  test_section_path =
    section_path "docs/Getting Started" == "/docs/getting-started/"
  ```
- wrote:
  ```
  test_section_path =
    want = "/docs/getting-started/"
    section_path "docs/Getting Started" == want
  ```
- why it matters: `error[formatting]: a single-expression constant is
  written inline`, and inline it is 83 columns. The rules together leave
  one legal shape, which is to name some part of the expression whether or
  not the name helps. Most of the `want = ...` lines in the test files exist
  for this reason. It is the same squeeze as F1 and F7: the 80-column rule
  has no continuation form for most expressions, so names do the wrapping.

### F26: std/os cannot remove a file or a directory
- kind: stdlib-gap
- severity: minor
- where: zola/cli.kso (`reported`), the output directory
- wanted: `os/remove_dir_all out .> (_ -> write_plan root out p)`, which is
  what Zola does to `public/` before every build.
- wrote: nothing; the README says the output directory should be fresh, and
  check.sh builds into new temporary directories.
- why it matters: `error[name]: unknown name \`os/remove_dir_all\``, and std/os
  has no remove of any kind. A generator that cannot clear its output leaves
  pages for deleted content in place, which is the bug static site
  generators clean `public/` to avoid. `os/run "rm" [...]` would work, at
  the cost of the program depending on a shell tool.

### F27: native code turned an ordinary accumulator into a quadratic copy, and a folded chain of effects into a crash
- kind: performance
- severity: blocker
- where: textkit/textkit.kso (`outside_tags`), zola/disk.kso (`write_plan`);
  bugs/native_accumulator/, bugs/effect_fold_stack.kso
- wanted: the two most ordinary loops in the program.
  ```
  fn outside_tags cs p inside acc
    ...
    kept = if (inside or next) acc (push acc c)
    outside_tags cs (p + 1) next kept

  written = list/fold files made (e f -> e .> (_ -> write_one out f))
  ```
- wrote:
  ```
  outside_tags cs (p + 1) next (kept_if acc c (not (inside or next)))

  pub fn kept_if acc c true
    push acc c

  pub fn kept_if acc _ false
    acc
  ```
  and, for the writes, a function that writes file `i` and binds a call to
  itself for `i + 1`, so the chain is built while the executor walks it.
- why it matters: on a generated 200-page site the interpreter built
  everything in 15 s and the dev binary took between 92 s and 117 s, slower
  than the oracle it is meant to outrun. Callgrind put 90% of the native
  instructions in the runtime's `k_survives_x`, `k_interior_survives`,
  `k_deep_copy` and `k_repair_*`, under `strip_tags`. Rewriting the push as
  an arm brought an 80-page build from 27.6 s to 0.6 s; I checked that by
  building the same tree twice with only that function changed. I do not
  know why it helped. A standalone reproduction of the same loop
  (bugs/native_accumulator/) is slow natively with either form: 4,000
  characters take about 7 s and 8,000 do not finish in two minutes, where
  the interpreter does 32,000 in 0.2 s. So the slow path depends on
  something about the calling context that I could not isolate, and a
  program that is fast today has no assurance that a nearby edit will keep
  it fast.

  After that fix, every native build that wrote more than about fifty files
  ended in `Segmentation fault` (ten generated posts were enough; the
  31-file sample site was not). The fold above builds a chain of nested
  binds before running any of them. The interpreter runs the small
  reproduction at n = 5000; native code reports "the program ran out of
  stack" at n = 100 there, and segfaulted in the full program. Building the
  chain step by step made the crash go away at every size tried. The
  differential law says the engines agree; here the program's meaning was
  the same and one engine could not run it. Final numbers for the 200-page
  site: interpreter 15.5 s, dev binary 1.3 s, output identical.

## What worked well

**Literal dispatch made the tables of a template engine read as tables.**
Every filter is one arm, and adding one is adding an arm:

```
fn builtin _ "upper" v _ _
  textkit/upper (as_text v)

fn builtin _ "lower" v _ _
  textkit/lower (as_text v)
```

The same shape holds the tag parser (`fn tag_at chunks p stops acc head
"for"`), the strftime directives (`fn directive "B" t`), the markdown block
kinds and the TOML escapes. In Rust these are `match` arms inside one long
function; here each is its own function with its own name in a stack trace.

**An index past the end answers `none`, so scanners have no bounds checks.**
The template lexer, the TOML parser and the markdown inline parser all walk a
list of characters with `cs[p]`, and the end of input is just one more arm:

```
fn span_at _ none _ _ st
  st
```

I wrote none of the `p < len` tests that a hand-written scanner in C, Go or
Rust needs on every read.

**The build is a pure function, and the I/O fits in one small file.**
`build` takes the files that were read (`site_input`) and answers a `plan`:
the files to write, the files to copy and the lines to print. Everything that
touches the disk is zola/disk.kso, about a hundred lines. `site check` is the
same build without the writes, which cost five lines. The content model is
tested by handing `read_content` a map of file names to text, with no
temporary directories:

```
test_drafts_dropped =
  files = { "d.md":"+++\ndraft = true\n+++\n" "e.md":"" }
  content_of files == ["e.md=/e/" "_index.md=/"]
```

**Failures carried their context out with very little plumbing.** A TOML
error raised four calls deep in the front-matter parser reaches the user as
`Error: content/post.md: line 2: unterminated string`. The only code that
knows about it is one arm per layer that adds the layer's name:

```
fn added _ who _ (err r) _
  err "{who}: {r}"
```

Folds pass an err along without being told to, so a failure in the 150th page
of a fold over pages stops the build with that page's message.

**A wrapper type decided autoescaping.** `type markup string` is text the
engine will not escape again, and output is one dispatch:

```
fn display m:markup _
  m

fn display s:string _
  textkit/escape_html s
```

`| safe` builds a `markup`, every string function still accepts one because
a wrapper flows wherever its parent flows, and there is no flag to forget.

**Structural equality made tests short.** Parsed front matter is compared
with a map literal (`meta_of src == { "draft":false "tags":["a" "b"] ... }`),
and the content model with a list of strings. No custom matchers were needed.

**Functions are values, closures included.** The host's template functions
are lambdas in a map that capture the site (`"get_section":(a -> get_section
vw a)`), which is how Zola registers them with Tera too.

**The loop was fast and the engines agreed.** The sample site builds in
0.2 s on the interpreter and a dev binary compiles in about 1.2 s. Apart
from F27, every fixture produced byte-identical trees on the interpreter, the
dev build and the release build on the first try, which made the interpreter
a trustworthy oracle for `check.sh --bless`.

**Several diagnostics named the fix outright.** "`macro_def` takes 2
argument(s), and a list element is one atom — write `(macro_def …)`";
"`text` is not imported here — a module's files share their declarations,
not their imports"; "`tera/engine` is foreign — only `tera` builds a
`engine`; ask it for one through a pub function". Each cost one edit.

## Summary

The five I would fix first:

1. **F27**, native code's super-linear copying of an accumulator list and
   its stack overflow on a folded chain of effects. A program the
   interpreter runs correctly took eight times longer natively or crashed,
   the only way to find the cause was a profiler on the runtime's
   internals, and the fix that worked in the program does not work in a
   small reproduction.
2. **F11**, positional construction and patterns against a field order the
   language forces to be alphabetical. Three of the real bugs in this port
   were arguments in reading order instead of alphabetical order, and all
   three compiled.
3. **F15** (with F6), the err rule that accepts a failure passed through a
   name and refuses the same failure passed as a call, combined with eager
   bindings, so that the easy way to satisfy the checker changes what the
   program does. Every step of the expression parser became two arms.
4. **F7** (with F1 and F25), the 80-column limit with no way to continue a
   string, a list literal, an `if` or an operator expression. It moved the
   expected output of the markdown tests out of the tests, split the
   built-in templates into fifty constants, and invented most of the
   `want =` names in the port.
5. **F10** (with F3), one namespace per module plus no shadowing plus ambient
   names, which made a new helper in one file break locals in another, and
   made `keys`, `values`, `entries`, `done` and `entry` unusable as names.

Writing a static site generator in kanso was mostly pleasant once the program
had its shape. A generator is a pure function from a tree of text to a tree of
text with a thin layer of I/O at each end, which is the shape kanso is built
around, and dispatch on literals suits a template engine and a markdown parser
well. The cost was paid in three places. The formatting rules, each
defensible alone, combined to dictate names and helper functions that exist
only to stay under 80 columns. The checker's rules about failure and absence
were easy to satisfy by spelling rather than by meaning, which kept me
thinking about how the checker reads a line instead of what the line does.
And the first time the program met a realistic amount of content, the native
engines turned out to have performance cliffs and a crash that the
interpreter does not have, found only with a profiler and bisection. The
runtime errors' locations (F12) made every one of those hunts longer than it
needed to be. The finished port is about 5,500 lines of kanso, 450 of them
unit tests, and builds every fixture identically on all three engines.
