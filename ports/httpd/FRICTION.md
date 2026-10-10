# FRICTION: httpd (todo-backend on a small HTTP/1.1 server)

The journal for the httpd port. Entries were written as the problems came up;
the code quoted in each one was compiled with the toolchain at
`/tmp/claude-0/kanso-main/kanso`.

## Entries

### F1: a file with both definitions and statements will not `run`
- kind: confusing-semantics
- severity: minor
- where: first experiment, before `main.kso` existed
- wanted: the shape every book sample uses, one file with a helper and a
  statement:
  ```
  fn answer c raw
    net/write c "HTTP/1.1 200 OK\r\n..."
  net/listen 0 .> (l -> ...)
  ```
  run with `kanso run a.kso`.
- wrote: a module directory holding the definitions and a `main.kso` that holds
  only statements and an `import "./srv"`.
- why it matters: the error (`a.kso is a library — nothing to run ... run its
  definitions beside their statements with kanso play`) is clear once read,
  but the book's panels say `kanso run` above samples that this rule refuses,
  so a program copied out of chapter 03 (`membership.kso`, for one) does not
  run the way the page says it does.

### F2: `net/accept` outside a parallel group fails at once with "nothing connected"
- kind: engine-bug
- severity: major
- where: `http/server.kso` (`listening`), `bugs/accept_outside_group/`
- wanted: the smallest server there is, one accept in the entry's chain:
  ```
  net/listen 48213 .> solo/serve      # serve = net/accept l .> ...
  ```
- wrote: the accept loop as one member of a two-line group, the other member
  a print that does nothing useful:
  ```
  pub fn listening srv l state banner
    { log } = srv
    log banner
    serve srv l state
  ```
- why it matters: both engines treat an accept that runs alone as one that can
  never succeed, because they assume the only thing that could connect is
  another fiber of the same program. A real server is connected to from
  outside the process, so the first standalone server anyone writes exits
  with `unhandled err reached the executor: "nothing connected"` before a
  client has had a chance. The comment in `runtime.c` states the assumption.

### F3: a bound proves an index only when it is spelled `i < 1 or length xs < i`
- kind: diagnostic
- severity: major
- where: `http/bytes.kso:hex_from`, and every byte loop after it
- wanted: the guard every loop over a 1-based list naturally starts with:
  ```
  fn hex_from ds i acc
    return acc if i > length ds
    hex_step ds i acc (hex_digit ds[i])
  ```
- wrote:
  ```
    return acc if i < 1 or length ds < i
  ```
- why it matters: `i > length ds` and `length ds < i` are both refused with
  "this can be a none and `hex_digit` has no arm for it", and the error does
  not say that a differently worded guard would be accepted. I found the
  accepted spelling by reading a comment in `lib/json`. Meanwhile
  `acc + xs[i]` under the same weak guard is accepted, so `+` and a call are
  held to different rules despite chapter 04 saying an operator is held to
  the same one.

### F4: `xs[1]!` is an effect, not an assertion
- kind: confusing-semantics
- severity: major
- where: `http/request.kso:head_lines` (first draft)
- wanted: "the first line is there; fail loudly if it is not":
  ```
  request_line bs lim at lines (text/split lines[1]! " ")
  ```
- wrote: an arm for the miss, in a helper whose only job is that arm:
  ```
  fn words none
    []

  fn words line
    split_on line " "
  ```
- why it matters: coming from Rust or Swift, `!` reads as "unwrap or crash".
  Here it turns a pure lookup into a box that only `.>` can open, so
  using it inside pure parsing code produced a type error three calls away,
  in `std/text`: "`s` holds an effect — `chunk_size` hands `text/trim`
  one". In pure code there is no way to say "this index is in range, trust
  me"; the choices are a none arm or a length guard.

### F5: `text/split` with a separator held in a variable is typed as fallible
- kind: stdlib-gap
- severity: minor
- where: `http/bytes.kso:split_on`
- wanted: `parts = text/split s sep` inside a helper whose callers pass
  `":"`, `"="` and `"?"`.
- wrote: a wrapper that puts the err down so the rest of the module does not
  have to:
  ```
  pub fn split_on s sep
    listed (text/split s sep)

  fn listed (err _)
    []

  fn listed parts
    parts
  ```
