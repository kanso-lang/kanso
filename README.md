# kanso

[![ci](https://github.com/ClayShentrup/kanso/actions/workflows/ci.yml/badge.svg)](https://github.com/ClayShentrup/kanso/actions/workflows/ci.yml)

*kanso* (簡素): simplicity through the elimination of clutter.

A language where the source file contains only decisions. Anything a style guide, linter, or code review would enforce by convention, kanso enforces by making the alternative a compile error or unrepresentable. Programs have a canonical form — one rendering per program — so no formatter tool exists: non-canonical whitespace is a syntax error.

**Docs:** https://clayshentrup.github.io/kanso/ · **Spec:** [kanso-spec.md](kanso-spec.md)

```
fn describe (err reason)
  "failed: {reason}"

fn describe n
  "result: {n}"

main =
  good = safe_ratio 10 2
  bad = safe_ratio 10 0
  print (describe good) >> print (describe bad)

fn safe_ratio a b
  a / b
```

```
$ kanso run examples/errors.kso
result: 5
failed: division by zero
```

No exceptions were thrown: division by zero returns `err`, which propagates as a value until an overload dispatches on it. No effects ran during evaluation: `main` returns a *description* of printing, which the runtime executes.

## quickstart

```
git clone https://github.com/ClayShentrup/kanso
cd kanso
cargo build --release
./target/release/kanso run examples/hello.kso
./target/release/kanso run examples/effects.kso --plan   # show the effect description instead of executing it
./target/release/kanso check examples/records.kso
```

## status

This is the **phase-1 reference interpreter** ([spec §15](kanso-spec.md)): a tree-walking evaluator in Rust covering a subset of the v0.1 design freeze. What runs today:

- purity and effects-as-descriptions: `print`, `>>` sequencing, `--plan` to inspect the description, a scripted executor for transcript-based tests
- failure as values: `err reason` and `none` propagate; division by zero and out-of-range `at` are values, not crashes
- overload dispatch on literals, concrete types (annotation or constructor destructuring), and generics, most-specific first
- single-constructor record types: typed fields, alphabetical order enforced, positional construction, destructuring both ways (positional patterns and keyed subset reads with rename-on-bind)
- no commas anywhere: flat juxtaposition application (`f a b`), space-separated enumerations
- arbitrary-precision `int`, string interpolation `"{expr}"`, lists with 1-based indexing, `.` pipe (piped value becomes the first argument), lambdas
- canonical form as grammar: indentation, spacing, blank-line placement, snake_case, alphabetical declarations and fields — all compile errors (see [tests/golden/errors](tests/golden/errors))
- nothing-wasted checks: unused bindings, unused expressions, and rebind-before-use are compile errors
- one name, one meaning: a binding may not shadow a declared function, type, or builtin
- constants: `tau = 6.28318` at top level; a value with no parameters is a constant, so `fn` always takes parameters and `main` is a constant description of the program
- modules: a directory is a module, split across any number of files sharing one namespace (`kanso test lib/json`); `_`-prefixed names are module-private, and an unused private declaration is a compile error

The error corpus in `tests/golden` matters as much as the success corpus — half this language's value is its compile errors.

### v0 interpretations and approximations

Decisions the spec leaves open (or that phase 1 approximates), flagged for revisit:

- **the endpoint rule is enforced at runtime**, not compile time — the real rule needs the §14.1 inference fixpoint, being formalized in [design/fixpoint.md](design/fixpoint.md)
- **generic parameters never bind `err`/`none`**; handle failure explicitly (literal `none`, `(err reason)`) or it propagates — a conservative stand-in for inferred pass-throughs
- **canonical declaration order**: types before functions, each alphabetical, overloads adjacent and most-specific first — an interpretation of the spec's "wherever order is semantically inert" rule
- **the pipe target parses as one application** and the piped value becomes its first argument; a non-callable target fails at runtime, not parse time
- `if cond, then, else` as a lazy call-shaped form is provisional (spec defers multi-way conditionals)
- not yet: typesets, modules/imports, maps, record update, `build` regions, processes, effect polymorphism, the LSP

## editor support

One TextMate bundle in [editors/](editors) highlights `.kso` in both JetBrains IDEs (Settings → Editor → TextMate Bundles → add `editors/kanso`) and VS Code (symlink into `~/.vscode/extensions`) — see [editors/README.md](editors/README.md).

## development

```
cargo test    # unit tests + golden-file corpus (examples, error diagnostics, --plan)
```

MIT licensed.
