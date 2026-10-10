# ugit, in kanso

A small git, written in kanso. It keeps a content-addressed object store of
blobs, trees and commits under `.ugit/`, with branches, tags and HEAD as
refs, a staging index, and the everyday commands: `init`, `add`, `commit`,
`status`, `diff`, `log`, `show`, `branch`, `tag`, `checkout`, `reset` and a
three-way `merge` that writes conflict markers.

The original is Nikita Leshenko's **ugit**, "DIY Git in Python"
(https://www.leshenko.net/p/ugit/), a tutorial that builds git's core one
step at a time. This port follows its design: objects stored whole under
their hash with a `kind NUL content` header, trees as one `kind id name`
line per entry, refs as small files that may point at other refs, a JSON
index, and the same history walk for `log` and `merge-base`. The code was
written from that design, not translated from its source. Where ugit
shells out to `diff` and `diff3`, this port does the work itself: a Myers
diff (delta/myers.kso), unified hunks (delta/unified.kso) and a diff3
merge (delta/merge3.kso). Output and messages follow git's wording where
the two overlap.

## Running it

The compiler is the only dependency.

    KANSO=/tmp/claude-0/kanso-main/kanso
    $KANSO build /path/to/ugit --release      # writes ./ugit
    ./ugit init
    ./ugit add .
    ./ugit commit -m "first"
    ./ugit log

or, without building, `$KANSO run /path/to/ugit -- status`, and
`--interp` before the `--` to use the interpreter.

Commit times come from `time/now`, so setting `KANSO_NOW` (milliseconds
since the epoch) pins them; the fixtures do this so that commit ids are
the same on every engine and every run. `UGIT_AUTHOR` sets the author
line, which defaults to `ugit <ugit@example.com>`.

## Commands

    init                         create .ugit/ with an empty master branch
    add <path>...                stage files; a directory stages everything
                                 below it, deletions included
    commit [-m <message>]        record the index; -m may be left out while
                                 a merge is waiting, which uses MERGE_MSG
    status                       staged, unstaged and untracked paths
    diff                         index against working tree
    diff --cached [<rev>]        HEAD (or <rev>) against the index
    diff <rev>                   a commit against the working tree
    diff <rev> <rev>             one commit against another
    log [--oneline] [<rev>]      history with branch and tag decorations
    show [<rev>]                 a commit and its diff against its parent
    branch [<name> [<rev>]]      list branches, or create one
    tag [<name> [<rev>]]         list tags, or create one
    checkout <branch>            switch branches
    checkout <rev>               detach HEAD at a commit
    checkout -b <name> [<rev>]   create a branch and switch to it
    checkout -- <path>...        restore files from the index
    reset [--hard] [<rev>]       move the branch, reset the index (and files)
    merge <rev>                  fast-forward, or a three-way merge that
                                 commits when clean and stops with conflict
                                 markers when not

Plumbing: `hash-object <file>`, `cat-file (-t | -p | <kind>) <object>`,
`write-tree`, `rev-parse <name>`, `merge-base <a> <b>`.

A name may be a branch, a tag, `HEAD` or `@`, a full object id or a prefix
of at least four hex digits, followed by any number of `^` or one `~n`.

Checkout, merge and reset refuse, as git does, to overwrite a file with
local changes that the target commit also changes, or an untracked file the
target would replace.

## What it leaves out

- SHA-1 and git's on-disk formats. ugit has its own, and this port hashes
  with SHA-256 (std/sha256), so a `.ugit` is not a `.git`.
- The original's `k` (a graphviz picture of the history) and its `fetch`
  and `push` between local repositories.
- Index stages. A conflicted file is written to the working tree with
  markers and the index keeps HEAD's version until it is added, so
  `status` shows it as modified rather than as unmerged.
- Combined diffs: `show` on a merge commit diffs against the first parent.
- File modes, symlinks, ignore files and binary files. Every file is read
  as UTF-8 text.
- Log ordering by date. `log` walks history the way the original does: a
  commit, then its first parent, with other parents queued at the back.

## Layout

    main.kso           the entry: arguments into ugit/run
    ugit/              the commands, argument parsing, status, log, diff,
                       checkout, merge, reset, date formatting
    store/             objects, trees, commits, refs, the index, name
                       resolution and the working tree
    delta/             pure text algorithms: lines, Myers diff, unified
                       hunks, three-way merge
    steps/             two helpers for running effects over a list in order
    fixtures/cases/    scripted sessions (*.sh) and their output (*.out)
    check.sh           unit tests, then every fixture on three engines
    bugs/              minimal reproductions of compiler and runtime bugs
    FRICTION.md        the journal of where kanso got in the way

## Tests

    sh check.sh              unit tests, then every fixture three ways
    sh check.sh merge_ff     only the named fixtures
    sh check.sh --bless      rewrite the expected output from the interpreter

Each fixture is a shell session run in a fresh empty directory, once on the
interpreter, once as a dev binary and once as a release binary. The script
fails if any engine's output differs from the expected file. The unit tests
(`kanso test delta`, `store`, `ugit`) cover the diff and merge algorithms,
commit encoding, date formatting, argument parsing, change detection and
the merge decision table.