- why it matters: `split` answers an err for an empty separator, so once the
  separator is a parameter, every function downstream of the helper is asked
  for an `(err _)` arm. The checker said
  "this can be an err and `header_field` has no arm for it" at a call to
  `split_once`, which is two functions away from the cause.

### F6: `text/bytes` answers a hidden "bytes" kind that `[]`, `text/concat` and `text/append` each treat differently
- kind: engine-bug
- severity: major
- where: `http/request.kso:chunk_sized`, `bugs/concat_of_bytes.kso`
- wanted: to accumulate a chunked body by joining byte slices, starting
  from an empty list:
  ```
  chunks bs (at + 4) [] max_body
  ...
  chunks bs (past + 2) (text/concat acc data) max
  ```
- wrote: an accumulator that starts as the bytes of an empty string, grown
  with `text/append`:
  ```
  chunk_from bs (at + 4) (text/bytes "") max_body
  ...
  chunk_from bs (past + 2) (text/append acc data) max
  ```
- why it matters: appendix b documents `text/bytes` as answering `int[]`,
  and the value prints as a list and indexes as one. But on the interpreter
  `text/concat [1] (text/bytes "ab")` stops with "concat takes two lists",
  while the native engines print `[1 97 98]`, so the engines disagree.
  `text/append [] (text/bytes "cd")` fails on every engine ("append takes
  bytes and a string, bytes, or byte"), so the empty list literal is the
  wrong way to start a byte accumulator, and nothing in the language says
  what the right way is. I learned that `text/bytes ""` is the empty bytes
  value by reading `lib/json`.

### F7: one namespace per module, plus every imported type's bare name, means a new name in one file breaks another
- kind: refactoring-hazard
- severity: major
- where: `http/response.kso`, `http/server.kso`, `http/request.kso`
- wanted: to name a helper `framing` in `response.kso`, a type `chunks`, a
  helper `after` in `server.kso`, and a function `step` that dispatches on
  the parser's answer.
- wrote: `wire_for`, `chunk_from`, `onward`, `on_parse`. Each rename was
  forced by an error in a different file from the one I had just edited:
  ```
  error[name]: `framing` is already a declaration; rename the binding
    --> http/request.kso:168:3
  error[name]: `after` is already a declaration; rename the binding
    --> http/request.kso:145:34
  error[arity]: `step` has 2 field(s), got 5 (construction is positional...)
    --> http/server.kso:63:3
  ```
- why it matters: a local binding in `request.kso` called `after` was fine
  until a function called `after` appeared in `server.kso`. The last error
  is the worst of the four: `step` is a type that `std/list` exports, and
  importing `std/list` puts its bare name into my module, so my own
  function `step` was read as a constructor call. The message never says
  `std/list`. `std/list` alone puts fourteen such type names in scope (`step`,
  `cursor`, `sorted`, `mapped`, `paired`, `counting`...), all of them
  ordinary words a program would want to use.

  Test files are part of the module's namespace too. Adding a constant
  called `pieces` to `http/response_test.kso` broke four functions in
  `http/response.kso` that had a parameter of that name. Adding
  `srv = server_with ...` to `http/server_test.kso` produced fourteen
  errors in `http/server.kso`, one per function that takes a server, and a
  helper called `asked` in the same test file added a fifteenth. Test
  helpers called `wire` and `closing` did the same thing on a smaller
  scale. A test that names a
  fixture the obvious way breaks production code it does not touch, and
  renaming it then made five assertion lines too wide, so the rename also
  meant rewriting the file.

### F8: a record field cannot be destructured under its own name when a type has that name
- kind: aesthetics
- severity: minor
- where: `http/server.kso:converse`, `answer`, `keep_alive?`
- wanted: `{ limits } = srv`, `fn answer ... (turn response state)`
- wrote: `{ limits:lim } = srv` and `(turn reply state)`
- why it matters: a field holding a `response` is naturally called
  `response`, but the no-shadowing rule treats the type name as already
  taken, so every keyed read of such a field needs a rename. The same rule
  bit on `values` (an ambient builtin) as a local name in `length_of`.

### F9: a long single-expression constant has to be broken with a binding nobody needs
- kind: aesthetics
- severity: minor
- where: `http/request_test.kso`, throughout
- wanted: a test too long for one line, wrapped:
  ```
  test_unsupported_version =
    outcome "GET / HTTP/2.0\r\n\r\n" == "505 unsupported version"
  ```
- wrote:
  ```
  test_unsupported_version =
    got = outcome "GET / HTTP/2.0\r\n\r\n"
    got == "505 unsupported version"
  ```
- why it matters: the first form is refused ("a single-expression constant
  is written inline") and the inline form is refused for width, so the
  only legal spelling adds a name. There is no continuation for `==`, only
  for `.` and the chain words. A test file of forty one-line assertions
  ends up a third `got =` lines.

### F10: positional construction in alphabetical field order reads backwards
- kind: aesthetics
- severity: minor
- where: `http/bytes.kso:split_once`
- wanted: a pair named for what it is, built in reading order:
  `cut before after`
- wrote: `cut rest parts[1]`, because the fields sort as `after before`.
- why it matters: the type is declared `type cut / after / before`, so the
  constructor takes the second half first. I wrote it the wrong way round
  the first time and only a test caught it; the checker cannot, since both
  fields are strings. The `request` record has seven fields and is built
  twice, and each time I had to count against the declaration.

### F11: formatting errors arrive one per run for some rules
- kind: tooling
- severity: nit
- where: `http/request.kso`, four `not (...)` guards
- wanted: one `kanso check` that lists the four redundant parentheses.
- wrote: four check runs, each reporting the next one up the file.
- why it matters: width errors are reported all at once, so the
  one-at-a-time behaviour of "these parentheses group nothing" looks like
  a fix that did not take.

### F12: a list literal too long for one line cannot be written at all
- kind: missing-feature
- severity: major
- where: `router/router_test.kso` (the route table), `app/app.kso` (routes)
- wanted:
  ```
  table = [
    (on "GET" "/todos" "list")
    (on "POST" "/todos" "create")
    ...
  ]
  ```
- wrote: a name per element, and the list of names:
  ```
  table = [listing creating showing editing files root]

  listing = on "GET" "/todos" "list"
  ```
- why it matters: both wrapped forms I tried are syntax errors once the
  content is long (`expected an expression`, `expected ')'`). When the
  content is short the same layout is refused as "needless continuation:
  this statement fits on one line", which suggests a wrapped form exists. A
  route table is a list of data. Being made to name each row is tolerable
  for six routes; `lib/sha256` shows what it costs for sixty-four constants
  (`k0` to `k10` joined with `text/concat`).

  A string literal cannot be wrapped either. The shell script the test
  client runs (`client/wire.kso`) is 160 characters, so it is assembled
  from four constants interpolated into each other, and the script is now
  harder to read than any other line in the port.

### F13: a keyed read of every field passes `kanso check` and fails at run time, reported against the wrong module
- kind: diagnostic
- severity: major
- where: `http/response.kso:with_header`, `http/server.kso:received`,
  `todo/todo.kso:as_json`
- wanted: `{ body headers status } = r` for a three-field record.
- wrote: the positional form, `response body headers status = r`.
- why it matters: `kanso check http` said `ok`. The tests then failed with
  ```
  error[runtime]: a keyed read omits at least one field; reading every
  field is the positional form
    --> todo:26:29
  ```
  The read is at `http/response.kso:26`; the report names module `todo`
  (the caller) and no file, so line 26 of `todo/todo.kso`, which holds a
  comment, was the first place I looked. The same rule applied to
  `{ items next_id } = st` produced a compile-time error, but one that said
  `unknown name next_id`, which describes the symptom rather than the rule.
  Records grow fields over time, so a keyed read that omitted one yesterday
  becomes a run-time failure when a field is removed.

### F14: a module that checks clean can fail to check once something imports it
- kind: confusing-semantics
- severity: major
- where: `todo/todo.kso:edited`
- wanted: a todo with no order to hold `none` in its `order` field and pass
  through `kept_order order t.order`, which `kanso check todo` and
  `kanso test todo` both accepted.
- wrote: a marker type `unordered`, and `fresh_order` so that a literal
  `none` is never an argument:
  ```
  fn fresh_order unset
    unordered
  ```
- why it matters: the error appeared only when an entry file imported the
  module: `this can be a none and todo/kept_order has no arm for it`. The
  checker proves exhaustiveness against the call sites it can see, so a
  library's verdict depends on who imports it, and its own test suite is not
  evidence that it compiles for a caller. The same thing happens to test
  files: `kanso check main.kso`, `run` and `build` all type-check every
  imported module's `*_test.kso` against the whole program. A test helper in
  `static/static_test.kso` that `kanso test static` accepted stopped the
  build with `this can be a none and list/fold has no arm for it ...
  (module cli)`, reported against `list/fold` (a std function I never
  called, reached through `list/map`) and against module `cli`, which is
  neither the file nor the module the line is in. Separately, a literal `none`
  passed as a default (`default order none`) needs a none arm in the
  callee even when that parameter is passed through untouched, so none
  cannot serve as an ordinary value.

### F15: a foreign `pub type` can be dispatched on and read, but not built or taken apart
- kind: confusing-semantics
- severity: minor
- where: `http/response.kso:field`, `http/server.kso:carry`,
  `app/app.kso:routed`
- wanted: in `app`, to match the router's answer by its shape, and in
  `static`, to build a header:
  ```
  fn routed req a (router/matched handler params)
    handler req params a

  http/respond 304 [(http/header "etag" tag)] ""
  ```
- wrote: an ascription and dot reads for the first, and a constructor
  function exported from `http` for the second:
  ```
  fn routed req a m:router/matched
    h = m.handler
    h req m.params a

  pub fn field name value
    header name value
  ```
- why it matters: the three operations (build, destructure, dot-read) on
  the same exported type get three different answers across a module
  boundary: refused (`only http builds a header`), refused (`its structure
  does not cross an import`), and allowed. The rule protects a module's
  invariants, which is reasonable, but `pub type` then means something
  narrower than it appears to, and every module ends up with a set of
  one-line wrappers (`field`, `chunked`, `carry`, `make_request`) whose only
  purpose is to be the constructor. The wrapper cannot have the type's
  name either, because that name is taken (F8).

### F16: `std/net` can listen but cannot connect
- kind: stdlib-gap
- severity: major
- where: `client/wire.kso:exchange`
- wanted: `net/dial "127.0.0.1" port .> (c -> net/write c request ...)`,
  so the test client is kanso end to end.
- wrote: the client builds its request bytes and parses the response in
  kanso, but the socket in between belongs to a shell pipeline:
  ```
  script = "{writer} | timeout 10 nc -N 127.0.0.1 \"$port\" | od -An -v -tx1"
  ...
  os/run "bash" argv .> (p -> unhexed p.stdout)
  ```
  The response comes back hex-encoded through `od`, because `os/run`
  returns stdout as a string and a binary body would not survive that.
- why it matters: the brief for this port asks for a test client that
  drives the server, and the language has no client socket. There is also
  no half-close (`shutdown(SHUT_WR)`), no read timeout and no way to read
  a fixed number of bytes, which a client needs for keep-alive. The
  workaround depends on OpenBSD netcat's `-N` flag.

### F17: a type error in a comparison is found only at run time, and reported with no location
- kind: diagnostic
- severity: major
- where: `cli/cli.kso:option_step`, `bugs/compare_without_location.kso`
- wanted: to thread an options record through an argument loop with a pipe:
  ```
  fn option_step args i opts "--quiet" _
    options opts.port true opts.root opts.words . read_options args (i + 1)
  ```
  The pipe puts the record first, so this is `read_options record args
  (i + 1)`, and inside it `i < 1` compares a list with an int.
- wrote: the plain call, `read_options args (i + 1) (options ...)`.
- why it matters: `kanso check main.kso` passed. Every run of the program
  then stopped with
  `error[runtime]: comparison requires two values of one comparable type`
  with no file, line, birthplace or trace, on both engines. I found it by
  bisecting the program with scratch entry files. Along the way I guessed
  wrong: I took `p == pause` (a string compared with a marker) to be the
  cause and rewrote it as a two-arm predicate, and only later checked that
  `==` across types answers `false`. A report with a
  line number would have saved both detours. The pipe's argument order (the
  piped value goes first, whatever the function's first parameter is for)
  is easy to get wrong in a function written to be called, not piped into.

### F18: debugging output has to pass the formatter too
- kind: tooling
- severity: minor
- where: `http/server.kso` (while finding the `shift` bug in the transport)
- wanted: a one-line trace dropped into a chain:
  ```
  net/read c .? gone .> (chunk -> print "DEBUG read [{chunk}]" .> (_ -> received srv c state ses chunk))
  ```
- wrote: a helper function for the print, because the line was over 80
  columns and the program would not run:
  ```
  fn dbg_read srv c state ses chunk
    print "DEBUG read [{chunk}]" .> (_ -> received srv c state ses chunk)
  ```
- why it matters: there is no debugger and no trace flag, so a print is the
  tool. Every temporary print has to meet the width, ordering and
  unused-binding rules before the program will run, and a scratch helper
  added for one print sits in the middle of a module. For effect chains,
  `--plan` stops at the first `<continuation>`, so it cannot show what a
  server does after it reads.

### F19: a read on one connection stops every fiber in the program
- kind: engine-bug
- severity: blocker
- where: `http/server.kso:serve`, `bugs/blocking_read/`
- wanted: the shape chapter 06 teaches for concurrent work, one fiber per
  connection:
  ```
  fn both l n c
    answering c
    accepting l (n - 1)
  ```
- wrote: a server that finishes one connection before it accepts the next.
- why it matters: `net/read` is a blocking system call on both engines. The
  scheduler parks a fiber on `sleep`, `accept` and `os/run`, but not on a
  read, so while one fiber waits for a slow client nothing else runs. The
  reproduction has two clients: one that connects and waits two seconds
  before sending, and one that connects at half a second and sends at
  once. Both are answered at the two-second mark. For a server, this means
  one idle keep-alive connection, or one slow client, stops all service.
  There is also no read timeout with which to drop such a client.

### F20: fibers cannot share state, so a concurrent server could not share its store anyway
- kind: missing-feature
- severity: major
- where: `http/server.kso:accepted`, `client/drive.kso:drive`
- wanted: a store that every connection's fiber reads and updates, or a
  channel to send updates through.
- wrote: the store as the value a sequential accept loop folds, connection
  by connection:
  ```
  fn accepted srv l state c
    fresh = session (text/bytes "") false 0
    converse srv c state fresh .> (s -> serve srv l s)
  ```
  To stop the in-process server at the end of a test, the client fiber is
  handed the server's listener and closes it, which makes the server's
  next `accept` fail; the server rescues that failure and ends its loop.
- why it matters: even if F19 were fixed, the group in F19's "wanted" has
  no way to pass the store from `answering c` to `accepting l (n - 1)`,
  because a group member's value is discarded at its own line. The model
  in chapter 06 (data crosses only at bindings and `.>`) covers
  fork-and-join work well, but a server is a long-lived loop that has to
  share state across connections, and neither the book nor `lib/` offers a
  construct for that. Chapter 06 says there is no channel because "the data
  flow is the channel", which holds for a join and not for a loop that
  never ends. The workaround for stopping
  the server (closing a socket out from under it) is the only way one fiber
  could signal another.

### F21: a native program's stdout is lost when it is killed
- kind: engine-bug
- severity: major
- where: `cli/cli.kso:logger`, `bugs/stdout_lost_on_kill/`
- wanted: the access log on stdout with `print`, the way a server usually
  logs, and the way the e2e mode prints its transcript.
- wrote: the log on stderr:
  ```
  fn logger false
    (line -> io/write_err "{line}\n")
  ```
- why it matters: a native binary block-buffers stdout when it is not a
  terminal and nothing in kanso flushes it. A server is stopped by being
  killed, so everything it printed while redirected to a file or a pipe
  disappears. The interpreter writes each line at once, so the two engines
  disagree on what a killed program printed. `check.sh` caught this on the
  standalone-server check: the interpreter's transcript had four log lines
  and both native ones had none.

### F22: `text/slice` past the end answers an empty result
- kind: confusing-semantics
- severity: minor
- where: `client/client_test.kso:test_scenario_chunked`
- wanted: `text/slice whole 98 200`, meaning "from character 98 to the
  end", which Python writes `whole[97:]` and Rust `&whole[97..]`.
- wrote: `text/slice whole 98 (length whole)`
- why it matters: an end beyond the length answers `""`, not the tail.
  Appendix B does say so ("out-of-range or inverted bounds yield an empty
  result"), but an out-of-range end looks like a clamped range and reads as
  "nothing there", so the test failed with `returned false` and no further
  clue.

### F23: a failing test says only `returned false`
- kind: tooling
- severity: minor
- where: every `*_test.kso`; `todo/todo_test.kso:test_not_json` was the
  first case where it cost real time
- wanted: the two sides of the `==` that failed, the way pytest, Go's
  `cmp.Diff`, or Rust's `assert_eq!` show them.
- wrote: a scratch entry file, in a copy of the port, that imports the
  module and prints the value, because a test file cannot print.
- why it matters: chapter 07 presents the bare boolean as the whole testing
  story, which works until the first failure. For this port, which compares
  long strings of HTTP and JSON, every failure meant writing a throwaway
  program to see what the value was.

### F24: no list patterns, so a command line is taken apart by index
- kind: missing-feature
- severity: minor
- where: `cli/cli.kso:dispatched`
- wanted:
  ```
  fn dispatched ["serve"] opts
    serving opts

  fn dispatched ["e2e" file] opts
    scenario file opts
  ```
- wrote: a dispatch on the first word, a length guard in each arm, and a
  none arm for an empty command line:
  ```
  fn dispatched words opts
    command words[1] words opts

  fn command none _ _
    wrong_usage

  fn command "e2e" words opts
    return wrong_usage if length words != 2
    scenario words[2] opts
  ```
- why it matters: the first form is `error[syntax]: expected a parameter
  pattern`. Dispatch is the language's only switch, and argument vectors,
  request lines and split header values are all short lists whose shape is
  the question, so each one becomes an index, a guard and a none arm.

### F25: JSON in a string literal is mostly backslashes
- kind: aesthetics
- severity: minor
- where: `todo/todo_test.kso`, `router/router_test.kso`
- wanted: the request body a test sends, written as it is sent:
  `{"title":"a","completed":"yes"}`.
- wrote:
  ```
  body = "\{\"title\":\"a\",\"completed\":\"yes\"}"
  ```
- why it matters: `{` opens an interpolation and `"` ends the string, so
  every brace and quote in a JSON document needs a backslash, and an
  expected value such as `"show 1 \{ \"id\":\"42\" }"` is hard to check by
  eye. There is no raw string literal. The scenario files exist partly to
  get request text out of kanso source.

### F26: two arms that tie must be split by a third arm that repeats one of them
- kind: aesthetics
- severity: nit
- where: `todo/todo.kso:edited`
- wanted: "no such todo" wins over "bad body":
  ```
  fn edited _ st none _
    missing st

  fn edited _ st _ (invalid reason status)
    outcome (http/failure status reason) st
  ```
- wrote: the same, plus an arm for the case where both hold, whose body is a
  copy of the first:
  ```
  fn edited _ st none (invalid _ _)
    missing st
  ```
- why it matters: the tie rule is right that a call could match both arms,
  and an extra arm is how the program says which one wins. But the only way
  to say it is to repeat a body, so a change to the 404 has to be made in
  two places. The ordering rule also asked me to reorder arms that could
  never match the same call (`option_step "--quiet" _` after
  `option_step "--port" v:string`), which carried no meaning.

## What worked well

**Dispatch as the connection's state machine.** The parser answers one of
four kinds of value, and the connection loop has an arm for each. The arm that
sends `100 Continue` is the only one that matches a session which has not
sent it yet and a body the client is waiting to send, so "exactly once" is
in the pattern rather than in a flag check:
```
fn on_parse srv c state (session buffer false served) (need_body true)
  go_on = "HTTP/1.1 100 Continue\r\n\r\n"
  next = session buffer true served
  net/write c go_on .> (_ -> more srv c state next)

fn on_parse srv c state ses need_head
  more srv c state ses
```
Literal arms did the same for the small tables a server is full of: reason
phrases, media types by extension, the two known HTTP versions, CLI
commands, and `connection_header "HTTP/1.0" false`.

**A refusal is a value.** `parse` answers `rejected reason status` for every
malformed request, so most of its twenty-one refusals are one `return ...
if` line each, and the server turns all of them into a response in one arm.
In Go or Python this would be an error type plus a mapping from errors to
status codes. `json/decode`'s failure arrives the same way, and
`fn from_json (err r)` reads its reason and byte position straight into the
message the client sees: `the body is not JSON: expected a string key at
byte 2`.

**The checker made me decide what absence means.** `lines[1]`, the digits
before a chunk extension, `params["id"]` and `methods[1]` all answer none,
and the checker would not let any of them reach a function without an arm
for it. Each arm is a decision a C or Go parser makes by accident: an
absent request line is a 400, a chunk size with no digits is a 400, an id
that names nothing is a 404, and a response the client cannot match to a
request is read as an answer to a GET.

**Rescue is a clean way to end a loop.** `net/accept l .? closed` turns
the error from a closed listener into a `listener_closed` marker that the
next arm dispatches on, and `net/read c .? gone` does the same for a reset
connection. Because the rule licenses `.?` only on failures raised by
another module, it reads as "the socket failed", never "my code failed".

**Pure handlers, tested without a server.** A todo handler is a function
from request, path parameters and store to response and store. The 22 tests
in `todo/todo_test.kso` call them directly, and the app lifts all six into
effects in one function:
```
fn on_todos h
  (req params a -> lifted a (h req params a.todos))
```
Effects as values also made the server testable as a whole: the e2e mode is
a two-line group, the server in one line and the client in the other.

**Three engines agreed.** Every fixture runs on the interpreter, a dev
binary and a release binary, and the transcripts, about 850 lines in all,
are compared byte for byte. The scheduler's determinism and `KANSO_NOW`
pinning the clock are what make a socket test reproducible enough to
compare that way. The engines disagreed in two places in this port, both
recorded above: F6, found when a unit test failed on the interpreter and the
same expression worked natively, and F21, found by `check.sh`'s
standalone-server comparison.

**Small things.** `return x if cond` let a parser read top to bottom. A
read past the end of a byte list answers none, so `bs[i + 1] == 10` needs
no length check in front of it: a line whose LF has not arrived yet simply
fails the test, and the parser answers `need_head`. `text/find2` made scanning for CR LF fast enough that the
parser re-reads the whole buffer on every segment without trouble.
`json/encode` sorts keys, so JSON output is deterministic without any work.
Test files reach private functions (`keep_alive?`, `safe?`, `sized`), so
the decisions inside the server are tested where they are made.

## Summary

The five I would fix first:

1. **F19 and F20, concurrency for servers.** A blocking read that holds the
   whole program, and no way for fibers to share state, together rule out
   the concurrent server that chapter 06's model seems to promise. A server
   can be written only as a sequential fold over connections, and one idle
   client stops it.
2. **F2, accept outside a group.** The first standalone server anyone writes
   exits before a client can connect. The fix is small and the confusion is
   large.
3. **F7, one namespace with every imported type name in it, test files
   included.** Naming a test fixture `srv` broke fifteen functions in
   another file, and `std/list`'s exported types (`step`, `cursor`,
   `sorted`...) collide with ordinary function names, with a message that
   does not mention `std/list`.
4. **F13 and F17, run-time errors without a location.** "Comparison
   requires two values of one comparable type", and a keyed read reported
   against the wrong module or no location at all, each cost a bisection
   by hand. Both mistakes passed `kanso check`.
5. **F16, no client socket in `std/net`.** A test client, a proxy, or any
   program that talks to another server needs `dial`, a half-close and a
   read with a timeout.

Close behind them: F14 (a module's check verdict depends on its importer,
and test files are checked against the program), F3 (the one accepted
spelling of a bounds guard), and F12 (no way to wrap a list or string
literal).

Writing this server in kanso split into two experiences. The protocol
layer was a pleasure: HTTP parsing is a set of shape questions about bytes,
and dispatch on markers, literals and records answered them more clearly
than the if-chains I would write in Go. The checker's insistence on none
arms turned into real 400 responses. The pure core and the effectful shell
split along the line the book draws, and the same server ran byte-identically
on three engines. The friction came from two other places. First, the
language's model of concurrency is fork and join, and a server is a
long-lived loop with shared state and blocking I/O, so the port is a
sequential server and says so. Second, the toolchain's rules are strict,
which was fine, but its reports were often far from the cause: a rename
in one file broke another, a module that checked clean failed once imported,
and two run-time errors had no location. About a third of the time went
to finding out which line a message was about. Most of the canonical-form
rules stopped costing anything after the first day; the 80-column limit
without any way to wrap a literal kept costing to the end.
