#!/usr/bin/env python3
"""Differential check of how a value prints.

Rendering is written twice — `render` in the interpreter and `k_render` in the
C runtime — and every `print` goes through it, so it is the widest surface in
the compiler where two implementations have to agree character for character.
The project already names it divergence-prone alongside float formatting and
utf-8; those two have harnesses and this did not.

The corpus pins the values it happens to print. This asks for the ones nobody
would think to print: a float that is an integer, a float that needs every
digit, the boundaries of the integer range, an empty container, a container
holding itself in miniature, a string that is nothing but escapes.

Agreement is the requirement. A disagreement is a finding.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# Each entry is a kanso expression. The value it names is printed, and the two
# engines must produce the same characters.
VALUES = [
    # integers, including where the range ends
    "0", "1", "-1", "7", "-7", "255", "256", "-256",
    "2147483647", "2147483648", "-2147483648",
    "4611686018427387904", "9223372036854775807", "-9223372036854775808",
    # floats: the shapes that catch a formatter out
    "0.0", "1.0", "-1.0", "0.5", "-0.5", "2.25", "100.0", "1000000.0",
    "0.1", "0.2", "3.14159265358979", "1.7976931348623157",
    "0.000001", "123456789.123456789", "-0.0",
    # arithmetic that lands on a float
    "1 / 2", "10 / 4", "1.0 / 3.0", "2.0 * 3.0", "7 % 3", "-7 % 3",
    # strings, including what has to survive a round trip
    '""', '"a"', '"a b"', '"tab\\there"', '"line\\nbreak"',
    '"quote\\"inside"', '"back\\\\slash"', '"héllo"', '"日本語"', '"emoji 🌱"',
    # the nullary values
    "true", "false", "none",
    # containers, empty and nested
    "[]", "[1]", "[1 2 3]", '["a" "b"]', "[[1] [2 3]]", '[1 "a" true none]',
    "[[]]", "[0.5 1.0]",
    '{ "a":1 }', '{ "a":1 "b":2 }', '{ "b":2 "a":1 }', '{ "a":"x" }',
    '{ "n":none }', '{ "xs":[1 2] }',
]

# A record prints through the same path and carries its type's name.
RECORDS = [
    ("a record", "point 1 2"),
    ("a record of strings", 'point "a" "b"'),
    ("a nested record", "point (point 1 2) 3"),
    ("a record in a list", "[(point 1 2)]"),
    ("a record holding none", "point none 1"),
    ("a record holding a float", "point 0.5 1.0"),
]

PRELUDE = "type point\n  x\n  y\n\n"


def run(source, engine):
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
    probes = [(v, f'pub play = print "{{{v}}}"\n') for v in VALUES]
    probes += [
        (label, PRELUDE + f'pub play = print "{{{expr}}}"\n') for label, expr in RECORDS
    ]
    disagreed = []
    for label, source in probes:
        native = run(source, [])
        interp = run(source, ["--interp"])
        if native != interp:
            disagreed.append((label, native, interp))
    print(f"{len(probes)} values rendered, {len(disagreed)} disagree")
    for label, native, interp in disagreed[:20]:
        print(f"  {label}")
        print(f"    native: {native[1]!r} {native[2].strip()[:80]!r}")
        print(f"    interp: {interp[1]!r} {interp[2].strip()[:80]!r}")
    if disagreed:
        print()
        print("the interpreter is the oracle: these are native's to match.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
