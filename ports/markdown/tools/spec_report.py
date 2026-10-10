"""Run a renderer over the CommonMark spec examples and report by section.

usage: python3 tools/spec_report.py <renderer command...>
The command reads markdown on stdin and writes HTML on stdout. Section
names come from fixtures/spec/sections.txt.
"""
import os
import subprocess
import sys

here = os.path.dirname(os.path.abspath(__file__))
spec = os.path.join(here, '..', 'fixtures', 'spec')
cmd = sys.argv[1:]
sections = {}
for line in open(os.path.join(spec, 'sections.txt')):
    num, name = line.rstrip('\n').split(' ', 1)
    sections[int(num)] = name

totals = {}
failed = []
for num in sorted(sections):
    stem = os.path.join(spec, '%04d' % num)
    md = open(stem + '.md', 'rb').read()
    want = open(stem + '.html', 'rb').read()
    got = subprocess.run(cmd, input=md, capture_output=True).stdout
    name = sections[num]
    ok, total = totals.get(name, (0, 0))
    totals[name] = (ok + (got == want), total + 1)
    if got != want:
        failed.append(num)

passed = sum(ok for ok, _ in totals.values())
for name, (ok, total) in totals.items():
    print('%-40s %3d/%3d' % (name, ok, total))
print('passed %d of %d' % (passed, len(sections)))
print('failed:', ' '.join(str(n) for n in failed))
