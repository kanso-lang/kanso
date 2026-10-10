# make, in kanso

A port of the core of GNU make to kanso. GNU make is the Free Software
Foundation's implementation of the Unix `make` utility, first written by
Richard Stallman and Roland McGrath and maintained for many years by Paul
Smith. This port was written from the GNU make manual and from watching GNU
make 4.3 run, not from its source. The fixtures were checked against GNU make
4.3 run as `make -rR`, and 79 of the 82 match it byte for byte. The three
that do not are the usage message, which lists only the options this port
has, and the two fixtures where this port refuses `define` and a
target-specific variable, which GNU make supports.

## Running it

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO run . --interp -- -n all        # on the interpreter
    $KANSO build . --release && ./make -k  # as a native binary

It reads `GNUmakefile`, `makefile` or `Makefile` from the current directory,
or the files named with `-f`, and runs recipes with `/bin/sh -c`.

`sh check.sh` runs the unit tests and every fixture on the interpreter, the
dev build and the release build, and fails unless all three agree with each
other and with the fixture's `expected` file. `sh check.sh --gnu` also runs
GNU make on each fixture and prints where it differs.

## What it covers

- Reading: logical lines with backslash continuation, comments and `\#`,
  rules with several targets, prerequisites and `|` order-only
  prerequisites, a recipe after `;` or on tab-indented lines, rules merged
  across lines (the rule with the recipe puts its prerequisites first, and a
  second recipe overrides the first with GNU make's two warnings), wildcards
  in targets and prerequisites, and a line that expands to a rule.
- Variables: `=`, `:=`, `::=`, `+=`, `?=` and `!=`; command-line `VAR=value`
  and `override`; `export`, passed to recipes; the environment as a fallback;
  `$(name)`, `${name}`, `$x`, `$$`, computed names, and substitution
  references `$(v:.c=.o)` and `$(v:%.c=%.o)`.
- Automatic variables: `$@ $< $^ $+ $? $| $*` and the `D` and `F` forms.
- Functions: `subst patsubst strip findstring filter filter-out sort word
  wordlist words firstword lastword dir notdir suffix basename addprefix
  addsuffix join wildcard foreach if or and call value origin shell info
  warning error`.
- Conditionals: `ifeq`, `ifneq` (both argument spellings), `ifdef`,
  `ifndef`, `else`, `else ifeq ...`, nesting, and their errors.
- `include`, `-include` and `sinclude`, with globs; an include that is
  missing but has a rule is made and every makefile is read again, as GNU
  make restarts.
- Rules: explicit rules, pattern rules (directory-aware, shortest stem first,
  chained through intermediate files, which are deleted at the end with
  `rm`), static pattern rules, `.PHONY`, the default goal, and the
  no-recipe, no-prerequisite target that is always remade.
- The update walk: modification times (read with `stat`; see FRICTION F15),
  phony targets, order-only prerequisites, a recipe that leaves its target
  unchanged not forcing its dependents, circular dependencies dropped with
  a message.
- Recipes: each line in its own shell, `@`, `-` and `+` prefixes, and the
  whole recipe expanded before its first line runs.
- Options: `-n`/`--just-print`/`--dry-run`/`--recon`, `-k`/`--keep-going`,
  `-s`, `-i`, `-B`, `-f FILE`/`--file=`/`--makefile=`, flags combined as in
  `-nk`.
- Messages: "No rule to make target", "Nothing to be done for", "is up to
  date", "Target ... not remade because of errors", recipe errors with and
  without "(ignored)", "missing separator" (with the 8-spaces hint),
  "recipe commences before first target", "missing 'endif'", "extraneous
  'else'", "Recursive variable ... references itself", "unterminated variable
  reference", "No targets", and "No targets specified and no makefile found",
  each pointing at the line GNU make points at.

## What it leaves out

- GNU make's built-in rules and variables: this port behaves like
  `make -rR`.
- Target-specific and pattern-specific variables, `define`/`endef` and
  `vpath`. Reading one stops with a message saying it is not supported.
- Double-colon rules, which are read as ordinary rules.
- Parallel jobs (`-j`), `-C`, `-q`, `-t`, `-p`, `-e`, `-W`, `-o`,
  `.ONESHELL`, `.SILENT`, `.IGNORE`, `.DELETE_ON_ERROR`, `.SECONDARY` and the
  other special targets except `.PHONY`, `$(eval)`, `$(file)`, `CURDIR`,
  `MAKEFLAGS` and recursive make.
- A recipe's output is shown when the recipe line finishes, stdout and then
  stderr, because `os/run` captures a child's output instead of handing it
  make's own (FRICTION F16).

## Layout

    main.kso        the entry: hands the arguments to cli
    cli/            options, finding the makefile, remaking includes, goals
    reader/         logical lines, line kinds, assignments, conditionals,
                    include, and the rules gathered into the database
    db/             the database: variables, rules, pattern rules, phony set
    expand/         variable expansion and make's functions
    update/         the update walk, pattern-rule search, running recipes
    glob/           shell wildcards for $(wildcard) and rule lines
    words/          text helpers std/text does not have
    report/         make's messages
    fixtures/       82 end-to-end cases: cmd, in/, optional times, expected
    bugs/           reductions of the compiler and runtime bugs met
    FRICTION.md     the journal
