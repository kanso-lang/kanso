# toml, in kanso

A TOML 1.0 decoder and encoder written in kanso, modeled on the Rust crate
[toml-rs](https://github.com/toml-rs/toml) (`toml::from_str`, `toml::to_string`,
`Value::get`) and tested in the convention of
[toml-test](https://github.com/toml-lang/toml-test), the language-neutral TOML
test suite by Martin Tournoij and the TOML authors. Neither project's source was
copied; the code here was written from the TOML 1.0 specification and from how
those two projects behave. The fixtures are reconstructions of toml-test's
categories, not copies of its files.

This port is one of twenty written to find where kanso gets in a working
programmer's way. FRICTION.md is the journal of what it found.

## What it does

```
kanso run main.kso -- decode FILE.toml        # TOML in, toml-test JSON out
kanso run main.kso -- encode FILE.json        # toml-test JSON in, TOML out
kanso run main.kso -- get FILE.toml PATH      # one value, e.g. servers.alpha.ip
```

With no FILE, `decode` and `encode` read standard input. `kanso build main.kso`
(or `--release`) gives a `main` binary that takes the same arguments.

The decoder covers all of TOML 1.0:

- comments, LF and CRLF line endings, a leading byte-order mark, and refusal of
  invalid UTF-8 and of control characters in strings and comments;
- bare, quoted and dotted keys, with blanks around the dots;
- basic and literal strings, on one line and across lines, with every escape
  (`\b \t \n \f \r \" \\ \uXXXX \UXXXXXXXX`), line-ending backslashes, and the
  rule that up to two quotes may sit against a closing delimiter;
- integers in decimal, hex, octal and binary, with underscores and a 64-bit
  range check; floats with fractions, exponents, `inf` and `nan`;
- offset date-times, local date-times, local dates and local times, kept as
  typed records (`offset_datetime`, `local_datetime`, `local_date`,
  `local_time`) with the calendar checked, leap years included;
- arrays (mixed types, nested, multi-line, trailing comma) and inline tables;
- `[table]` and `[[array.of.tables]]` headers, with TOML's rules about what may
  be defined once, what may be extended by a later header, and what dotted keys
  may add to.

A malformed document is reported the way toml-rs reports it, with the line,
the column (in characters), the offending line and a caret:

```
TOML parse error at line 2, column 12
  |
2 | b = {x = 1,}
  |            ^
trailing comma in inline table
```

The library is the `toml/` module. Its public surface:

| name | what it does |
|---|---|
| `toml/decode s` | a string to a value tree, or an err holding a `toml_error` |
| `toml/decode_bytes bs` | the same from the bytes `os/read_bytes` gives |
| `toml/encode m` | a value tree to TOML text |
| `toml/value_toml v` | one value as TOML writes it |
| `toml/lookup doc path` | one value by a path like `products[2].name`, or none |
| `toml/to_json v` | the toml-test JSON form |
| `toml/from_tagged j` | the toml-test JSON form (decoded by `std/json`) back to values |
| `toml/error_report e` | the report above, for a `toml_error` |

Tables come out as maps, arrays as lists, and strings, ints, floats and
booleans as themselves.

## What it leaves out

- TOML 1.1 additions (`\e`, `\xHH`, optional seconds, newlines in inline
  tables). The decoder refuses them, as a 1.0 decoder must.
- toml_edit's format-preserving editing. Comments and layout are not kept.
- serde-style mapping onto user types. kanso has no derive mechanism; a caller
  dispatches on the maps and records instead.
- Pretty-printing options for the encoder. It writes one canonical layout:
  plain values first, then each sub-table, then each array of tables, with
  arrays and nested inline tables on one line.
- Date-time arithmetic and time-zone conversion.

## Layout

```
main.kso            the entry: hands the arguments to cli/run
cli/cli.kso         the three commands and their error output
toml/
  toml.kso          decode, the error type and its report
  document.kso      the line-by-line loop: pairs and headers
  table.kso         TOML's table rules, kept in a flat index of slots,
                    and the tree built from it at the end
  index.kso         a persistent search tree (why it exists: FRICTION.md F14)
  key.kso           bare, quoted and dotted keys
  value.kso         values, arrays and inline tables
  string.kso        the four kinds of string and their escapes
  number.kso        integers and floats
  datetime.kso      the four date-time kinds, parsed, checked and written
  scan.kso          blanks, comments, line ends, positions
  utf8.kso          where a document stops being UTF-8
  tojson.kso        the toml-test JSON writer
  encode.kso        the TOML writer
  untag.kso         toml-test JSON back to values
  lookup.kso        paths into a decoded document
  toml_test.kso     unit tests (kanso test toml)
tests/
  valid/            documents and the JSON they decode to
  invalid/          documents and the report each one gets
  encode/           toml-test JSON and the TOML it encodes to, or the refusal
  get/              lookups and their output
bugs/               reductions of the compiler and runtime bugs met on the way
check.sh            every fixture on all three engines
```

## Testing

```
sh check.sh
```

runs the unit tests, builds a dev and a release binary, and then runs every
fixture on the interpreter and both binaries. Each output must match its
expected file and the other engines byte for byte. Every valid document also
makes a round trip on every engine: decode it, encode the JSON back to TOML,
decode that, and the JSON must not change. `CHECK_WRITE=1 sh check.sh`
rewrites the expected files from the interpreter's output; review the diff
before keeping it.

The expected files were generated by this decoder and then checked by hand
against the TOML 1.0 specification, so they say what the specification says
only as far as that reading was right.

## Performance

On a release build, a 300 KB document of 2,000 array-of-tables entries decodes
and prints in about 0.4 seconds, and a 330 KB document with 16,000 keys in one
table in about 0.5 seconds. The interpreter is roughly ninety times slower;
the 3,000-line fixture takes about two seconds there. The first two designs
took 10 and 32 seconds on the 300 KB document; FRICTION.md F14 says why, and
toml/index.kso is the result.
