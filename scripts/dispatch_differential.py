#!/usr/bin/env python3
"""Differential check of which arm answers a call.

Dispatch is the mechanism the language is built on: there is no `if` on type,
no `match` statement, no method table — a group's arms are the whole story, and
every feature that looks like control flow is arms underneath. It is written
twice, once as the interpreter's arm search and once as the emitter's switch
and guard chains, and nothing was comparing the two on the cases that decide an
arm rather than the cases that happen to appear in the corpus.

What is probed is which arm answers, not what the arm computes: each probe
returns a marker naming the arm, so a difference reads as `native took the
literal arm where the interpreter took the wildcard` rather than as a number.

Order matters and is part of the contract — arms are tried as written — so
probes deliberately overlap: a literal that a wildcard would also match, a
narrow record pattern beside a wide one, an err arm beside a value arm.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# (label, declarations, the expression `pub play` prints). Each runs on both
# engines and both must name the same arm.
PROBES = [
    (
        "a literal beats a later wildcard",
        'fn which 0\n  "zero"\n\nfn which _\n  "any"\n',
        "which 0",
    ),
    (
        "a wildcard takes what no literal claims",
        'fn which 0\n  "zero"\n\nfn which _\n  "any"\n',
        "which 7",
    ),
    (
        "a string literal arm",
        'fn which "a"\n  "the a"\n\nfn which _\n  "any"\n',
        'which "a"',
    ),
    (
        "a string literal that misses",
        'fn which "a"\n  "the a"\n\nfn which _\n  "any"\n',
        'which "b"',
    ),
    (
        "true and false are separate arms",
        'fn which true\n  "yes"\n\nfn which false\n  "no"\n',
        "which false",
    ),
    (
        "none is an arm of its own",
        'fn which none\n  "nothing"\n\nfn which _\n  "something"\n',
        "which none",
    ),
    (
        "a value is not none",
        'fn which none\n  "nothing"\n\nfn which _\n  "something"\n',
        "which 0",
    ),
    (
        "arity picks the group",
        'fn which _\n  "one"\n\nfn which _ _\n  "two"\n',
        "which 1 2",
    ),
    (
        "a record pattern binds its fields",
        'type point\n  x\n  y\n\nfn which (point a b)\n  "{a}-{b}"\n\nfn which _\n  "any"\n',
        "which (point 1 2)",
    ),
    (
        "a record pattern declines another type",
        'type point\n  x\n  y\n\ntype label\n  text\n\nfn which (point _ _)\n  "point"\n\nfn which _\n  "any"\n',
        'which (label "l")',
    ),
    (
        "a nested record pattern",
        'type point\n  x\n  y\n\nfn which (point (point _ _) _)\n  "nested"\n\nfn which _\n  "any"\n',
        "which (point (point 1 2) 3)",
    ),
    (
        "a nested pattern that does not nest",
        'type point\n  x\n  y\n\nfn which (point (point _ _) _)\n  "nested"\n\nfn which _\n  "any"\n',
        "which (point 1 2)",
    ),
    (
        "a literal inside a record pattern",
        'type point\n  x\n  y\n\nfn which (point 0 _)\n  "on the axis"\n\nfn which _\n  "any"\n',
        "which (point 0 5)",
    ),
    (
        "the same pattern, other value",
        'type point\n  x\n  y\n\nfn which (point 0 _)\n  "on the axis"\n\nfn which _\n  "any"\n',
        "which (point 1 5)",
    ),
    (
        "the first of two overlapping records",
        'type point\n  x\n  y\n\nfn which (point 0 0)\n  "origin"\n\nfn which (point 0 _)\n  "axis"\n\nfn which _\n  "any"\n',
        "which (point 0 0)",
    ),
    (
        "the second of two overlapping records",
        'type point\n  x\n  y\n\nfn which (point 0 0)\n  "origin"\n\nfn which (point 0 _)\n  "axis"\n\nfn which _\n  "any"\n',
        "which (point 0 9)",
    ),
    (
        "an err arm beside a value arm",
        'fn which (err _)\n  "failed"\n\nfn which _\n  "fine"\n',
        'which (err "boom")',
    ),
    (
        "a value where an err arm waits",
        'fn which (err _)\n  "failed"\n\nfn which _\n  "fine"\n',
        "which 1",
    ),
    (
        "a bool is not an int",
        'fn which 1\n  "one"\n\nfn which _\n  "any"\n',
        "which true",
    ),
    (
        "a computed zero is a zero",
        'fn which 0\n  "zero"\n\nfn which _\n  "any"\n',
        "which (1 - 1)",
    ),
    (
        "a string arm and an empty string",
        'fn which ""\n  "empty"\n\nfn which _\n  "any"\n',
        'which ""',
    ),
    (
        "a group called through a value",
        'fn which 0\n  "zero"\n\nfn which _\n  "any"\n',
        "f = which\n  print \"{f 0}\"",
    ),
]


def source_for(decls, expr):
    if "\n" in expr:
        return decls + "\npub play =\n  " + expr + "\n"
    return decls + f'\npub play = print "{{{expr}}}"\n'


def run(decls, expr, engine):
    with tempfile.TemporaryDirectory() as work:
        entry = pathlib.Path(work) / "probe.kso"
        entry.write_text(source_for(decls, expr))
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
    disagreed, hung, refused = [], [], []
    for label, decls, expr in PROBES:
        native = run(decls, expr, [])
        interp = run(decls, expr, ["--interp"])
        if native[0] == "TIMED OUT" or interp[0] == "TIMED OUT":
            hung.append((label, native, interp))
        elif native != interp:
            disagreed.append((label, native, interp))
        elif native[0] != 0 or not native[1].strip():
            # A probe that did not compile, did not run, or printed nothing is
            # not the engines agreeing about dispatch — it is two identical
            # silences, which compare equal and mean nothing. Every probe here
            # names the arm that answered, so an empty answer is a broken probe.
            refused.append((label, native[2].strip()[:70] or "printed nothing"))
    print(
        f"{len(PROBES)} dispatch cases, {len(disagreed)} disagree, "
        f"{len(hung)} never returned, {len(refused)} never compiled"
    )
    for label, why in refused:
        print(f"  never compiled: {label}: {why}")
    for label, native, interp in hung[:10]:
        print(f"  never returned: {label}")
    for label, native, interp in disagreed[:20]:
        print(f"  {label}")
        print(f"    native: code={native[0]} out={native[1]!r} err={native[2].strip()[:80]!r}")
        print(f"    interp: code={interp[0]} out={interp[1]!r} err={interp[2].strip()[:80]!r}")
    if disagreed or hung or refused:
        print()
        print("the interpreter is the oracle: these are native's to match.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
