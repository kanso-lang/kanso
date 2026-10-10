# mustache, in kanso

A [Mustache](https://mustache.github.io/) template engine written in kanso,
tested against the cases of the
[mustache spec](https://github.com/mustache/spec). Mustache is Chris
Wanstrath's logic-less template language; the original implementation is the
Ruby gem `mustache`, and the spec is maintained by the mustache organization
on GitHub. This port was written from my understanding of the language and
the spec. No code was copied, and the fixtures in `fixtures/spec/` were
written out from memory of the spec's YAML files, so a few test names or
details may differ from the published suite.

## What it covers

All of the spec's required modules, and three of its optional ones:

| suite | tests | |
|---|---:|---|
| interpolation | 42 | `{{name}}`, `{{{name}}}`, `{{&name}}`, dotted names, `{{.}}` |
| sections | 34 | truthy values, lists, nested contexts, implicit iterators |
| inverted | 22 | `{{^name}}` |
| partials | 12 | `{{>name}}`, recursion, standalone indentation |
| comments | 12 | `{{! ... }}`, single and multi-line |
| delimiters | 14 | `{{=<% %>=}}` |
| lambdas (optional) | 10 | interpolation and section lambdas, alternate delimiters |
| dynamic names (optional) | 18 | `{{>*name}}` |
| inheritance (optional) | 21 | `{{<parent}}`, `{{$block}}` |
| port extensions | 5 | the lambda render helper described below |

Standalone lines (a section, comment, partial or delimiter tag alone on its
line takes the whole line with it) follow the spec, including `\r\n` line
endings and a template that ends without a newline.

Beyond the spec, from the Ruby gem and mustache.js:

- `tokens`, which prints the parsed tree, like the gem's `--tokens`.
- `--strict`, the gem's `raise_on_context_miss`: a variable or section whose
  name is not in the data stops the render with its line number. Inverted
  sections are exempt, since testing for absence is their job.
- `--no-escape`, for output that is not HTML.
- Partials from a directory of `name.mustache` files, defaulting to the
  template's own directory, as the gem does.
- Parse errors name the line and column of the tag at fault, and an unclosed
  section names where it opened.

### Lambdas

kanso functions are pure, so lambdas cannot keep a counter in a variable,
which is what the spec's "Interpolation - Multiple Calls" test does. In this
port a lambda is `mustache/lambda_of f`, where `f` takes three arguments:

1. the section's raw source (`""` for an interpolated lambda),
2. a state value the render threads through every lambda call, starting at 0,
3. a function that renders template text in the current context and answers
   the string, as mustache.js passes `render` to a section lambda.

It answers `mustache/answered state text`, and the text is rendered as a
template, with the section's delimiters for a section lambda and the
defaults for an interpolated one. The counter in the spec becomes
`(_ calls _ -> mustache/answered (calls + 1) "{calls + 1}")`.

JSON cannot hold a function, so the spec suite writes each lambda as
`{"__tag__": "code", "kanso": "<name>"}`, the shape the spec uses for its
per-language source, and `spec/lambdas.kso` holds the functions by name.

## What it leaves out

- Inheritance's indentation rules: the spec's "Standalone parent",
  "Standalone block", "Block reindentation", "Intrinsic indentation",
  "Nested block reindentation" and "Override parent with newlines" tests are
  not in the suite. They need a line holding only a parent or block tag pair
  to count as standalone, and block content to be re-indented where it is
  used. This port judges each tag on its own line separately, which is what
  the required modules ask for.
- YAML data. The Ruby gem's command line reads YAML; this one reads JSON,
  through std/json.
- Values print the way kanso prints them: the JSON number `12.0` renders as
  `12.0`, and a list or map interpolated whole renders in kanso's notation.
  The spec does not test either case.

## Layout

    main.kso               entry: hands the arguments to cli/main
    mustache/              the engine
      nodes.kso            the parsed tree's node types
      parse.kso            template bytes to nodes, standalone lines, errors
      render.kso           nodes and data to text: context stack, lambdas,
                           partials and their parse cache, inheritance
      outline.kso          the tree as text, for `tokens`
      mustache_test.kso    unit tests
    spec/                  the spec runner and the lambda registry
    cli/                   the command line
    fixtures/
      spec/                suites in the spec's JSON format, with .expected
      render/              templates, data and partials, with expected output
      errors/              templates and data that must fail, and how
      tokens/              templates and their printed trees
      options/             --strict and --no-escape
      cli/                 usage errors
    check.sh               runs everything on all three engines
    FRICTION.md            the journal of what kanso got in the way of
    bugs/                  reduced reproductions of compiler and runtime bugs

## Running it

The toolchain is `/tmp/claude-0/kanso-main/kanso`; set `KANSO` to use another.

    kanso test mustache                                   # unit tests
    kanso run . -- spec fixtures/spec/sections.json       # one suite
    kanso run . -- render fixtures/render/page/template.mustache \
        fixtures/render/page/data.json                    # one page
    kanso run . -- render --strict t.mustache data.json partials.json
    kanso run . -- tokens fixtures/render/tree/entry.mustache
    sh check.sh                                           # everything

`kanso build` names the binary after the directory, and this directory holds
a module of the same name, so build from somewhere else:
`mkdir -p build && cd build && kanso build ..`.

`check.sh` runs the unit tests, then builds a dev and a release binary and
runs every fixture on the interpreter and on both binaries. It fails if any
engine's stdout, stderr or exit status differs from the expected files or
from another engine's. `sh check.sh --bless` rewrites the expected files from
the interpreter; review the result before keeping it.
