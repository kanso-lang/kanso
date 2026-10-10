# Friction journal: mustache in kanso

Entries are in the order I hit them. Every code sample was compiled with
`/tmp/claude-0/kanso-main/kanso`; where a bug has a reproduction, it is in
`bugs/`.

## Entries

### F1: standard-library names take over a module's own types without an import
- kind: engine-bug
- severity: major
- where: mustache/parse.kso:9 (`type scanner`, first `cursor`), :18 (`type stretch`, first `run`), mustache/render.kso:24 (`type scope`, first `env`)
- wanted:
  ```
  type run
    cursor
    last
    nodes

  fn finish (run _ _ nodes)
    nodes
  ```
- wrote: renamed `run` to `stretch`, `cursor` to `scanner`, and `env` to
  `scope`. The parse module imports only std/list and std/text, yet `run`
  resolved to std/os's `pub fn run cmd argv`:
  ```
  error[arity]: `run` takes 2 argument(s), got 3 (module mustache)
  ```
  `cursor` was taken by std/list's pub type and `env` by `os/env`. The `env`
  case cascaded into three `error[effect]: e holds an effect` messages, because
  my record constructor had become a call to an effect. Reproduction:
  `bugs/std_name_leaks_without_import.kso`, a file with no imports at all.
- why it matters: Appendix b says eight names are ambient. In practice every
  pub name in the standard library is reserved in every module, and nobody can
  hold that list in their head. The error messages talk about a function the
  file never mentions.

### F2: a file's opening comment cannot stand apart from its first declaration
- kind: diagnostic
- severity: nit
- where: mustache/nodes.kso:1
- wanted: a file-level comment, a blank line, then the first declaration
  with its own comment.
- wrote: one comment block with a `#` line between the paragraphs.
- why it matters: The message is `the file may not begin with a blank line`,
  pointing at line 3. The file began with a comment, so the message describes
  a different file. Something like "a comment belongs to the declaration below
  it" would say what the rule is.

### F3: the canonical arm order pulls arms out of a byte table
- kind: aesthetics
- severity: minor
- where: mustache/parse.kso:56 (`act`)
- wanted: the `act` arms in sigil order, so the table reads 33, 35, 36, 38,
  47, 60 and so on, with the closing tag's arms in their place:
  ```
  fn act 33 cs cur until acc _ s   # ! comment
  fn act 35 cs cur until acc t s   # # section
  ...
  fn act 47 cs _ top _ t _         # / with nothing open
  fn act 47 cs cur (opened name at) acc t s
  fn act 60 cs cur until acc t s   # < parent
  ```
- wrote: both `act 47` arms at the head of the group, above `act 33`. An arm
  with a literal and a marker (or a record pattern) outranks an arm with a
  literal alone, across the whole group, even though no call could reach
  both:
  ```
  error[formatting]: overloads of `act` appear most-specific first: literal,
  then concrete type, then generic
  ```
- why it matters: The rule makes a file read in resolution order, but for a
  dispatch table keyed on bytes the reader looks things up by byte, and the
  `/` arms are now the only ones out of place. The diagnostic names the rule
  without saying which arm this one must move above, so finding the legal
  order took two tries.

### F4: a qualified marker in a pattern does not count as using its import
- kind: engine-bug
- severity: major
- where: mustache/render.kso:112 (`falsey?`), :144 (`interpolate`), :286 (`section`)
- wanted:
  ```
  import "std/json"

  fn falsey? json/json_null
    true
  ```
- wrote: `fn falsey? _:json/json_null`. The bare `json_null` and the
  qualified `json/json_null` both draw `error[unused]: unused import
  "std/json"`; remove the import and the same pattern becomes a fresh binding
  (`unused binding json/json_null`). Reproduction:
  `bugs/qualified_marker_in_pattern.kso`.
- why it matters: An engine that takes JSON data has to name JSON's null in its
  dispatch, and the obvious spelling is refused with a message about the
  import line. If the import were present for some other reason, a typo in
  the marker name would silently become a catch-all arm.

