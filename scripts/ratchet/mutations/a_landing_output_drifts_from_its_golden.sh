#!/bin/sh
# A landing-page output panel shows a run the example no longer produces.
#
# The landing page titles its outputs `output`, and book_panels matched only
# titles that start with `kanso`, so these panels were copied by hand and
# drifted with nothing failing. The mutation changes the last digit of the
# factorial under dispatch.kso.
set -e
page=docs/index.html
line='<pre><code>20! = 2432902008176640000</code></pre>'
grep -qF "$line" "$page" || {
  echo "dispatch.kso's output panel moved; this mutation needs rewriting" >&2
  exit 1
}
sed -i 's|<pre><code>20! = 2432902008176640000</code></pre>|<pre><code>20! = 2432902008176640001</code></pre>|' "$page"
