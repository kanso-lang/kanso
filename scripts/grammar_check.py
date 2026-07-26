#!/usr/bin/env python3
"""What a reader sees when the editor grammar paints a kanso file.

The website's tokenizer and this grammar are two implementations of one
answer, and they drift silently: nothing fails when a rule stops matching,
the colour just quietly goes away, and the person who notices is whoever
opens a file weeks later.

So this asserts the painting rather than the rules — for each line, which
words come out as types, strings, or comments. A rule can be rewritten
however it likes as long as the reader's answer is unchanged.
"""
import json
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
GRAMMAR = ROOT / "editors/kanso/syntaxes/kanso.tmLanguage.json"

# line, then every word that must come out painted as a type
TYPED = [
    ("type person", ["person"]),
    ("  name:string", ["string"]),
    ("  partner:person", ["person"]),
    ("  peers:person[]", ["person[]"]),
    ("fn shout s:string", ["string"]),
    ("type post_body:string", ["post_body", "string"]),
]


def scanner(grammar):
    order = [p["include"].lstrip("#") for p in grammar["patterns"] if "include" in p]
    rules = []
    for name in order:
        rule = grammar["repository"][name]
        if "match" in rule:
            try:
                rules.append((re.compile(rule["match"]), rule))
            except re.error:
                continue  # a rule this engine cannot compile is not this check's business
    return rules


def painted(line, rules):
    """Every (text, scope) the first-match-wins scan produces."""
    out, pos = [], 0
    while pos < len(line):
        for rx, rule in rules:
            m = rx.search(line, pos)
            if m and m.start() == pos:
                if rule.get("name"):
                    out.append((m.group(0), rule["name"]))
                for idx, cap in rule.get("captures", {}).items():
                    if m.group(int(idx)):
                        out.append((m.group(int(idx)), cap["name"]))
                pos = max(m.end(), pos + 1)
                break
        else:
            pos += 1
    return out


def main():
    rules = scanner(json.loads(GRAMMAR.read_text()))
    failures = []
    for line, expected in TYPED:
        types = [t for t, scope in painted(line, rules) if "entity.name.type" in scope]
        for want in expected:
            if want not in types:
                failures.append(f"{line!r}: {want!r} is not painted as a type (types: {types})")
    for line in failures:
        print(f"FAIL  {line}")
    if not failures:
        print(f"PASS  every type in {len(TYPED)} lines is painted as one")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
