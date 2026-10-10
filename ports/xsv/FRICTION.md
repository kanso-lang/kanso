# FRICTION: porting xsv to kanso

The journal for port #6. Entries were written as each problem came up, in
the order they came up. Every code sample compiled (or failed to compile,
where that is the point) against the compiler at
`/tmp/claude-0/kanso-main/kanso` on 2026-10-10.

## Entries

### F1: a guard does not narrow an index, so a loop over positions owes a `none` arm it can never take
- kind: missing-feature
- severity: major
- where: csv/write.kso:10 (first draft), xsv/args.kso:21, xsv/cli.kso
- wanted:
  ```
  fn fields_onto acc fields delim i
    return text/append acc 10 if i > length fields
    led = if (i == 1) acc (text/append acc delim)
    fields_onto (field_onto led fields[i] delim) fields delim (i + 1)
  ```
- wrote: either a `none` arm on the callee that the guard proves unreachable
  (`fn arg_at _ _ _ given rest none`), or a rewrite that avoids positions
  altogether (`text/join (list/to_list (list/map fields ...))`).
- why it matters: the checker reports
  `error[exhaustive]: this can be a none and field_onto has no arm for it`
  on a read the line above has bounds-checked. Every index-walking loop in
  the port carries a dead arm, and a reader cannot tell a dead `none` arm
  from a live one. Sometimes the rewrite was better code; often it was not.

### F2: an 80-column limit with no way to continue a string literal
- kind: aesthetics
- severity: minor
- where: csv/read.kso:136, csv/csv_test.kso:50
- wanted: one literal for xsv's own error text,
  `err "CSV error: record {i - 1} (line: {line}, byte: {b}): found record with {n} fields, but the previous record has {width} fields"`
- wrote:
  ```
  at = "record {i - 1} (line: {line}, byte: {starts[i] - 1})"
  found = "found record with {length rows[i]} fields"
  err "CSV error: {at}: {found}, but the previous record has {width} fields"
  ```
