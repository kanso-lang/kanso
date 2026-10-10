# datalog — a small Datalog engine in kanso

This is a bottom-up Datalog engine in the style of
[Soufflé](https://souffle-lang.github.io/) and Frank McSherry's
[datafrog](https://github.com/rust-lang/datafrog). Soufflé compiles Datalog
programs to parallel C++ and is used for static analysis of Java and smart
contracts; datafrog is the small semi-naive engine inside the Rust compiler's
borrow checker. Both evaluate a program bottom-up: start from the facts, apply
every rule until nothing new appears, and read the answers out of the result.
This port follows the same design. The code is written from the published
descriptions of those systems and of semi-naive evaluation, not from their
source.

It was written as one of twenty ports whose purpose is to find where kanso
gets in a working programmer's way. The record of that is in
[FRICTION.md](FRICTION.md).

## What it does

A program is a list of facts, rules, queries and directives:

```
% transitive closure
edge(a, b).
edge(b, c).
path(X, Y) :- edge(X, Y).
path(X, Z) :- path(X, Y), edge(Y, Z).

?- path(a, X).
.output path
```

```
$ kanso run main.kso -- closure.dl
?- path(a, X).
  X = b
  X = c
  (2 answers)
.output path (3 tuples)
  path(a, b).
  path(a, c).
  path(b, c).
-- evaluation
stratum 1: path: 3 rounds, 3 new
relations: edge 2, path 3
total: 5 tuples after 3 rounds
```

The language:

- **Terms.** Variables start with an uppercase letter or an underscore. `_`
  on its own is a wildcard. Constants are integers (`42`, `-7`), bare
  lowercase words (`alice`) and quoted strings (`"Ann Lee"`, with `\n`, `\t`,
  `\"` and `\\` escapes). A quoted string that spells a bare word is the same
  constant as the word.
- **Rules.** `head :- literal, literal, ...` where a literal is an atom
  `p(X, a)`, a negated atom `!p(X, _)` or `not p(X, _)`, or a comparison
  `X = Y`, `X != Y`, `X < Y`, `X <= Y`, `X > Y`, `X >= Y`. Comparisons order
  integers numerically, strings by their bytes, and every integer before every
  string. `=` with one side unbound binds it. Atoms of arity zero (`ready.`,
  `go :- ready, !raining.`) are allowed.
- **Queries.** `?- path(a, X).` prints the distinct bindings of the query's
  variables, sorted, or `yes`/`no` for a query with no variables.
- **Directives.** `.decl name(a, b)` declares a relation (column types such as
  `a: number` are accepted and ignored). `.output name` prints a relation.
  `.printsize name` prints its size. `.input name` reads tuples from
  `name.facts` beside the program, and `.input name "file"` from a named
  file, one tuple per line with tab-separated columns, as Soufflé does.
- **Comments.** `%` and `//` to the end of the line, and `/* ... */`.

Before evaluating, the engine checks the program and refuses it with a
message and exit status 1 when:

- a relation is used with two different arities;
- a body reads a relation that no fact, rule, `.input` or `.decl` defines;
- a fact holds a variable, or a head holds `_`;
- a rule is not range-restricted: a variable in the head, in a negated atom
  or in a comparison is bound by no positive atom (directly or through `=`);
- negation is not stratifiable. The message names the cycle:

```
datalog: win.dl:7: the program cannot be stratified: `win` depends on its own negation at line 7
datalog: cycle.dl:4: the program cannot be stratified: `p` depends on the negation of `q` at line 4, and `q` depends on `p` (q -> r -> p)
```

Syntax errors give a line and a column.

## How it evaluates

1. **Stratification.** The rules' dependency graph is split into strongly
   connected components, ordered so that every relation a component reads,
   positively or under negation, is complete before the component runs.
   Each component is a stratum.
2. **Interning.** Every constant gets a small integer id, handed out in
   sorted order (integers, then strings), so comparing ids compares
   constants. A tuple is stored as one integer, its *code*: the ids written as
   the digits of a number in base (number of constants + 1). Codes are map
   keys, and sorting codes sorts tuples.
3. **Join plans.** Each rule compiles to a list of steps: scans of positive
   atoms in the order written, each comparison and negation placed as soon as
   its variables are bound. A scan whose bound columns form a key reads one
   bucket of a hash index on those columns; the indexes a program needs are
   worked out from its plans before evaluation starts.
4. **Semi-naive iteration.** The first round of a stratum runs every rule over
   the full relations. Each later round runs, for every rule and every body
   atom that reads the stratum's own relations, a plan with that atom reading
   only the tuples new in the previous round. The stratum ends when a round
   derives nothing new.

The `-- evaluation` block reports, for each stratum, its relations, the
number of rounds and the tuples it derived, then every relation's size. These
numbers are deterministic, so they are part of what the three engines must
agree on.

## What it leaves out

Soufflé's aggregates (`count`, `sum`, `min`, `max`), arithmetic in heads and
bodies, typed columns, records and ADTs, components, `.output` to files,
choice and subsumption, provenance, and parallel evaluation. Join order is the
order the rules are written in; there is no query planner and no magic-sets
rewriting.

## Running it

The compiler is `/tmp/claude-0/kanso-main/kanso` in this environment.

```
kanso run main.kso -- tests/points_to.dl          # natively
kanso run main.kso --interp -- tests/points_to.dl # on the interpreter
kanso run main.kso < tests/tc_small.dl            # program on stdin
kanso run main.kso -- --time tests/tc_large.dl    # time on stderr
kanso test datalog                                # unit tests
sh check.sh                                       # every fixture, 3 engines
sh bench.sh                                       # timings, 3 engines
kanso run gen/main.kso -- tree 2500               # generate a large program
```

`check.sh` runs the unit tests, builds a dev and a release binary, and runs
every `tests/*.dl` (plus a missing file and a program on standard input) on
the interpreter and both binaries. Each run's output, error output and exit
status must match `tests/<name>.expected` and each other. `sh check.sh
--update` rewrites the expected files from the interpreter.

## The programs

| fixture | what it is |
|---|---|
| `tc_small` | transitive closure over a graph with a cycle |
| `tc_large` | transitive closure over 2,998 edges (`gen tree 2500`): 24,331 path tuples in 15 rounds |
| `points_to` | Andersen-style points-to analysis of a small program with fields and a linked list |
| `same_generation` | same generation and cousins over a family tree, with negation |
| `same_generation_large` | same generation over 400 generated people: 18,014 tuples |
| `negation` | eight strata of reachability, cycles and dead ends |
| `recursion` | mutual recursion, non-linear closure, symmetric closure |
| `comparisons`, `constants`, `atoms` | the language's corners |
| `inputs` | `.input` from tab-separated files, and `.printsize` |
| `error_*` | one refusal each: syntax, arity, unsafe rules, stratification, missing input |

The large programs were cross-checked against an independent computation of
the closure and of same-generation in Python, which agreed on every count and
every query answer.

## Timings

Wall-clock milliseconds from reading the source to printing the last line, as
reported by `--time`, on the container these ports were written in
(2026-10-10). Single runs; the release figures vary by about 20% between runs.

| program | tuples | interpreter | dev build | release build |
|---|---|---|---|---|
| `tc_large` (2,998 edges) | 27,329 | 2,664 | 89 | 52 |
| `same_generation_large` (400 people) | 18,813 | 1,820 | 120 | 39 |
| `gen tree 10000` (11,999 edges, not a fixture) | 127,821 | 24,115 | 671 | 488 |

The first working version took 1,197 ms in release on `tc_large`; FRICTION.md
F18 and F19 explain where the other 1,145 went.

## Layout

```
main.kso           entry: hands the arguments to datalog/main
datalog/           the engine, one module
  syntax.kso       the syntax tree
  lexer.kso        source text to tokens
  parser.kso       tokens to clauses
  show.kso         clauses back to source text
  check.kso        arities, definitions, range restriction
  strata.kso       dependency graph, components, stratification
  intern.kso       constants to sorted ids
  plan.kso         rules to join plans
  relation.kso     tuple codes, sets and hash indexes
  eval.kso         semi-naive evaluation
  answer.kso       queries, .output, the evaluation report
  facts.kso        reading .input files
  run.kso          the pipeline and the command line
  *_test.kso       unit tests (`kanso test datalog`)
gen/               generators for the large fixtures
tests/             fixtures and their expected output
bugs/              reproductions of compiler bugs met on the way
```
