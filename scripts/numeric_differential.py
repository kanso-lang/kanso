#!/usr/bin/env python3
"""Differential check of integer arithmetic between the two engines.

The interpreter is the oracle, so agreement is the requirement and any
disagreement is a finding. This generates whole programs and runs each through
both engines, because that is the boundary the differential law actually
governs — what a program prints, not what a function returns.

Ranges are chosen to straddle the places arithmetic changes character: around
zero, around the i32 and i64 boundaries, and well past 2^63 where the native
build's ceiling lives. A known divergence there is recorded in KNOWN, so this
reports it once rather than drowning in it, and fails if the set of divergences
grows — or shrinks without the entry being removed.
"""
import pathlib
import random
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# The divergence this project has accepted and pinned (task #46): above the
# native ceiling the oracle is exact and the native build refuses. It refuses in
# two voices — a literal too wide is caught while compiling, and an operation
# that overflows is caught while running — and matching only the second counted
# the first as four hundred bugs. Every other disagreement is a bug.
CEILING = (
    "integer overflow",
    "does not fit this build's 64-bit int",
)


def at_ceiling(native, interp):
    return any(m in native[2] or m in interp[2] for m in CEILING)

EDGES = [
    0, 1, -1, 2, -2, 7, -7, 10, 255, 256, -256,
    2**31 - 1, 2**31, -(2**31), 2**32, -(2**32),
    2**62, 2**63 - 1, -(2**63), 2**63, 2**64,
]


def program(cases):
    """One program per batch: a native run compiles with clang, so a process
    per case is minutes where a process per batch is seconds."""
    return "".join(f"print ({a} {op} {b})\n" for a, op, b in cases)


def run(source, engine):
    with tempfile.TemporaryDirectory() as work:
        entry = pathlib.Path(work) / "main.kso"
        entry.write_text(source)
        done = subprocess.run(
            [str(KANSO), "run", ".", *engine],
            cwd=work,
            capture_output=True,
            text=True,
        )
    return done.returncode, done.stdout, done.stderr


def agree(cases):
    """Whether both engines answer a batch identically. A batch that dies part
    way answers with what it printed first, which is enough to disagree on."""
    native = run(program(cases), [])
    interp = run(program(cases), ["--interp"])
    return native == interp, native, interp


def cases(rounds):
    for a in EDGES:
        for b in EDGES:
            for op in ("+", "-", "*"):
                yield a, op, b
            if b != 0:
                yield a, "/", b
                yield a, "%", b
    rng = random.Random(2685821657736338717)
    for _ in range(rounds):
        a = rng.randint(-(2**70), 2**70)
        b = rng.randint(-(2**70), 2**70)
        yield a, rng.choice("+-*"), b


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    rounds = int(sys.argv[1]) if len(sys.argv) > 1 else 200
    everything = list(cases(rounds))
    batches = [everything[i : i + 200] for i in range(0, len(everything), 200)]

    suspect = []
    for batch in batches:
        same, _, _ = agree(batch)
        if not same:
            suspect.extend(batch)

    known, bad = 0, []
    for case in suspect:
        same, native, interp = agree([case])
        if same:
            continue
        if at_ceiling(native, interp):
            known += 1
            continue
        bad.append((program([case]).strip(), native, interp))

    print(
        f"{len(everything)} programs in {len(batches)} batches, "
        f"{known} at the known ceiling, {len(bad)} disagreements"
    )
    for source, native, interp in bad[:10]:
        print(f"  {source}")
        print(f"    native: code={native[0]} out={native[1]!r} err={native[2].strip()!r}")
        print(f"    interp: code={interp[0]} out={interp[1]!r} err={interp[2].strip()!r}")
    if bad:
        print()
        print("the interpreter is the oracle: these are native's to fix.")
        return 1
    if known == 0:
        print()
        print("nothing reached the native ceiling — either the gap closed, in")
        print("which case delete CEILING here and in tests/numeric_parity.rs, or")
        print("the ranges above stopped straddling it.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