- why it matters: a CLI is mostly messages, and every long message becomes
  two or three throwaway bindings whose names say nothing. The same happens
  to expected outputs in tests. Adjacent-literal concatenation (C, Python)
  or a `\` line continuation inside a literal would keep the text whole.

### F3: types must sit above every function, so a helper's record cannot live beside the helper
- kind: aesthetics
- severity: nit
- where: csv/read.kso:4-20
- wanted: `type gathered` (the records plus their start offsets) declared
  just above `records`, the only function that builds it.
- wrote: moved to the top of the file after
  `error[formatting]: canonical order places type declarations before functions; move gathered up`.
- why it matters: aesthetic. The appendix says declaration order carries
  narrative and is the author's, except for this one rule, and this rule is
  the one that breaks the narrative of a file built from small private
  records.

### F4: a runtime error names the module, not the file
- kind: diagnostic
- severity: minor
- where: csv/read.kso (any runtime failure inside the module)
- wanted: `--> csv/read.kso:104:28`
- wrote: nothing; the test runner printed
  `error[runtime]: append takes bytes and a string, bytes, or byte --> csv:104:28`
- why it matters: `csv` is a directory of three files. Line 104 had to be
  found by opening each one. Compile errors name the file; runtime errors
  should too.

### F5: `[]` is a list but not "bytes", and nothing says so until run time
- kind: confusing-semantics
- severity: minor
- where: csv/read.kso:45, csv/write.kso:21
- wanted: `quoted src (p + 1) [] fields`, and `list/fold bs [34] ...`, with
  the accumulator handed to `text/append`.
- wrote: `quoted src (p + 1) (text/bytes "") fields` and
  `text/append (text/bytes "") 34`.
- why it matters: `kanso check` passed; the tests failed with
  `append takes bytes and a string, bytes, or byte`. A byte buffer prints
  and indexes like a list of ints, so the only way to learn that `[]`
  is not one is to run the program. The checker infers every other type in
  the file; this one it could have caught.

### F6: `list/first` can answer `none`, and the checker lets it reach `length`
- kind: engine-bug
- severity: minor
- where: csv/read.kso:135, bugs/first_none_unchecked.kso
- wanted: `width = length (list/first rows)` refused at compile time, as
  `length rows[1]` is.
- wrote: `return [] if rows == []` above it, found only because the empty
  input test failed at run time with
  `error[runtime]: length takes a list, string, or map, not <none>`, which
  carries no position at all.
- why it matters: the language is strict about `none` from a subscript and
  silent about `none` from a library function documented to answer it. The
  strictness is only useful if it is uniform. Appendix B also still says
  `xs[9] . describe` "propagates the none untouched"; the checker refuses
  that line now, so the book and the compiler disagree.

### F7: no list patterns in a parameter
- kind: missing-feature
- severity: nit
- where: csv/write.kso:10
- wanted: `pub fn record_onto acc [""] _` for the one record the writer
  must quote specially.
- wrote: `return text/append acc "\"\"\n" if fields == [""]`.
- why it matters: string and int literals are patterns and the book makes
  dispatch the only switch, so reaching for a list literal is natural. The
  error, `expected a parameter pattern`, does not say lists are excluded.

### F8: a `none` arm makes disjoint record arms "tie"
- kind: engine-bug
- severity: major
- where: xsv/args.kso:47, bugs/disjoint_arms_tie.kso
- wanted:
  ```
  fn long_value _ _ _ _ _ none _ _
    err "unknown flag"

  fn long_value specs argv i given rest (flag long _ true) v true
    ...
  ```
- wrote: a separate function, `known`, that strips the `none` before
  `long_value` sees it.
- why it matters: a value cannot be both `none` and a `flag`, so no call can
  match both arms, yet the checker rejects all four record arms with
  `these long_value arms tie: each is the more specific one somewhere`.
  Dispatch is the only switch, so a false tie forces an extra function
  into the design every time an optional record meets literal arms.

### F9: std/text has no prefix test, no lowercase, no padding
- kind: stdlib-gap
- severity: major
- where: xsv/strs.kso
- wanted: `text/starts_with? a "--"`, `text/lower key`, `text/pad s 8`,
  `text/repeat " " n`.
- wrote: a helper file of my own:
  ```
  fn prefix? s p
    text/slice s 1 (length p) == p

  fn lowered acc b
    if (64 < b and b < 91) (text/append acc (b + 32)) (text/append acc b)

  fn spaces n
    text/join (list/to_list (list/take (list/repeat " ") n)) ""
  ```
- why it matters: these are the first four string functions a command-line
  tool needs, and each of the twenty ports will write its own. Mine is
  ASCII-only lowercase, which is wrong for `join --no-case` on non-ASCII
  keys, and a standard library version would not be.

### F10: a local binding may not share a name with any function anywhere in the module
- kind: refactoring-hazard
- severity: major
- where: xsv/selection.kso:164 (`chosen`), xsv/selection.kso:50 (`item`)
- wanted: `chosen = list/fold items [] ...` inside `resolve`, and a pattern
  `(read item next)` that binds the field to `item`.
- wrote: `picked` and `it`, after
  `error[name]: chosen is already a declaration; rename the binding`.
  `chosen` is a private helper in a different file, xsv/args.kso.
- why it matters: a module is one namespace across files, so adding a
  private helper to args.kso can break a local variable in selection.kso,
  and the error appears in the file nobody touched. With a dozen files the
  set of names a local may not use is every function in the module. The
  same rule caught `item` against the function `item` in the same file,
  which is fair; across files it is a trap. It went on happening for the
  whole port, eleven times in all:
  - renaming the continuation `done` to `finish` (F12) collided with
    `finish` in cli.kso, so it became `built`;
  - adding `type got` to cli.kso broke a parameter `got` in
    selection_test.kso;
  - a test helper `fn bounds` in selection_test.kso broke the binding
    `bounds = slice_bounds o total` in slice.kso, so a test file can break
    the program it tests;
  - a test constant `specs` broke fourteen parameters named `specs` in
    args.kso;
  - `pad`, `cells`, `wanted`, `zeros`, `counted` and `positions` each
    collided with a function in another file;
  - `values` collided with the ambient builtin `values`.
  Each rename was one edit, and each was forced by code somewhere else.
  The rule exists so that a name means one thing where it is read; scoped
  to a function body it would keep that and lose the action at a
  distance.

### F11: `&` on a record constructor checks clean and then fails, differently on each engine
- kind: engine-bug
- severity: minor
- where: xsv/selection.kso:44, bugs/constructor_partial.kso
- wanted: `selectors cs start [] (&pick inverted)`
- wrote: `selectors cs start [] (xs -> pick inverted xs)`
- why it matters: the interpreter says
  `no overload of finish matches these arguments` and the native backend
  says `pick as a bare value is not yet supported`. The first message
  blames the wrong function. Either form should be refused by
  `kanso check`, or both engines should accept it.

### F12: a parameter named `done` silently becomes a pattern for the `done` marker
- kind: confusing-semantics
- severity: major
- where: xsv/selection.kso:46, bugs/param_named_done.kso
- wanted: `fn selectors cs p items done`, with `done` the continuation that
  builds the finished selection.
- wrote: `fn selectors cs p items finish`
- why it matters: this cost the longest debugging session of the port. The
  checker passed; every test then failed with
  `no overload of selectors matches these arguments`, for a function with
  one arm whose four parameters were all plain names. `done` is a marker
  (the value a write yields), and a marker's name in parameter position is
  a pattern. A local binding named after a declaration is a compile error
  (F10); a parameter named after one should be too, or at least a warning.

### F13: an err inside a list literal is kept as an element, not propagated
- kind: confusing-semantics
- severity: major
- where: xsv/selection.kso:180, bugs/err_in_list_literal.kso
- wanted:
  ```
  fn positions it cols
    one = endpoint it cols 0
    [one]
  ```
  to fail when `endpoint` fails.
- wrote:
  ```
  fn positions it cols
    alone (endpoint it cols 0)

  fn alone e@(err _)
    e

  fn alone i
    [i]
  ```
- why it matters: the selection `zz` against a header without `zz` came
  back as `[err "Selector name 'zz' ..."]`, a one-element list that
  `length` counts as 1. Every other construct I tried passes an err on.
  This one holds it, silently, and the test only caught it because it
  asserted the failure text. A record constructor goes the other way: in
  xsv/cli.kso, `push gs (fetched path (text/utf8 bs))` with a bad file
  failed the whole read at the executor, because the err given to
  `fetched` made the construction itself an err. So a failure handed to
  `[x]` is kept and a failure handed to `fetched x` is raised, and the
  program had to turn the failure into a marker (`not_text`) before
  either would do what was wanted.

### F14: a call that can fail must meet an `(err _)` arm, but the same value through a name need not
- kind: confusing-semantics
- severity: minor
- where: xsv/selection.kso:160, xsv/selection_test.kso:9
- wanted: `resolve (parse_selection sel) cols`, letting the err ride the
  railway as chapter 04 describes.
- wrote: an arm whose only job is to pass the failure on,
  ```
  fn resolve e@(err _) _
    e
  ```
  The checker's message offered the other fix: bind the call to a name
  first, because "the checker reads calls, not the names they are bound
  to".
- why it matters: the two spellings mean the same thing, and one compiles
  while the other does not. The same holds for `none`: `selected o.rest[1] sh`
  is refused until `selected` grows a `none` arm, while
  `sh = sheets[1]` followed by `sh.body` is accepted, and a field read on a
  none fails at run time
  (`` `.` reads a field of a record, not <none> ``, again with no position).
  Binding to a name is the escape hatch from both
  checks, which makes the checks feel like a toll on writing calls inline. In practice this pushes code toward binding
  every fallible call to a throwaway name, or toward pass-through arms
  that add nothing. The finished port has 24 `e@(err _)` arms whose
  body is `e` (`grow`, `alone`, `resolve`, `sided`, `joined`, ...). The rule exists to make sure someone handles the
  failure, and neither workaround handles anything.

### F15: no multi-line string or list literal, so help text is a chain of `push`
- kind: missing-feature
- severity: minor
- where: xsv/commands.kso:4, every `usage_of` arm (about 200 lines)
- wanted: a usage screen written as the text it prints,
  ```
  usage = """
  Prints a count of the number of records in the CSV data.

  Usage:
      xsv count [options] [<input>]
  """
  ```
  or at least a list literal that may continue onto indented lines.
- wrote:
  ```
  fn usage_of "count"
    ["Prints a count of the number of records in the CSV data."]
      . push ""
      . push "Usage:"
      . push "    xsv count [options] [<input>]"
      . usage_text
  ```
- why it matters: a newline inside a literal is `unterminated string`, and
  a list broken across lines is `expected an expression`. The pipe trick
  works and reads tolerably, but I found it by experiment; nothing in the
  book shows how to write ten lines of fixed text, and every CLI port
  needs it. A records list has the same problem:
  `[flag "--delimiter" "-d" true flag ...]` has to be written as
  `[(flag ...) (flag ...)]`, two to a line, to stay under 80 columns. That
  diagnostic, at least, says exactly what to do.

### F16: `text/slice` answers an empty result when the end is past the end
- kind: confusing-semantics
- severity: major
- where: xsv/frequency.kso:64, xsv/slice.kso:74, xsv/strs.kso:47
- wanted: `kept = if (limit > 0) (text/slice ranked 1 limit) ranked`, which
  in Python (`ranked[:limit]`), Rust (`.take(limit)`) or Go (with a min)
  keeps at most `limit` items.
- wrote:
  ```
  fn clamped xs from to
    lo = if (from < 1) 1 from
    hi = if (to > length xs) (length xs) to
    text/slice xs lo hi
  ```
- why it matters: `xsv frequency` with its default limit of 10 printed
  only the header for any column with fewer than ten distinct values, and
  `xsv slice -e 100` on a short file printed nothing. Appendix B does say
  out-of-range bounds give an empty result "so slicing never surprises
  you", but losing every row is the surprise. Clamping is what every
  caller in this port wanted.

### F17: a map entry's parts cannot be read with a dot
- kind: confusing-semantics
- severity: nit
- where: xsv/frequency.kso:63
- wanted: `list/map (entries tallied) (e -> tally_row e.value (shown e.key))`
- wrote: a function with a positional pattern,
  ```
  fn tally_of (entry v n)
    tally_row n (shown v)
  ```
- why it matters: `entries` answers records that print as
  `entry "x" 2`, and every other record reads with a dot, but this one
  says `no record type has a field key`. The field names are not
  documented anywhere I could find.

### F18: an imported name joins the whole module's overloads, from any file
- kind: refactoring-hazard
- severity: major
- where: xsv/join.kso:170 (`matches`), xsv/search.kso:3 (`import "std/regexp"`)
- wanted: a private helper in join.kso,
  ```
  fn matches _ none
    []

  fn matches index k
    or_list index[k]
  ```
- wrote: renamed it `hits_for`.
- why it matters: search.kso imports std/regexp, which exports `matches`.
  Imported pub names join the short-name overload space, and a module is
  one namespace, so regexp's `matches` became extra arms of my `matches` in
  a different file. The symptom was
  `this can be an err and != wants a value` on
  `matches index (key_of o r two.picked) != []`, because regexp's arms can
  answer an err. Nothing in the message mentions regexp. I found it by
  bisecting with throwaway `test_` constants until `matches {} "x"` alone
  reproduced it, and then by reading regexp's export list. A collision
  between a local function and an import should be an error that names
  both, or imports should stay qualified-only.

### F19: "this can be an err" never says where the err comes from
- kind: diagnostic
- severity: major
- where: xsv/join.kso:161, xsv/join.kso:68 (first draft)
- wanted: a note under the error such as
  `the err can come from: side_of (join.kso:76) via unless_failed`.
- wrote: three rounds of guessing. The first cause was real: I had
  checked `one` and `two` for failure through a helper,
  `unless_failed bad (_ -> joined o one two)`, which the checker cannot see
  through, so `one.sh.body` still carried the err. Then F18.
- why it matters: whole-program inference decides that a value may be an
  err, and the error points at the place the err is finally refused, which
  can be several calls and files away from where it entered. Without a
  trace the only tool is bisection.

### F20: an err pass-through arm beside literal arms has no legal order
- kind: diagnostic
- severity: major
- where: xsv/table.kso:37, xsv/cli.kso:114, xsv/search.kso:44,
  bugs/order_hides_tie.kso
- wanted:
  ```
  fn laid e@(err _) _
    e

  fn laid rows 0
    "plain {rows}"

  fn laid rows limit
    "{rows} cut at {limit}"
  ```
- wrote: a second function every time, so the err arm and the literal arms
  live in different groups (`sheet_of`/`laid_out`, `table`'s
  `unless_failed`/`laid`).
- why it matters: `laid (err ...) 0` matches two arms, so the checker is
  right to object, but how it objects depends on the order: one order says
  `overloads of laid appear most-specific first`, the other says
  `these laid arms tie`. Following the first message's advice produces the
  second. In the four-parameter version both orders produced the
  formatting message. The fix is always to split the function, and the
  message never says so. Passing a failure along is the most common thing
  an arm does in this program, so this came up in four files.

### F21: counting by key is quadratic in native code
- kind: performance
- severity: major
- where: xsv/stats.kso (mode, cardinality), xsv/frequency.kso,
  xsv/join.kso:150, bugs/map_read_then_put_quadratic.kso
- wanted: `list/tally values` for a frequency table and
  `put m k (push (or_list m[k]) i)` to index a join's second input, both
  linear.
- wrote: a stable merge sort (xsv/order.kso) and `counted_values`, which
  sorts the values and measures runs of equal neighbours; the join index is
  built the same way and then `put` once per key, without reading the map
  first.
- why it matters: a fold that reads a map and then puts into it took 4.4 s
  natively for 40,000 distinct keys and 0.2 s on the interpreter. Without
  the read it takes 0.12 s. `list/tally` has exactly that shape, so
  `xsv stats --cardinality` on 20,000 rows took 2.7 s, almost all of it in
  tally. After the rewrite, `stats --everything` over the same file takes
  0.5 s and `frequency` 0.17 s. Nothing in the source distinguishes the
  slow fold from the fast one, and the standard library's own counting
  function is the slow one.

### F22: `text/concat` onto a fold's accumulator is quadratic, `push` is not
- kind: performance
- severity: minor
- where: xsv/join.kso:118 (`flat`)
- wanted: `list/fold lists [] (acc xs -> text/concat acc xs)` to flatten the
  per-record row lists of a join.
- wrote: `list/fold lists [] (acc xs -> list/fold xs acc (a x -> push a x))`
- why it matters: a 20,000-row self-join took 3.3 s and 0.33 s after this
  one-line change. Appendix B says `push` reuses the buffer when the list
  is uniquely owned; it says nothing about `concat`, and the two look
  equally functional at the call site. A `list/flatten` or `list/concat_all`
  in std/list would remove the choice.

### F23: "write it inline" and "80 columns" can both apply, and only a new binding satisfies them
- kind: aesthetics
- severity: nit
- where: xsv/xsv_test.kso:47, xsv/selection_test.kso:48
- wanted:
  ```
  test_args_switch_with_value =
    refused_with? ["--reverse=yes"] "flag --reverse takes no value"
  ```
- wrote:
  ```
  test_args_switch_with_value =
    why = "flag --reverse takes no value"
    refused_with? ["--reverse=yes"] why
  ```
- why it matters: written on one line the constant is 85 columns and is
  refused for width; written on two it is refused with
  `a single-expression constant is written inline`. The only way out is a
  binding that exists to satisfy the formatter. Aesthetic, but it came up
  in every test file.

### F24: a list "cannot hold a none", except that it can
- kind: confusing-semantics
- severity: minor
- where: xsv/sort.kso:66, xsv/xsv_test.kso:99, bugs/none_in_list.kso
- wanted: `compare_numeric [none 1] [none 2] == 0` in a test, to pin the
  rule that two non-numbers tie.
- wrote: the lists built the way the program builds them,
  `numbers_at ["x" "1"] [1 2]`, which maps fields through a function that
  answers a number or `none`.
- why it matters: the literal is refused with
  `a list cannot hold a none: a lookup answers "not found" with one, so an element would be indistinguishable`.
  The program's own lists hold nones without complaint, made by
  `list/map` and `push`, on both engines, and `sort -N` depends on it. If
  the rule is real it should hold everywhere; if it is not, the literal
  should be allowed.

### F25: a file that is not UTF-8 can only be handled by reading bytes
- kind: stdlib-gap
- severity: minor
- where: xsv/cli.kso:84
- wanted: `os/read_file path` to answer a third outcome for bytes that are
  not text, the way it answers `file_not_found` for a missing file.
- wrote: `os/read_bytes path`, then `text/utf8`, then a marker type
  `not_text` to carry the verdict out of the bind (see F13 for why an err
  could not).
- why it matters: before the change, `xsv count badutf8.csv` printed
  `error[endpoint]: unhandled err reached the executor: "cannot read badutf8.csv: the bytes are not text"`,
  which is a kanso diagnostic rather than a message for the person who
  ran xsv. `io/stdin` has no bytes form at all, so the same file piped in
  still ends that way.

### F26: a type and a function with one name in two files of a module are not reported
- kind: diagnostic
- severity: minor
- where: xsv/order.kso:6, bugs/type_fn_same_name/
- wanted: `type run` (a run of equal neighbours) in order.kso, beside
  `pub fn run argv` in cli.kso, refused with `the name run is already
  taken`, which is what the same pair gets in one file.
- wrote: `type stretch`.
- why it matters: across files the type was silently dropped, and the
  error was `this can be a none and run has no arm for it` at the
  construction site. It took a while to see that `run` there meant the
  program's entry point.

## What worked well

**Dispatch on the next byte made the CSV reader small, and `none` made the
end of input a case like any other.** The reader in csv/read.kso is a set
of arms keyed on a byte, with no lexer, no state enum and no bounds checks:

```
fn opened src p fields 34
  quoted src (p + 1) (text/bytes "") fields

