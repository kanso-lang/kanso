# httpd: a todo-backend on a small HTTP/1.1 server, in kanso

This is a port of two things that are usually separate projects. The first
is [Todo-Backend](https://www.todobackend.com/), a shared specification for a
small REST API (a collection of todos at one URL, each todo with a title, a
completed flag, an order and a URL of its own) that people implement in many
languages and frameworks so the implementations can be compared. The second
is the HTTP/1.1 server underneath it, which in those implementations comes
from a framework. Here it is written from scratch on `std/net`, in the
manner of the many small teaching servers (Go's `net/http` server loop,
Python's `http.server`, the server chapters of most systems-programming
books).

No code was copied from any of them. The API follows the Todo-Backend spec
and the protocol follows RFC 9110 and RFC 9112.

## What it does

The server (`http/`):

- parses requests incrementally from a byte buffer, so a request may arrive
  in any number of reads, and several requests may arrive in one;
- enforces the framing rules that keep a server safe: one content-length
  (copies must agree), no content-length beside transfer-encoding, decimal
  digits only, `Host` required on HTTP/1.1, no whitespace before a header's
  colon, no obsolete line folding, limits on head and body size;
- decodes chunked request bodies (hex sizes, chunk extensions, trailers) and
  answers `Expect: 100-continue`;
- keeps connections alive by the HTTP/1.1 and HTTP/1.0 rules, including the
  `Connection` token list, and caps the requests on one connection;
- sends chunked responses to HTTP/1.1 clients and sized ones to HTTP/1.0
  clients, leaves the body off HEAD, 204 and 304 responses, and dates every
  response.

The router (`router/`) matches `:name` and `*rest` patterns, decodes each
path segment after splitting so an escaped slash stays inside its segment,
answers HEAD from GET routes, and answers 405 with an `Allow` header when a
path exists under other methods.

The application (`app/`, `todo/`, `static/`):

- the Todo-Backend API at `/todos`: `GET`, `POST` and `DELETE` on the
  collection, `GET`, `PATCH` and `DELETE` on `/todos/:id`, with absolute
  `url` fields built from the `Host` header, `Location` on create, and JSON
  errors (400, 404, 405, 415, 422) that say what was wrong;
- CORS on every response and an answer to preflight requests, which the
  spec's browser-based test runner needs;
- static files from `public/`, with media types, an ETag from the content's
  SHA-256 and `If-None-Match` answered with 304, directory index pages, and
  refusal of hidden files and of paths that climb out of the root.

The test client (`client/`) reads scenario files, frames requests itself
(including chunked bodies), sends them over real sockets, and parses the
responses independently of the server's parser: status lines, headers,
content-length, chunked and read-to-close bodies, interim 100 responses and
HEAD.

## What it leaves out

- **Concurrency.** Connections are served one after another. A read on a
  socket blocks every fiber in a kanso program, and fibers have no way to
  share the todo store, so a concurrent server is not expressible yet
  (FRICTION F19 and F20). One idle keep-alive client therefore holds the
  server until it goes away; there is no read timeout to drop it.
- **A client socket.** `std/net` can listen but not connect, so the test
  client's bytes travel through `nc` (OpenBSD netcat, for its `-N`
  half-close) and come back hex-encoded through `od` (F16). Everything on
  either side of that pipe is kanso.
- TLS, HTTP/2, content codings such as gzip (refused with 501), range
  requests, `Last-Modified` (`std/os` has no file times), and persistence of
  the todo store across restarts.
- A signal handler: the standalone server runs until it is killed. Its access
  log goes to stderr, because a native binary's stdout is block-buffered and
  is lost when the process is killed (F21).

## Running it

The toolchain is the compiler on main:

    KANSO=/tmp/claude-0/kanso-main/kanso

Serve the API and the static files:

    $KANSO run main.kso -- serve --port 48213 --root public
    curl -s -H 'content-type: application/json' \
         -d '{"title":"walk the dog"}' http://127.0.0.1:48213/todos
    curl -s http://127.0.0.1:48213/todos

`--quiet` turns off the access log. Open `http://127.0.0.1:48213/` in a
browser for a small page that uses the API.

Play a scenario against a server started inside the same process, and print
the transcript:

    $KANSO run main.kso -- e2e fixtures/todo_api.scn

The server listens on the port given with `--port`, or on one the operating
system picks if none is given or the given one is taken. When the scenario
ends, the client closes the server's listener, which ends its accept loop,
and checks that nothing answers on the port any more. The last line of every
transcript reports that check.

Run everything:

    sh check.sh            # unit tests, then every fixture on three engines
    sh check.sh --update   # rewrite the expected transcripts

`check.sh` runs the `test_*` constants in each module, builds a dev-tier and
a release binary, plays every `fixtures/*.scn` on the interpreter and on both
binaries, each on its own random high port, and fails if any transcript
differs from `fixtures/*.out` or from the other engines'. It then starts each
engine as a standalone server in the background, drives it with curl, kills
it, and checks the port is free again. `KANSO_NOW` is pinned so the `Date`
header is the same on every run.

## Scenario files

One step per line:

    # text              a comment
    note TEXT           print TEXT into the transcript
    conn                open a connection; the steps after it are sent on it
    send TEXT           send TEXT and CR LF ("send" alone sends CR LF)
    data TEXT           send TEXT with nothing after it
    pause               send what came before, wait 0.3s, then go on
    request M T [BODY]  a whole request, with Host and a JSON body's length
    chunked M T A|B|C   a whole request whose body goes out as chunks A, B, C
    curl ARGS...        run curl; {base} in ARGS is the server's address

Lines are trimmed, so `send` cannot end in a space.

## Layout

    main.kso            the entry: hands the arguments to cli/
    cli/                argument parsing; the serve and e2e commands
    http/               bytes, request parsing, responses, the server loop,
                        the Date header
    router/             patterns, matching, 405s
    todo/               the store and the Todo-Backend handlers
    static/             files under the static root
    app/                the route table, CORS, and the app state
    client/             scenarios, the netcat transport, the response reader
    fixtures/           *.scn scenarios and *.out expected transcripts
    public/             the static root the fixtures serve
    bugs/               reproductions of the compiler and runtime bugs found
    FRICTION.md         the journal of where kanso got in the way
