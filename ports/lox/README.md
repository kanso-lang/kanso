# lox: jlox in kanso

A port of **jlox**, the tree-walking interpreter for the Lox language from
Robert Nystrom's [*Crafting Interpreters*](https://craftinginterpreters.com)
(Part II, chapters 4 to 13). The original is Java, released under the MIT
license, and lives at
[munificent/craftinginterpreters](https://github.com/munificent/craftinginterpreters).
This port was written from my understanding of the book's design. No Java
source was translated line by line, and the kanso program is shaped very
differently: it has no mutable objects, no exceptions and no visitor classes.

It is one of twenty ports made to find where kanso gets in a working
programmer's way. The findings are in [FRICTION.md](FRICTION.md).

## What it covers

Everything in jlox:

- **Scanner** (`lox/scanner.kso`): every token, line counting, multi-line
  strings, `//` comments, and jlox's two scan errors, reported without
  stopping.
- **Parser** (`lox/parser.kso`): recursive descent over the full grammar,
  with `for` taken apart into `while`, the "Invalid assignment target." and
  255-argument errors that do not stop the parse, and jlox's panic-mode
  recovery (`synchronize`) at every declaration, including inside blocks.
- **Resolver** (`lox/resolver.kso`): static scope resolution and all eight
  of jlox's resolution errors. The resolver rewrites the tree so each local
  says which scope and slot it lives in, and records which locals a closure
  captures.
- **Interpreter** (`lox/interpreter.kso`): closures, classes, fields,
  methods, `this`, initializers (including `init` returning `this` and
  `return;` inside it), single inheritance, `super`, the native `clock()`,
  and every runtime error, with jlox's messages and line numbers.
- **Values** (`lox/values.kso`): Lox's truthiness, equality (Java's
  `Double.equals`, so `NaN == NaN` and `0 != -0`), and number printing in
  Java's `Double.toString` format with jlox's trailing-`.0` trim (`1.0E7`,
  `1.5E-4`, `Infinity`, `-0`).
- **Driver** (`lox/run.kso`): exit status 65 for scan, parse and resolve
  errors, 70 for runtime errors, 64 for bad usage.
- **AST printer** (`lox/printer.kso`): chapter 5's printer, used by the
  parser tests.

### How it differs from jlox

- **No REPL.** kanso can only read standard input whole, so `lox` with no
  argument runs all of stdin as one script.
- **`clock()` is frozen** at the time the program started. Evaluation is
  pure, so it cannot read the clock in the middle of running.
- **Deep recursion stops at 1,000 Lox calls** with clox's runtime error
  "Stack overflow.". jlox dies with a Java stack trace, and the three kanso
  engines each crash at a different depth (see FRICTION F21).
- **Output is buffered per top-level statement**: a program's output appears
  after each top-level statement finishes, rather than line by line.
- **No garbage collection.** Memory a Lox program drops is not reclaimed on
  the native builds until the program ends (FRICTION F18). A loop that
  allocates objects 200,000 times needs gigabytes.

## Running it

The compiler is the kanso binary on main:

    KANSO=/tmp/claude-0/kanso-main/kanso

    $KANSO run main.kso -- tests/closures.lox            # native, cached
    $KANSO run main.kso --interp -- tests/closures.lox   # the interpreter
    $KANSO build main.kso --release && ./main tests/inheritance.lox
    echo 'print "hi";' | ./main                          # script on stdin

    $KANSO test lox      # unit tests
    sh check.sh          # unit tests plus every fixture on all three engines

## Layout

    main.kso               entry: hands the arguments to lox/start
    lox/                   the module (one namespace across its files)
      scanner.kso          source text -> tokens
      ast.kso              syntax tree node types
      parser.kso           tokens -> syntax tree
      printer.kso          syntax tree -> prefix string (tests)
      resolver.kso         syntax tree -> resolved tree
      cells.kso            a persistent 4-ary trie used as the store
      chain.kso            newest-first accumulation, turned into a list once
      values.kso           Lox values, equality, printing
      interpreter.kso      evaluation
      run.kso              the driver: reading, error reporting, exit codes
      *_test.kso           unit tests (`kanso test lox`)
    tests/*.lox            fixture programs
    tests/*.expected       stdout, stderr and exit status for each
    check.sh               runs the fixtures three ways
    bugs/                  minimal reproductions of compiler/runtime bugs
    FRICTION.md            the journal

## How the interpreter holds state

jlox mutates environments and instances in place. kanso values never change,
so the whole of jlox's mutable state is one `machine` record that every
evaluation step takes and returns. Locals are addressed by (depth, slot)
from the resolver. A local that no closure captures lives in a stack region
that is reused when its scope ends. A captured local, and every instance
field, lives in a heap region that only grows. Both regions are the trie in
`cells.kso`, because kanso's native maps are append-only logs that cannot
take repeated overwrites (FRICTION F4).

## Testing

- `kanso test lox` runs 44 unit tests across the scanner, parser, resolver,
  value printing, the store and the interpreter.
- `check.sh` runs 28 fixture programs in `tests/` on `kanso run --interp`, a
  dev build and a release build, and requires all three to match the
  expected file byte for byte. The fixtures cover each language feature, four
  larger programs (a linked list with higher-order methods, binary trees, a
  bank ledger kept in closures, a class hierarchy), every runtime error, every
  scan and resolve error, and parse errors with recovery.
- The expected outputs were compared against the reference jlox, built from
  the book's repository, and agree with it except for
  `error_stack_overflow`. The port was also run against the book's own test
  suite (246 programs, excluding the benchmarks and the chapter-specific
  scanning and expression tests): all agree with jlox except
  `limit/stack_overflow.lox` (jlox crashes, this port reports "Stack
  overflow.") and `string/literals.lox` (jlox prints non-ASCII as `?`
  because of the JVM's default charset here; this port prints the UTF-8 the
  test expects).