### F5: arms whose first arguments can never overlap are called a tie
- kind: engine-bug
- severity: minor
- where: mustache/render.kso:141 (`interpolate`)
- wanted:
  ```
  fn interpolate none _ _ _ acc
    acc

  fn interpolate s:string true _ _ acc
    emit acc (escape s)

  fn interpolate s:string false _ _ acc
    emit acc s
  ```
- wrote: one generic arm that renders the value and a helper that decides on
  the escape flag:
  ```
  fn interpolate v escaped _ _ acc
    emit acc (escaped_if escaped "{v}")

  fn escaped_if true s
    escape s

  fn escaped_if false s
    s
  ```
  The refusal was `error[dispatch]: these interpolate arms tie: each is the
  more specific one somewhere, and a call could match both`. No call can match
  both: no value is both `none` and a string. Reproduction:
  `bugs/disjoint_arms_called_a_tie.kso`.
- why it matters: The workaround is fine code, but I reached it by guessing.
  The message claims an overlap that does not exist, so the reader goes
  looking for a bug in their own arms.

### F6: every mustache template in kanso source needs its braces escaped
- kind: aesthetics
- severity: minor
- where: mustache/mustache_test.kso:41 onward
- wanted:
  ```
  test_hello = renders? "Hello, {{name}}!" { "name":"world" } "Hello, world!"
  ```
- wrote:
  ```
  test_hello = renders? "Hello, \{\{name}}!" { "name":"world" } "Hello, world!"
  ```
  Without the escapes the parser reads `{{name}}` as an interpolation of a
  map literal and reports `name is not a literal: a map's keys are
  literals`. There is no raw-string form.
- why it matters: It is aesthetic, but it bites any program that handles a
  brace-delimited format: templates, JSON in tests (`"\{\"a\": 1}"`), C-like
  code. The spec suite moved into JSON files partly so the fixtures would be
  readable.

### F7: the interpreter's text/concat refuses a list made by text/bytes
- kind: engine-bug
- severity: major
- where: mustache/parse.kso:134 (`closer`)
- wanted:
  ```
  fn closer 123 close
    text/concat [125] close
  ```
- wrote:
  ```
  fn closer 123 close
    text/bytes "}{text/utf8 close}"
  ```
  Native code prints `[1 97]` for `text/concat [1] (text/bytes "a")`; the
  interpreter, and so `kanso test`, stops with `error[runtime]: concat takes
  two lists`. Reproduction: `bugs/concat_rejects_bytes_in_interp.kso`.
- why it matters: The interpreter is the oracle and `kanso test` runs on it, so
  a byte scanner that works natively fails its own unit tests. The checker
  accepted the program, so nothing warned me before run time.

### F8: a maybe-none in two positions needs four arms, one of them redundant
- kind: confusing-semantics
- severity: minor
- where: mustache/parse.kso:255 (`placed`)
- wanted: a standalone tag needs a line start behind it and a line end ahead
  of it, and either can be absent:
  ```
  fn spot_of cs t true
    first = back cs (t.start - 1)
    resume = line_end cs t.stop
    return inline t if first == none or resume == none
    spot (piece cs first (t.start - 1)) (first - 1) resume
  ```
- wrote:
  ```
  fn placed _ t none none
    inline t

  fn placed _ t none _
    inline t

  fn placed _ t _ none
    inline t

  fn placed cs t first resume
    spot (piece cs first (t.start - 1)) (first - 1) resume
  ```
  The guard does not narrow: `resume` stays maybe-none, the `spot` record
  carries it, and six call sites of `moved cur s.resume` were refused with
  `this can be a none and mustache/moved has no arm for it`. Two arms,
  `(none, _)` and `(_, none)`, are refused as a tie. The checker also asks for
  a none arm in each position where a none can arrive, so a helper that
  handles one position and delegates the other is refused at the call. The
  `(none, none)` arm exists only to break the tie.
