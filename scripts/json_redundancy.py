#!/usr/bin/env python3
"""How much repetition a json document actually holds.

Written to settle whether hash-consing pays before building it. Counts the
three kinds of repetition separately, because they call for different
techniques and only one of them turned out to exist: identical subtrees
(maximal sharing's target), object keys (string interning's), and string
values.
"""
import collections
import json
import pathlib
import sys


def main():
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "bench/large.json")
    raw = path.read_bytes()
    doc = json.loads(raw)

    keys, values, subtrees, sizes = (
        collections.Counter(),
        collections.Counter(),
        collections.Counter(),
        {},
    )

    def walk(node):
        if isinstance(node, (dict, list)):
            shape = json.dumps(node, sort_keys=True, separators=(",", ":"))
            subtrees[shape] += 1
            sizes[shape] = len(shape)
        if isinstance(node, dict):
            for key, child in node.items():
                keys[key] += 1
                walk(child)
        elif isinstance(node, list):
            for child in node:
                walk(child)
        elif isinstance(node, str):
            values[node] += 1

    walk(doc)

    def report(label, counter, measure):
        total, distinct = sum(counter.values()), len(counter)
        if not total:
            print(f"{label:16} none")
            return
        repeated = sum(measure(k) * (n - 1) for k, n in counter.items() if n > 1)
        print(
            f"{label:16} {total:6,} occurrences  {distinct:6,} distinct  "
            f"{100 * (1 - distinct / total):5.1f}% redundant  "
            f"{repeated:7,} bytes ({100 * repeated / len(raw):4.1f}% of file)"
        )

    print(f"{path} — {len(raw):,} bytes\n")
    report("subtrees", subtrees, lambda k: sizes[k])
    report("object keys", keys, len)
    report("string values", values, len)


if __name__ == "__main__":
    main()
