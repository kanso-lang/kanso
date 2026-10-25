# kanso

[![ci](https://github.com/kanso-lang/kanso/actions/workflows/ci.yml/badge.svg)](https://github.com/kanso-lang/kanso/actions/workflows/ci.yml)

*kanso* (簡素): simplicity through the elimination of clutter.

A language where the source file contains only decisions. Anything a style guide, linter, or code review would enforce by convention, kanso enforces by making the alternative a compile error or unrepresentable. Programs have a canonical form, one rendering per program, so there is no formatter: non-canonical whitespace is a syntax error.

**Docs:** https://kanso-lang.dev/ · **Book:** https://kanso-lang.dev/book/ · **Spec:** https://kanso-lang.dev/spec.html · **Compiler:** https://kanso-lang.dev/compiler.html

```
# Division by zero answers a value you dispatch on rather than a failure you
# catch. The specific type is `divide_by_zero`; its parent `math_failure` is
# there for a handler that does not care which kind it was.
fn describe f:divide_by_zero
  "no divisor: {f}"

fn describe f:math_failure
  "some math failure: {f}"

fn describe n
  "result: {n}"

fn safe_ratio a b
  a / b

good = safe_ratio 10 2
bad = safe_ratio 10 0
print (describe good) .> (_ -> print (describe bad))
```

```
$ kanso play examples/errors.kso
result: 5
no divisor: division by zero
```

Nothing is thrown. `10 / 0` answers a `divide_by_zero` value, and the arms of `describe` dispatch on it like any other type. Nothing is printed while the program is evaluated either: the last line builds a description of two prints, and the runtime performs it.

## quickstart

```
git clone https://github.com/kanso-lang/kanso
cd kanso
cargo build --release
./target/release/kanso run examples/hello.kso
./target/release/kanso run examples/effects.kso --plan   # show the effects instead of performing them
./target/release/kanso play examples/errors.kso
./target/release/kanso check examples/hello.kso
```

`run` takes a file of statements or a directory with a `main.kso`. `play` takes one file that holds definitions beside its statements. `check` reports what `run` would refuse and stops. `test` runs the `test_*` constants in each `*_test.kso` file, `build` writes a native binary, and `repl` evaluates expressions as you type them. `kanso` with no verb prints the usage, which lists the rest.

## status

Three engines run kanso: a tree-walking interpreter, which is the semantics oracle, a native compiler through LLVM, and a browser engine compiled to wasm. Differential goldens hold them byte-identical. The language they run:

- purity, with effects as descriptions: `print`, sequencing with `.> (_ -> ...)`, and `--plan` to read the description without performing it
- failure as values: `err`, `none` and failure kinds such as `divide_by_zero` propagate until an arm dispatches on them
- overload dispatch on literals, types and generics, most specific first
- single-constructor record types, with fields in the order you declare them, positional construction, positional destructuring and keyed reads
- typesets: a name for "one of these" that arms dispatch on
- no commas anywhere: application is juxtaposition (`f a b`), and lists and maps are space-separated
- integers of any size on every engine, string interpolation `"{expr}"`, lists indexed from 1, maps, lambdas
- canonical form as grammar: indentation, spacing, blank lines, snake_case, and the order of imports, keyed reads and typeset members are all compile errors when they are wrong (see [tests/golden/errors](tests/golden/errors))
- nothing wasted: an unused binding or expression is a compile error
- modules: a directory is a module split across any number of files that share one namespace, `pub` marks what it exports, and `kanso install` fetches and pins what it imports

The error corpus in `tests/golden/errors` matters as much as the programs that run: much of the language is in its compile errors.

## editor support

One TextMate bundle in [editors/](editors) highlights `.kso` in JetBrains IDEs (Settings → Editor → TextMate Bundles → add `editors/kanso`) and in VS Code (symlink it into `~/.vscode/extensions`). See [editors/README.md](editors/README.md).

## development

```
cargo test    # unit tests and the golden corpora: examples, diagnostics, engines
```

`sh scripts/install_hooks.sh` points a clone's git at `hooks/`, whose
`post-merge` rebuilds the compiler after each pull. Git will not let a
repository set that for itself, so it is one command per clone.

MIT licensed.
