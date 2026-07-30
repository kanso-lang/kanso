#!/usr/bin/env python3
"""The mechanical half of the writing rules, run over every page we publish.

CLAUDE.md names the families and gives one regex. That regex catches
`is not X. it's Y` and misses `is not X; it is Y`, `not X—it is Y` and
`not a X, but Y` — which is where all thirteen tells cut from the book on
2026-07-30 were living. A rule nothing enforces is a rule that drifts back.

Three families are mechanically detectable and are checked here. The rest —
the milked metaphor, manufactured rhythm, the epigram bolted to the end of a
paragraph — are not, and the rules say so: a clean run is necessary and never
sufficient. Read the draft aloud.

Code samples are excluded. A `<pre>` block is what a program said, and a
program is allowed to say anything.
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent

# Every shape the flip takes. The point of the family is the same each time:
# a claim stated by denying its opposite first, which no person writing
# plainly does twice in a page.
FAMILIES = {
    # The span may not carry a clause-joining conjunction: without that, the
    # first pattern reaches across a sentence boundary and calls "does not
    # compile, and either way she stays asleep. it is said" a flip.
    "antithesis flip": [
        r"\b(is|are|was|were|does|do)\s+not\s+(?:(?!\band\b|\bor\b)[^.;<]){3,70}\."
        r"\s*(it|they|that)('s|\s+(is|are|does))\b",
        r"\b(is|are|was|were)\s+not\s+(?:(?!\band\b|\bor\b)[^.;<]){3,70}[;,]"
        r"\s+(it|they|that)\s+(is|are)\b",
        r"\bnot\s+(?:(?!\band\b|\bor\b)[^.;<]){3,60}—\s*(it|they|that)\s+(is|are|does)\b",
        r"\bnot\s+(a|an|the)\s+(?:(?!\band\b|\bor\b)[^.;<]){3,50},\s+but\b",
        r"\bdoesn't\s+(?:(?!\band\b|\bor\b)[^.;<]){3,60}\.\s*it\s+\w+s\b",
    ],
    "self-announcing": [
        r"\bthe interesting (part|thing|question) (is|here)\b",
        r"\bhere's the thing\b",
        r"\bworth slowing down\b",
        r"\btake this slowly\b",
        r"\bthe trick under the trick\b",
        r"\bwhat follows is\b",
    ],
    "throat-clearing": [
        r"<p>\s*(it is|it's) worth noting\b",
        r"<p>\s*let's be honest\b",
        r"<p>\s*the interesting\b",
        r"<p>\s*of course,\b",
    ],
}

PAGES = ["docs/*.html", "docs/book/*.html"]


def prose(path):
    """The page with its code stripped, so a sample is never a finding."""
    text = path.read_text()
    text = re.sub(r"<pre.*?</pre>", " ", text, flags=re.S)
    return re.sub(r"<code>.*?</code>", " ", text, flags=re.S)


def main():
    findings = []
    pages = sorted({p for pattern in PAGES for p in ROOT.glob(pattern)})
    for path in pages:
        text = prose(path)
        for family, patterns in FAMILIES.items():
            for pattern in patterns:
                for hit in re.finditer(pattern, text, re.I):
                    line = text[: hit.start()].count("\n") + 1
                    quote = re.sub(r"\s+", " ", hit.group(0)).strip()
                    findings.append((path.relative_to(ROOT), line, family, quote))
    print(f"{len(pages)} pages read, {len(findings)} machine-detectable tells")
    for where, line, family, quote in findings:
        print(f"  {where}:{line}  {family}")
        print(f"    {quote[:150]}")
    if findings:
        print()
        print("say it once, in the positive. the families this cannot see —")
        print("milked metaphors, manufactured triples, epigram endings — still")
        print("want a read.")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
