# A second oracle for `regex --table`: Python's `re`, with re.ASCII so that
# \d \w \s and \b mean what they mean in RE2. Python spells some syntax
# differently and refuses some RE2 syntax outright (\z, (?U), [[:alpha:]]),
# so its errors are not comparable with Go's or the port's; this prints
# "python error" for them. Offsets are code points, as in the port.
#
#   python3 -I oracle/table.py < tests/table_basic.tsv
import re
import sys

for line in sys.stdin.read().split("\n"):
    if line == "" or line.startswith("#"):
        continue
    pattern, _, subject = line.partition("\t")
    subject = subject.replace("\\n", "\n")
    try:
        rx = re.compile(pattern, re.ASCII)
    except re.error:
        print(f"{line}\tpython error")
        continue
    found = []
    for m in rx.finditer(subject):
        slots = []
        for g in range(rx.groups + 1):
            s, e = m.span(g)
            slots.append("- -" if s < 0 else f"{s} {e}")
        found.append("(" + " ".join(slots) + ")")
    print(f"{line}\t{' '.join(found) if found else 'no match'}")
