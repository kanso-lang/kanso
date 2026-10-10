# FRICTION: porting toml-rs to kanso

A journal kept while writing a TOML 1.0 decoder and encoder in kanso. Every
entry cites code that was compiled. Line numbers are from the tree as it stands
at the end of the port; where an entry describes code that a later redesign
removed, it says so and quotes it. Scratch programs that are not in the tree are
quoted in full or kept under `bugs/`.

## Entries

### F1: no byte or character literal, so every dispatch table is numbers
- kind: missing-feature
- severity: major
- where: toml/value.kso:8 (`value_at`), toml/string.kso (`line_at`, `block_at`, `escape_at`), toml/scan.kso (`end_at`, `space_at`)
- wanted: `fn value_at cs '"' p` or Rust's `b'"'`; `fn escape_at _ 'n' p`
- wrote:
  ```
  # A value is chosen by its first byte: 34 `"`, 39 `'`, 43 `+`, 45 `-`, 91 `[`,
  # 102 `f`, 105 `i`, 110 `n`, 116 `t`, 123 `{`, and digits.
  fn value_at cs 34 p
    string_value cs p 34
  ```
- why it matters: byte dispatch is the parser idiom the book teaches in
  chapter 08, and it suits TOML well, but a reader has to translate 92, 110 and
  117 back into `\`, `n` and `u` on every arm. The escape table in string.kso
  is twelve arms of numbers. I kept an ASCII chart open for the whole port, and
  the comments that translate the numbers are a second copy of the program that
  nothing checks.

### F2: a catch-all arm does not cover `none`, so every byte dispatch needs a `none` arm
- kind: confusing-semantics
- severity: major
- where: toml/scan.kso:22 (`blank? none`), toml/key.kso:35 (`key_dot _ none`), toml/number.kso (`token_byte? none`, `based? none`, `sign_width none`, `radix_of none`, `run_at _ none`), toml/string.kso (`lines_at _ none`, `trim_at _ none`), toml/value.kso (`array_open_at`, `inline_open`, `inline_next`)
- wanted:
  ```
  fn blank? 9
    true
  fn blank? 32
    true
  fn blank? _
    false
  ```
  with `_` answering for the end of input too, which is what the arm says.
- wrote: the same group plus `fn blank? none` / `false`, and the same extra arm
  on nineteen other functions. The first `kanso check` of the parser produced
  nineteen `error[exhaustive]: this can be a none and ... has no arm for it`
  reports, all at a `cs[p]` argument. The test file hit the same rule from the
  other side: `same? (lookup { "a":1 } "a.b") none` is refused because the
  literal `none` needs a `none` arm in `same?` (toml/toml_test.kso, `absent?`).
- why it matters: the book says a catch-all "answers for ints, strings,
  records, markers—anything", and at run time it does: a `none` that reaches a
  generic parameter through a name is accepted (F3). The checker and the
  runtime disagree about what `_` means. In a byte parser the end of input is
  often handled exactly like "some other byte", so most of the extra arms
  repeat the catch-all's body. The check also covers only index expressions:
  my own `stored` (toml/index.kso) can answer `none` and the checker never
  asks for an arm at its call sites.

### F3: the exhaustiveness check reads calls, so a binding or a constructor launders an err or a none past it
- kind: refactoring-hazard
- severity: major
- where: toml/document.kso (`pair_key`, `header_names`), toml/value.kso (`inline_key`, `inline_value`), toml/number.kso (`dec_int`, `exponent`), and the first version of toml/table.kso
- wanted: one rule. Either `f (g x)` and `y = g x` / `f y` are both checked, or
  neither is.
- wrote: a scratch module showed three shapes of the same call:
  ```
  pub direct3 = finish (boom 5)              # error[exhaustive]
  pub vianame =                               # compiles; err rides past
    v = boom 1
    r = parsed 3 v
    finish r
  pub direct = finish (parsed 3 (boom 3))     # compiles too
  ```
  The first version of table.kso leaned on the second shape for `none`:
  ```
  fn header_walk t names i p mode
    return t if i < 1 or length names < i
    k = names[i]
    existing = t.members[k]
    child = header_child names i p mode existing
    with_entry t k child
  ```
  `header_child names i p mode t.members[k]` was refused; the version above
  compiled, and at run time the `none` in `existing` reached `table_last`'s
  `none` arm as intended. The whole parser is written in the binding style so
  that each step's possible failure rides past the next step without an
  `e@(err _)` arm on every continuation, which is what kq does instead.
- why it matters: the rule is documented ("the checker reads calls, not the
  names they are bound to"), so this is not a bug, but it makes inlining a
  binding a compile error and extracting one a way to switch the check off.
  A reader cannot tell from the source which steps the checker has looked at.

### F4: proving an index in range takes a guard on both ends, and the message does not say so
- kind: diagnostic
- severity: minor
- where: toml/table.kso:79, :126 (`return slots if i < 1 or length names < i`), toml/lookup.kso (`path_steps`)
- wanted: `return t if length names < i` before `names[i]`, since `i` starts at
  1 and only grows; or a diagnostic that names the guard it would accept.
- wrote:
  ```
  fn first_guard xs i
    return 0 if length xs < i
    need_int xs[i]          # still error[exhaustive]: this can be a none
  fn both_guard xs i
    return 0 if i < 1 or length xs < i
    need_int xs[i]          # accepted
  ```
  Every walker in table.kso and lookup.kso carries an `i < 1 or` that can never
  fire.
- why it matters: the fix is in the standard library's comments ("the bound
  is written where the read is, so the checker proves the read in range"), not
  in the diagnostic, which offers only a `none` arm.

### F5: a local name collides with every declaration in the module and every ambient builtin
- kind: refactoring-hazard
- severity: major
- where: toml/datetime.kso (`written` in `padded`), toml/number.kso (`spelling`, `zero_led`), toml/string.kso (`whole` in `closed`), toml/table.kso (`names`), toml/toml.kso (`source` in `error_report`), toml/lookup.kso (`after_key`)
- wanted: `shown = "{n}"` inside `padded`; `fn pair_key cs start (parsed p keys) d`; `fn with_entry (tbl entries kind) k v`; `done = text/append ...`; a helper named `stepped` in lookup.kso
- wrote: renamed each one after `error[name]: `shown` is already a
  declaration; rename the binding`. The collisions were with `shown` (a
  function in scan.kso), `word` (value.kso), `padded` (datetime.kso), `done`
  (a builtin marker), and `keys` and `entries` (ambient builtins). `keys` was
  the natural name for the list of keys in a dotted key, so every function in
  key.kso, table.kso and document.kso says `names`. Late in the port, adding
  `fn stepped` to the new lookup.kso broke `stepped = segment_step ...` in
  table.kso, a file the change did not touch.
- why it matters: a module is one namespace across its files, so any new
  top-level name can break a binding in any file. The no-shadowing rule is
  reasonable inside one function; across a fifteen-file module it makes adding
  a helper a search of the whole module.

### F6: the 80-column cap has no continuation form for `return … if …`, and a long constant cannot be a block
- kind: aesthetics
- severity: minor
- where: toml/datetime.kso:97, :166, :172, toml/number.kso:107, :144, toml/scan.kso:40, toml/string.kso:190, toml/table.kso:182, toml/toml_test.kso:123
- wanted:
  ```
  return fail (base + i - 1) "invalid offset: expected `Z` or ±HH:MM" if not shape
  ```
  and
  ```
  test_error_table_twice =
    refused? "[a]\n[a]" "2:2 table `a` is defined more than once"
  ```
- wrote:
  ```
  why = "invalid offset: expected `Z` or ±HH:MM"
  return fail (base + i - 1) why if not shape
  ```
  and, for the test, a `want = …` binding, because the block form is refused
  with "a single-expression constant is written inline" while the inline form
  is 81 columns.
- why it matters: continuation lines exist only for `.`, `.>`, `.!` and `.?`.
  A guard with a descriptive message is one statement that runs long, and the
  only way to fit it is to name its parts. About twenty-five statements in this
  port gained a `why`, `want`, `extra`, `wide` or `zero_led` binding whose only
  job is line length. There is also no way to continue a long string literal.

### F7: a section comment cannot stand on its own
- kind: aesthetics
- severity: nit
- where: every file; for example toml/number.kso before `fn dec_int`
- wanted:
  ```
  # Integers.

  fn dec_int tok base
  ```
- wrote: the comment pressed against the first declaration of the section,
  where it reads as that one function's documentation.
- why it matters: `error[formatting]: exactly one blank line separates
  top-level declarations` fires on the blank line after a comment. Long files
  want headings (string.kso has four kinds of string in it), and the grammar
  has nowhere to put one.

### F8: a run-time error names the module and a line, not the file
- kind: diagnostic
- severity: minor
- where: toml/string.kso (the escape table, at the time); toml/toml_test.kso (`test_invalid_utf8`)
- wanted: `--> toml/string.kso:42:20`
- wrote: the report said
  ```
  error[runtime]: append takes bytes and a string, bytes, or byte
    --> toml:42:20
  ```
  and I printed line 42 of all ten files to find it. A unit test later failed
  with `error[runtime]: find2 takes bytes` / `--> toml:92:10`, the same way.
- why it matters: compile-time diagnostics always name the file; run-time ones
  name the module, which in a multi-file module is not enough to find the
  line.

### F9: the book's `kanso run file.kso` refuses a file that mixes definitions and statements
- kind: tooling
- severity: minor
- where: scratch file; book sample docs/book/samples/ch05/currying.kso
- wanted: `kanso run currying.kso`, as chapter 05 prints it
- wrote: `kanso play currying.kso`. `kanso run` answers "`currying.kso` is a
  library — nothing to run. give the module a main.kso entry, or run its
  definitions beside their statements with `kanso play`". For this port it
  pushed every definition out of main.kso into `cli/`, which is the right
  layout anyway.
- why it matters: the first program I wrote, following the book, did not run,
  and `pub play =` (which the book also uses with `kanso run`) did not help.

### F10: parentheses that only clarify are refused
- kind: aesthetics
- severity: nit
- where: toml/index.kso:31 (`hash < h or hash == h and key < k`), toml/number.kso (`if not between`)
- wanted: `return parsed p acc if not (digit? cs[p])` and
  `hash < h or (hash == h and key < k)`
- wrote: `not digit? cs[p]` and `hash < h or hash == h and key < k`, after
  "these parentheses group nothing — the expression parses the same without
  them".
- why it matters: `not digit? cs[p]` reads as `(not digit?) cs[p]` to anyone
  arriving from another language, and whether `and` binds tighter than `or` is
  a fact many programmers look up every time. The parentheses were for the
  reader, not the parser.

### F11: `pub fn name =` gets a worse message than `pub fn name`
- kind: diagnostic
- severity: nit
- where: scratch module
- wanted: the helpful message either way.
- wrote: `pub fn floats =` gives `error[syntax]: expected a parameter pattern`;
  `pub fn table_demo` (no `=`) gives "a value with no parameters is a
  constant: `table_demo = ...`", which is the message I needed both times.
  Writing `pub bare_n n = …` for a function gives "a re-export is `pub name` or
  `pub theirs:yours`", a third message for a third near miss.
- why it matters: small, but these are the first walls a newcomer hits when
  writing a constant with a body.

### F12: a `_` parameter accepts an err, and the body runs
- kind: engine-bug
- severity: major
- where: toml/number.kso:142 (`run_last`), :256 (`float_text`); bugs/wildcard-swallows-err
- wanted: a validation step whose result the next step ignores:
  ```
  fn exponent tok i base
    j = i + sign_width tok[i]
    k = digit_run tok j 10 base
    rest = trailing tok k base "float"
    float_done tok rest

  fn float_done tok _
    float_text tok
  ```
- wrote: every validation step answers something the next step uses
  (`run_last` answers the index of the run's last byte, and `float_text tok
  last` slices to it), so the err reaches a named parameter and the call
  short-circuits.
- why it matters: chapter 04 says a function handed an err never runs. With
  `_` in that position it does, and the err disappears. `a = 1e` was decoded
  by way of `text/to_float "1e"`, whose own err then reached the CLI as
  something other than a `toml_error`, and the error report crashed with "no
  overload of `toml/error_report` matches". Only a fixture caught it. With the
  unused-binding rule, there is no way to say "I take this argument only so
  that its failure stops me" except to find a real use for it.

### F13: compiled code runs an arm on an err when a sibling arm has `_` in that position
- kind: engine-bug
- severity: blocker
- where: toml/value.kso:125 (`inline_after`); bugs/native-runs-arm-on-err
- wanted:
  ```
  fn inline_value cs start names (parsed p v) slots
    q = skip_blank cs p
    grown = insert_kv slots [] names v start
    inline_delim cs cs[q] q grown
  ```
  where `inline_delim`'s `125` arm builds the table and its last two arms say
  `_` for the table.
- wrote: a one-arm gate, `fn inline_after cs q slots` / `inline_delim cs
  cs[q] q slots`, so the err stops at a function whose every parameter is
  named.
- why it matters: on the release build `a = {b = 1, b = 2}` reported
  "error[runtime]: `.` reads a field of a record, not <value>" while the
  interpreter and the dev build reported "duplicate key `b`". The reduction is
  twenty lines: with `fn pick 125 xs` and `fn pick _ _`, `pick 125 grown` runs
  the first arm with the err and prints `size 6` on dev and release, and the
  interpreter reports the err. The differential law is broken by an
  ordinary-looking dispatch group, and nothing in the source hints at it.

### F14: a map read between writes costs a full sort in compiled code, and a map inside a record copies on every update
- kind: performance
- severity: blocker
- where: toml/index.kso (the whole file exists because of this), toml/table.kso:228 (`build_tree`); bugs/record-field-put-copies, bugs/named-put-superlinear
- wanted: the textbook shape: a nested map per table with a kind tag, or a
  flat map from path to slot:
  ```
  fn insert_at slots base names i v p
    ...
    existing = slots[key]
    return set_new slots key here names i existing v p if i == length names
  fn set_new slots key here _ _ none v _
    put slots key (slot (kind_of v) here v)
  ```
- wrote: three designs, timed on a 300 KB document of 2,000 `[[servers]]`
  tables with a release build:
  1. tables as `tbl members kind` records nested in maps: 10.1 s. Every
     update to a map inside a record copied the map, on every engine
     (bugs/record-field-put-copies: 20,000 inserts take 14 s through a record
     against 0.03 s on a bare map).
  2. one flat map from path to slot, with continuation-passing so that every
     `put` was an argument to a tail call: 32 s. Reading `slots[key]` before
     each `put` made the runtime sort every pair again on the next read. The
     runtime source explains it: a `put` that compiled code does not prove
     unique appends a pair and gives the map a fresh header with no sorted
     view, and the next read sorts all the pairs.
  3. a persistent search tree written in kanso (toml/index.kso, about 70
     lines), with the nested maps built once at the end: 0.41 s for the same
     document, output byte-identical.
  Smaller probes showed that binding a `put` result to a name (`next = put m k
  v` and then a tail call) takes 4,000 inserts from 0.016 s to over 30 s in
  compiled code while the interpreter stays at 0.02 s, and that any use of a
  map before `put` in the same function, even `length m`, makes the loop
  quadratic.
- why it matters: "check the key, then insert" is the inner loop of every
  parser, symbol table and config reader, and it is the access pattern the
  native map does not survive. Appendix B promises that `put` "reuses the
  buffer in place when the map is uniquely owned" and says nothing about which
  spellings the compiler can prove unique. I found the rules by timing about
  twenty variants, and a reader of index.kso cannot see why it exists without
  the comment at its top. The interpreter is linear in most of these cases, so
  the two engines also disagree about complexity.

### F15: pushing onto a list a helper handed back crashes compiled code at 64 elements
- kind: engine-bug
- severity: blocker
- where: toml/index.kso:62 (`walk`, `visit`, `also_right`); bugs/push-through-helper-segfault
- wanted:
  ```
  fn gathered (branch _ _ l r v) acc
    gathered r (push (gathered l acc) v)
  ```
- wrote: a walk that keeps its own list of subtrees still to visit, so every
  `push` is an argument to a tail call.
- why it matters: `fn helper xs i` / `push xs i`, called from a loop, works at
  63 elements and segfaults at 64 on dev and release; the interpreter is fine.
  In the decoder the same shape showed up as "Fatal glibc error: malloc.c:2599
  (sysmalloc): assertion failed" while decoding a re-encoded table.toml: heap
  corruption from a two-line pure function, in two engines of three.

### F16: calling a continuation is never a tail call
- kind: engine-bug
- severity: major
- where: an abandoned continuation-passing version of toml/document.kso and toml/table.kso; bugs/closure-call-not-tail-call
- wanted: `insert_kv slots current names v start (grown -> parse_document cs next current grown)`, with each insertion step ending in `then (put slots key made)`.
- wrote: insertion answers the new index directly, which index.kso makes
  cheap.
- why it matters: continuation-passing was the one way I found to keep `put`
  in place (F14). The interpreter then ran out of stack at 10,000 steps, and
  the dev build segfaulted at 100,000 with no diagnostic where the interpreter
  printed one. Appendix A promises "a tail call costs no stack at all"; a call
  to a lambda in tail position is not treated as one.

### F17: floats are printed with exponents that a float literal cannot be written with
- kind: missing-feature
- severity: minor
- where: toml/toml_test.kso:53 (`test_floats`)
- wanted: `{ "c":6.5e-1 }` in a test, or `0.65 == 6.5e-1`
- wrote: `0.65`. `6.5e-1` is `error[name]: unknown name `e``, though kanso
  itself prints `1e300` as `1.0e+300` and `1e-7` as `1.0e-07`.
- why it matters: a value the language prints cannot be pasted back into a
  test. For a TOML port that writes floats back out through `"{x}"`, the
  printed form is the one I wanted to assert against.

### F18: a map literal's value must be an atom, and the message points at the wrong thing
- kind: diagnostic
- severity: minor
- where: toml/toml_test.kso:63 (`test_local_date`)
- wanted: `{ "d":local_date 27 5 1979 }`
- wrote: `{ "d":(local_date 27 5 1979) }`, after `error[syntax]: expected `:`
  after a map key` with the caret on `5`.
- why it matters: the same mistake in a list literal, `[empty_table header]`,
  gets an excellent message ("`empty_table` takes 1 argument(s), and a list
  element is one atom … Write `(empty_table …)` to call it"). The map literal
  deserves the same one.

### F19: the checker judges an overload group as a whole, not the arm the argument selects
- kind: confusing-semantics
- severity: minor
- where: toml/encode.kso:16 (`value_toml`)
- wanted:
  ```
  pub fn value_toml m:map[string some]
    encode m
  ```
  `encode` has a `map` arm, which cannot fail, and a catch-all arm that
  answers an err for a non-table.
- wrote: `text/utf8 (section_onto (text/bytes "") m [])`, repeating `encode`'s
  map arm, after `io/write (toml/value_toml m)` in the CLI was refused with
  "this can be an err and `io/write` has no arm for it".
- why it matters: the type of `m` already selects the arm that cannot fail, and
  the checker has that information, since it dispatches on it. A function with
  one fallible arm makes every caller handle failure, including callers that
  can never reach that arm.

### F20: one name, two arities, two files: the private group hides the public arm
- kind: refactoring-hazard
- severity: major
- where: toml/lookup.kso (`pub fn lookup doc path`), toml/index.kso (then `fn lookup t:branch hash key`, now `search`)
- wanted: a public two-argument `lookup` in lookup.kso next to a private
  three-argument `lookup` in index.kso.
- wrote: renamed the private one `search`. Before that, the CLI's
  `toml/lookup doc query` was refused with "`lookup` is private to module
  `toml` — only pub names cross an import", though the arm it named was
  declared `pub`.
- why it matters: arity is part of the name for dispatch (appendix A says a
  call with a count no arm takes is an arity error), but visibility is decided
  for the whole group, apparently by the arm that is not `pub`. The message
  named the right symbol and the wrong reason, and the fix was in a file the
  message did not mention.

### F21: a strict index's box passed to a function that wants a value compiles
- kind: diagnostic
- severity: minor
- where: scratch timing program
- wanted: a compile error, like the book's `length os/args` example.
- wrote:
  ```
  os/args .> (a -> os/read_file! a[1]!) .> (s -> print "{length (toml/decode s)}")
  ```
  compiled and failed at run time with `error[runtime]: read_file takes a path
  string`. The working spelling chains the box: `os/args .> (a -> a[1]!) .>
  os/read_file! .> …`.
- why it matters: chapter 05 says the compiler refuses what it can prove is a
  box, and `a[1]!` is a box by its syntax.

### F22: bytes and a list of ints are different types that look the same
- kind: stdlib-gap
- severity: minor
- where: toml/toml_test.kso (`test_invalid_utf8`), toml/string.kso (`escape_at` answers a byte, not `[9]`)
- wanted: `decode_bytes [97 32 61 32 34 255 34]` in a test, and `text/append acc [9]`
- wrote: `decode_bytes (text/to_bytes [97 32 61 32 34 255 34])`, and escapes
  that answer a single byte as an int.
- why it matters: `text/find2` refuses an int list ("find2 takes bytes"),
  `text/append` refuses one ("append takes bytes and a string, bytes, or
  byte"), and `text/to_bytes` refuses bytes ("to_bytes takes a list of byte
  values"). Both print as `[97 98]`, no pattern tells them apart, and appendix
  B describes `text/bytes` as answering `int[]`. All three refusals came at
  run time.

### F23: `std/list` has no `sort_by`
- kind: stdlib-gap
- severity: minor
- where: toml/table.kso:228 (`build_tree`)
- wanted: `list/sort_by slots (s -> path_key s.path)`
- wrote:
  ```
  spelled = list/map (stored_values slots) (s -> path_key s.path)
  ordered = list/sort (list/to_list spelled)
  items = list/to_list (list/map ordered (k -> stored slots k))
  ```
  Sorting `[key slot]` pairs fails at run time with "comparison requires two
  values of one comparable type", because records do not compare.
- why it matters: sorting records by a key is common, and the workaround
  needs a second structure to find each record again.

### F24: `group_by` and `tally` read the map they are filling
- kind: performance
- severity: minor
- where: toml/table.kso:228 (`build_tree`, comment)
- wanted: `list/group_by items (s -> s.path[depth])` to split the slots by
  their next key, which was the first version of `build_tree`.
- wrote: a sort and a linear scan over the sorted slots (`members`,
  `run_end`, `elements`).
- why it matters: lib/list/list.kso writes `put acc k (push (bucket acc[k])
  x)`, the read-then-write pattern from F14. `group_by` over 16,000 distinct
  strings took 1.32 s on release against 0.33 s on the interpreter. Building a
  table with 16,000 keys took 1.75 s with `group_by` and 0.55 s with the sort.

### F25: the interpreter is about ninety times slower than release on this workload
- kind: performance
- severity: minor
- where: check.sh
- wanted: an oracle fast enough to run the same large fixtures as the
  binaries.
- wrote: the large fixture is 3,000 lines (two seconds on the interpreter);
  the 300 KB benchmark document is not a fixture.
- why it matters: the 300 KB document decodes in 0.41 s on release and 36 s on
  the interpreter, and the interpreter's decode time grows faster than the
  input (1,000 tables 7.1 s, 500 tables 2.8 s). check.sh spends most of its
  time on the interpreter, so it decides how large a fixture can be.

### F26: there is no way to see where a program spends its time
- kind: tooling
- severity: minor
- where: the investigation behind F14 and F24
- wanted: `kanso run --profile` or a counters flag listed in `kanso`'s usage.
- wrote: about twenty-five probe programs timed with `date` around each run,
  and a reading of src/runtime.c to learn why a map read was slow.
- why it matters: every performance finding in this journal came from timing
  variants of a program by hand. kq's spec script mentions `--counters`, but
  it is not in the usage text and I did not find it documented.

## What worked well

**Byte dispatch with `none` as the end of input.** Once the numbers were
learned, the string scanner read like the grammar. Each kind of string is a
group of arms on the next byte, and running off the end is one more arm:

```
fn line_at cs 34 p start acc 34
  closed cs p start acc 1

