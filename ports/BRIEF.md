# Ports: what each agent is asked to do

Twenty open-source projects, each rewritten in kanso by one agent. The point
is to use the language the way a working programmer would, on a program big
enough to need modules, tests, error handling and refactoring, and to write
down every place kanso got in the way. A language committee then reads all
twenty journals together and looks for the problems several projects hit
independently.

The port is the means. The journal is the product.

## The toolchain

    KANSO=/tmp/claude-0/kanso-main/kanso

That binary is the compiler on main. Do not build the compiler, do not edit
anything under /home/user/kanso, and do not run git. The verbs you need:

    $KANSO run <dir|file>              compile natively and run
    $KANSO run <dir|file> --interp     run on the interpreter (the oracle)
    $KANSO check <dir|file>            report what run would refuse
    $KANSO test <dir|file>             evaluate every test_* constant
    $KANSO build <dir|file>            native binary, dev tier
    $KANSO build <dir|file> --release  native binary, optimized

## Learning the language

kanso is unlike most languages, so read before writing.

- /home/user/kanso/docs/book/ch01.html to ch12.html and appa to appc: the
  book is the specification. Read chapters 1 to 6 before writing code, and the
  rest as you need them. Strip the HTML with
  `sed -e 's/<[^>]*>//g' ch03.html | less` or similar.
- /home/user/kanso/docs/book/samples/: every runnable example in the book,
  each with its expected output.
- /home/user/kanso/lib/: the standard library's source. This is where to look
  for what exists (text, list, math, regexp, json, os, io, net, net/http, path,
  time, sha256, bits, render, testing, expect).
- /home/user/kanso/examples/: short programs, one feature each.
- /home/user/kq/: a jq clone in kanso, the largest real program written so
  far. Read it for project layout, module structure and test style.

Write idiomatic kanso. Once you know the idiom the book teaches, use it. When
the idiom fights the problem, that is a journal entry, not a reason to write
around it silently.

## Where your work goes

    /home/user/wt-ports/ports/<name>/

Touch nothing outside that directory. Lay it out like a real project:

- `main.kso` (or a module directory with an entry) that runs the program.
- One or more module directories for the implementation.
- Tests: `test_*` constants under `kanso test`, plus fixture files with
  expected output.
- `check.sh`: runs every fixture three ways (`--interp`, `build`,
  `build --release`) and fails if any output differs from the expected file
  or from each other. Run it before you finish.
- `README.md`: what the original project is, what this port covers and what
  it leaves out, and how to run it. Credit the original. Write the code
  yourself from your understanding of the project; do not paste its source.
- `FRICTION.md`: the journal, described below.
- `bugs/`: a minimal reproduction for every compiler or runtime bug you hit,
  one file each, with a comment saying what it should do and what it does.
  Work around the bug and keep going.

## Scope

Aim for the real core of the project, not a toy. A useful target is 1,000 to
3,000 lines of kanso and enough fixtures that each feature is exercised. If
the language makes some part impossible, say so in the journal and port the
rest. Do not shrink the project to dodge friction: the friction is what we
are here to find.

## The journal

`FRICTION.md` has three parts.

**Entries.** One per distinct problem, recorded while you work, not
reconstructed at the end:

    ### F<n>: <short title>
    - kind: stdlib-gap | missing-feature | feature-to-cut | confusing-semantics
            | diagnostic | refactoring-hazard | aesthetics | performance
            | tooling | engine-bug
    - severity: blocker | major | minor | nit
    - where: <file:line>
    - wanted: <the code you wanted to write, or how another language says it>
    - wrote: <the code you had to write>
    - why it matters: <one to three sentences>

Every entry needs code that you actually compiled. A hunch about the language
is not an entry. Record small things too: a confusing error message, a name
you had to look up three times, a refactor that took ten edits where one
should do. Aesthetic objections are welcome; say plainly that they are
aesthetic.

**What worked well.** The features that made this port easier than it would
have been in the original language, with examples. The committee needs these
as much as the complaints, so it does not propose removing something people
relied on.

**Summary.** The five entries you would fix first, in order, and one
paragraph on what writing this program in kanso was like.

## Finishing

Before you report back, run `check.sh` and confirm it passes. Then reply with:
what you built, its line count (`find . -name '*.kso' | xargs wc -l`), the
number of tests and fixtures, whether all three engines agree, and your top
five journal entries.
