#!/usr/bin/env python3
"""Differential check of the utf-8 validator against an independent reference.

The validator's text is extracted from src/runtime.c at run time rather than
copied, so this cannot drift from the function it checks. The reference in
scripts/utf8/harness.c is a plain scalar decoder written straight from the
grammar, so agreement between them is evidence rather than a tautology.

Exhaustive over every string of three bytes or fewer, then sampled over the
lengths that straddle the sixteen-byte vector boundary.
"""
import pathlib
import re
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent.parent
SIGNATURE = "static KValue k_utf8_bad(const char* data, long long len, const char* origin) {"


def extracted(source):
    """The real validator, rewritten to return a bool the harness can compare."""
    start = source.index(SIGNATURE)
    depth = 0
    for j in range(source.index("{", start), len(source)):
        if source[j] == "{":
            depth += 1
        elif source[j] == "}":
            depth -= 1
            if depth == 0:
                end = j + 1
                break
    body = source[start:end]
    body = body.replace(SIGNATURE, "static int harness_utf8_ok(const char* data, long long len) {")
    body = re.sub(r"return k_err\([^;]*\);", "return 0;", body)
    body = body.replace("return k_none();", "return 1;")
    body = body.replace("k_stat_utf8_bytes += len;", "")
    header = (
        "#include <stdint.h>\n"
        "#if defined(__aarch64__)\n#include <arm_neon.h>\n"
        "#elif defined(__x86_64__)\n#include <tmmintrin.h>\n#endif\n"
    )
    return header + body + "\n"


def main():
    source = (ROOT / "src/runtime.c").read_text()
    with tempfile.TemporaryDirectory() as work:
        work = pathlib.Path(work)
        (work / "extracted.h").write_text(extracted(source))
        binary = work / "harness"
        build = subprocess.run(
            ["clang", "-O2", "-o", str(binary), str(ROOT / "scripts/utf8/harness.c"), f"-I{work}"],
            capture_output=True,
            text=True,
        )
        if build.returncode != 0:
            print(build.stderr, file=sys.stderr)
            return 1
        run = subprocess.run([str(binary)], capture_output=True, text=True)
        print(run.stdout.strip())
        if run.returncode != 0:
            print("FAIL  the validator and the reference disagree", file=sys.stderr)
            return 1
        print("PASS  the validator agrees with the reference everywhere checked")
        return 0


if __name__ == "__main__":
    sys.exit(main())
