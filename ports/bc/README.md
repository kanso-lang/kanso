# bc in kanso

A port of [GNU bc](https://www.gnu.org/software/bc/), the arbitrary-precision
calculator language, to kanso. GNU bc was written by Philip A. Nelson and is
maintained by Ken Pizzini; it implements the POSIX bc language with GNU
extensions. This port was written from the bc manual and from running GNU bc
1.07.1 side by side, not from GNU's source, and the `-l` library in
`lib/math.bc` is new code.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO build main.kso --release      # writes ./main
    echo 'scale = 50; 4 * a(1)' | ./main -l
    ./main -l prog.bc < /dev/null

or without building, on the interpreter:

    $KANSO run main.kso --interp -- -l prog.bc < /dev/null

Like GNU bc, it reads the library (with `-l`), then each file named on the
command line, then standard input. `-q` and the other flags GNU accepts are
ignored. `BC_LINE_LENGTH` sets where long output lines break, and 0 turns
breaking off.

## What it covers

- Numbers: arbitrary-precision decimals with bc's scale rules for each
  operator (`+ - * / % ^`, `sqrt`), truncating toward zero exactly as GNU
  does. Numbers are kanso's big integers with a scale beside them
  (`bc/num.kso`).
- `ibase` (2 to 36) and `obase` (2 and up). Constants are read in the ibase
  in force when the statement runs, a lone digit keeps its value, and in a
  longer number a digit too big for the base counts as base - 1. Output above
  base 16 is written in space-separated decimal groups, and fractions get as
  many digits as GNU prints.
- Output lines break with a backslash at 70 columns, strings and numbers
  alike.
- Variables, arrays (`a[i]`, up to index 16777215), `scale`, `ibase`,
  `obase`, `last` and `.`.
- `define` with parameters, array parameters passed by value, `*a[]`
  parameters passed by reference, `auto` locals with bc's dynamic scoping,
  `return` with and without a value, and `define void`.
- `if`/`else`, `while`, `for` with any part left out, `break`, `continue`,
  `halt`, `quit` (which stops bc when it is read, as GNU's does), `print`
  with its escapes, string statements, `limits`, `length()`, `scale()`.
- All the operators, with GNU's precedence: assignment binds tighter than
  comparison, unary minus tighter than `^`, and `&&` and `||` short-circuit.
- The `-l` library: `s c a l e j`, written in bc (`lib/math.bc`) and
  embedded in the binary. On every case in the fixtures it prints the same
  digits as GNU's library.
- GNU's error behaviour: a syntax error abandons the rest of its line and
  parsing resumes on the next; a runtime error abandons the rest of the line
  being run, keeps what it already did, and restores the autos of the
  functions it leaves. Error and warning messages use GNU's wording.

## What it leaves out

- `read()` reports a runtime error. kanso reads standard input as one
  string, so there is no way to read one number from it partway through a
  program (FRICTION F19).
- For the same reason standard input is read to its end before any of it
  runs, so the port is not an interactive calculator: typed lines produce
  nothing until end of input.
- `warranty` prints a short notice of this port's own, and `-v`, `-h`, `-s`,
  `-w` and `-i` are accepted and ignored.

## Divergences from GNU bc

`compare_gnu.sh` runs every fixture through both and shows the differences.
Beyond the fixtures, random programs were run through both
(`fuzz/run.sh`): 38,000 arithmetic statements over random scales and output
bases, 4,000 numbers read in random input bases, and 1,800 math-library
calls. The differences left are the ones listed here.

- Runtime errors and warnings print `adr=0`. GNU prints the address of a
  bytecode instruction, which this port does not have. The comparison
  script rewrites GNU's address before comparing.
- `tests/diverge_array_arg.bc`: for an argument of the wrong kind (an array
  where a number is expected, or the reverse) GNU 1.07.1 prints a second,
  unrelated message after the first, names the wrong parameter, and on the
  third such call crashes. The port prints one message naming the right
  parameter.
- `tests/diverge_mathlib_accuracy.bc`: at low scales GNU's `-l` library
  sometimes prints a wrong last digit (`scale=4; c(32.69)` prints `.2925`;
  the true value is .29238...). The port's library carries more guard digits
  and prints the truncated true value. A differential run of 1,800 random
  library calls at scales 0 to 40 found 22 such digits, and in each the
  port's digit was the one the true value, computed 40 digits further,
  truncates to. There was no other difference.
- `tests/diverge_negative_zero.bc`: a GNU power whose result truncates to
  zero keeps its minus sign, so `(-.1)^3` at scale 1 prints `-0`, compares
  less than zero, and makes `sqrt` fail. The port has no negative zero.
- After `x = 1 + v()` with `v` a void function, GNU reports both the void
  operand and the void assignment; the port reports the first.
- Recursion: GNU bc recursed 20,000 calls deep without complaint. The
  interpreter refuses recursion past ten thousand of its own frames, which
  is between 2,000 and 2,500 nested bc calls, with `error[runtime]: the
  program ran out of stack`; a native build, on an 8 MB stack, dies with a
  segmentation fault somewhere between 5,000 and 8,000 (FRICTION F18).
- Memory: a long loop in a native build keeps everything it allocates until
  the statement ends, about 1.8 KB an iteration, so a million-iteration
  `for` uses nearly 2 GB where GNU uses a few hundred kilobytes (FRICTION
  F17).

## Layout

    main.kso            entry: hands the arguments to bc/start
    bc/num.kso          decimal arithmetic on kanso integers
    bc/digits.kso       reading numbers in ibase, writing them in obase
    bc/lexer.kso        tokens, and the lexer's diagnostics
    bc/ast.kso          the tree
    bc/parser.kso       recursive descent with precedence climbing
    bc/machine.kso      the interpreter's state, and output with wrapping
    bc/eval.kso         expressions
    bc/calls.kso        function calls and dynamic scope
    bc/exec.kso         statements
    bc/run.kso          the command line and the read-run-write loop
    bc/mathlib.kso      lib/math.bc, embedded (generated by embed_mathlib.sh)
    bc/*_test.kso       unit tests: `kanso test bc`
    lib/math.bc         the -l library, in bc
    tests/              fixtures: NAME.bc, NAME.expected, and optional
                        NAME.args, NAME.env and NAME.stdin
    check.sh            unit tests, then every fixture on all three engines
    compare_gnu.sh      every fixture against GNU bc, when it is installed
    fuzz/               random programs run through GNU bc and the port
    bugs/               reproductions of the compiler and runtime bugs met
    FRICTION.md         the journal

`sh check.sh` runs the unit tests and the fixtures on the interpreter, a dev
build and a release build, and fails if any output differs from the expected
file or from another engine.
