#!/usr/bin/env python3
"""One number for the whole project, and a ratchet under it.

The cost goldens each pin one counter, which catches a regression in that
counter and nothing about the trade a change made across counters. A feature
that spends compile work to buy runtime work passes every golden while leaving
the project better or worse, and nobody can say which without an argument.

So: combine what running costs and what compiling costs into a single scalar,
and hold a floor under it. Every term is deterministic — allocation counts,
arena blocks, fixpoint rounds, emitted lines — so the number moves only when
somebody changes the compiler, never because the box was busy. Wall clock is
deliberately absent for that reason.

    welfare = 100 * (baseline_cost / current_cost)

Cost is a weighted sum of normalised terms, so a hundred is the reference
point and higher is better. The weights say what the project is buying:
runtime speed and footprint dominate, compile cost matters and matters less.

Usage:
    welfare.py                    print the current score against the floor
    welfare.py --set "why"        record the score as the new floor, with the
                                  reason it moved

The sum is the objective. A term getting worse while the sum rises is the
trade the weights exist to license, so the per-term breakdown below the score
says where a move came from and never excuses one. A sum that falls means the
change is worse by the weights as written — the argument to have is whether
the weights are right, not whether this term deserves a pass.
"""
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
FLOOR = ROOT / "bench/welfare_floor.json"

# What the project is buying, and how much each part is worth. Runtime is
# two thirds of it because that is the claim the front page makes; compile
# cost is a third because a language nobody can iterate in is not fast.
WEIGHTS = {
    "decode_allocs": 0.25,
    "decode_arena_blocks": 0.10,
    "encode_allocs": 0.20,
    "encode_arena_blocks": 0.10,
    "compile_rounds": 0.15,
    "compile_visits": 0.10,
    "emitted_lines": 0.10,
}


def counters(path):
    pairs = (line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    return {k: int(v) for k, v in pairs}


def compile_totals(path):
    totals = {"rounds": 0, "visits": 0, "lines": 0}
    for line in path.read_text().splitlines():
        if line.startswith("#") or "=" not in line:
            continue
        fields = dict(f.split("=", 1) for f in line.split() if "=" in f)
        for key in totals:
            totals[key] += int(fields.get(key, 0))
    return totals


def terms():
    decode = counters(ROOT / "bench/cost_golden.txt")
    encode = counters(ROOT / "bench/cost_golden_encode.txt")
    comp = compile_totals(ROOT / "bench/compile_golden.txt")
    return {
        "decode_allocs": decode["allocs"],
        "decode_arena_blocks": decode["arena_blocks"],
        "encode_allocs": encode["allocs"],
        "encode_arena_blocks": encode["arena_blocks"],
        "compile_rounds": comp["rounds"],
        "compile_visits": comp["visits"],
        "emitted_lines": comp["lines"],
    }


def satisfaction(ratio):
    """How much a term being `ratio` times better than baseline is worth.

    A straight ratio says the tenth halving is worth as much as the first,
    which is false: past a point a program is fast enough that making it twice
    as fast again buys almost nothing. A logarithm is the usual reach, and it
    still pays every doubling equally. This saturates instead — each doubling
    is worth less than the one before, and no single term can run away with
    the score.

        ratio  0.25   0.5    1     2     4     8    16     inf
        value  0.20   0.33  0.50  0.67  0.80  0.89  0.94   1.00

    It is deliberately asymmetric around the baseline. Halving a term's
    performance costs more than doubling it gains, which is the right shape
    for a ratchet: a project protecting what it has should not be able to buy
    a regression with a speculative win somewhere else.
    """
    return ratio / (ratio + 1.0)


# every term sits at exactly this when it matches baseline, so a project that
# has changed nothing scores exactly 100
BASELINE_SATISFACTION = 0.5


def score(now, base):
    """The one number. A weighted sum of per-term satisfaction, scaled so the
    recorded baseline reads 100. Two hundred is the unreachable ceiling where
    every term costs nothing."""
    total = 0.0
    for key, weight in WEIGHTS.items():
        # a term that reached zero is better than any baseline, and dividing
        # by it would say infinity rather than "as good as this gets"
        current = now[key] if now[key] else 0.5
        total += weight * satisfaction(base[key] / current)
    return 100.0 * total / BASELINE_SATISFACTION


def main():
    now = terms()
    if not FLOOR.exists() or "--set" in sys.argv:
        if not FLOOR.exists():
            FLOOR.write_text(json.dumps({"baseline": now, "floor": 100.0}, indent=2) + "\n")
            print("floor established at 100.0")
            return 0
        held = json.loads(FLOOR.read_text())
        value = score(now, held["baseline"])
        reasons = [a for a in sys.argv[1:] if a != "--set"]
        if not reasons:
            print("--set records why the objective moved: welfare.py --set \"reason\"",
                  file=sys.stderr)
            return 2
        held["floor"] = value
        held.setdefault("history", []).append({"floor": round(value, 2), "why": " ".join(reasons)})
        FLOOR.write_text(json.dumps(held, indent=2) + "\n")
        print(f"floor moved to {value:.2f}: {' '.join(reasons)}")
        return 0

    held = json.loads(FLOOR.read_text())
    value = score(now, held["baseline"])
    floor = held["floor"]
    print(f"welfare {value:.2f}   floor {floor:.2f}")
    for key, weight in sorted(WEIGHTS.items()):
        base, cur = held["baseline"][key], now[key]
        if base != cur:
            print(f"  {key:22} {base:>12,} -> {cur:>12,}   {100 * (base / (cur or 0.5) - 1):+6.1f}%")
    # a hundredth of a point is below anything a real change moves, and
    # leaves room for a term that rounds
    if value < floor - 0.01:
        print(f"\nFAIL  welfare fell {floor - value:.2f} below the floor.")
        print("The sum is the objective, so this change is worse by the weights as")
        print("written. Either it goes, or the argument is that the weights are wrong —")
        print("make that argument, then welfare.py --set \"the reason\".")
        return 1
    if value > floor + 0.01:
        print(f"\nwelfare is {value - floor:.2f} above the floor; run --set to hold the gain.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
