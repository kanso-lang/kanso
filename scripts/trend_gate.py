#!/usr/bin/env python3
"""A worsened counter demands a written why; this check reads for it.

The welfare gate already forces a sentence when one of its seven terms
regresses, and the cost goldens already refuse silent movement. The seam
between them: a PR can regenerate a golden, bank a welfare rise, and still
worsen a counter the chart shows — permanent slots, emitted lines — with no
written trade. The literal-interning entry did exactly that, and the rise
sat unexplained until somebody read the chart.

So: diff every golden counter this branch changed against main's copy,
classify each move by the direction table below, and look for each
worsened counter's name in the branch's compiler-log delta. The reflection
is the sentence; this only refuses silence.

What is enforced, per Clay's rule: nothing may get worse unless at least
one other thing got better — a pure regression fails outright. A genuine
trade (worsened here, improved there) passes this gate and is arbitrated
by the welfare scalar, which weighs the sides. The name-each-worsening
listing below stays advisory: the sentence is reflection, not enforcement.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Which way is better, per counter. Presence counters (kernel pins) are
# news in either direction, but the hard golden diff already forces those
# to be deliberate; here they are down-is-worse so a dropped kernel reads
# as a worsening and wants its sentence too.
LOWER_IS_BETTER = {
    "allocs",
    "alloc_bytes",
    "arena_blocks",
    "perm_allocs",
    "beat_iters",
    "bytes_malloc",
    "thunk_allocs",
    "thunk_escaped",
    "thunk_live_exit",
    "arena_peak_bytes",
    "bytes_peak",
    # the sh_ family is bytes held, by category
    "sh_str",
    "sh_rec",
    "sh_buf",
    "sh_map",
    "sh_bytes",
    "lines",
    "calls",
    "branches",
    "defines",
    "rounds",
    "visits",
}
HIGHER_IS_BETTER = {
    "el_parses",
    "ryu_renders",
    "append_fast",
    "utf8_zerocopy",
    "buf_reuse",
    "thunk_frees",
}

BENCH_GOLDENS = {
    "bench/cost_golden.txt": "",
    "bench/cost_golden_encode.txt": "encode_",
    "bench/cost_golden_oneshot.txt": "oneshot_",
    "bench/cost_golden_basket.txt": "basket_",
    "bench/compile_golden.txt": "",
}


def goldens():
    """Every pinned counter, bench veins and the mem corpus alike.

    The mem corpus is a counter vein like the others, and allocation shape
    is exactly the kind of thing that moves quietly: a change can worsen
    every .mem file in the tree without a sentence anywhere. Each fixture
    is prefixed by its own name so a move says which program it came from.
    """
    out = dict(BENCH_GOLDENS)
    for path in sorted((ROOT / "tests/golden/mem").glob("*.mem")):
        out[f"tests/golden/mem/{path.name}"] = f"{path.stem}_"
    return out


def counters(text, prefix):
    out = {}
    for line in text.splitlines():
        if line.startswith("#"):
            continue
        for field in line.split():
            if "=" in field:
                key, value = field.split("=", 1)
                if value.lstrip("-").isdigit():
                    out[f"{prefix}{key}"] = out.get(f"{prefix}{key}", 0) + int(value)
    return out


def from_git(ref, path):
    run = subprocess.run(
        ["git", "show", f"{ref}:{path}"], capture_output=True, text=True, cwd=ROOT
    )
    return run.stdout if run.returncode == 0 else ""


def log_delta(base):
    run = subprocess.run(
        ["git", "diff", base, "--", "design/compiler-log.md"],
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    return "\n".join(l[1:] for l in run.stdout.splitlines() if l.startswith("+"))


def main():
    base = sys.argv[1] if len(sys.argv) > 1 else "origin/main"
    added_log = log_delta(base)
    worsened = []
    improved = []
    worse_all = []
    for golden, prefix in goldens().items():
        base_text = from_git(base, golden)
        # a fixture this branch adds has nothing to be compared against, and
        # counting its counters as movement would read every new pin as a
        # regression
        if not base_text:
            continue
        ours = counters((ROOT / golden).read_text(), prefix)
        mains = counters(base_text, prefix)
        for key in sorted(set(ours) | set(mains)):
            before, after = mains.get(key, 0), ours.get(key, 0)
            if before == after:
                continue
            bare = key.removeprefix(prefix)
            worse = (
                after > before
                if bare in LOWER_IS_BETTER
                else after < before
                if bare in HIGHER_IS_BETTER
                else False
            )
            arrow = "worsened" if worse else "improved"
            print(f"{arrow}: {key} {before:,} -> {after:,}  ({golden})")
            (worse_all if worse else improved).append(key)
            if worse and bare not in added_log and key not in added_log:
                worsened.append(key)
    if worsened:
        print()
        print("UNPRICED — these worsened with no sentence naming them in the")
        print("compiler-log delta of this branch:")
        for key in worsened:
            print(f"  {key}")
        print("movement is fine; silence is not. say why in design/compiler-log.md.")
    elif worse_all or improved:
        print("every changed counter is priced (or improved).")
    if worse_all and not improved:
        print()
        print("FAIL  a pure regression: something got worse and nothing got")
        print("better. that trade has no other side, and it is the one move")
        print("the gate refuses outright.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