fn line_at _ none p _ _ _
  fail p "unterminated string"

fn line_at _ 10 p _ _ _
  fail p "newline in a single-line string"
```

There are no bounds checks before reads anywhere in the parser, which is the
point chapter 08 makes about the JSON library.

**The failure railway.** A failed step returns `fail p reason`, the value
travels out through every later step that binds it to a parameter, and `decode`
turns it into a line and column once:

```
fn finish cs (err (parse_failure p reason))
  err (toml_error (column_of cs p) (line_of cs p) reason (line_text cs p))
```

No step tracks the line or column, and none checks whether the step before it
failed. (F3 and F12 are the price; the shape is still good.)

**Records and dispatch on type for the date-time values.** The four TOML
date-time kinds are four records, and the JSON writer, the TOML writer and
`type_name` each pick them apart with one arm apiece (`fn type_name
_:local_date` / `"date-local"`). Adding a fifth kind would be one arm per
writer.

**Structural equality made the tests short.** A test is a decode compared with
a literal: `decodes? "a.b.c = 1" { "a":{ "b":{ "c":1 } } }`. Fifty-six of them
cover values, errors, positions, the writers and the internals.

**Maps iterate in key order.** The JSON and TOML writers produce sorted, stable
output with no sorting code, so the fixtures are deterministic on every engine.

**Exact integers.** The 64-bit range check is `-9223372036854775808 <= v and v
<= 9223372036854775807` on the value as read; there is no overflow to guard
against while reading the digits.

**Diagnostics that teach.** `error[type]: `aot` has 2 fields and this arm takes
1, so it can never match`, the list-literal message in F18, the overload-order
message and the 80-column counter each told me exactly what to change.

**std/json for the encode direction.** Reading the toml-test JSON took one call
to `json/decode`, and dispatching on its failure record from another module
worked as chapter 07 describes:

```
fn why r:json/parse_failure
  "invalid JSON at byte {r.position}: {r.reason}"
