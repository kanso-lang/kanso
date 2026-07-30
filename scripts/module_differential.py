#!/usr/bin/env python3
"""Differential check of what a module lets through, and what it refuses.

A module is a directory of files sharing one namespace, `pub` is the whole
visibility system, and imports arrive qualified with enrollment making the
qualifier optional. Those rules interact — a name is visible or not depending
on which file declares it, whether it is `pub`, whether the reader is inside
the module or importing it, and whether the qualifier was written — and no
sweep was checking them.

The bug that prompted this was in exactly that seam: a call to a function
declared in a sibling file was not arity-checked at all, because the pass that
checks it sees one file and a module is several. That was found by accident.

Every case here is a whole module on disk, run through `kanso check` and, when
it should compile, through both engines. What is compared is the verdict and
the wording: two engines that accept different programs are two languages.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# (label, {relative path: contents}). `main.kso` is the entry.
CASES = [
    (
        "a sibling file's name is in scope",
        {
            "a.kso": 'pub fn greet _\n  "hello"\n',
            "main.kso": 'print "{greet 0}"\n',
        },
    ),
    (
        "a private name is in scope inside the module",
        {
            "a.kso": 'fn quiet _\n  "hush"\n\npub fn loud n\n  quiet n\n',
            "main.kso": 'print "{loud 0}"\n',
        },
    ),
    (
        "a type declared in one file, built in another",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": "pub fn origin _\n  point 0 0\n",
            "main.kso": 'print "{origin 0}"\n',
        },
    ),
    (
        "a field of a type from a sibling file",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": "pub fn across p\n  p.x\n",
            "main.kso": 'print "{across (point 3 4)}"\n',
        },
    ),
    (
        "an arm in each file of one group",
        {
            "a.kso": 'pub fn which 0\n  "zero"\n',
            "b.kso": 'pub fn which _\n  "any"\n',
            "main.kso": 'print "{which 0} {which 9}"\n',
        },
    ),
    (
        "a call to a sibling at the wrong arity",
        {
            "a.kso": "pub fn one _\n  1\n",
            "b.kso": "pub fn misuse k\n  one k 2\n",
            "main.kso": 'print "{misuse 1}"\n',
        },
    ),
    (
        "a call from the entry at the wrong arity",
        {
            "a.kso": "pub fn one _\n  1\n",
            "main.kso": 'print "{one 1 2}"\n',
        },
    ),
    (
        "a name declared twice in one module",
        {
            "a.kso": "pub fn twice _\n  1\n",
            "b.kso": "pub fn twice _\n  2\n",
            "main.kso": 'print "{twice 0}"\n',
        },
    ),
    (
        "a binding shadowing a sibling's declaration",
        {
            "a.kso": "pub fn taken _\n  1\n",
            "b.kso": "pub fn tries k\n  taken = 2\n  taken + k\n",
            "main.kso": 'print "{tries 1}"\n',
        },
    ),
    (
        "a type and a function sharing a name",
        {
            "a.kso": "pub type thing\n  n\n",
            "b.kso": "pub fn thing _\n  1\n",
            "main.kso": 'print "{thing 0}"\n',
        },
    ),
    (
        "a name nothing declares",
        {
            "a.kso": "pub fn known _\n  1\n",
            "main.kso": 'print "{unknown 0}"\n',
        },
    ),
    (
        "an entry file that is only statements",
        {
            "main.kso": 'print "{1}"\n',
        },
    ),
    (
        "a module with no entry",
        {
            "a.kso": "pub fn only _\n  1\n",
        },
    ),
    (
        "a private name used nowhere",
        {
            "a.kso": "fn unused _\n  1\n\npub fn used _\n  2\n",
            "main.kso": 'print "{used 0}"\n',
        },
    ),
    (
        "a getter of a sibling's type, as a value",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": 'import "std/list"\n\npub fn xs ps\n  list/to_list (list/map ps _.x)\n',
            "main.kso": 'print "{xs [(point 1 2) (point 3 4)]}"\n',
        },
    ),
]


def run(root, verb, engine=()):
    """The engine flag follows the path — `kanso run <dir> --interp`. Putting
    it first makes the CLI print its usage, which is a different exit code on a
    different stream, and reads as the two engines disagreeing about a program
    neither of them was asked about.

    Both engines are given the SAME directory, because a diagnostic names the
    module it is about and two temporary directories have two names."""
    try:
        done = subprocess.run(
            [str(KANSO), verb, str(root), *engine],
            cwd=root.parent,
            capture_output=True,
            text=True,
            timeout=30,
        )
    except subprocess.TimeoutExpired:
        return "TIMED OUT", "", ""
    return done.returncode, done.stdout, done.stderr


def written(work, files):
    root = pathlib.Path(work) / "m"
    root.mkdir()
    for name, text in files.items():
        (root / name).write_text(text)
    return root


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    disagreed, hung = [], []
    checked = 0
    for label, files in CASES:
        with tempfile.TemporaryDirectory() as work:
            root = written(work, files)
            # `check` has one answer, so it is asked once; `run` is asked of
            # both engines, and a module that does not compile is not run
            verdict = run(root, "check")
            if verdict[0] == "TIMED OUT":
                hung.append((label, verdict, verdict))
                continue
            checked += 1
            if verdict[0] != 0:
                continue
            native = run(root, "run")
            interp = run(root, "run", ["--interp"])
        if "TIMED OUT" in (native[0], interp[0]):
            hung.append((label, native, interp))
        elif native != interp:
            disagreed.append((label, native, interp))
    print(
        f"{len(CASES)} modules, {checked} checked, {len(disagreed)} disagree, "
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
