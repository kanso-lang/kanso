#!/usr/bin/env python3
"""One line of the performance history the compiler page charts.

Only machine-invariant numbers go in. The counters are algorithm-level events
and the compile golden counts fixpoint rounds and expression visits, so a
noisy runner cannot move them — a change here is somebody's deliberate edit,
which is exactly the trend worth publishing. Wall-clock belongs on the board
Clay measures by hand, not here.
"""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The counters worth a chart: the flat-memory guarantee, the rewind count, and
# each kernel's presence. The rest of the golden stays in the golden.
WATCHED = ("allocs", "alloc_bytes", "arena_blocks", "beat_iters", "el_parses", "utf8_bytes")


def counters(path):
    pairs = (line.split("=", 1) for line in path.read_text().splitlines() if "=" in line)
    return {k: int(v) for k, v in pairs if k in WATCHED}


def compile_work(path):
    """Totals across the samples: what deciding cost, and what got written."""
    rounds = visits = lines = 0
    for line in path.read_text().splitlines():
        if line.startswith("#") or "=" not in line:
            continue
        fields = dict(f.split("=", 1) for f in line.split() if "=" in f)
        rounds += int(fields.get("rounds", 0))
        visits += int(fields.get("visits", 0))
        lines += int(fields.get("lines", 0))
    return {"compile_rounds": rounds, "compile_visits": visits, "emitted_lines": lines}


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
    record.update(counters(ROOT / "bench/cost_golden.txt"))
    record.update(compile_work(ROOT / "bench/compile_golden.txt"))
    json.dump(record, sys.stdout)
    print()


if __name__ == "__main__":
    main()
