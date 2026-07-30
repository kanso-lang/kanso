#!/usr/bin/env python3
"""Differential check of how descriptions COMBINE.

A description is the most novel thing in the language and the least probed.
The other sweeps ask what a value renders as, what a std function answers, and
what a wrong call says; none of them asks what happens when two effects are
sequenced, bound, joined, or nested inside one another. Those combinations are
written twice — the interpreter's executor and the C runtime's `k_run` — and
the two have no shared code, so they can drift anywhere the corpus does not
happen to go.

What is probed is the shape of the combination, never the wall clock: a join
runs its arms concurrently and interleaving is the scheduler's business, so
every probe here either has one arm producing output or collects the arms'
answers rather than their order.

Both engines must agree on the bytes and on the exit code. A disagreement is a
finding, and so is a legal combination that never returns.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

MODULES = ["io", "list", "math", "text"]


def source_for(body):
    """A probe is a whole program, so it carries exactly the imports it uses
    and no others — an unused import is an error, and an error compares equal
    on both engines, which would read as agreement."""
    used = [m for m in MODULES if f"{m}/" in body]
    imports = "".join(f'import "std/{m}"\n' for m in used)
    head = f"{imports}\n" if imports else ""
    # a single-expression constant is written on one line; a body with a
    # binding in it is indented under the name
    if "\n" in body:
        return head + "pub play =\n  " + body + "\n"
    return head + "pub play = " + body + "\n"

# (label, the body of `pub play`). Each is run on both engines. A body with a
# newline in it is a block, and adjacent statements in a block are a parallel
# group — the scheduler is deterministic, so their output is comparable.
PROBES = [
    # sequencing and binding at their simplest, so a difference here is not
    # hidden inside a harder case
    ("write then write", 'io/write "a" >> io/write "b"'),
    ("three writes", 'io/write "a" >> io/write "b" >> io/write "c"'),
    ("bind ignores its value", 'io/write "a" . (_ -> io/write "b")'),
    ("bind passes its value", 'io/env "PATH" . (v -> io/write "{v == none}")'),
    ("bind chained twice", 'io/write "a" . (_ -> io/write "b") . (_ -> io/write "c")'),
    ("a value re-enters", 'io/write "" . (_ -> 7) . (n -> io/write "{n}")'),
    ("a value computed on", 'io/write "" . (_ -> 7) . (n -> io/write "{n + 1}")'),
    # nesting: a description built inside a bind, and a bind inside a bind
    ("bind returns a bind", 'io/write "a" . (_ -> io/write "b" . (_ -> io/write "c"))'),
    ("seq inside a bind", 'io/write "a" . (_ -> io/write "b" >> io/write "c")'),
    ("bind after a seq", 'io/write "a" >> io/write "b" . (_ -> io/write "c")'),
    # a parallel group: adjacent statements in a block
    ("two lines", 'io/write "a"\n  io/write "b"'),
    ("three lines", 'io/write "a"\n  io/write "b"\n  io/write "c"'),
    ("a wall between groups", 'io/write "a"\n  io/write "b" >> io/write "c"'),
    ("a bind beside a write", 'io/write "a"\n  io/write "b" . (_ -> io/write "c")'),
    ("a group of binds", 'io/env "A" . (_ -> io/write "a")\n  io/env "B" . (_ -> io/write "b")'),
    # the effects that read the world, pinned so they answer the same anywhere
    ("an unset variable", 'io/env "KANSO_NO_SUCH_VAR" . (v -> io/write "{v}")'),
    ("a path that is not there", 'io/exists "/no/such/path" . (v -> io/write "{v}")'),
    ("the args of no args", 'io/args . (a -> io/write "{length a}")'),
    ("a directory that is not there", 'io/list_dir "/no/such/dir" . (v -> io/write "{v}")'),
    # a failure inside a combination: it takes the chain with it, and the two
    # engines have to agree on where it stops and what reaches the executor
    ("exit inside a bind", 'io/write "a" . (_ -> io/exit 3)'),
    ("exit before a write", 'io/write "a" . (_ -> io/exit 3) . (_ -> io/write "b")'),
    ("exit in a parallel group", 'io/write "a"\n  io/exit 3'),
    ("a read that fails", 'io/read_file "/no/such/file" . (_ -> io/write "read")'),
    ("a failed read, bound twice", 'io/read_file "/no/f" . (_ -> io/write "a") . (_ -> io/write "b")'),
    ("a failed read beside a write", 'io/read_file "/no/such/file"\n  io/write "b"'),
    # a description held in a local, and one used twice
    ("a description in a local", 'w = io/write "a"\n  w >> io/write "b"'),
    ("the same description twice", 'w = io/write "a"\n  w >> w'),
    ("a description from a list", 'ws = [(io/write "a") (io/write "b")]\n  ws[1]! >> ws[2]!'),
    ("a description built by a fn", 'io/write "a" >> io/write_err "e"'),
    # descriptions and the ordinary value world
    ("a description compares", 'w = io/write "a"\n  io/write "{w == w}" >> w'),
    ("a description renders", 'w = io/write "a"\n  io/write "{w}" >> w'),
]


def run(body, engine):
    source = source_for(body)
    with tempfile.TemporaryDirectory() as work:
        entry = pathlib.Path(work) / "probe.kso"
        entry.write_text(source)
        try:
            done = subprocess.run(
                [str(KANSO), "play", "probe.kso", *engine],
                cwd=work,
                capture_output=True,
                text=True,
                timeout=20,
            )
        except subprocess.TimeoutExpired:
            return "TIMED OUT", "", ""
    return done.returncode, done.stdout, done.stderr


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    disagreed, hung = [], []
    for label, body in PROBES:
        native = run(body, [])
        interp = run(body, ["--interp"])
        if native[0] == "TIMED OUT" or interp[0] == "TIMED OUT":
            hung.append((label, native, interp))
        elif native != interp:
            disagreed.append((label, native, interp))
    print(
        f"{len(PROBES)} combinations, {len(disagreed)} disagree, "
        f"{len(hung)} never returned"
    )
    for label, native, interp in hung[:10]:
        print(f"  never returned: {label}")
    for label, native, interp in disagreed[:20]:
        print(f"  {label}")
        print(f"    native: code={native[0]} out={native[1]!r} err={native[2].strip()[:90]!r}")
        print(f"    interp: code={interp[0]} out={interp[1]!r} err={interp[2].strip()[:90]!r}")
    if disagreed or hung:
        print()
        print("the interpreter is the oracle: these are native's to match.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