- why it matters: Two optional values is an ordinary situation. A guard that
  says "if either is missing, stop" reads the way the logic does, and the
  four-arm version looks like it handles four cases when there are two.

### F9: errors in a library appear only when another module imports it
- kind: diagnostic
- severity: major
- where: mustache/parse.kso:334 (`sigil_of`), and every none arm in parse.kso
- wanted: `kanso check mustache` to report what is wrong inside `mustache`.
- wrote: nothing different; the point is when I learned of it. `kanso check
  mustache` said `mustache: ok`. When the spec module imported it, the check
  of `./spec` reported errors located in `./mustache/parse.kso`, such as
  ```
  error[exhaustive]: this can be a none and `mustache/standalone?` has no arm
  for it ... (in parse.kso) (module ./spec)
  ```
  I reproduced it again after the fact by putting the old line back
  (`sigil = if (sigil? cs[after]) cs[after] 0`, with a `none` arm on
  `sigil?`): `kanso check mustache` and `kanso test mustache` pass, and
  `kanso check .` reports two errors inside parse.kso, attributed to module
  `./spec`. The spec runner passes the template as whatever JSON value the
  suite holds, where the CLI and the tests pass a string, so the checker's
  verdict on code inside the library depends on what a caller hands it. A
  two-module reduction did not reproduce it, so there is no file in `bugs/`.
- why it matters: The natural loop is to finish a module, check it, and move
  on. Here "ok" meant "not yet examined", and the errors arrived later,
  attributed to the module that happened to import it first.

### F10: a pub type cannot be built outside its module
- kind: confusing-semantics
- severity: minor
- where: mustache/render.kso:53, :83 (`answered`, `lambda_of`)
- wanted: the spec runner builds a lambda for the engine:
  ```
  mustache/lambda (s calls -> mustache/answer calls (f s))
  ```
- wrote: two pub constructor functions in the engine:
  ```
  pub fn answered state s
    answer state s

  pub fn lambda_of f
    lambda f
  ```
  The refusal: `error[opacity]: mustache/lambda is foreign — only mustache
  builds a lambda; ask it for one through a pub function`.
- why it matters: The message is clear and the rule is defensible, but
  chapter 07 lists `pub type` beside `pub fn` without saying that a pub type
  exports its name and not its constructor. A type that is only a bag of two
  fields now needs a function whose body is the constructor call.

### F11: a string passed where a list of names was meant ran without complaint
- kind: refactoring-hazard
- severity: minor
- where: mustache/parse.kso:176
- wanted: the checker to notice that `lookup ctx name` received a string,
  when every other caller passes a list of path parts.
- wrote: the fix, `inverted_node nodes (path t.content)` instead of
  `inverted_node nodes t.content`. Before it, `lookup` ran on the string:
  `length "boolean"` is 7 and `"boolean"[1]` is `"b"`, so it looked up the
  key `b`. My first hand test used a key named `b` and passed. The spec
  fixtures caught it.
- why it matters: Strings and lists answer the same `length` and `[i]`, and a
  record field holds whatever the program puts in it, so a path that is
  sometimes a list and sometimes a string type-checks. A field annotation
  would have caught it, and the book says not to write one.

### F12: the build names the binary after the directory, and there is no flag to change it
- kind: tooling
- severity: minor
- where: check.sh
- wanted: `kanso build . -o build/mustache`
- wrote: `cd build/dev && kanso build ../..`. From the project root the build
  refuses: `this build is named mustache, and a directory of that name is
  here`. The directory is the engine module, which is named for the project
  for the same reason the project is.
- why it matters: A project whose main module shares the project's name is
  the common layout. The workaround is easy once known, and the message says
  what to do, but there is no way to name the output.

### F13: every input of a three-file command needs its own function
- kind: confusing-semantics
- severity: minor
- where: cli/cli.kso:116 (`read_template`), and `read_data`, `decoded_data`, `json_partials`, `checked_partials`
- wanted: one function that takes the three things read from disk and
  reports whichever one is missing or malformed:
  ```
  fn rendered (file_not_found f) _ _
    fail "no such file: {f}"

  fn rendered _ (file_not_found f) _
    fail "no such file: {f}"

  fn rendered _ _ (file_not_found f)
    fail "no such file: {f}"

  fn rendered template data partials
    output (mustache/render template (json/decode data) (json/decode partials))
  ```
