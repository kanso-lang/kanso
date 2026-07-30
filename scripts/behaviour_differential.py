#!/usr/bin/env python3
"""Differential check of what the library ANSWERS, not what it complains about.

`diagnostic_differential.py` asks std functions for the wrong thing and
compares the complaints. This asks them for the awkward thing and compares the
answers: a slice that starts past the end, a sort of things that compare equal,
a take of more than there is, a map read that misses, a round that lands on a
half. Those are the places where two implementations of the same function
drift, because they are the places the obvious implementation does not say.

Every probe here is a legal call. If the engines answer differently, one of
them is wrong about the library's contract — and if the contract does not say,
that is worth knowing too.
"""
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
KANSO = ROOT / "target/release/kanso"

# (module needed, expression). Each is printed and compared.
PROBES = [
    # text/slice: both bounds, past either end, inverted, unicode
    ("text", 'text/slice "abcdef" 1 3'),
    ("text", 'text/slice "abcdef" 4 6'),
    ("text", 'text/slice "abcdef" 1 99'),
    ("text", 'text/slice "abcdef" 0 3'),
    ("text", 'text/slice "abcdef" 5 2'),
    ("text", 'text/slice "abcdef" 7 9'),
    ("text", 'text/slice "" 1 1'),
    ("text", 'text/slice "héllo" 1 2'),
    ("text", 'text/slice "日本語" 1 2'),
    ("text", 'text/slice "emoji 🌱 here" 7 7'),
    # text: joining and splitting at the edges
    ("text", 'text/join [] "-"'),
    ("text", 'text/join ["a"] "-"'),
    ("text", 'text/join ["" ""] "-"'),
    ("text", 'text/split "" ""'),
    ("text", 'text/split "aaa" "aa"'),
    ("text", 'text/concat "" ""'),
    ("text", 'text/chars ""'),
    ("text", 'text/chars "日本語"'),
    ("text", 'text/bytes "é"'),
    ("text", 'text/to_int "007"'),
    ("text", 'text/to_int "-0"'),
    ("text", 'text/to_float "1"'),
    ("text", 'text/char_code "a"'),
    ("text", 'text/from_code 10'),
    # list: empty, one, past the end, equal keys
    ("list", "list/to_list (list/take [1 2 3] 0)"),
    ("list", "list/to_list (list/take [1 2 3] 99)"),
    ("list", "list/to_list (list/drop [1 2 3] 0)"),
    ("list", "list/to_list (list/drop [1 2 3] 99)"),
    ("list", "list/to_list (list/sort [])"),
    ("list", "list/to_list (list/sort [3 1 2])"),
    ("list", 'list/to_list (list/sort ["b" "a"])'),
    ("list", "list/to_list (list/sort [1 1 1])"),
    ("list", "list/first []"),
    ("list", "list/last []"),
    ("list", "list/first [1 2]"),
    ("list", "list/last [1 2]"),
    ("list", "list/max []"),
    ("list", "list/min []"),
    ("list", "list/sum []"),
    ("list", "list/mean []"),
    ("list", "list/mean [1 2]"),
    ("list", "list/count [] (x -> true)"),
    ("list", "list/to_list (list/zip [1 2] [3])"),
    ("list", "list/to_list (list/zip [] [1])"),
    ("list", "list/tally []"),
    ("list", 'list/tally ["a" "a" "b"]'),
    # `repeat` and `iterate` are infinite by design, so they are taken from
    # rather than forced — forcing one is not an awkward call, it is a
    # different program that never ends
    ("list", "list/to_list (list/take (list/repeat 5) 3)"),
    ("list", "list/to_list (list/take (list/repeat 5) 0)"),
    # map: reads that miss, and the order entries come back in
    ("list", 'list/to_h [["b" 2] ["a" 1]]'),
    ("list", 'list/transform_values { "a":1 } (v -> v + 1)'),
    # math at the awkward values
    ("math", "math/round 0.5"),
    ("math", "math/round 1.5"),
    ("math", "math/round -0.5"),
    ("math", "math/round -1.5"),
    ("math", "math/round 2.5"),
    ("math", "math/sqrt 0"),
    ("math", "math/sqrt 2"),
    ("math", "math/sqrt 0.25"),
    # indexing, the non-strict form that answers none
    ("list", "[1 2 3][0]"),
    ("list", "[1 2 3][4]"),
    ("list", '{ "a":1 }["b"]'),
    ("list", '"abc"[2]'),
]


def run(source, engine):
    with tempfile.TemporaryDirectory() as work:
        entry = pathlib.Path(work) / "probe.kso"
        entry.write_text(source)
        try:
            done = subprocess.run(
                [str(KANSO), "play", "probe.kso", *engine],
                cwd=work,
                capture_output=True,
                text=True,
                timeout=20,
            )
        except subprocess.TimeoutExpired:
            # a legal call that never returns is a finding in its own right,
            # and a louder one than a wording difference
            return "TIMED OUT", "", ""
    return done.returncode, done.stdout, done.stderr


def main():
    if not KANSO.exists():
        sys.exit("build the toolchain first: cargo build --release")
    disagreed, hung = [], []
    for module, expr in PROBES:
        print(f"  probe: {expr}", flush=True)
        source = f'import "std/{module}"\n\npub play = print "{{{expr}}}"\n'
        native = run(source, [])
        interp = run(source, ["--interp"])
        # Two timeouts compare equal, and calling that agreement would hide
        # the worst answer a legal call can give. A call that does not return
        # is a finding on whichever engine it happens.
        if native[0] == "TIMED OUT" or interp[0] == "TIMED OUT":
            hung.append((expr, native, interp))
        elif native != interp:
            disagreed.append((expr, native, interp))
    print(
        f"{len(PROBES)} awkward calls, {len(disagreed)} disagree, {len(hung)} never returned"
    )
    for expr, native, interp in hung[:10]:
        print(f"  never returned: {expr}")
        print(f"    native={native[0]!r} interp={interp[0]!r}")
    for expr, native, interp in disagreed[:20]:
        print(f"  {expr}")
        print(f"    native: {native[1]!r} {native[2].strip()[:90]!r}")
        print(f"    interp: {interp[1]!r} {interp[2].strip()[:90]!r}")
    if disagreed or hung:
        print()
        print("the interpreter is the oracle: these are native's to match.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
