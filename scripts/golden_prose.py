#!/usr/bin/env python3
"""Prose numbers that quote a golden must match it, mechanically.

The panels learned this discipline first: book_panels.py rewrites any code
or output panel that drifts from its sample. Prose lagged behind — the
landing page and the compiler page each quoted an allocation count the
ratchet had long since moved past, and nothing but a manual sweep could
notice. This closes the class: any element carrying a data-golden
attribute names a counter, and its text must equal that counter's current
value in the golden file.

    <strong data-golden="decode.allocs">8,341,214</strong>

`decode.` keys read bench/cost_golden.txt, `encode.` keys read
bench/cost_golden_encode.txt. The displayed text may use thousands
separators; comparison strips them. `--write` rewrites drifted numbers in
place, preserving the separator style; the bare run reports and fails,
which is what ci wants.
"""
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GOLDENS = {
    "decode": os.path.join(ROOT, "bench/cost_golden.txt"),
    "encode": os.path.join(ROOT, "bench/cost_golden_encode.txt"),
}
TAGGED = re.compile(
    r'(<(\w+)[^>]*data-golden="([a-z_]+)\.([a-z_]+)"[^>]*>)([^<]*)(</\2>)'
)


def counters(path):
    pairs = (line.split("=", 1) for line in open(path) if "=" in line)
    return {k: int(v) for k, v in pairs}


def main():
    write = "--write" in sys.argv
    values = {family: counters(path) for family, path in GOLDENS.items()}
    drift = 0
    for dirpath, _, files in os.walk(os.path.join(ROOT, "docs")):
        for name in files:
            if not name.endswith(".html"):
                continue
            path = os.path.join(dirpath, name)
            content = open(path).read()

            def check(m):
                nonlocal drift
                family, key = m.group(3), m.group(4)
                if family not in values or key not in values[family]:
                    print(f"UNKNOWN KEY {family}.{key} in {name}")
                    drift += 1
                    return m.group(0)
                actual = values[family][key]
                shown = m.group(5).strip()
                if shown.replace(",", "") == str(actual):
                    return m.group(0)
                drift += 1
                styled = f"{actual:,}" if "," in shown else str(actual)
                print(
                    f"{'rewrote' if write else 'drifted'}: {name} :: "
                    f"{family}.{key} shows {shown}, golden says {styled}"
                )
                if write:
                    return m.group(1) + styled + m.group(6)
                return m.group(0)

            updated = TAGGED.sub(check, content)
            if write and updated != content:
                open(path, "w").write(updated)
    print(f"{drift} golden-quoting number(s) drifted")
    return 1 if drift and not write else 0


if __name__ == "__main__":
    sys.exit(main())
