#!/usr/bin/env python3
"""One line of the performance history the compiler page charts.

Only machine-invariant numbers go in. The counters are algorithm-level events
and the compile golden counts fixpoint rounds and expression visits, so a noisy
runner cannot move them — a change here is somebody's deliberate edit, which is
exactly the trend worth publishing. Wall-clock belongs on the board Clay
measures by hand, not here.

Three veins feed it: what decoding costs, what encoding costs, and what
deciding and emitting cost the compiler.
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Decode: the flat-memory guarantee, the rewind count, and each kernel's
# presence. A kernel silently dropped in a merge turns one of these to zero.
DECODE = (
    "allocs",
    "alloc_bytes",
    "arena_blocks",
    "perm_allocs",
    "beat_iters",
    "el_parses",
    "utf8_bytes",
    "find2_calls",
    "bytes_peak",
)

# Encode: the write path, where the builder and the float renderer live.
ENCODE = (
    "allocs",
    "arena_blocks",
    "ryu_renders",
    "append_fast",
    "append_grow",
    "utf8_zerocopy",
    "bytes_peak",
)


# One shot: decode, hold, print — the peak the rewinds cannot hide.
ONESHOT = (
    "allocs",
    "arena_blocks",
    "arena_peak_bytes",
    "bytes_peak",
)


def counters(path, watched):
    pairs = (line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    return {k: int(v) for k, v in pairs if k in watched}


def compile_work(path):
    """Totals across the samples: what deciding cost, and what got written."""
    totals = dict.fromkeys(("rounds", "visits", "lines", "calls", "branches", "defines"), 0)
    for line in path.read_text().splitlines():
        if line.startswith("#") or "=" not in line:
            continue
        fields = dict(f.split("=", 1) for f in line.split() if "=" in f)
        for key in totals:
            totals[key] += int(fields.get(key, 0))
    return {
        "compile_rounds": totals["rounds"],
        "compile_visits": totals["visits"],
        "emitted_lines": totals["lines"],
        "emitted_calls": totals["calls"],
        "emitted_branches": totals["branches"],
        "emitted_defines": totals["defines"],
    }


def main():
    head = subprocess.run(
        ["git", "log", "-1", "--format=%h %cI %s"],
        capture_output=True,
        text=True,
        cwd=ROOT,
        check=True,
    ).stdout.strip()
    sha, when, subject = head.split(" ", 2)
    record = {"commit": sha, "date": when, "subject": subject}
    record.update(counters(ROOT / "bench/cost_golden.txt", DECODE))
    # the encode vein shares counter names with the decode one, so it is
    # prefixed rather than merged
    encode = counters(ROOT / "bench/cost_golden_encode.txt", ENCODE)
    record.update({f"encode_{k}": v for k, v in encode.items()})
    oneshot = counters(ROOT / "bench/cost_golden_oneshot.txt", ONESHOT)
    record.update({f"oneshot_{k}": v for k, v in oneshot.items()})
    record.update(compile_work(ROOT / "bench/compile_golden.txt"))
    json.dump(record, sys.stdout)
    print()


if __name__ == "__main__":
    main()
