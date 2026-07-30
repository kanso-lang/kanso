#!/usr/bin/env python3
"""Differential check of what the engines SAY, not just what they compute.

The interpreter is the oracle for values, and it is the oracle for diagnostics
too — a program that fails has to fail the same way on every engine, or the
reader's experience depends on which one ran. Nothing was checking that: the
golden corpus pins the diagnostics it happens to contain, and no golden asked
a std function for the wrong type until one was written by hand and turned up
`length` saying half as much on native as on the interpreter.

So this asks every exported std function for the wrong thing and requires both
engines to answer identically. A record is the wrong thing for nearly
everything and is never a literal, so it slips past the compile-time literal
check and reaches the runtime path where the messages are written separately
for each engine.

One probe per function, with the record in every argument position at once.
That reaches whichever type check the function makes first, which is the one
whose wording can drift; making each position wrong on its own would need a
valid filler for the others, and a valid filler differs per function.

Agreement is the requirement. A disagreement is a finding, and so is a crash
with no message at all.
"""
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# `pub fn name a b` — the exported surface, with how many arguments it takes.
SIGNATURE = re.compile(r"^pub fn ([a-z_0-9?!+*/%<>=-]+)((?: +[a-z_0-9()?]+)*)\s*$")

# What a wrong argument is made of. A record is wrong for every std function
# and cannot be written as a literal, so the literal checker lets it through.
PRELUDE = """type wrong
  a
  b

"""


def surface():
    """Every `pub fn` in the shipped library, by module."""
    found = []
    for module in sorted((ROOT / "lib").iterdir()):
        if not module.is_dir():
            continue
        for path in sorted(module.glob("*.kso")):
            if path.name.endswith("_test.kso"):
                continue
            for line in path.read_text().splitlines():
                match = SIGNATURE.match(line)
                if not match:
                    continue
                name, params = match.group(1), match.group(2).split()
                # a destructuring parameter means the arm takes a shape, not a
                # value; the group's other arms cover the same name
                if any("(" in p for p in params):
                    continue
                found.append((module.name, name, len(params)))
    # one entry per name and arity: an overload group answers once
    return sorted(set(found))


def program(module, name, arity):
    args = " ".join(["bad"] * arity)
    return (
        f'import "std/{module}"\n\n'
        + PRELUDE
        + "pub play =\n"
        + "  bad = wrong 1 2\n"
        + f'  print "{{{module}/{name} {args}}}"\n'
    )


def answer(source, engine):
    with tempfile.TemporaryDirectory() as work:
        entry = pathlib.Path(work) / "probe.kso"
        entry.write_text(source)
        done = subprocess.run(
            [str(KANSO), "play", "probe.kso", *engine],
            cwd=work,
            capture_output=True,
            text=True,
            timeout=60,
        )
    return done.returncode, done.stdout, done.stderr


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    checked, disagreed = 0, []
    for module, name, arity in surface():
        if arity == 0:
            continue
        source = program(module, name, arity)
        native = answer(source, [])
        interp = answer(source, ["--interp"])
        checked += 1
        if native != interp:
            disagreed.append((module, name, native, interp))
    print(f"{checked} std functions asked for the wrong thing, {len(disagreed)} disagree")
    for module, name, native, interp in disagreed[:20]:
        print(f"  {module}/{name}")
        print(f"    native: code={native[0]} err={native[2].strip()[:150]!r}")
        print(f"    interp: code={interp[0]} err={interp[2].strip()[:150]!r}")
    if disagreed:
        print()
        print("the interpreter is the oracle: these are native's to match.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