fn opened _ p fields none
  cut (push fields "") p

fn opened src p fields _
  bare src p p fields
```

The middle arm exists because the checker refused the first draft with
`this can be a none and opened has no arm for it`. That was a real bug:
`a,` at the end of a file, a trailing delimiter with no newline, would have
lost its last empty field. The exhaustiveness rule that cost dead arms in
F1 found a live one here, and `test_trailing_delimiter` pins it.

**String-literal dispatch across files is a command registry for free.**
Each command is one file that adds arms to three groups:

```
fn flags_of "count"
  flags_with []

fn usage_of "count"
  ...

fn compute "count" _ sheets
  "{length sheets[1].body}\n"
```

The catch-all arms (`fn flags_of _` answering `none`, which cli.kso turns
into "unknown command") live in commands.kso. Adding `frequency` touched no
other file. In Rust xsv this is a `match` in main.rs plus a `mod` line.

**Effects as values kept all of the I/O in one short file.** cli.kso reads
every input with a fold over effects,

```
fn read_all paths
  list/fold paths (effect []) (acc p -> acc .> (gs -> read_one p gs))
```

and everything after that is pure: `compute name o sheets` takes parsed
data and answers a string or an err. The unit tests call `resolve`,
`aligned`, `sorted_by`, `counted_values` and the argument parser directly,
with no files and nothing faked. Seventy-five `test_` constants cost about
as much to write as their expected values.

**Missing files are data.** `os/read_file` answering `file_not_found`
turned a missing input into one arm,

```
fn loaded _ (fetched path (file_not_found _))
  err "{path}: No such file or directory"