```

**Three engines.** Running every fixture on the interpreter, a dev build and a
release build found three engine bugs (F13, F15, F16) that a single engine
would have hidden, and the 154 cases now agree byte for byte.

## Summary

The five entries I would fix first, in order:

1. **F13**, compiled code running an arm on an err when a sibling arm has `_`
   in that position. It silently breaks the differential law in an
   ordinary-looking program.
2. **F15**, the segfault and heap corruption from pushing onto a list a
   helper returned. A crash in pure code, in compiled code only.
3. **F14**, the native map's cost when reads and writes interleave and its
   copies inside records. It decides how every stateful program has to be
   structured, and the rules are written nowhere.
4. **F12**, `_` accepting an err. It is the root of F13, and it makes the
   failure railway leak exactly where a programmer says "I don't need this
   value".
5. **F5**, local names colliding with every declaration in the module. It
   turns adding a helper in one file into a compile error in another, and it
   hit this port seven times.

Writing the decoder itself was pleasant. Byte dispatch, the failure railway and
exact integers fit a parser closely, and the first full run of the decoder
produced correct JSON for the TOML specification's example. The canonical form
cost a steady stream of small renamings and line splits, irritating but never
blocking. What cost real time was performance and the engines disagreeing.
About half of the effort went into finding out why a 300 KB document took ten
seconds and then thirty-two, into reducing four engine bugs and two
performance cliffs to programs of a page or less, and into designing around
them. The final shape (a flat
persistent tree while reading, nested maps built once at the end) is a good
design that I would not have chosen without being pushed, and a reader needs
the comments to see why the obvious design was not used.
