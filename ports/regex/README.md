# regex: the core of RE2, in kanso

[RE2](https://github.com/google/re2) is the regular-expression library Russ
Cox wrote at Google, and Go's `regexp` package is its sibling. Its promise is
that matching takes time linear in the length of the input, whatever the
pattern, because it never backtracks. Cox's articles
["Regular Expression Matching Can Be Simple And Fast"](https://swtch.com/~rsc/regexp/regexp1.html)
and ["Regular Expression Matching: the Virtual Machine Approach"](https://swtch.com/~rsc/regexp/regexp2.html)
describe the design this port follows. RE2 is BSD-licensed; none of its code
is used here. The engine was written from scratch in kanso from those
articles and from RE2's and Go's documented syntax, without `std/regexp`.

This directory is one of twenty ports written to find where kanso gets in a
working programmer's way. FRICTION.md is the journal of that, and the main
product; this file describes the program.

## What it does

    regex [-L] [-c] [-i] [-n] [-o] [-v] [--groups] [--steps] PATTERN [FILE]
    regex --explain PATTERN
    regex --scale PATTERN UNIT TAIL N...
    regex [-L] --table [FILE]

The first form is a small grep. It prints the lines of FILE (or standard
input) that match PATTERN. `-o` prints each match instead, `-n` numbers
lines, `-c` counts them, `-v` inverts the selection, `-i` ignores case, and
`--groups` prints every match with each capture group's text and offsets.
`-L` asks for the leftmost-longest match, as POSIX grep finds it, instead of
the leftmost-first match Perl and RE2 find by default. `--steps` adds a last
line with the number of steps the matcher took. A `--` ends the options, so
a pattern may begin with a dash. The exit status is grep's: 0 when a line
was selected, 1 when none was, 2 for a bad pattern, a missing file or a bad
command line. A bad pattern is reported with Go's message and a caret:

    $ regex 'fo+*' poem.txt
    regex: error parsing regexp: invalid nested repetition operator: `+*`
      fo+*
        ^

`--explain` prints the pattern as parsed, after simplification, and as the
program the matcher runs:

    $ regex --explain '(a*)*b'
    pattern:    (a*)*b
    parsed:     cat{star{cap1{star{lit{a}}}}lit{b}}
    simplified: cat{star{cap1{star{lit{a}}}}lit{b}}
    groups:     1
    program:
    1. save 0
    2. split 3 9
    3. save 2
    4. split 5 7
    5. char a
    6. jmp 4
    7. save 3
    8. split 3 9
    9. char b
    10. save 1
    11. match

`--scale` is the linear-time demonstration. For each N it matches PATTERN
against UNIT repeated N times followed by TAIL, and prints the steps the
Pike VM took beside the steps a backtracking matcher (in re/backtrack.kso,
kept only for this) takes on the same input:

    $ regex --scale '(a*)*b' a '' 1 2 4 8 12 16
    pattern (a*)*b, subject a x n then ""
           n  match      pike   backtrack
           1     no        21          17
           2     no        32          38
           4     no        54         160
           8     no        98        2564
          12     no       142       40968
          16     no       186      655372
    backtracking stops at 1000000 steps, and is skipped past n = 64

Each extra `a` costs the Pike VM eleven steps; the backtracker's count
roughly doubles. At 4,000 a's the Pike VM takes 44,010 steps. A step is one
instruction executed by one thread, including the jumps and splits followed
while a thread is added, so the count depends only on the program and the
input. The interpreter and both native builds print the same numbers, and
check.sh compares them byte for byte.

`--table` reads lines of `PATTERN<TAB>SUBJECT` (with `\n` in the subject
standing for a newline) and prints every match of each as its offsets, in a
format that oracle/table.go prints from Go's `regexp`. That is how the
engine was checked against an independent implementation.

## What the port covers

- A parser from pattern text to a syntax tree, with RE2's syntax and Go's
  error messages: literals; `.`; `^` and `$`, with `(?m)` for line anchors;
  `\A`, `\z`, `\b`, `\B`; alternation; `*`, `+`, `?` and their lazy forms;
  counted repetition `{n}`, `{n,}`, `{n,m}` up to 1000, with a `{` that
  does not spell a count read as a literal; capturing, non-capturing and
  named groups `(?P<name>…)` and `(?<name>…)`; the flags `i`, `m`, `s` and
  `U`, as `(?flags)` and `(?flags:…)`; bracket classes with ranges,
  negation, the Perl escapes and POSIX names such as `[[:alpha:]]`;
  `\d \w \s \D \W \S`; `\x41`, `\x{263a}`, `\n` and the other control
  escapes; `\Q…\E`.
- Case folding for ASCII, Latin-1, Greek and Cyrillic, including the
  three-member orbits (k, K and the Kelvin sign; s, S and long s; the three
  sigmas; micro and mu; å and the angstrom sign).
- Simplification: counted repetition expanded into copies and nested
  optionals, nested sequences and alternations flattened, empty pieces
  dropped, loops of loops collapsed, and an alternation of single
  characters merged into one class.
- Compilation to a Pike VM program of `char`, `class`, `any`, `split`,
  `jmp`, `save`, `assert` and `match` instructions. A star over something
  that can match the empty string is compiled as `(x+)?`, as Go does, so
  that `(a*)*` on `b` reports group 1 as matching the empty string.
- The Pike VM: all threads advance in lockstep over the input, in priority
  order, which gives leftmost-first submatches without backtracking, and a
  leftmost-longest mode.
- Find-all with RE2's rule for empty matches: one that begins where the
  previous match ended is skipped.
- Code-point offsets: the input is matched as Unicode code points, and
  offsets count code points, not bytes.

## What it leaves out

- Unicode classes (`\pL`, `\p{Greek}`) and the full Unicode case-folding
  table. `\p` is reported as an invalid escape.
- RE2's DFA and one-pass matchers, and its literal-prefix search. This port
  has only the Pike VM, so it is linear but not fast: about half a
  microsecond per step on the release build.
- Matching on bytes rather than code points, `\C`, and the latin-1 mode.
- Replacement, `Set` and the other parts of RE2's API around the matcher.
- A program-size budget in bytes. Instead a pattern is refused with
  "expression too large" when it would compile to more than 20,000
  instructions, and, as in Go, counted repetitions may not nest to more
  than 1000 copies.

## Where it differs from the oracles

check.sh with `--oracle` compares the three `--table` fixtures with Go 1.24's
`regexp` (oracle/table.go) and agrees on every line. It differs from Go in
one deliberate place, kept in tests/table_re2_only.tsv: a capture name used
twice is refused, as RE2's C++ library and Python refuse it; Go accepts it.

Python's `re` (oracle/table.py, run with `re.ASCII`) answers differently on
fourteen lines of tests/table_basic.tsv, all for known reasons:

- Python finds an empty match right after a previous match, so `a*` on
  `baaa` gives a fourth, empty match at the end. RE2 and Go skip it.
- Python has no POSIX classes. It reads `[[:alpha:]]` as an ordinary set
  followed by a literal `]`.
- Python reads `a{,3}` as `a{0,3}`; RE2 reads it as five literal characters.
- Python supports lookaround and possessive repetition, which RE2 refuses.
- With `re.ASCII`, Python folds case only for ASCII; without it, its
  answers for the Greek, Cyrillic, Kelvin and long-s cases agree with this
  port.

## Running it

The toolchain is the kanso binary on main:

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO run main.kso -- -n fox tests/data/poem.txt
    $KANSO run main.kso --interp -- --scale '(a*)*b' a '' 10 20 30
    $KANSO test re
    $KANSO build main.kso --release && ./main -o '\w+@\w+' tests/data/poem.txt
    sh check.sh              # unit tests, then every fixture on all three engines
    sh check.sh --oracle     # also compare with Go's regexp and Python's re

## Layout

    main.kso          the entry: hands the command line to grep/run
    re/               the engine
      ast.kso           syntax tree types
      parse.kso         the parser: sequences, repetition, atoms, escapes
      group.kso         groups, named groups and flags
      bracket.kso       bracket expressions
      classes.kso       spans of code points, Perl and POSIX classes
      fold.kso          case folding
      simplify.kso      the tree rewrites
      compile.kso       tree to program, and the program listing
      pike.kso          the Pike VM
      backtrack.kso     the backtracker the Pike VM is measured against
      api.kso           the public surface
      dump.kso          the one-line tree format used by --explain and tests
      *_test.kso        unit tests (kanso test re)
    grep/             the command line: cli.kso, explain.kso, scale.kso,
                      table.kso, cli_test.kso
    tests/            fixtures: NAME.args (one argument per line), optional
                      NAME.stdin, NAME.expected; data/ holds the input files
    oracle/           table.go and table.py, the two oracles for --table
    bugs/             reproductions of the compiler bugs met on the way
    FRICTION.md       the journal