- wrote: a chain of one-step functions, each claiming one failure in one
  position and passing the rest on: `read_template` (template missing),
  `read_data` (data missing), `decoded_data` (data not JSON), and for the
  partials `json_partials` (missing) and `checked_partials` (not JSON), with
  `output` claiming a render error at the end. The three arms above tie, as
  in F8, and `json/decode` called inline is refused because
  `mustache/render` has no err arm.
- why it matters: The design of each piece is sound: every failure is
  claimed where it can arrive. But a command line that reads three inputs is
  the simplest real program there is, and it turned into a chain of six
  one-step functions whose names exist only to carry the next argument.

### F14: interpolating into an accumulator is quadratic, and far worse natively
- kind: performance
- severity: major
- where: mustache/render.kso:56 (`emit`)
- wanted:
  ```
  fn emit acc s
    sink acc.state "{acc.written}{s}"
  ```
- wrote:
  ```
  fn emit acc s
    sink acc.state (text/append acc.written s)
  ```
  with the output held as bytes and turned into a string once, at the end.
  Rendering a 10,000-item list took 28.8 s with the release binary (26 s of
  it system time) and 3.9 s on the interpreter. With `text/append` it takes
  0.034 s. Reproduction: `bugs/interpolation_accumulator_slow_native.kso`,
  where 40,000 pieces take 21.6 s natively and 1.9 s on the interpreter.
- why it matters: String interpolation is the first way the book teaches to
  build a string, and a fold over it is the obvious renderer. The native
  engine was ten times slower than the interpreter on it, so the cost
  appeared only after building, and only with large input. `text/append` is
  mentioned in chapter 08 as a builder, but nothing points a reader from
  interpolation to it.

### F15: adding a field to a record means editing every construction and every positional pattern
- kind: refactoring-hazard
- severity: minor
- where: mustache/render.kso:45 (`type sink`) and its seven construction sites
- wanted: to add a partial cache to the accumulator the render threads, as
  one new field, touching only the code that reads or writes the cache.
  Something like `acc with cache = put acc.cache key nodes`.
- wrote: `type sink` gained `cache` as its first field (fields are
  alphabetical, so a new field lands wherever its name sorts), and every
  `sink state written` construction became `sink acc.cache state written`,
  and so did the positional pattern `(sink state written)` in
  `interpolate_rendered`. Of the six constructions, one writes the cache and
  the other five only carry it along.
- why it matters: Records are built positionally and there is no update
  syntax, so a threaded state record is the most expensive kind of record to
  grow, and threading state through a record is what a pure renderer needs
  for lambda state and caches alike. The checker caught every site I missed,
  which is the good half of this.

### F16: a function named like a std type: native calls the function, the interpreter calls the type
- kind: engine-bug
- severity: major
- where: mustache/mustache_test.kso:8 (`count_up`, first written `counting`)
- wanted:
  ```
  fn counting _ calls
    answered (calls + 1) "{calls + 1}"

  test_counter_lambda_threads_state =
    data = { "n":(lambda_of counting) }
  ```
- wrote: renamed it `count_up`. The test failed under `kanso test` with
  `error[runtime]: counting has 1 field(s), got 2`: std/list exports
  `pub type counting` (the state behind `list/naturals`), and the
  interpreter took my function value to be that constructor. The reduced
  case, `bugs/function_named_like_std_type.kso`, prints 2 natively and fails
  on the interpreter.
- why it matters: This is F1 again, but worse: no compile error, and the two
  engines that are supposed to agree byte for byte give different answers.
  A program can pass every native test and fail on the oracle, or the other
  way around.

