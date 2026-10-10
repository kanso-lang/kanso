# mal in kanso

A port of [mal (Make-A-Lisp)](https://github.com/kanaka/mal), Joel Martin's
Clojure-flavoured Lisp that has been implemented in some ninety languages by
following the same eleven steps. This version was written for kanso from the
mal process guide and test suite, without copying any implementation's
source.

It covers the whole of mal through step A: the reader and printer, `def!`,
`let*`, `do`, `if`, `fn*` with `&` rest parameters, closures, tail calls,
`quote`, `quasiquote` with `unquote` and `splice-unquote` (lists and
vectors), `defmacro!` and `macroexpand`, `try*`/`catch*` and `throw`, atoms,
metadata, `eval`, `load-file`, `*ARGV*`, `*host-language*`, and the core
namespace (arithmetic, comparison, sequences, hash-maps, strings,
predicates, `apply`, `map`, `readline`, `slurp`, `read-string`, `time-ms`).

The mal-in-mal implementation in `examples/self_host/` runs on top of it and
passes the same step tests.

## Running it

```
KANSO=/path/to/kanso

$KANSO run .                         # the REPL over stdin
$KANSO run . -- prog.mal a b         # run a file; *ARGV* is ("a" "b")
$KANSO run . -- --test tests/step4_if_fn_do.mal
$KANSO run . -- --test tests/step4_if_fn_do.mal --via examples/self_host/stepA_mal.mal
$KANSO build . --release && ./mal    # the same, natively
```

`--test` reads a file in the format of mal's `runtest.py`: an input line,
then `;/regex` lines for what it prints and a `;=>value` line for its answer.
A file containing `;>>> read-only=True` (step 1) is read and printed back
without evaluation. `--via` loads a mal written in mal first and sends every
case through its `rep`.

`sh check.sh` runs the kanso unit tests, then every fixture on the
interpreter, a dev build and a release build, and fails if any output
differs from the expected file or between engines. `sh check.sh --update`
rewrites the expected files from the interpreter.

## Layout

```
main.kso                 entry: hands the command line to lisp/main
lisp/types.kso           mal values: symbols, keywords, lists, vectors, maps, closures
lisp/machine_types.kso   the evaluator's continuation frames and the world record
lisp/reader.kso          tokenizer and recursive-descent reader
lisp/printer.kso         pr_str, readable and not
lisp/env.kso             environments, globals, atoms
lisp/eval.kso            the evaluator, a CEK machine
lisp/quasi.kso           quasiquote expansion
lisp/core.kso            the core namespace
lisp/repl.kso            REPL, file runner, runtest runner, the effects
lisp/lisp_test.kso       unit tests (kanso test lisp)
tests/                   step 1 to step A cases in runtest format, and their data
fixtures/                REPL transcripts, programs, a mal-in-mal session
examples/self_host/      mal written in mal
bugs/                    reductions of compiler and runtime problems met on the way
FRICTION.md              the journal of where kanso got in the way
```

## How it is built

kanso has no mutation and no exceptions, and its effects are values handed to
an executor, so the evaluator is a CEK machine: every step takes an
expression, an environment, a continuation and a `world`, and hands on the
next step. Writing the continuation down as data is what gives mal its tail
calls (a call in tail position reuses the caller's continuation), its
`try*`/`catch*` (a throw walks the continuation to the nearest `k_try`), and
a way to read a file in the middle of an evaluation (the machine stops with
`wants_file`, the driver performs the read and resumes it).

Environments are immutable frames. A top-level `def!` writes a globals map
that every closure reaches by name, so later definitions are visible to
earlier closures. A closure bound by `let*`, or by a `def!` inside a `do`,
is stored with a placeholder environment that is replaced by the binding
frame when the name is looked up, which is how recursive and mutually
recursive local functions work without a cycle in the data. Atoms are ids
into a trie held in the world.

## What it leaves out, and what differs

- The REPL reads all of stdin before it evaluates the first line, because
  kanso's `io/stdin` yields the whole input at once. It works on piped input
  and transcripts, and does not answer line by line at a terminal.
- Numbers are integers, as in mal's tests; there are no floats.
- `def!` inside a function binds for the rest of the enclosing `do` only. A
  `def!` buried in another expression (an `if` branch, say) inside a function
  binds nothing; other mal ports bind it in the call's frame.
- Hash-map keys must be strings or keywords. Maps print with their keys
  sorted, keywords first.
- No host interop form (`kanso-eval`): kanso has no `eval`.
- Error messages are this port's own: `'x' not found`, `EOF`,
  `nth: index out of range`. An exception nobody catches prints
  `Error: ...` in the REPL; a file run prints it on stderr and exits 1.
- Natively, memory is not reclaimed while the evaluator runs (see F8 in
  FRICTION.md), so a long computation holds everything it allocated until it
  exits. The interpreter engine does not have this problem.

## Credit

mal is by Joel Martin and contributors, MPL 2.0. The step tests in `tests/`
are reconstructed from the mal test suite's cases; the data files and the
mal-in-mal in `examples/self_host/` were written for this port.
