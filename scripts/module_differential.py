#!/usr/bin/env python3
"""What a module lets through, what it refuses, and whether the engines agree.

A module is a directory of files sharing one namespace, `pub` is the whole
visibility system, and imports arrive qualified with enrollment making the
qualifier optional. Those rules interact — a name is visible or not depending
on which file declares it, whether it is `pub`, whether the reader is inside
the module or importing it, and whether the qualifier was written.

Two questions are asked of every case, because this seam has produced bugs of
both kinds. The first is agreement: both engines get the same directory and
must answer identically. The second is the verdict itself — a module that
compiles must print what it is supposed to print, and one that is refused must
be refused for the stated reason. Agreement alone cannot see a rule that is
wrong in both engines, and this surface has now had one of those.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# (label, {file: contents}, verdict, expected). Verdicts:
#   prints        — compiles, and running it writes exactly `expected`
#   check refuses — `kanso check` fails, saying something containing `expected`
#   run refuses   — checks clean, and running it fails with `expected`
CASES = [
    (
        "a sibling file's name is in scope",
        {"a.kso": 'pub fn greet _\n  "hello"\n', "main.kso": 'print "{greet 0}"\n'},
        "prints",
        "hello\n",
    ),
    (
        "a private name is in scope inside the module",
        {
            "a.kso": 'fn quiet _\n  "hush"\n\npub fn loud n\n  quiet n\n',
            "main.kso": 'print "{loud 0}"\n',
        },
        "prints",
        "hush\n",
    ),
    (
        "a type declared in one file, built in another",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": "pub fn origin _\n  point 0 0\n",
            "main.kso": 'print "{origin 0}"\n',
        },
        "prints",
        "point 0 0\n",
    ),
    (
        "a field of a type from a sibling file",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": "pub fn across p\n  p.x\n",
            "main.kso": 'print "{across (point 3 4)}"\n',
        },
        "prints",
        "3\n",
    ),
    (
        "an arm in each file of one group",
        {
            "a.kso": 'pub fn which 0\n  "zero"\n',
            "b.kso": 'pub fn which _\n  "any"\n',
            "main.kso": 'print "{which 0} {which 9}"\n',
        },
        "prints",
        "zero any\n",
    ),
    (
        "a getter of a sibling's type, as a value",
        {
            "a.kso": "pub type point\n  x\n  y\n",
            "b.kso": 'import "std/list"\n\npub fn xs ps\n  list/to_list (list/map ps _.x)\n',
            "main.kso": 'print "{xs [(point 1 2) (point 3 4)]}"\n',
        },
        "prints",
        "[1 3]\n",
    ),
    (
        "a type and a function sharing a name",
        {
            "a.kso": "pub type thing\n  n\n",
            "b.kso": "pub fn thing _\n  1\n",
            "main.kso": 'print "{thing 0}"\n',
        },
        "prints",
        "thing 0\n",
    ),
    (
        "an entry file that is only statements",
        {"main.kso": 'print "{1}"\n'},
        "prints",
        "1\n",
    ),
    (
        "a private name used nowhere",
        {
            "a.kso": "fn unused _\n  1\n\npub fn used _\n  2\n",
            "main.kso": 'print "{used 0}"\n',
        },
        "prints",
        "2\n",
    ),
    (
        "a call to a sibling at the wrong arity",
        {
            "a.kso": "pub fn one _\n  1\n",
            "b.kso": "pub fn misuse k\n  one k 2\n",
            "main.kso": 'print "{misuse 1}"\n',
        },
        "check refuses",
        "error[arity]: no 2-argument arm of `one` (arms take 1)",
    ),
    (
        "a call from the entry at the wrong arity",
        {"a.kso": "pub fn one _\n  1\n", "main.kso": 'print "{one 1 2}"\n'},
        "check refuses",
        "error[arity]: no 2-argument arm of `one` (arms take 1)",
    ),
    (
        "an arm no call can reach",
        {
            "a.kso": "pub fn twice _\n  1\n",
            "b.kso": "pub fn twice _\n  2\n",
            "main.kso": 'print "{twice 0}"\n',
        },
        "check refuses",
        "error[dispatch]: overlapping overloads of `twice` are illegal",
    ),
    (
        "a binding shadowing a sibling's declaration",
        {
            "a.kso": "pub fn taken _\n  1\n",
            "b.kso": "pub fn tries k\n  taken = 2\n  taken + k\n",
            "main.kso": 'print "{tries 1}"\n',
        },
        "check refuses",
        "error[name]: `taken` is already a declaration; rename the binding",
    ),
    (
        "a name nothing declares",
        {"a.kso": "pub fn known _\n  1\n", "main.kso": 'print "{unknown 0}"\n'},
        "check refuses",
        "error[name]: unknown name `unknown`",
    ),
    (
        "a module with no entry",
        {"a.kso": "pub fn only _\n  1\n"},
        "run refuses",
        "error[name]: a program defines `main`",
    ),
]

# Behaviour that is wrong, and known to be wrong. Recorded the way
# tests/golden/wasm_gaps.txt records an engine's declines: a defect written
# down stops being a surprise, and a defect that quietly changes shape is how
# it survives a rewrite. An entry here that stops matching fails this sweep,
# which is the reminder to settle it and delete the entry — never to update
# the expectation to whatever it does now.
KNOWN_WRONG = [
    (
        "a module's own pub, shadowed by a dependency's name (task #53)",
        {
            "dep/dep.kso": 'import "std/text"\n\n'
            'pub fn pick a b\n  text/join [a b] "-"\n\n'
            'pub fn join a b\n  "OWN({a},{b})"\n',
            "app/main.kso": 'import "../dep"\n\nprint "{dep/join "x" "y"}"\n',
        },
        "app",
        "check refuses",
        "`join` is private to module `dep`",
    ),
]


def written(work, files):
    root = pathlib.Path(work) / "m"
    root.mkdir()
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
    return root


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


def judge(root, verdict, expected):
    """What went wrong with this module, or None.

    The verdict is checked before the engines are compared, because a case that
    stops compiling would otherwise be reported as a disagreement and send the
    reader to the wrong half of the compiler."""
    check = run(root, "check")
    if check[0] == "TIMED OUT":
        return "check never returned"
    if verdict == "check refuses":
        if check[0] == 0:
            return f"expected a refusal saying {expected!r}, but it compiled"
        if expected not in check[2]:
            return f"refused, but not with {expected!r}: {check[2].strip()[:120]!r}"
        return None
    if check[0] != 0:
        return f"expected it to compile: {check[2].strip()[:120]!r}"

    native = run(root, "run")
    interp = run(root, "run", ["--interp"])
    if "TIMED OUT" in (native[0], interp[0]):
        return "a run never returned"
    if native != interp:
        return (
            f"the engines disagree — native {native[0]} {native[1]!r} "
            f"{native[2].strip()[:60]!r} against interp {interp[0]} {interp[1]!r} "
            f"{interp[2].strip()[:60]!r}"
        )
    if verdict == "run refuses":
        if native[0] == 0:
            return f"expected running it to fail with {expected!r}"
        if expected not in native[2]:
            return f"failed, but not with {expected!r}: {native[2].strip()[:120]!r}"
        return None
    if native[1] != expected:
        return f"printed {native[1]!r}, expected {expected!r}"
    return None


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    wrong = []
    for label, files, verdict, expected in CASES:
        with tempfile.TemporaryDirectory() as work:
            fault = judge(written(work, files), verdict, expected)
        if fault:
            wrong.append((label, fault))

    healed = []
    for label, files, entry, verdict, expected in KNOWN_WRONG:
        with tempfile.TemporaryDirectory() as work:
            fault = judge(written(work, files) / entry, verdict, expected)
        if fault:
            healed.append((label, fault))

    print(
        f"{len(CASES)} modules, {len(wrong)} wrong; "
        f"{len(KNOWN_WRONG)} known defects, {len(healed)} no longer as recorded"
    )
    for label, fault in wrong:
        print(f"  {label}\n    {fault}")
    for label, fault in healed:
        print(f"  {label}\n    {fault}")
    if healed:
        print()
        print("a known defect is recorded as it behaves, never as it should.")
        print("if one is settled, delete its entry — do not update it.")
    if wrong or healed:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