### F17: one none arm satisfies the check, and the catch-all quietly takes the other none cases
- kind: confusing-semantics
- severity: minor
- where: mustache/render.kso:86 (`missed?`)
- wanted: to know that every way a none can arrive is handled, which is how
  the `error[exhaustive]: this can be a none and X has no arm for it`
  message reads.
- wrote:
  ```
  fn missed? none (settings _ true)
    true

  fn missed? _ _
    false
  ```
  This is what I meant, and it works: a none with `strict` false falls to
  `missed? _ _`. But it shows the check is weaker than it sounds. Drop the
  first arm and the program is refused; keep it and the catch-all receives
  none in every case the first arm does not cover. A one-file test:
  ```
  fn pick none true
    "none, strict"

  fn pick v _
    "caught {v}"
  ```
  `pick m["zz"] false` answers `caught <none>`.
- why it matters: The rule reads as "absence is always handled on purpose".
  What it enforces is "somebody mentioned none once in this position". The
  `_` that appears to mean "any value" also means "none, sometimes".

### F18: `x = not (f y)` before a return guard is refused as misplaced
- kind: engine-bug
- severity: minor
- where: cli/cli.kso:66 (`unknown_flag?`), :77 (`main`)
- wanted:
  ```
  pub fn main argv
    flags = list/to_list (list/select argv flag?)
    args = list/to_list (list/reject argv flag?)
    stray = list/find flags (f -> not (known_flag? f))
    raw = not (has? flags "--no-escape")
    return usage if length args < 1
    return fail "unknown option {stray}" if stray != none
    command args[1] args (mustache/settings_of raw (has? flags "--strict"))
  ```
- wrote: the negation moved into its own predicate, `fn unknown_flag? f`
  with body `not (f == "--no-escape" or f == "--strict")`, and `(not raw)`
  moved into the final call. The refusal was `error[formatting]: a return
  sits with the bindings, before the effect chain`, pointing at a return
  that does sit with the bindings. Bisecting showed that a binding of the
  form `not (call ...)` is what triggers it, in a function that answers an
  effect; `not name` is fine. Reproduction:
  `bugs/not_call_binding_before_return.kso`.
- why it matters: The message describes a rule the code already follows,
  so the only way forward was to bisect the function line by line. It took
  six variants to find.

### F19: the book's `pub play =` samples do not run under `kanso play`, and `kanso run` refuses single files with definitions
- kind: tooling
- severity: minor
- where: scratch experiments; the port's own entry point is `main.kso`
- wanted: to try a one-file experiment the way the book writes one:
  ```
  fn go args
    ...

  pub play = go ["a"]
  ```
- wrote: bare statements and `kanso play file.kso`. `kanso run file.kso` on
  a file with a type, a binding and a print says `file.kso is a library —
  nothing to run`; `kanso play` with `pub play =` says `pub play is a
  library's export — import this module from an entry file`; and a
  `main.kso` that defines a function gets `an entry file holds statements
  only`. Each message is clear on its own. Together they make three file
  shapes with three verbs, and the book's samples are written in the shape
  none of the verbs accepts for a single file.
- why it matters: Small experiments are how I learned every rule in this
  journal, and the first minute of each was spent getting the file shape
  right.

### F20: changing how many arguments a callback takes is not checked until it runs
- kind: refactoring-hazard
- severity: major
- where: mustache/render.kso:147, :289 (the lambda arms of `interpolate` and `section`), spec/lambdas.kso, mustache/mustache_test.kso:8
- wanted: when a lambda went from `call text state` to `call text state
  expand`, for the checker to list the five places that build a two-argument
  function and store it in a `lambda`.
- wrote: the change to the call sites, then `kanso check .` said
  `./main.kso: ok`. The mismatches surfaced at run time, one at a time:
  ```
  error[runtime]: this function takes 2 argument(s), got 3
  error[runtime]: no overload of `count_up` matches these arguments
  ```
  I found them all only because the spec fixtures call every lambda.
