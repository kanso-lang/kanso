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

Report-only for now — it prints verdicts and always exits 0, so the
friction is measured on real pull requests before anybody flips it to
gating. Flip: exit 1 where UNPRICED is printed.
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

GOLDENS = (
    "bench/cost_golden.txt",
    "bench/cost_golden_encode.txt",
    "bench/cost_golden_oneshot.txt",
    "bench/compile_golden.txt",
)


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
    for golden in GOLDENS:
        prefix = {
            "bench/cost_golden_encode.txt": "encode_",
            "bench/cost_golden_oneshot.txt": "oneshot_",
        }.get(golden, "")
        ours = counters((ROOT / golden).read_text(), prefix)
        mains = counters(from_git(base, golden), prefix)
        for key in sorted(set(ours) | set(mains)):
            before, after = mains.get(key, 0), ours.get(key, 0)
            if before == after:
                continue
            bare = key.removeprefix("encode_")
            worse = (
                after > before
                if bare in LOWER_IS_BETTER
                else after < before
                if bare in HIGHER_IS_BETTER
                else False
            )
            arrow = "worsened" if worse else "improved"
            print(f"{arrow}: {key} {before:,} -> {after:,}  ({golden})")
            if worse and bare not in added_log and key not in added_log:
                worsened.append(key)
    if worsened:
        print()
        print("UNPRICED — these worsened with no sentence naming them in the")
        print("compiler-log delta of this branch:")
        for key in worsened:
            print(f"  {key}")
        print("movement is fine; silence is not. say why in design/compiler-log.md.")
    else:
        print("every changed counter is priced (or improved).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
