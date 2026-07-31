#!/usr/bin/env python3
"""Every diagnostic the compiler can raise, against the ones a golden pins.

A message with no golden is a message that can be reworded, weakened or lost
without anything going red. The error corpus is the only place a diagnostic's
exact text is checked, and it grew by whoever remembered — which is how
forty-one of them ended up unpinned at once.

So this is a ratchet rather than a report. The messages already unpinned are
listed in tests/golden/unpinned_diagnostics.txt and tolerated; a message that
is not on that list and has no golden fails. Adding a diagnostic means adding
a golden, and the list only shrinks.

Only literal text is checked. A message assembled entirely from runtime values
has nothing stable to match, and pretending otherwise would pin a format
string rather than a sentence.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
LISTED = ROOT / "tests/golden/unpinned_diagnostics.txt"

# The leading literal run of a diagnostic's message: everything before the
# first interpolation or escape, which is the part a golden can match on.
CALL = re.compile(r'Diagnostic::new\(\s*"([a-z]+)"\s*,\s*(.*?),\s*[a-z_]', re.S)
LITERAL = re.compile(r'"((?:[^"\\]|\\.){8,})"')


def raised():
    """(kind, leading literal) for every diagnostic the sources construct."""
    found = set()
    for path in sorted(ROOT.glob("src/*.rs")):
        text = path.read_text()
        for call in CALL.finditer(text):
            pieces = LITERAL.findall(call.group(2))
            if not pieces:
                continue
            head = pieces[0].split("{")[0].split("\\")[0].strip()
            if len(head) >= 12:
                found.add((call.group(1), head))
    return found


def pinned():
    return "\n".join(
        p.read_text(errors="ignore") for p in ROOT.glob("tests/golden/**/*.stderr")
    )


def listed():
    if not LISTED.exists():
        return set()
    out = set()
    for line in LISTED.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            out.add(line)
    return out


def main():
    corpus = pinned()
    tolerated = listed()
    missing, healed = [], []
    for kind, message in sorted(raised()):
        if message in corpus:
            if message in tolerated:
                healed.append((kind, message))
        elif message not in tolerated:
            missing.append((kind, message))
    print(
        f"{len(raised())} literal diagnostics, {len(tolerated)} listed as unpinned, "
        f"{len(missing)} newly unpinned, {len(healed)} now pinned"
    )
    for kind, message in missing:
        print(f"  no golden: [{kind}] {message[:88]}")
    for kind, message in healed:
        print(f"  now pinned, delete its line: [{kind}] {message[:70]}")
    if missing:
        print()
        print("a diagnostic ships with a golden. add one to tests/golden/errors,")
        print("or say why it cannot be reached and list it.")
    if healed:
        print()
        print("the list only shrinks: remove what a golden now covers.")
    return 1 if missing or healed else 0


if __name__ == "__main__":
    sys.exit(main())
