#!/bin/sh
# The cost-goldens workflow's steps go back to the shell a step gets by
# default, which starts whatever it runs with no limit on what it may
# allocate (kanso#2441).
set -e
grep -q '^    shell: bash -c "if \[ -f \$GITHUB_WORKSPACE/scripts/bounded.sh \]' .github/workflows/ci.yml || {
  echo "ci.yml's default shell moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i '/^    shell: bash -c "if \[ -f \$GITHUB_WORKSPACE\/scripts\/bounded.sh \]/s/.*/    shell: bash -e {0}/' .github/workflows/ci.yml
! grep -q 'scripts/bounded.sh ]; then' .github/workflows/ci.yml
grep -q '^    shell: bash -e {0}$' .github/workflows/ci.yml
