# rg: ripgrep in kanso

[ripgrep](https://github.com/BurntSushi/ripgrep) is Andrew Gallant's
recursive line-oriented search tool, dual-licensed MIT and Unlicense. It walks
a directory tree, skips what `.gitignore` and friends say to skip, and prints
the lines that match a regex. This directory is a port of its core to kanso,
written from the behavior of `rg` 14.1 and its documentation rather than from
its source. No ripgrep code is copied here.

The port exists to find out where kanso gets in the way of a working
programmer. FRICTION.md is the journal of that, and it is the main product.

## Running it

```
KANSO=/tmp/claude-0/kanso-main/kanso
$KANSO run . -- -n TODO src             # compile natively and run
$KANSO run . --interp -- -n TODO src    # on the interpreter
$KANSO build main.kso --release         # writes ./main
./main -C1 -g '*.rs' 'fn \w+' src
```

`./main --help` lists every flag.

## What it covers

- Recursive search in path order, with the path, line number (`-n`), and
  column (`--column`, counted in bytes as rg does) in front of each line.
  A single file named on its own prints without its path, as rg does.
- Regexes through `std/regexp`: several patterns (`-e`, `-f FILE`), literal
  patterns (`-F`), `-i`, `-s`, smart case (`-S`), whole words (`-w`), whole
  lines (`-x`), inverted matching (`-v`), and `-m` to stop after N matches.
- Context: `-A`, `-B`, `-C`, with `--` between groups and between files,
  overlapping windows merged, and `--passthru`.
- Output modes: matching lines, `-o` (each match), `-r` (replacement with
  `$1`, `${1}`, `$0`, `$$`), `-c`, `--count-matches`, `--include-zero`,
  `-l`, `--files-without-match`, `-q`, `--files`, `--vimgrep`, `--heading`,
  and `-0` (a NUL after every path).
- Ignore rules: `.gitignore`, `.ignore` and `.rgignore` in every directory,
  lowest precedence first, deeper files overriding shallower ones; negation
  with `!`, directory-only rules with a trailing `/`, rules anchored by a
  leading or inner `/`, and `*`, `?`, `**` and `[...]` globs. Ignore files in
  directories between the working directory and a search root apply too.
  Hidden files and directories are skipped unless `--hidden`.
  `--no-ignore`, and `-u`, `-uu`, `-uuu`.
- Globs (`-g`, `!` to exclude) that take precedence over ignore files, hidden
  names and types, and file types (`-t`, `-T`, `--type-list`) for seventeen
  of ripgrep's types, with the same globs.
- Binary files: a file with a NUL byte found by walking is skipped; one named
  on the command line, or any under `--binary`, reports
  `binary file matches (found "\0" byte around offset N)`; `-a` searches it
  as text. Invalid UTF-8 is searched, with each broken sequence read as
  U+FFFD.
- `--max-depth`, the exit status (0 matched, 1 nothing matched, 2 an error),
  and ripgrep's wording for the errors a user is likely to meet.

## What it leaves out

- Parallel search. kanso can run effects concurrently, but the port walks one
  file at a time so that output is always in path order, as `rg --sort path`
  prints it.
- Anything that depends on knowing whether stdout or stdin is a terminal:
  colors, the default `--heading` and line numbers rg uses on a terminal, and
  searching standard input when no path is given. kanso has no `isatty`, so
  the port always behaves as rg does when piped, and with no path it searches
  the working directory.
- Symlinks (`-L`), `--max-filesize`, and memory-mapped or streaming reads:
  `std/os` has no `lstat` or file size, so every file is read whole.
- Ignore files above the working directory, the global gitignore, and
  `.git/info/exclude`. The port also does not require a git repository for
  `.gitignore` to apply, which is rg's `--no-require-git`.
- Multiline search (`-U`), PCRE2, encodings other than UTF-8, compressed
  files, preprocessors, `--json`, `--stats`, and most of ripgrep's long tail
  of flags.

## Known differences from rg

`fixtures/divergences.txt` lists every fixture whose expected output is not
ripgrep's, with the reason. In short: invalid UTF-8 prints as U+FFFD rather
than raw bytes; regex errors are `std/regexp`'s one-line reasons; `-i` folds
ASCII letters only; the type table is a subset; and the port reads a parent
directory's anchored ignore rule the way git does in one case where rg 14
does not.

## Layout

```
main.kso              the entry: hands the arguments to search/main
search/cli.kso        the command line, read into a config record
search/pattern.kso    the patterns and flags, built into one regex
search/glob.kso       the glob matcher shared by ignore files, -g and -t
search/ignore.kso     ignore-file rules and -g overrides
search/types.kso      the file-type table
search/decode.kso     bytes to text, NUL detection, line splitting
search/lines.kso      searching one file and formatting what it prints
search/walk.kso       the effects: the walk, reads, writes and exit status
search/*_test.kso     unit tests (`kanso test search`)
fixtures/             the search tree, the cases, and expected outputs
bugs/                 reproductions of the compiler bugs met on the way
```

## Tests

```
sh check.sh      # unit tests, then every fixture on three engines
sh oracle.sh     # every fixture through the real rg, compared
```

`check.sh` runs the unit tests and then each case in `fixtures/cases.txt`
three ways: on the interpreter, as a dev-tier binary, and as a release
binary. Each must match `fixtures/expected/NAME.out` (stdout, stderr and the
exit status) byte for byte.

The search tree is built by `fixtures/make_tree.sh` into a temporary
directory on every run, not checked in. The tree holds `.gitignore` files of
its own, and committed they would make the enclosing repository drop the
files they exist to hide.

The expected files came from ripgrep 14.1 itself: `sh oracle.sh --write`
runs every case through `rg --sort path --no-require-git --no-ignore-global`
and saves the result. Cases listed in `fixtures/divergences.txt` are written
from the port's own output by `fixtures/accept.sh` instead.
