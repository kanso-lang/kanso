#!/usr/bin/env python3
"""The log runs ahead of the page; this says how far is too far.

design/compiler-log.md is append-only chronology and docs/compiler.html is the
settled design presented to a reader. The rule has always been that the second
follows the first, and the rule was kept by memory — which failed exactly the
way memory fails: ten shipped optimisations landed in the log across one day,
each with its numbers, and the page went the whole day without an edit. Nobody
noticed because no single pull request was wrong.

So this is a budget rather than a mandate. An entry that records a diagnosis, a
revert or a handoff has no business on the page, and demanding one per commit
would teach people to write the sentence and mean nothing by it. Several
entries with no page edit at all is a different thing, and that is what fails
here.
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BUDGET = 3


def run(*args):
    return subprocess.run(args, capture_output=True, text=True, cwd=ROOT).stdout.strip()


def main():
    last_page = run("git", "log", "-1", "--format=%H", "--", "docs/compiler.html")
    if not last_page:
        print("no commit has touched docs/compiler.html — nothing to measure against")
        return 0
    added = run("git", "diff", f"{last_page}..HEAD", "--", "design/compiler-log.md")
    entries = [l[1:].strip() for l in added.splitlines() if l.startswith("+## ")]
    if len(entries) <= BUDGET:
        print(f"page drift {len(entries)}/{BUDGET} log entries since docs/compiler.html moved")
        return 0
    print(f"the log is {len(entries)} entries ahead of the page, and the budget is {BUDGET}:")
    for e in entries:
        print(f"  {e}")
    print()
    print("design/compiler-log.md is the chronology and docs/compiler.html is what")
    print("a reader is shown. Several entries with no page edit means the presented")
    print("design has fallen behind what the compiler actually does. Write the entry")
    print("the page owes — one for a campaign is fine, it need not be one per commit.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
