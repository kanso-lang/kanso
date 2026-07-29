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

# What the project is buying. Each term carries two numbers, because weight
# alone cannot say what a second is worth here.
#
#   weight    how much this dimension matters at all. Runtime carries two
#             thirds because that is the claim the front page makes; compile
#             cost carries a third, because a language nobody can iterate in
#             is not fast either.
#
#   satiation how fast further improvement stops mattering, which differs by
#             dimension and is not a matter of importance. A compiler that
#             already finishes in six milliseconds gains almost nothing from
#             finishing in three — nobody can tell. A decoder in a hot loop
#             that gets eight times faster is eight times faster, and the
#             eighth doubling still shows up in somebody's bill. Larger
#             satiation means the term keeps paying for longer.
TERMS = {
    "decode_allocs": (0.133, 2.0),
    "decode_arena_blocks": (0.054, 1.5),
    # the gauntlet's true peak: arena high water plus malloc-backed builder
    # storage at its high water. A looping process's leak accumulates in
    # exactly the storage the arena number cannot see — this term is what
    # made the 21 MB transient-builder leak a priced regression instead of
    # an invisible one. Runtime-class weight, late satiation.
    "decode_peak_bytes": (0.074, 2.0),
    # the encode workload's true peak, priced like the other two: the
    # burned-final chunks (one ~270 KB buffer per iteration, 85 MB of
    # encodebench RSS) lived exactly in the storage no other term saw.
    "encode_peak_bytes": (0.048, 2.0),
    "encode_allocs": (0.114, 2.0),
    "encode_arena_blocks": (0.047, 1.5),
    # the one-shot peak: decode a document, hold it, print — kq's shape.
    # the looping gauntlet's rewinds hide exactly the garbage this term
    # sees, and peak footprint is the scoreboard row the project still
    # loses, so it enters at runtime-class weight with late satiation.
    # the arena-block terms cede the most, because peak bytes now measure
    # what block counts only proxied.
    "oneshot_peak_bytes": (0.101, 2.0),
    # The basket. Three programs and a compile sampled a narrow shelf of what
    # this language does, and what a model leaves out it weights at zero: a
    # string build that got twenty times faster and a sort that shed two
    # hundred times its allocations both left the number untouched. This term
    # reads a spread — string accumulation, map read-write over repeating and
    # growing key sets, list build and index read, lazy map/select/fold,
    # group_by and tally, integer and float arithmetic, records, join, slice
    # and sort — so the index tracks the language the way a price index tracks
    # a basket rather than a single good. Runtime-class weight, late satiation.
    "basket_allocs": (0.09, 2.0),
    "basket_peak_bytes": (0.06, 2.0),
    "compile_rounds": (0.11, 0.5),
    "compile_visits": (0.085, 0.5),
    "emitted_lines": (0.084, 0.5),
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
    oneshot = counters(ROOT / "bench/cost_golden_oneshot.txt")
    basket = counters(ROOT / "bench/cost_golden_basket.txt")
    comp = compile_totals(ROOT / "bench/compile_golden.txt")
    return {
        "decode_allocs": decode["allocs"],
        "decode_arena_blocks": decode["arena_blocks"],
        "decode_peak_bytes": decode["arena_peak_bytes"] + decode["bytes_peak"],
        "encode_allocs": encode["allocs"],
        "encode_arena_blocks": encode["arena_blocks"],
        "encode_peak_bytes": encode["arena_peak_bytes"] + encode["bytes_peak"],
        # the true one-shot peak: arena blocks plus malloc-backed builder
        # storage at its high water. The arena number alone missed exactly
        # the leak bytes_peak was built to expose.
        "oneshot_peak_bytes": oneshot["arena_peak_bytes"] + oneshot["bytes_peak"],
        "basket_allocs": basket["allocs"],
        "basket_peak_bytes": basket["arena_peak_bytes"] + basket["bytes_peak"],
        "compile_rounds": comp["rounds"],
        "compile_visits": comp["visits"],
        "emitted_lines": comp["lines"],
    }


def satisfaction(ratio, satiation):
    """How much a term being `ratio` times better than baseline is worth.

    A straight ratio says the tenth halving is worth as much as the first,
    which is false: past a point a program is fast enough that making it twice
    as fast again buys almost nothing. A logarithm is the usual reach and still
    pays every doubling equally. This saturates instead, at a rate the term
    chooses, so a dimension that stops mattering early can say so:

        ratio          0.5     1      2      4      8     inf
        satiation 0.5  0.50   0.67   0.80   0.89   0.94   1.00   (compile)
        satiation 2.0  0.20   0.33   0.50   0.67   0.80   1.00   (runtime)

    Both are asymmetric around the baseline — halving a term's performance
    costs more than doubling it gains — which is the right shape for a ratchet.

    Measured rather than assumed, the asymmetry is sharpest where satiation is
    low. A doubling of compile rounds costs 5.4 points against a 0.15 weight,
    while a doubling of decode allocations costs 7.2 against 0.25 — so per unit
    of weight, the satiated term loses more. That reads oddly until you say it
    in words: a compiler that already finishes imperceptibly and then takes
    twice as long has moved from instant toward noticeable, which is a real
    loss, while a decoder that was already the expensive part just got worse at
    being expensive. Satiation cuts both ways, and it should.
    """
    return ratio / (ratio + satiation)


def score(now, base):
    """The one number: a weighted sum of per-term satisfaction, on a scale
    whose ceiling is a hundred and whose origin is arbitrary.

    Not a percentage of anything. Anchoring the baseline at 100 made it read
    as "complete", which invited the question of what a hundred percent of the
    work would be — there is no such quantity. What the number is for is its
    direction and the size of its moves. A hundred is the unreachable limit
    where every term costs nothing.
    """
    total = 0.0
    for key, (weight, satiation) in TERMS.items():
        # a term that reached zero is better than any baseline, and dividing
        # by it would say infinity rather than "as good as this gets"
        current = now[key] if now[key] else 0.5
        total += weight * satisfaction(base[key] / current, satiation)
    return 100.0 * total


def main():
    now = terms()
    if not FLOOR.exists() or "--set" in sys.argv:
        if not FLOOR.exists():
            FLOOR.write_text(json.dumps({"baseline": now, "floor": 100.0}, indent=2) + "\n")
            print("floor established at 100.0")
            return 0
        held = json.loads(FLOOR.read_text())
        # a term new to the model enters at ratio one: its own introduction
        # is never a score move, and only improvement from here pays
        for key in now:
            held["baseline"].setdefault(key, now[key])
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
    # the era: recalibrations reshuffle the scale, so a score is only
    # comparable within its model version — say which one this is
    era = len(held.get("history", [])) + 1
    last = held.get("history", [{}])[-1].get("why", "initial model")
    print(f"welfare {value:.2f}   floor {floor:.2f}   (ceiling 100, ratchet {era}: {last[:52]}…)")
    worse = []
    for key in sorted(TERMS):
        base, cur = held["baseline"][key], now[key]
        if base != cur:
            print(f"  {key:22} {base:>12,} -> {cur:>12,}   {100 * (base / (cur or 0.5) - 1):+6.1f}%")
        if cur > base:
            worse.append((key, base, cur))
    # the sum is the gate, but a term that went backwards under the cover of
    # another term's gain is a trade, and a trade has to be said out loud
    if worse:
        print("\nTERMS THAT GOT WORSE — say why each one was worth it:")
        for key, base, cur in worse:
            weight, satiation = TERMS[key]
            cost = 100 * weight * (
                satisfaction(base / base, satiation) - satisfaction(base / cur, satiation)
            )
            print(f"  {key:22} {base:>12,} -> {cur:>12,}   costs {cost:.3f} points")
    # a hundredth of a point is below anything a real change moves, and
    # leaves room for a term that rounds
    if value < floor - 0.01:
        print(f"\nFAIL  welfare fell {floor - value:.2f} below the floor.")
        print("The sum is the objective, so this change is worse by the weights as")
        print("written. Either it goes, or the argument is that the weights are wrong —")
        print("make that argument, then welfare.py --set \"the reason\".")
        return 1
    if value > floor + 0.01:
        print(f"\nFAIL  welfare is {value - floor:.2f} above the floor and the gain is not held.")
        print("A rise nobody ratchets is a rise the next change is free to spend. Bank it")
        print("in this same PR: welfare.py --set \"what improved and why\".")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