- why it matters: The checker infers the type of nearly everything else and
  catches a wrong field count in a pattern at once (`var_node has 3 fields
  and this arm takes 2, so it can never match`). A function stored in a
  record field is where that inference stops, and callbacks are exactly the
  values whose shape changes during a refactor.

### F21: std/text has no case conversion
- kind: stdlib-gap
- severity: minor
- where: spec/lambdas.kso:64 (`upper`, `upper_char`)
- wanted: `text/upper s`
- wrote:
  ```
  fn upper s
    text/join (list/to_list (list/map (text/chars s) upper_char)) ""

  fn upper_char c
    code = text/char_code c
    return c if code < 97 or 122 < code
    text/from_code (code - 32)
  ```
  which handles ASCII only.
- why it matters: Upper- and lower-casing are among the first helpers a
  template user asks for, and the lambda that demonstrates the render helper
  is the textbook one. The hand-written version is wrong for every
  non-ASCII letter, which is the part a library is for.

### F22: alphabetical fields put `close` before `open`
- kind: confusing-semantics
- severity: minor
- where: mustache/parse.kso:222 (`default_delims`), :86 (`act 61`)
- wanted: `delims "{{" "}}"`, open then close, the order every mustache
  document and every reader uses.
- wrote:
  ```
  default_delims = delims "}}" "\{\{"
  next = scanner (text/bytes words[2]) line (text/bytes words[1]) s.resume
  ```
  Fields are alphabetical and construction is positional, so `close` comes
  first, and the `{{=<% %>=}}` handler passes the second word before the
  first.
- why it matters: Both fields are strings, so swapping them type-checks and
  runs, and the mistake would show only as a template that never finds its
  tags. Alphabetical order is a fine rule for a reader scanning a type
  declaration. At a construction site it fixes the argument order by
  spelling, which is unrelated to what the arguments mean. The keyed read
  (`{ open close } = d`) exists for taking records apart; nothing like it
  exists for building one.

### F23: a set of ten bytes is thirty-three lines of arms
- kind: aesthetics
- severity: nit
- where: mustache/parse.kso:301 (`sigil?`)
- wanted: `fn sigil? c` answering whether `c` is one of `! # $ & / < = > ^ {`,
  in one line.
- wrote: ten arms of the form
  ```
  fn sigil? 33
    true
  ```
  and a catch-all answering false. `list/any? [33 35 36 ...] (b -> b == c)`
  would also work, but the book teaches literal arms as the idiom for this,
  and the JSON library does the same.
- why it matters: It is aesthetic. Each arm is clear, but membership in a
  fixed set is one fact, and it takes a screen to state. A byte literal
  syntax would also help: every arm here needs a comment to say which
  character the number is.

## What worked well

**Dispatch on the next byte made the scanner short.** The parser has no
token type and no switch. Each tag goes to an `act` arm chosen by its sigil
byte, and the standalone-line rules are two small tables read in opposite
directions:
```
fn ahead _ p none
  p

fn ahead _ p 10
  p + 1

fn ahead cs p 13
  if (cs[p + 1] == 10) (p + 2) none

fn ahead cs p 32
  line_end cs (p + 1)
```
Reading past the end of the template answers `none`, so "the template ends
here" is one more arm, and a tag on the last line with no newline after it is
standalone without a special case. The spec's dozen "Standalone Without
Newline" and "Standalone Line Endings" tests passed the first time they ran.

**Errors travel without plumbing.** A parse error is raised several calls
deep in a recursive descent (`fail cs t.start "..."`) and arrives at the command
line's `output name (err r)` arm with its line and column intact. Strict
mode's `context_miss` is raised inside a `list/fold` over the nodes, and the
fold carries it out: each later step is handed the err and returns it
unread. I wrote the arms the checker asked for and nothing else.

**Immutable data removed the classic renderer bug.** A section pushes onto
the context stack with `push ctx x` and renders its children with the new
list. There is nothing to pop afterwards, so a section that fails or
iterates cannot leave the stack wrong for its siblings. An engine with a
mutable stack has to guard against exactly that.

