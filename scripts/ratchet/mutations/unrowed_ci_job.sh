#!/bin/sh
# A new gating job that nobody said how to prove.
set -e
cat >> .github/workflows/ci.yml <<'YAML'

  unrowed:
    name: a job nobody said how to prove
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: nothing at all
        run: true
YAML
grep -q 'a job nobody said how to prove' .github/workflows/ci.yml
