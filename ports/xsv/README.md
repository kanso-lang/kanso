# xsv, in kanso

A port of [xsv](https://github.com/BurntSushi/xsv), Andrew Gallant's fast
command-line toolkit for CSV data, written in Rust. This is a rewrite from
my understanding of xsv's documented behavior and its `--help` texts, not a
translation of its source. xsv is dual-licensed MIT and Unlicense; all
credit for the design and the command set belongs to its author.

The port is one of twenty written to find where kanso gets in a working
programmer's way. The journal of that is `FRICTION.md`.

## What it covers

The CSV layer (`csv/`) reads and writes RFC 4180:

- quoted fields holding the delimiter, doubled quotes and line breaks;
- LF, CRLF and lone-CR record endings; blank lines skipped;
- any single-byte delimiter, chosen with `-d`, `-d '\t'`, or by a `.tsv` or
  `.tab` file extension;
- the csv crate's leniency for a quote inside a bare field, text after a
  closing quote, and an unterminated quote at the end of input;
- an error for a record whose width differs from the first, in xsv's
  wording, with record number, line and byte offset;
- minimal quoting on output: a field is quoted only when it holds the
  delimiter, a quote, CR or LF, or when it is the only field of its record
  and empty.

The commands (`xsv/`):

| command     | options                                                        |
|-------------|----------------------------------------------------------------|
| `count`     |                                                                |
| `headers`   | `-j/--just-names`, `--intersect`, several inputs               |
| `select`    | the full selection syntax: indexes, names, `name[n]`, ranges in either direction, open ranges, quoted names, `!` negation |
| `search`    | `-i`, `-v`, `-s/--select`                                      |
| `sort`      | `-s/--select`, `-N/--numeric`, `-R/--reverse` (stable)         |
| `stats`     | `-s`, `--everything`, `--median`, `--mode`, `--cardinality`, `--nulls` |
| `frequency` | `-s`, `-l/--limit`, `-a/--asc`, `--no-nulls`                   |
| `join`      | inner, `--left`, `--right`, `--full`, `--cross`, `--no-case`, `--nulls`, multi-column keys |
| `slice`     | `-s/--start`, `-e/--end`, `-l/--len`, `-i/--index`             |
| `table`     | `-w/--width`, `-p/--pad`, `-c/--condense`                      |

Every command takes `-n/--no-headers`, `-d/--delimiter`, `-o/--output` and
`-h/--help`, reads standard input when no file (or `-`) is given, and
writes comma-separated output.

## What it leaves out

- The other xsv commands: `cat`, `fixlengths`, `flatten`, `fmt`, `index`,
  `input`, `partition`, `reverse`, `sample`, `split`, `behead`, `explode`.
- Indexes (`xsv index`) and the parallelism that goes with them. Every
  command reads its whole input into memory.
- Non-UTF-8 data. xsv works on bytes; this port reports a file that is not
  UTF-8 and stops. Standard input that is not UTF-8 ends at the runtime's
  own error.
- `join --no-case` folds ASCII letters only; std/text has no case mapping.

Where the original's exact output was not something I could check, I chose
and wrote it down here:

- `frequency` breaks ties in count by value, in byte order, so the output
  never depends on hash order. `stats --mode` picks the smallest of the
  most common values for the same reason.
- `stats` computes mean and standard deviation in two passes rather than
  online, so the last digit of a float can differ from xsv's. Floats are
  printed the way Rust prints an `f64` (`6`, not `6.0`).
- Under `--no-headers`, `stats` and `frequency` name columns from 1.

## Layout

    main.kso          the entry: hands the arguments to xsv/run
    csv/              RFC 4180 reader and writer, with its tests
    xsv/cli.kso       arguments in, inputs read, one output or error out
    xsv/args.kso      a docopt-style flag parser
    xsv/selection.kso the column-selection language
    xsv/<command>.kso one file per command: its flags, help and work
    xsv/*_test.kso    unit tests
    fixtures/data/    input files
    fixtures/cases/   NAME.cmd (a command line) and NAME.out (expected)
    bugs/             minimal reproductions of compiler and runtime bugs
    check.sh          the whole suite, on all three engines

Each command contributes arms to three dispatch groups, `flags_of`,
`usage_of` and `compute`, keyed on its name. Adding a command is adding a
file.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso

    $KANSO run . -- count fixtures/data/people.csv
    $KANSO run . -- select name,city fixtures/data/people.csv
    $KANSO run . --interp -- stats --everything fixtures/data/sales.csv
    $KANSO build . --release && ./xsv join --left city a.csv city b.csv

    $KANSO test csv       # 24 tests
    $KANSO test xsv       # 51 tests
    sh check.sh           # unit tests, then 122 fixtures on interp, dev
                          # and release, compared to the expected files
                          # and to each other

`sh check.sh --bless` rewrites the expected files from the interpreter.