**Structural equality makes tests short.** Parse trees and error records
compare with `==`:
```
test_parse_keeps_text_and_tags =
  nodes = parse "Hi \{\{&who}}"
  nodes == [(text_node "Hi ") (var_node false 1 ["who"])]

test_strict_names_the_miss =
  ...
  testing/when_failed got (r -> r == context_miss 2 "y")
```

**JSON data and kanso functions live in the same map.** `std/json` decoded
every spec suite, including the unicode fixtures, and a lambda is just
another value in the decoded map. Dispatch then reads like the spec's own
description of a section. These are the arm heads:
```
fn section none _ _ _ _ _ acc            # a miss renders nothing
fn section false _ _ _ _ _ acc
fn section _:json/json_null _ _ _ _ _ acc
fn section (lambda call) _ d raw ctx e acc
fn section xs:[]some children _ _ ctx e acc
fn section v children _ _ ctx e acc      # any other value is a context
```

**Absence is data at the edges.** `os/read_file` answers `file_not_found`
rather than failing, so every missing input gets its own sentence in the
command line, and an optional argument is an index that may be `none`:
```
fn partials_source template none
  partials_from (path/dirname template)

fn partials_source _ given
  partials_from given
```

**The engines agree.** Apart from the bugs in `bugs/`, the interpreter, the
dev build and the release build produce the same bytes, the same stderr and
the same exit status on all 35 cases in `check.sh`, including rendering
errors.

**Many diagnostics told me the fix.** Three examples, verbatim:
`text_node takes 1 argument(s), and a list element is one atom — this reads
as 5 separate elements. Write (text_node …) to call it`; `var_node has 3
fields and this arm takes 2, so it can never match`; `comparing to false
asks a question the value already answers — write not the value`. When I
added line numbers to three node types, the second of these found every
stale pattern in one run.

**Canonical form left the important choices to me.** Declarations need only
types first, so helpers sit next to the function they serve. Nothing in the
formatting rules ever needed a decision, and the code reads the same in
every file.

## Summary

The five I would fix first, in order:

1. **F1, with F16 as its worst case.** Every pub name in the standard library
   behaves as if it were declared in every module. In F1 it produced errors
   about functions the file never mentions. In F16 it made the interpreter
   and native code run different functions for the same name, with no
   diagnostic. A name should come from an import or from the module.
2. **F7.** The interpreter's `text/concat` rejects byte lists, so the
   oracle and `kanso test` fail on code the checker accepts and native code
   runs. Byte scanning is how the book teaches parsing, and this sits
   directly in its path.
3. **F14.** Accumulating a string by interpolation is quadratic, and native
   code is ten times slower than the interpreter at it. A template engine,
   a pretty-printer and a report generator all hit this on their first
   large input.
4. **F20.** The arity of a function stored in a record is not checked. The
   checker is otherwise thorough about shapes, and callbacks are the values
   whose shape changes in a refactor.
5. **F9.** `kanso check` on a library can say ok and then report errors in
   that library when another module imports it. It weakens the plain
   promise of `check`.

Writing a mustache engine in kanso went better than I expected. The parser
and renderer together are about 680 lines, comments included. Comments,
delimiters and interpolation passed in full the first time they ran. The
other six suites showed six failures on their first run: four in inverted
and one in lambdas, from two bugs of mine, and one dynamic-names test whose
expected output I had guessed wrong. The language's central ideas fit
the problem. A template is a tree you dispatch on, missing names are `none`,
broken templates are errs that reach the user with a line number, and
nothing mutates, so the context stack cannot be corrupted. The friction
came mostly from the edges: standard-library names leaking into every
module, two engine disagreements, a performance cliff in the most obvious
way to build a string, and a checker whose none and err rules sometimes
demanded arms that exist only to satisfy it (F8, F13, F17). Purity forced
the state-threading design for lambdas, which is more explicit than
mutation and turned out to be the mechanism the partial cache needed too.
It also meant that adding a field or an argument to that threaded state
touched every construction site, and in the callback case nothing caught the
misses until run time.