```

instead of an error path through the effect machinery.

**The railway carried every user-facing error to one place.** Selection
errors, bad flag values, ragged records and bad regexes are all errs born
deep inside pure code, and they reach a single arm,

```
fn finish _ (err r)
  failure "{r}\n"
```

which writes to stderr and exits 1. No function between the birth and that
arm mentions them, apart from the pass-through arms of F14.

**Record patterns in parameters read well for option parsing.**
`fn long_value specs argv i given rest (flag long _ true) _ false` says
"a flag that takes a value, written without `=`" in the head of the arm.

**The three engines agreed on everything.** 122 fixtures, including CRLF
input, quoted newlines, unicode, regexes, float formatting and a 1,500-row
file, produced identical bytes on the interpreter, the dev build and the
release build on the first run, and on every run after. I never had to
think about which engine I was using except for speed.

**Native code is fast.** On a 20,000-row file the release build counts in
about 40 ms, runs `frequency -s city` in 90 ms, sorts numerically in half a
second and self-joins in 0.36 s, with no tuning beyond the two rewrites in
F21 and F22. The interpreter is about twenty times slower, which was still
fast enough to run every fixture on it.

**Formatting diagnostics are precise.** When a list of records was written
`[flag "--delimiter" "-d" true flag ...]`, the compiler said
`flag takes 3 argument(s), and a list element is one atom ... Write (flag …) to call it`.
The 80-column, ordering and spacing errors always named the fix. I never
had to decide how to lay anything out.

## Summary

The five entries I would fix first, in order:

1. **F10 and F18: one namespace per module, for locals and imports too.**
   A binding in one file collides with a function, a test helper, a test
   constant or an imported name in any other file of the module, eleven
   times in this port, and an import in one file silently adds arms to a
   function of the same name in another. The module-wide namespace is
   pleasant for functions; it should stop at function bodies and at
   imports.
2. **F13 (with F24): an err in a list literal is kept, not raised.** It is
   the one place the railway silently stops, and it inverted the result of
   a selection. A record constructor raises the same err, so the two
   containers disagree.
3. **F12: a parameter named `done` becomes a pattern.** The most expensive
   bug of the port to find, with a runtime message that points at the wrong
   thing. Refuse it at compile time as F10's rule already does for local
   bindings.
4. **F21: counting by key is quadratic in native code.** `list/tally` is the
   slow shape, the interpreter is fast at it, and nothing in the source
   says which fold is which.
5. **F20 and F8: arms that pass an err or match `none` tie with literal
   arms.** The diagnostic changes with arm order and never says to split
   the function, and in F8 the arms cannot overlap at all.

Writing xsv in kanso was mostly a pleasure at the level of single
functions and mostly friction at the level of the module. Dispatch on
bytes, strings and record shapes fits a CSV reader and a command-line tool
well, the failure railway put every error message in one place, and the
three engines never disagreed. What slowed the work down was almost never
the problem domain. It was the checker's model of `none` and `err`
(pass-through arms, dead `none` arms, ties, an err that a list swallows and
a constructor raises), a namespace in which every name in every file is in
play, and messages that report where a problem surfaced rather than where it
began. The hardest bugs, F12, F18 and F26, all came from the namespace. The
standard library was thin for text work, with no prefix test, no case
mapping, no padding and a `slice` that empties rather than clamps, and its
counting function was the slowest code in the program.
